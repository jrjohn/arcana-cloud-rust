//! Authentication controller.

use crate::{
    extractors::{AuthenticatedUser, ValidatedJson},
    responses::{ok, AppError, ApiResult},
    state::AppState,
};
use arcana_core::ErrorResponse;
use arcana_service::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use axum::{extract::State, routing::post, Router};
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
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = AuthResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Username or email already exists", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<RegisterRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Registration request for: {}", request.username);

    let response = state.auth_service.register(request).await?;
    ok(response)
}

/// Login with username/email and password.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<LoginRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Login request for: {}", request.username_or_email);

    let response = state.auth_service.login(request).await?;
    ok(response)
}

/// Refresh access token using refresh token.
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = AuthResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<RefreshTokenRequest>,
) -> ApiResult<AuthResponse> {
    debug!("Token refresh request");

    let response = state.auth_service.refresh_token(request).await?;
    ok(response)
}

/// Logout (invalidate tokens).
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Logout successful", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn logout(
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
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current user info", body = AuthUserInfo),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn get_current_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<AuthUserInfo> {
    debug!("Get current user: {}", user.username);

    let user_info = state.auth_service.get_current_user(&user.0).await?;
    ok(user_info)
}
