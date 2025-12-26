//! Authentication controller.

use crate::{
    extractors::AuthenticatedUser,
    responses::{ok, AppError, ApiResult},
    state::AppState,
};
use arcana_service::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use axum::{extract::State, routing::post, Json, Router};
use tracing::debug;

/// Creates the auth router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/me", axum::routing::get(get_current_user))
}

/// Register a new user.
async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Registration request for: {}", request.username);

    let response = state.auth_service.register(request).await?;
    ok(response)
}

/// Login with username/email and password.
async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Login request for: {}", request.username_or_email);

    let response = state.auth_service.login(request).await?;
    ok(response)
}

/// Refresh access token using refresh token.
async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Token refresh request");

    let response = state.auth_service.refresh_token(request).await?;
    ok(response)
}

/// Logout (invalidate tokens).
async fn logout(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<MessageResponse> {
    debug!("Logout request for: {}", user.username);

    let user_id = user.user_id().ok_or_else(|| {
        AppError(arcana_core::ArcanaError::Internal("Missing user ID in token".to_string()))
    })?;

    let response = state.auth_service.logout(user_id).await?;
    ok(response)
}

/// Get current authenticated user.
async fn get_current_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<AuthUserInfo> {
    debug!("Get current user: {}", user.username);

    let user_info = state.auth_service.get_current_user(&user.0).await?;
    ok(user_info)
}
