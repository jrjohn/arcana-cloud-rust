//! OpenAPI documentation configuration.
//!
//! This module provides OpenAPI/Swagger documentation generation for the REST API.

use arcana_core::{ErrorResponse, FieldError, UserRole, UserStatus, UserId};
use arcana_service::{
    AuthResponse, AuthUserInfo, ChangePasswordRequest, CreateUserRequest, LoginRequest,
    MessageResponse, PasswordResetConfirmRequest, PasswordResetRequest, RefreshTokenRequest,
    RegisterRequest, UpdateUserRequest, UpdateUserRoleRequest, UpdateUserStatusRequest,
    UserListResponse, UserResponse,
};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// OpenAPI documentation for Arcana Cloud Rust API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Arcana Cloud Rust API",
        version = "1.0.0",
        description = "RESTful API for Arcana Cloud platform",
        contact(
            name = "Arcana Team",
            url = "https://github.com/arcana/arcana-cloud-rust"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    paths(
        // Auth endpoints
        crate::controllers::auth_controller::register,
        crate::controllers::auth_controller::login,
        crate::controllers::auth_controller::refresh_token,
        crate::controllers::auth_controller::logout,
        crate::controllers::auth_controller::get_current_user,
        // User endpoints
        crate::controllers::user_controller::list_users,
        crate::controllers::user_controller::create_user,
        crate::controllers::user_controller::get_user,
        crate::controllers::user_controller::update_user,
        crate::controllers::user_controller::delete_user,
        crate::controllers::user_controller::update_user_role,
        crate::controllers::user_controller::update_user_status,
        crate::controllers::user_controller::change_password,
        // Health endpoints
        crate::controllers::health_controller::health_check,
        crate::controllers::health_controller::readiness_check,
        crate::controllers::health_controller::liveness_check,
    ),
    components(
        schemas(
            // Core types
            UserId,
            UserRole,
            UserStatus,
            ErrorResponse,
            FieldError,
            // Auth DTOs
            LoginRequest,
            RegisterRequest,
            RefreshTokenRequest,
            AuthResponse,
            AuthUserInfo,
            PasswordResetRequest,
            PasswordResetConfirmRequest,
            MessageResponse,
            // User DTOs
            CreateUserRequest,
            UpdateUserRequest,
            UpdateUserRoleRequest,
            UpdateUserStatusRequest,
            ChangePasswordRequest,
            UserResponse,
            UserListResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "health", description = "Health check endpoints")
    )
)]
pub struct ApiDoc;

/// Security addon for JWT Bearer authentication.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT Bearer token authentication"))
                        .build(),
                ),
            );
        }
    }
}
