//! Authentication service implementation.

use crate::dto::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use arcana_config::SecurityConfig;
use arcana_core::{ArcanaError, ArcanaResult, Interface, UserId, ValidateExt};
use arcana_domain::{Email, User, UserStatus};
use arcana_repository::UserRepository;
use arcana_security::{Claims, PasswordHasher, TokenProvider};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Authentication service trait.
#[async_trait]
pub trait AuthService: Interface + Send + Sync {
    /// Registers a new user.
    async fn register(&self, request: RegisterRequest) -> ArcanaResult<AuthResponse>;

    /// Logs in a user.
    async fn login(&self, request: LoginRequest) -> ArcanaResult<AuthResponse>;

    /// Refreshes an access token.
    async fn refresh_token(&self, request: RefreshTokenRequest) -> ArcanaResult<AuthResponse>;

    /// Validates an access token and returns claims.
    async fn validate_token(&self, token: &str) -> ArcanaResult<Claims>;

    /// Logs out a user (invalidates refresh token).
    async fn logout(&self, user_id: UserId) -> ArcanaResult<MessageResponse>;

    /// Gets the current user from claims.
    async fn get_current_user(&self, claims: &Claims) -> ArcanaResult<AuthUserInfo>;
}

/// Authentication service implementation.
pub struct AuthServiceImpl<R: UserRepository> {
    user_repository: Arc<R>,
    password_hasher: Arc<PasswordHasher>,
    token_provider: Arc<TokenProvider>,
}

impl<R: UserRepository> AuthServiceImpl<R> {
    /// Creates a new authentication service.
    pub fn new(
        user_repository: Arc<R>,
        password_hasher: Arc<PasswordHasher>,
        security_config: Arc<SecurityConfig>,
    ) -> Self {
        let token_provider = Arc::new(TokenProvider::new(security_config));
        Self {
            user_repository,
            password_hasher,
            token_provider,
        }
    }

    /// Creates an auth response for a user.
    fn create_auth_response(&self, user: &User) -> ArcanaResult<AuthResponse> {
        let tokens = self.token_provider.generate_tokens(
            user.id,
            &user.username,
            user.email.as_str(),
            user.role,
        )?;

        Ok(AuthResponse {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            token_type: tokens.token_type,
            expires_in: tokens.access_expires_at - chrono::Utc::now().timestamp(),
            user: AuthUserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.to_string(),
                role: user.role,
                first_name: user.first_name.clone(),
                last_name: user.last_name.clone(),
            },
        })
    }
}

#[async_trait]
impl<R: UserRepository + 'static> AuthService for AuthServiceImpl<R> {
    async fn register(&self, request: RegisterRequest) -> ArcanaResult<AuthResponse> {
        debug!("Registering user: {}", request.username);

        // Validate request
        request.validate_request()?;

        // Check for existing username
        if self.user_repository.exists_by_username(&request.username).await? {
            return Err(ArcanaError::Conflict(format!(
                "Username '{}' already exists",
                request.username
            )));
        }

        // Check for existing email
        if self.user_repository.exists_by_email(&request.email).await? {
            return Err(ArcanaError::Conflict(format!(
                "Email '{}' already exists",
                request.email
            )));
        }

        // Parse email
        let email = Email::new(&request.email)
            .map_err(|e| ArcanaError::Validation(e.to_string()))?;

        // Hash password
        let password_hash = self.password_hasher.hash(&request.password)?;

        // Create user
        let mut user = User::new(
            request.username,
            email,
            password_hash,
            request.first_name,
            request.last_name,
        );

        // For now, auto-activate users (in production, you'd send verification email)
        user.activate();

        // Save user
        let saved_user = self.user_repository.save(&user).await?;

        info!("User registered: {}", saved_user.id);

        // Generate tokens
        self.create_auth_response(&saved_user)
    }

    async fn login(&self, request: LoginRequest) -> ArcanaResult<AuthResponse> {
        debug!("Login attempt for: {}", request.username_or_email);

        // Validate request
        request.validate_request()?;

        // Find user
        let user = self
            .user_repository
            .find_by_username_or_email(&request.username_or_email)
            .await?
            .ok_or_else(|| {
                warn!("Login failed: user not found - {}", request.username_or_email);
                ArcanaError::InvalidCredentials
            })?;

        // Check user status
        if !user.status.can_login() {
            warn!("Login failed: user status {:?} - {}", user.status, user.id);
            return Err(match user.status {
                UserStatus::Suspended => ArcanaError::Forbidden("Account is suspended".to_string()),
                UserStatus::Locked => ArcanaError::Forbidden("Account is locked".to_string()),
                UserStatus::Deleted => ArcanaError::InvalidCredentials,
                _ => ArcanaError::Forbidden("Account is not active".to_string()),
            });
        }

        // Verify password
        if !self.password_hasher.verify(&request.password, &user.password_hash)? {
            warn!("Login failed: invalid password - {}", user.id);
            return Err(ArcanaError::InvalidCredentials);
        }

        // Update last login
        let mut updated_user = user.clone();
        updated_user.record_login();
        let _ = self.user_repository.update(&updated_user).await;

        info!("User logged in: {}", user.id);

        // Generate tokens
        self.create_auth_response(&user)
    }

    async fn refresh_token(&self, request: RefreshTokenRequest) -> ArcanaResult<AuthResponse> {
        debug!("Refreshing token");

        // Validate refresh token
        let claims = self.token_provider.validate_refresh_token(&request.refresh_token)?;

        // Get user to ensure they still exist and are active
        let user_id = claims.user_id().ok_or_else(|| {
            ArcanaError::InvalidToken("Invalid refresh token: missing user ID".to_string())
        })?;

        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ArcanaError::InvalidToken("User no longer exists".to_string()))?;

        if !user.status.can_login() {
            return Err(ArcanaError::Forbidden("Account is not active".to_string()));
        }

        info!("Token refreshed for user: {}", user.id);

        // Generate new tokens
        self.create_auth_response(&user)
    }

    async fn validate_token(&self, token: &str) -> ArcanaResult<Claims> {
        self.token_provider.validate_access_token(token)
    }

    async fn logout(&self, user_id: UserId) -> ArcanaResult<MessageResponse> {
        debug!("Logging out user: {}", user_id);

        // In a full implementation, you would invalidate refresh tokens in the database
        // For now, we just return success (tokens will expire naturally)

        info!("User logged out: {}", user_id);
        Ok(MessageResponse::new("Successfully logged out"))
    }

    async fn get_current_user(&self, claims: &Claims) -> ArcanaResult<AuthUserInfo> {
        let user_id = claims.user_id().ok_or_else(|| {
            ArcanaError::InvalidToken("Invalid token: missing user ID".to_string())
        })?;

        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", user_id))?;

        Ok(AuthUserInfo {
            id: user.id,
            username: user.username,
            email: user.email.to_string(),
            role: user.role,
            first_name: user.first_name,
            last_name: user.last_name,
        })
    }
}

impl<R: UserRepository> std::fmt::Debug for AuthServiceImpl<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthServiceImpl").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::Page;
    use arcana_domain::{Email, User, UserRole};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock user repository for testing.
    struct MockUserRepository {
        users: Mutex<HashMap<UserId, User>>,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        fn with_user(user: User) -> Self {
            let repo = Self::new();
            repo.users.lock().unwrap().insert(user.id, user);
            repo
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().get(&id).cloned())
        }

        async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.username == username)
                .cloned())
        }

        async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.email.as_str() == email)
                .cloned())
        }

        async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.username == identifier || u.email.as_str() == identifier)
                .cloned())
        }

        async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values()
                .any(|u| u.username == username))
        }

        async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values()
                .any(|u| u.email.as_str().to_lowercase() == email.to_lowercase()))
        }

        async fn find_all(&self, page: arcana_core::PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
            let total = users.len() as u64;
            Ok(Page::new(users, page.page, page.size, total))
        }

        async fn find_by_role(&self, role: UserRole, page: arcana_core::PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values()
                .filter(|u| u.role == role)
                .cloned()
                .collect();
            let total = users.len() as u64;
            Ok(Page::new(users, page.page, page.size, total))
        }

        async fn save(&self, user: &User) -> ArcanaResult<User> {
            let mut users = self.users.lock().unwrap();
            users.insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn update(&self, user: &User) -> ArcanaResult<User> {
            let mut users = self.users.lock().unwrap();
            users.insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
            let mut users = self.users.lock().unwrap();
            Ok(users.remove(&id).is_some())
        }

        async fn count(&self) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().len() as u64)
        }

        async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().values()
                .filter(|u| u.role == role)
                .count() as u64)
        }
    }

    fn create_test_config() -> Arc<SecurityConfig> {
        Arc::new(SecurityConfig {
            jwt_secret: "test-secret-key-for-testing-only".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test-issuer".to_string(),
            jwt_audience: "test-audience".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        })
    }

    fn create_active_user_with_password(password: &str) -> User {
        let hasher = PasswordHasher::new();
        let mut user = User::new(
            "testuser".to_string(),
            Email::new_unchecked("test@example.com".to_string()),
            hasher.hash(password).unwrap(),
            Some("Test".to_string()),
            Some("User".to_string()),
        );
        user.activate();
        user
    }

    fn create_auth_service(repo: MockUserRepository) -> AuthServiceImpl<MockUserRepository> {
        AuthServiceImpl::new(
            Arc::new(repo),
            Arc::new(PasswordHasher::new()),
            create_test_config(),
        )
    }

    #[tokio::test]
    async fn test_register_success() {
        let repo = MockUserRepository::new();
        let service = create_auth_service(repo);

        let request = RegisterRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
        };

        let result = service.register(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.access_token.is_empty());
        assert!(!response.refresh_token.is_empty());
        assert_eq!(response.user.username, "newuser");
    }

    #[tokio::test]
    async fn test_register_duplicate_username() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = RegisterRequest {
            username: "testuser".to_string(),
            email: "other@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.register(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Conflict(msg) => assert!(msg.contains("Username")),
            _ => panic!("Expected Conflict error"),
        }
    }

    #[tokio::test]
    async fn test_register_duplicate_email() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = RegisterRequest {
            username: "otheruser".to_string(),
            email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.register(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Conflict(msg) => assert!(msg.contains("Email")),
            _ => panic!("Expected Conflict error"),
        }
    }

    #[tokio::test]
    async fn test_login_success() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.access_token.is_empty());
        assert_eq!(response.user.username, "testuser");
    }

    #[tokio::test]
    async fn test_login_with_email() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "WrongPassword".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::InvalidCredentials => {}
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        let repo = MockUserRepository::new();
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "nonexistent".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::InvalidCredentials => {}
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[tokio::test]
    async fn test_login_suspended_user() {
        let mut user = create_active_user_with_password("Password123");
        user.status = UserStatus::Suspended;
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Forbidden(msg) => assert!(msg.contains("suspended")),
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[tokio::test]
    async fn test_login_locked_user() {
        let mut user = create_active_user_with_password("Password123");
        user.status = UserStatus::Locked;
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };

        let result = service.login(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Forbidden(msg) => assert!(msg.contains("locked")),
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[tokio::test]
    async fn test_refresh_token_success() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user.clone());
        let service = create_auth_service(repo);

        // First login to get tokens
        let login_request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };
        let login_response = service.login(login_request).await.unwrap();

        // Now refresh
        let refresh_request = RefreshTokenRequest {
            refresh_token: login_response.refresh_token,
        };
        let result = service.refresh_token(refresh_request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_invalid() {
        let repo = MockUserRepository::new();
        let service = create_auth_service(repo);

        let request = RefreshTokenRequest {
            refresh_token: "invalid-token".to_string(),
        };

        let result = service.refresh_token(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_success() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let login_request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };
        let login_response = service.login(login_request).await.unwrap();

        let result = service.validate_token(&login_response.access_token).await;
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.username, "testuser");
    }

    #[tokio::test]
    async fn test_validate_token_invalid() {
        let repo = MockUserRepository::new();
        let service = create_auth_service(repo);

        let result = service.validate_token("invalid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout() {
        let user = create_active_user_with_password("Password123");
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let result = service.logout(user_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().message.contains("logged out"));
    }

    #[tokio::test]
    async fn test_get_current_user_success() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user);
        let service = create_auth_service(repo);

        let login_request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        };
        let login_response = service.login(login_request).await.unwrap();
        let claims = service.validate_token(&login_response.access_token).await.unwrap();

        let result = service.get_current_user(&claims).await;
        assert!(result.is_ok());
        let user_info = result.unwrap();
        assert_eq!(user_info.username, "testuser");
        assert_eq!(user_info.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_create_auth_response() {
        let user = create_active_user_with_password("Password123");
        let repo = MockUserRepository::with_user(user.clone());
        let service = create_auth_service(repo);

        let response = service.create_auth_response(&user);
        assert!(response.is_ok());
        let auth = response.unwrap();
        assert!(!auth.access_token.is_empty());
        assert!(!auth.refresh_token.is_empty());
        assert_eq!(auth.token_type, "Bearer");
        assert!(auth.expires_in > 0);
    }
}
