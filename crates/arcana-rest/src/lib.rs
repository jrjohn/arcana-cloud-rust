//! # Arcana REST
//!
//! REST API layer using Axum for Arcana Cloud Rust.
//! Provides HTTP endpoints for user management, authentication, and health checks.

pub mod controllers;
pub mod extractors;
pub mod middleware;
pub mod openapi;
pub mod responses;
pub mod router;
pub mod state;

pub use openapi::*;
pub use router::*;
pub use state::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controllers::{auth_controller, health_controller, user_controller},
        middleware::{auth_middleware, AuthMiddlewareState},
    };
    use arcana_config::SecurityConfig;
    use arcana_core::{ArcanaError, ArcanaResult, Page, PageRequest, UserId};
    use arcana_core::{Email, User, UserRole};
    use arcana_repository::UserRepository;
    use arcana_security::{Claims, TokenProvider, TokenProviderInterface};
    use arcana_service::{
        AuthResponse, AuthService, AuthUserInfo, ChangePasswordRequest, CreateUserRequest,
        LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest, UpdateUserRequest,
        UpdateUserRoleRequest, UpdateUserStatusRequest, UserListResponse, UserResponse, UserService,
    };
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{header, Method, Request, StatusCode},
        middleware as axum_middleware,
        Router,
    };
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    // =============================================================================
    // Test Fixtures and Mocks
    // =============================================================================

    fn create_test_security_config() -> Arc<SecurityConfig> {
        Arc::new(SecurityConfig {
            jwt_secret: "test-secret-key-for-testing-minimum-32-chars".to_string(),
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


    fn create_test_user() -> User {
        let mut user = User::new(
            "testuser".to_string(),
            Email::new_unchecked("test@example.com".to_string()),
            "hashed_password".to_string(),
            Some("Test".to_string()),
            Some("User".to_string()),
        );
        user.activate();
        user
    }

    fn create_admin_user() -> User {
        let mut user = User::new(
            "admin".to_string(),
            Email::new_unchecked("admin@example.com".to_string()),
            "hashed_password".to_string(),
            Some("Admin".to_string()),
            Some("User".to_string()),
        );
        user.activate();
        user.change_role(UserRole::Admin);
        user
    }

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

        fn with_users(users: Vec<User>) -> Self {
            let repo = Self::new();
            for user in users {
                repo.users.lock().unwrap().insert(user.id, user);
            }
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
            Ok(self.users.lock().unwrap().values().any(|u| u.username == username))
        }

        async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values().any(|u| u.email.as_str().to_lowercase() == email.to_lowercase()))
        }

        async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
            let total = users.len() as u64;
            let start = page.offset();
            let end = std::cmp::min(start + page.limit(), users.len());
            let items = if start < users.len() { users[start..end].to_vec() } else { vec![] };
            Ok(Page::new(items, page.page, page.size, total))
        }

        async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values()
                .filter(|u| u.role == role).cloned().collect();
            let total = users.len() as u64;
            Ok(Page::new(users, page.page, page.size, total))
        }

        async fn save(&self, user: &User) -> ArcanaResult<User> {
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn update(&self, user: &User) -> ArcanaResult<User> {
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().remove(&id).is_some())
        }

        async fn count(&self) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().len() as u64)
        }

        async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().values().filter(|u| u.role == role).count() as u64)
        }
    }

    /// Mock user service for controller tests.
    struct MockUserService {
        users: Arc<Mutex<HashMap<UserId, User>>>,
    }

    impl MockUserService {
        fn new() -> Self {
            Self {
                users: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn with_users(users: Vec<User>) -> Self {
            let service = Self::new();
            for user in users {
                service.users.lock().unwrap().insert(user.id, user);
            }
            service
        }
    }


    #[async_trait]
    impl UserService for MockUserService {
        async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse> {
            let user = User::new(
                request.username,
                Email::new_unchecked(request.email),
                "hashed".to_string(),
                request.first_name,
                request.last_name,
            );
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(UserResponse::from(user))
        }

        async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse> {
            self.users.lock().unwrap().get(&id)
                .map(|u| UserResponse::from(u.clone()))
                .ok_or_else(|| ArcanaError::not_found("User", id))
        }

        async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse> {
            self.users.lock().unwrap().values()
                .find(|u| u.username == username)
                .map(|u| UserResponse::from(u.clone()))
                .ok_or_else(|| ArcanaError::not_found("User", username))
        }

        async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse> {
            let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
            let total = users.len() as u64;
            Ok(UserListResponse::from(Page::new(users, page.page, page.size, total)))
        }

        async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse> {
            let mut users = self.users.lock().unwrap();
            let user = users.get_mut(&id).ok_or_else(|| ArcanaError::not_found("User", id))?;
            user.update_profile(request.first_name, request.last_name, request.avatar_url);
            Ok(UserResponse::from(user.clone()))
        }

        async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse> {
            let mut users = self.users.lock().unwrap();
            let user = users.get_mut(&id).ok_or_else(|| ArcanaError::not_found("User", id))?;
            user.change_role(request.role);
            Ok(UserResponse::from(user.clone()))
        }

        async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse> {
            let mut users = self.users.lock().unwrap();
            let user = users.get_mut(&id).ok_or_else(|| ArcanaError::not_found("User", id))?;
            user.status = request.status;
            Ok(UserResponse::from(user.clone()))
        }

        async fn change_password(&self, _id: UserId, _request: ChangePasswordRequest) -> ArcanaResult<()> {
            Ok(())
        }

        async fn delete_user(&self, id: UserId) -> ArcanaResult<()> {
            self.users.lock().unwrap().remove(&id)
                .map(|_| ())
                .ok_or_else(|| ArcanaError::not_found("User", id))
        }

        async fn username_exists(&self, username: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values().any(|u| u.username == username))
        }

        async fn email_exists(&self, email: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values().any(|u| u.email.as_str() == email))
        }
    }

    /// Mock auth service for controller tests.
    struct MockAuthService {
        token_provider: Arc<TokenProvider>,
        users: Arc<Mutex<HashMap<UserId, User>>>,
    }

    impl MockAuthService {
        fn new(config: Arc<SecurityConfig>) -> Self {
            Self {
                token_provider: Arc::new(TokenProvider::new(config)),
                users: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn with_user(config: Arc<SecurityConfig>, user: User) -> Self {
            let service = Self::new(config);
            service.users.lock().unwrap().insert(user.id, user);
            service
        }
    }


    #[async_trait]
    impl AuthService for MockAuthService {
        async fn register(&self, request: RegisterRequest) -> ArcanaResult<AuthResponse> {
            let mut user = User::new(
                request.username,
                Email::new_unchecked(request.email),
                "hashed".to_string(),
                request.first_name,
                request.last_name,
            );
            user.activate();
            self.users.lock().unwrap().insert(user.id, user.clone());

            let tokens = self.token_provider.generate_tokens(
                user.id, &user.username, user.email.as_str(), user.role,
            )?;

            Ok(AuthResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_type: tokens.token_type,
                expires_in: tokens.access_expires_at - chrono::Utc::now().timestamp(),
                user: AuthUserInfo {
                    id: user.id,
                    username: user.username,
                    email: user.email.to_string(),
                    role: user.role,
                    first_name: user.first_name,
                    last_name: user.last_name,
                },
            })
        }

        async fn login(&self, request: LoginRequest) -> ArcanaResult<AuthResponse> {
            let user = self.users.lock().unwrap().values()
                .find(|u| u.username == request.username_or_email || u.email.as_str() == request.username_or_email)
                .cloned()
                .ok_or(ArcanaError::InvalidCredentials)?;

            let tokens = self.token_provider.generate_tokens(
                user.id, &user.username, user.email.as_str(), user.role,
            )?;

            Ok(AuthResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_type: tokens.token_type,
                expires_in: tokens.access_expires_at - chrono::Utc::now().timestamp(),
                user: AuthUserInfo {
                    id: user.id,
                    username: user.username,
                    email: user.email.to_string(),
                    role: user.role,
                    first_name: user.first_name,
                    last_name: user.last_name,
                },
            })
        }

        async fn refresh_token(&self, request: RefreshTokenRequest) -> ArcanaResult<AuthResponse> {
            let claims = self.token_provider.validate_refresh_token(&request.refresh_token)?;
            let user_id = claims.user_id().ok_or(ArcanaError::InvalidToken("No user ID".to_string()))?;
            let user = self.users.lock().unwrap().get(&user_id).cloned()
                .ok_or(ArcanaError::InvalidToken("User not found".to_string()))?;

            let tokens = self.token_provider.generate_tokens(
                user.id, &user.username, user.email.as_str(), user.role,
            )?;

            Ok(AuthResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_type: tokens.token_type,
                expires_in: tokens.access_expires_at - chrono::Utc::now().timestamp(),
                user: AuthUserInfo {
                    id: user.id,
                    username: user.username,
                    email: user.email.to_string(),
                    role: user.role,
                    first_name: user.first_name,
                    last_name: user.last_name,
                },
            })
        }

        async fn validate_token(&self, token: &str) -> ArcanaResult<Claims> {
            self.token_provider.validate_access_token(token)
        }

        async fn logout(&self, _user_id: UserId) -> ArcanaResult<MessageResponse> {
            Ok(MessageResponse::new("Successfully logged out"))
        }

        async fn get_current_user(&self, claims: &Claims) -> ArcanaResult<AuthUserInfo> {
            let user_id = claims.user_id().ok_or(ArcanaError::InvalidToken("No user ID".to_string()))?;
            let user = self.users.lock().unwrap().get(&user_id).cloned()
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

    /// Creates a test router with mock services.
    fn create_test_router(
        user_service: Arc<dyn UserService>,
        auth_service: Arc<dyn AuthService>,
        token_provider: Arc<dyn TokenProviderInterface>,
    ) -> Router {
        let state = AppState::new(user_service, auth_service);
        let auth_state = AuthMiddlewareState::new(token_provider);

        let api_router = Router::new()
            .nest("/auth", auth_controller::router())
            .nest("/users", user_controller::router())
            .layer(axum_middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
            .with_state(state);

        Router::new()
            .merge(health_controller::router())
            .nest("/api/v1", api_router)
    }

    /// Helper to create an authenticated request.
    fn create_auth_header(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Helper to parse response body as JSON.
    async fn parse_body<T: serde::de::DeserializeOwned>(body: Body) -> T {
        let bytes = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // =============================================================================
    // Health Controller Tests
    // =============================================================================

    #[tokio::test]
    async fn test_health_check() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert_eq!(body["status"], "healthy");
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .uri("/live")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // =============================================================================
    // Auth Controller Tests
    // =============================================================================

    #[tokio::test]
    async fn test_register_success() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "username": "newuser",
            "email": "new@example.com",
            "password": "Password123",
            "first_name": "New",
            "last_name": "User"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert!(body["data"]["access_token"].as_str().is_some());
        assert_eq!(body["data"]["user"]["username"], "newuser");
    }

    #[tokio::test]
    async fn test_login_success() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "username_or_email": "testuser",
            "password": "Password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert!(body["data"]["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config)); // No users
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "username_or_email": "nonexistent",
            "password": "Password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_refresh_token_success() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_id = user.id;

        // Generate a refresh token
        let tokens = token_provider.generate_tokens(
            user_id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "refresh_token": tokens.refresh_token
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/refresh")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert!(body["data"]["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_get_me_authenticated() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user.clone()));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["username"], "testuser");
    }

    #[tokio::test]
    async fn test_get_me_unauthenticated() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_logout_authenticated() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/logout")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert!(body["data"]["message"].as_str().unwrap().contains("logged out"));
    }

    // =============================================================================
    // User Controller Tests
    // =============================================================================

    #[tokio::test]
    async fn test_list_users_as_admin() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();
        let test_user = create_test_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone(), test_user]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["total_elements"], 2);
    }

    #[tokio::test]
    async fn test_list_users_forbidden_for_regular_user() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user(); // Regular user, not admin

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_get_user_by_id_own_profile() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_id = user.id;

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![user.clone()]));
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/users/{}", user_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["username"], "testuser");
    }

    #[tokio::test]
    async fn test_create_user_as_admin() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "username": "newuser",
            "email": "new@example.com",
            "password": "Password123",
            "first_name": "New",
            "last_name": "User"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/users")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["username"], "newuser");
    }

    #[tokio::test]
    async fn test_update_user_own_profile() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_id = user.id;

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![user.clone()]));
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "first_name": "Updated",
            "last_name": "Name"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(&format!("/api/v1/users/{}", user_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["first_name"], "Updated");
    }

    #[tokio::test]
    async fn test_delete_user_as_admin() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();
        let user_to_delete = create_test_user();
        let delete_id = user_to_delete.id;

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone(), user_to_delete]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(&format!("/api/v1/users/{}", delete_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_update_user_role_as_admin() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();
        let user = create_test_user();
        let user_id = user.id;

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone(), user]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "role": "moderator"
        });

        let request = Request::builder()
            .method(Method::PATCH)
            .uri(&format!("/api/v1/users/{}/role", user_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["role"], "moderator");
    }

    #[tokio::test]
    async fn test_update_user_status_as_admin() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();
        let user = create_test_user();
        let user_id = user.id;

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone(), user]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "status": "suspended",
            "reason": "Test suspension"
        });

        let request = Request::builder()
            .method(Method::PATCH)
            .uri(&format!("/api/v1/users/{}/status", user_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["status"], "suspended");
    }

    #[tokio::test]
    async fn test_change_password() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_id = user.id;

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![user.clone()]));
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        let body = json!({
            "current_password": "OldPassword123",
            "new_password": "NewPassword456"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(&format!("/api/v1/users/{}/password", user_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_user_not_found() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new()); // Empty, no users
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let fake_id = UserId::new();
        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/users/{}", fake_id))
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_invalid_user_id_format() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users/invalid-uuid")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_users_with_pagination() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone()]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users?page=0&size=10")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = parse_body(response.into_body()).await;
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["page"], 0);
        assert_eq!(body["data"]["size"], 10);
    }

    // =============================================================================
    // Middleware Tests
    // =============================================================================

    #[tokio::test]
    async fn test_auth_middleware_with_valid_token() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user.clone()));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Access a protected endpoint with valid token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_invalid_token() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Access a protected endpoint with invalid token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header(header::AUTHORIZATION, "Bearer invalid-token")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_without_token() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Access a protected endpoint without token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_malformed_header() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Access with malformed authorization header (no "Bearer " prefix)
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header(header::AUTHORIZATION, "InvalidHeader")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_wrong_secret_token() {
        // Create a token with a different secret
        let wrong_config = Arc::new(SecurityConfig {
            jwt_secret: "wrong-secret-key-definitely-not-matching-32".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test-issuer".to_string(),
            jwt_audience: "test-audience".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        });
        let wrong_token_provider = Arc::new(TokenProvider::new(wrong_config.clone()));
        let user = create_test_user();

        // Generate a token with the wrong secret
        let tokens = wrong_token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        // Use the correct config for the router
        let correct_config = create_test_security_config();
        let correct_token_provider = Arc::new(TokenProvider::new(correct_config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(correct_config));
        let router = create_test_router(user_service, auth_service, correct_token_provider);

        // Try to use the token signed with wrong secret
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_public_endpoints_without_auth() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Health endpoint should work without auth
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login_endpoint_without_auth() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user();
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Login should work without auth (but needs valid credentials)
        let body = json!({
            "username_or_email": "testuser",
            "password": "Password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register_endpoint_without_auth() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::new(config));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Register should work without auth
        let body = json!({
            "username": "newuser",
            "email": "new@example.com",
            "password": "Password123",
            "first_name": "New",
            "last_name": "User"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_role_based_access_admin_only() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let user = create_test_user(); // Regular user, not admin

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::new());
        let auth_service = Arc::new(MockAuthService::with_user(config, user));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Try to access admin-only endpoint
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_role_based_access_admin_allowed() {
        let config = create_test_security_config();
        let token_provider = Arc::new(TokenProvider::new(config.clone()));
        let admin = create_admin_user();

        let tokens = token_provider.generate_tokens(
            admin.id, &admin.username, admin.email.as_str(), admin.role,
        ).unwrap();

        let user_service = Arc::new(MockUserService::with_users(vec![admin.clone()]));
        let auth_service = Arc::new(MockAuthService::with_user(config, admin));
        let router = create_test_router(user_service, auth_service, token_provider);

        // Admin should access admin-only endpoint
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header(header::AUTHORIZATION, create_auth_header(&tokens.access_token))
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
