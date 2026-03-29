//! Authentication service trait.

use crate::dto::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use arcana_core::{ArcanaResult, Interface, UserId};
use arcana_repository::UserRepository;
use arcana_security::Claims;
use async_trait::async_trait;

/// Error message for inactive account status.
const ACCOUNT_NOT_ACTIVE_MSG: &str = "Account is not active";
/// Error message for suspended account status.
const ACCOUNT_SUSPENDED_MSG: &str = "Account is suspended";
/// Error message for locked account status.
const ACCOUNT_LOCKED_MSG: &str = "Account is locked";
/// Error message for missing user ID in refresh token.
const REFRESH_TOKEN_MISSING_USER_ID_MSG: &str = "Invalid refresh token: missing user ID";
/// Error message for missing user ID in token.
const TOKEN_MISSING_USER_ID_MSG: &str = "Invalid token: missing user ID";
/// Error message when user no longer exists during token refresh.
const USER_NO_LONGER_EXISTS_MSG: &str = "User no longer exists";
/// Creates a conflict message for duplicate username.
fn conflict_username_msg(username: &str) -> String {
    format!("Username '{}' already exists", username)
}

/// Creates a conflict message for duplicate email.
fn conflict_email_msg(email: &str) -> String {
    format!("Email '{}' already exists", email)
}

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
