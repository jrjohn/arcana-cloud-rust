//! User management controller.

use crate::{
    extractors::{AuthenticatedUser, PaginationQuery, ValidatedJson},
    responses::{created, no_content, ok, AppError, ApiResult},
    state::AppState,
};
use arcana_core::{ArcanaError, ErrorResponse, UserId};
use arcana_core::UserRole;
use arcana_security::ClaimsExt;
use arcana_service::{
    ChangePasswordRequest, CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest,
    UpdateUserStatusRequest, UserListResponse, UserResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, put},
    Json, Router,
};
use tracing::debug;

/// Creates the user router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/:id/role", patch(update_user_role))
        .route("/:id/status", patch(update_user_status))
        .route("/:id/password", put(change_password))
}

/// List all users (admin only).
#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    params(
        ("page" = Option<usize>, Query, description = "Page number (0-indexed)"),
        ("size" = Option<usize>, Query, description = "Page size (max 100)")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of users", body = UserListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse)
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<UserListResponse> {
    debug!("List users request");

    user.require_role(UserRole::Admin)?;

    let response = state.user_service.list_users(pagination.into()).await?;
    ok(response)
}

/// Create a new user (admin only).
#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    request_body = CreateUserRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 409, description = "Username or email already exists", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn create_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    ValidatedJson(request): ValidatedJson<CreateUserRequest>,
) -> Result<(StatusCode, Json<crate::responses::ApiResponse<UserResponse>>), AppError> {
    debug!("Create user request: {}", request.username);

    user.require_role(UserRole::Admin)?;

    let response = state.user_service.create_user(request).await?;
    Ok(created(response))
}

/// Get a user by ID.
#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User details", body = UserResponse),
        (status = 400, description = "Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only view own profile or need moderator role", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<UserResponse> {
    debug!("Get user request: {}", id);

    let user_id = parse_user_id(&id)?;

    // Users can view themselves, admins can view anyone
    let current_user_id = user.user_id().ok_or_else(|| {
        AppError(ArcanaError::Internal("Missing user ID in token".to_string()))
    })?;

    if current_user_id != user_id {
        user.require_role(UserRole::Moderator)?;
    }

    let response = state.user_service.get_user(user_id).await?;
    ok(response)
}

/// Update a user's profile.
#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    request_body = UpdateUserRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 400, description = "Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only update own profile or need admin role", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateUserRequest>,
) -> ApiResult<UserResponse> {
    debug!("Update user request: {}", id);

    let user_id = parse_user_id(&id)?;

    // Users can update themselves, admins can update anyone
    let current_user_id = user.user_id().ok_or_else(|| {
        AppError(ArcanaError::Internal("Missing user ID in token".to_string()))
    })?;

    if current_user_id != user_id {
        user.require_role(UserRole::Admin)?;
    }

    let response = state.user_service.update_user(user_id, request).await?;
    ok(response)
}

/// Delete a user (admin only).
#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 400, description = "Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    debug!("Delete user request: {}", id);

    user.require_role(UserRole::Admin)?;

    let user_id = parse_user_id(&id)?;
    state.user_service.delete_user(user_id).await?;

    Ok(no_content())
}

/// Update a user's role (admin only).
#[utoipa::path(
    patch,
    path = "/users/{id}/role",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    request_body = UpdateUserRoleRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User role updated", body = UserResponse),
        (status = 400, description = "Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required, super_admin for elevating to super_admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn update_user_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateUserRoleRequest>,
) -> ApiResult<UserResponse> {
    debug!("Update user role request: {} -> {:?}", id, request.role);

    user.require_role(UserRole::Admin)?;

    // Prevent changing to super admin unless you're a super admin
    if request.role == UserRole::SuperAdmin {
        user.require_super_admin()?;
    }

    let user_id = parse_user_id(&id)?;
    let response = state.user_service.update_user_role(user_id, request).await?;
    ok(response)
}

/// Update a user's status (admin only).
#[utoipa::path(
    patch,
    path = "/users/{id}/status",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    request_body = UpdateUserStatusRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User status updated", body = UserResponse),
        (status = 400, description = "Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn update_user_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateUserStatusRequest>,
) -> ApiResult<UserResponse> {
    debug!("Update user status request: {} -> {:?}", id, request.status);

    user.require_role(UserRole::Admin)?;

    let user_id = parse_user_id(&id)?;
    let response = state.user_service.update_user_status(user_id, request).await?;
    ok(response)
}

/// Change a user's password.
#[utoipa::path(
    put,
    path = "/users/{id}/password",
    tag = "users",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    request_body = ChangePasswordRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 204, description = "Password changed successfully"),
        (status = 400, description = "Invalid user ID format or current password incorrect", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only change own password", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse)
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<ChangePasswordRequest>,
) -> Result<StatusCode, AppError> {
    debug!("Change password request: {}", id);

    let user_id = parse_user_id(&id)?;

    // Users can only change their own password
    let current_user_id = user.user_id().ok_or_else(|| {
        AppError(ArcanaError::Internal("Missing user ID in token".to_string()))
    })?;

    if current_user_id != user_id {
        return Err(AppError(ArcanaError::Forbidden(
            "Cannot change another user's password".to_string(),
        )));
    }

    state.user_service.change_password(user_id, request).await?;
    Ok(no_content())
}

/// Helper to parse user ID from path parameter.
fn parse_user_id(id: &str) -> Result<UserId, AppError> {
    UserId::parse(id).map_err(|_| AppError(ArcanaError::Validation(format!("Invalid user ID: {}", id))))
}
