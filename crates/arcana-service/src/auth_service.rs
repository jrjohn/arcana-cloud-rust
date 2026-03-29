//! Authentication service trait.

use crate::dto::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use arcana_core::{ArcanaResult, Interface, UserId};
use arcana_repository::UserRepository;
use arcana_security::Claims;
use async_trait::async_trait;

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
