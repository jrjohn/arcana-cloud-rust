//! # Arcana gRPC
//!
//! gRPC service layer using Tonic for Arcana Cloud Rust.
//! Provides gRPC endpoints for user management, authentication, and health checks.
//!
//! Also includes gRPC clients for inter-layer communication in distributed deployments.

pub mod clients;
pub mod interceptors;
pub mod proto;
pub mod server;
pub mod services;
pub mod tls;

pub use clients::*;
pub use server::*;
pub use services::*;
pub use tls::{TlsConfigBuilder, build_client_tls_from_ca, build_client_tls_from_config};

#[cfg(test)]
mod tests {
    use super::proto::{auth, common, user};
    use super::services::{AuthGrpcService, UserGrpcService};
    use arcana_config::SecurityConfig;
    use arcana_core::{ArcanaError, ArcanaResult, Page, PageRequest, UserId};
    use arcana_core::{Email, User};
    use arcana_security::{Claims, TokenProvider};
    use arcana_service::{
        AuthResponse, AuthService, AuthUserInfo, ChangePasswordRequest, CreateUserRequest,
        LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest, UpdateUserRequest,
        UpdateUserRoleRequest, UpdateUserStatusRequest, UserListResponse, UserResponse, UserService,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tonic::Request;

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

    /// Mock user service for gRPC tests.
    struct MockUserService {
        users: Arc<Mutex<HashMap<UserId, User>>>,
    }

    impl MockUserService {
        fn new() -> Self {
            Self {
                users: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn with_user(user: User) -> Self {
            let service = Self::new();
            service.users.lock().unwrap().insert(user.id, user);
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

    /// Mock auth service for gRPC tests.
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

    // =============================================================================
    // User gRPC Service Tests
    // =============================================================================

    #[tokio::test]
    async fn test_grpc_get_user_success() {
        let user = create_test_user();
        let user_id = user.id;
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::GetUserRequest {
            user_id: user_id.to_string(),
        });

        let response = user::user_service_server::UserService::get_user(&service, request)
            .await
            .unwrap();
        let user = response.into_inner().user.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_grpc_get_user_not_found() {
        let service = UserGrpcService::new(Arc::new(MockUserService::new()));

        let request = Request::new(user::GetUserRequest {
            user_id: UserId::new().to_string(),
        });

        let result = user::user_service_server::UserService::get_user(&service, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_grpc_get_user_by_username() {
        let user = create_test_user();
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::GetUserByUsernameRequest {
            username: "testuser".to_string(),
        });

        let response = user::user_service_server::UserService::get_user_by_username(&service, request)
            .await
            .unwrap();
        let user = response.into_inner().user.unwrap();
        assert_eq!(user.username, "testuser");
    }

    #[tokio::test]
    async fn test_grpc_list_users() {
        let user = create_test_user();
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::ListUsersRequest {
            page: Some(common::PageRequest { page: 0, size: 10 }),
            role_filter: None,
        });

        let response = user::user_service_server::UserService::list_users(&service, request)
            .await
            .unwrap();
        let inner = response.into_inner();
        assert_eq!(inner.users.len(), 1);
        assert_eq!(inner.page_info.unwrap().total_elements, 1);
    }

    #[tokio::test]
    async fn test_grpc_create_user() {
        let service = UserGrpcService::new(Arc::new(MockUserService::new()));

        let request = Request::new(user::CreateUserRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
        });

        let response = user::user_service_server::UserService::create_user(&service, request)
            .await
            .unwrap();
        let user = response.into_inner().user.unwrap();
        assert_eq!(user.username, "newuser");
        assert_eq!(user.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_grpc_update_user() {
        let user = create_test_user();
        let user_id = user.id;
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::UpdateUserRequest {
            user_id: user_id.to_string(),
            first_name: Some("Updated".to_string()),
            last_name: Some("Name".to_string()),
            avatar_url: None,
        });

        let response = user::user_service_server::UserService::update_user(&service, request)
            .await
            .unwrap();
        let user = response.into_inner().user.unwrap();
        assert_eq!(user.first_name, Some("Updated".to_string()));
        assert_eq!(user.last_name, Some("Name".to_string()));
    }

    #[tokio::test]
    async fn test_grpc_delete_user() {
        let user = create_test_user();
        let user_id = user.id;
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::DeleteUserRequest {
            user_id: user_id.to_string(),
        });

        let result = user::user_service_server::UserService::delete_user(&service, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_grpc_update_user_role() {
        let user = create_test_user();
        let user_id = user.id;
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::UpdateUserRoleRequest {
            user_id: user_id.to_string(),
            role: user::UserRole::Admin.into(),
        });

        let response = user::user_service_server::UserService::update_user_role(&service, request)
            .await
            .unwrap();
        let user = response.into_inner().user.unwrap();
        assert_eq!(user.role, user::UserRole::Admin as i32);
    }

    #[tokio::test]
    async fn test_grpc_username_exists() {
        let user = create_test_user();
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::UsernameExistsRequest {
            username: "testuser".to_string(),
        });

        let response = user::user_service_server::UserService::username_exists(&service, request)
            .await
            .unwrap();
        assert!(response.into_inner().exists);
    }

    #[tokio::test]
    async fn test_grpc_email_exists() {
        let user = create_test_user();
        let service = UserGrpcService::new(Arc::new(MockUserService::with_user(user)));

        let request = Request::new(user::EmailExistsRequest {
            email: "test@example.com".to_string(),
        });

        let response = user::user_service_server::UserService::email_exists(&service, request)
            .await
            .unwrap();
        assert!(response.into_inner().exists);
    }

    #[tokio::test]
    async fn test_grpc_invalid_user_id() {
        let service = UserGrpcService::new(Arc::new(MockUserService::new()));

        let request = Request::new(user::GetUserRequest {
            user_id: "invalid-uuid".to_string(),
        });

        let result = user::user_service_server::UserService::get_user(&service, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    // =============================================================================
    // Auth gRPC Service Tests
    // =============================================================================

    #[tokio::test]
    async fn test_grpc_register_success() {
        let config = create_test_security_config();
        let service = AuthGrpcService::new(Arc::new(MockAuthService::new(config)));

        let request = Request::new(auth::RegisterRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
        });

        let response = auth::auth_service_server::AuthService::register(&service, request)
            .await
            .unwrap();
        let inner = response.into_inner();
        assert!(!inner.access_token.is_empty());
        assert!(!inner.refresh_token.is_empty());
        assert_eq!(inner.user.unwrap().username, "newuser");
    }

    #[tokio::test]
    async fn test_grpc_login_success() {
        let config = create_test_security_config();
        let user = create_test_user();
        let service = AuthGrpcService::new(Arc::new(MockAuthService::with_user(config, user)));

        let request = Request::new(auth::LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        });

        let response = auth::auth_service_server::AuthService::login(&service, request)
            .await
            .unwrap();
        let inner = response.into_inner();
        assert!(!inner.access_token.is_empty());
        assert_eq!(inner.user.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_grpc_login_invalid_credentials() {
        let config = create_test_security_config();
        let service = AuthGrpcService::new(Arc::new(MockAuthService::new(config))); // No users

        let request = Request::new(auth::LoginRequest {
            username_or_email: "nonexistent".to_string(),
            password: "Password123".to_string(),
            device_id: None,
        });

        let result = auth::auth_service_server::AuthService::login(&service, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_grpc_refresh_token_success() {
        let config = create_test_security_config();
        let token_provider = TokenProvider::new(config.clone());
        let user = create_test_user();

        // Generate a refresh token
        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let service = AuthGrpcService::new(Arc::new(MockAuthService::with_user(config, user)));

        let request = Request::new(auth::RefreshTokenRequest {
            refresh_token: tokens.refresh_token,
        });

        let response = auth::auth_service_server::AuthService::refresh_token(&service, request)
            .await
            .unwrap();
        let inner = response.into_inner();
        assert!(!inner.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_grpc_validate_token_success() {
        let config = create_test_security_config();
        let token_provider = TokenProvider::new(config.clone());
        let user = create_test_user();

        let tokens = token_provider.generate_tokens(
            user.id, &user.username, user.email.as_str(), user.role,
        ).unwrap();

        let service = AuthGrpcService::new(Arc::new(MockAuthService::with_user(config, user)));

        let request = Request::new(auth::ValidateTokenRequest {
            token: tokens.access_token,
        });

        let response = auth::auth_service_server::AuthService::validate_token(&service, request)
            .await
            .unwrap();
        let inner = response.into_inner();
        assert!(inner.valid);
        assert_eq!(inner.username, Some("testuser".to_string()));
    }

    #[tokio::test]
    async fn test_grpc_validate_token_invalid() {
        let config = create_test_security_config();
        let service = AuthGrpcService::new(Arc::new(MockAuthService::new(config)));

        let request = Request::new(auth::ValidateTokenRequest {
            token: "invalid-token".to_string(),
        });

        let result = auth::auth_service_server::AuthService::validate_token(&service, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_grpc_logout() {
        let config = create_test_security_config();
        let service = AuthGrpcService::new(Arc::new(MockAuthService::new(config)));

        let request = Request::new(auth::LogoutRequest {
            all_sessions: false,
        });

        let result = auth::auth_service_server::AuthService::logout(&service, request).await;
        assert!(result.is_ok());
    }
}
