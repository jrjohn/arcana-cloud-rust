//! Authentication gRPC service implementation.

use crate::proto::{auth, common, user as user_proto};
use arcana_service::dto::{LoginRequest, RefreshTokenRequest, RegisterRequest};
use arcana_service::AuthService;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, error};

/// Authentication gRPC service implementation.
pub struct AuthGrpcService {
    auth_service: Arc<dyn AuthService>,
}

impl AuthGrpcService {
    /// Creates a new auth gRPC service.
    pub fn new(auth_service: Arc<dyn AuthService>) -> Self {
        Self { auth_service }
    }
}

#[tonic::async_trait]
impl auth::auth_service_server::AuthService for AuthGrpcService {
    async fn register(
        &self,
        request: Request<auth::RegisterRequest>,
    ) -> Result<Response<auth::AuthResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC Register: {}", req.username);

        let register_request = RegisterRequest {
            username: req.username,
            email: req.email,
            password: req.password,
            first_name: req.first_name.filter(|s| !s.is_empty()),
            last_name: req.last_name.filter(|s| !s.is_empty()),
        };

        let response = self
            .auth_service
            .register(register_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(to_proto_auth_response(response)))
    }

    async fn login(
        &self,
        request: Request<auth::LoginRequest>,
    ) -> Result<Response<auth::AuthResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC Login: {}", req.username_or_email);

        let login_request = LoginRequest {
            username_or_email: req.username_or_email,
            password: req.password,
            device_id: req.device_id.filter(|s| !s.is_empty()),
        };

        let response = self
            .auth_service
            .login(login_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(to_proto_auth_response(response)))
    }

    async fn refresh_token(
        &self,
        request: Request<auth::RefreshTokenRequest>,
    ) -> Result<Response<auth::AuthResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC RefreshToken");

        let refresh_request = RefreshTokenRequest {
            refresh_token: req.refresh_token,
        };

        let response = self
            .auth_service
            .refresh_token(refresh_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(to_proto_auth_response(response)))
    }

    async fn validate_token(
        &self,
        request: Request<auth::ValidateTokenRequest>,
    ) -> Result<Response<auth::ValidateTokenResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC ValidateToken");

        let claims = self
            .auth_service
            .validate_token(&req.token)
            .await
            .map_err(to_status)?;

        Ok(Response::new(auth::ValidateTokenResponse {
            valid: true,
            user_id: claims.user_id().map(|id| id.to_string()),
            username: Some(claims.username.clone()),
            role: Some(to_proto_role(claims.role).into()),
            expires_at: Some(claims.exp as i64),
        }))
    }

    async fn logout(
        &self,
        request: Request<auth::LogoutRequest>,
    ) -> Result<Response<common::Empty>, Status> {
        let _req = request.into_inner();
        debug!("gRPC Logout");

        // For logout without user_id in request, we'd typically get it from the auth context
        // For now, just return success
        Ok(Response::new(common::Empty {}))
    }

    async fn get_current_user(
        &self,
        _request: Request<common::Empty>,
    ) -> Result<Response<auth::CurrentUserResponse>, Status> {
        debug!("gRPC GetCurrentUser");

        // In a real implementation, we'd extract the claims from the request metadata
        // For now, return an error since we don't have the auth context
        Err(Status::unauthenticated("No authentication context available"))
    }
}

// Helper functions

fn to_status(err: arcana_core::ArcanaError) -> Status {
    use arcana_core::ArcanaError;

    error!("gRPC auth error: {:?}", err);

    match err {
        ArcanaError::NotFound { .. } => Status::not_found(err.to_string()),
        ArcanaError::Validation(msg) => Status::invalid_argument(msg),
        ArcanaError::Conflict(msg) => Status::already_exists(msg),
        ArcanaError::Unauthorized(_) => Status::unauthenticated("Unauthorized"),
        ArcanaError::Forbidden(msg) => Status::permission_denied(msg),
        ArcanaError::InvalidCredentials => Status::unauthenticated("Invalid credentials"),
        ArcanaError::InvalidToken(msg) => Status::unauthenticated(msg),
        ArcanaError::RateLimitExceeded => Status::resource_exhausted(err.to_string()),
        _ => Status::internal(err.to_string()),
    }
}

fn to_proto_auth_response(response: arcana_service::dto::AuthResponse) -> auth::AuthResponse {
    auth::AuthResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        token_type: response.token_type,
        expires_in: response.expires_in,
        user: Some(auth::AuthUserInfo {
            id: response.user.id.to_string(),
            username: response.user.username,
            email: response.user.email,
            role: to_proto_role(response.user.role).into(),
            first_name: response.user.first_name,
            last_name: response.user.last_name,
        }),
    }
}

fn to_proto_role(role: arcana_domain::UserRole) -> user_proto::UserRole {
    match role {
        arcana_domain::UserRole::User => user_proto::UserRole::User,
        arcana_domain::UserRole::Moderator => user_proto::UserRole::Moderator,
        arcana_domain::UserRole::Admin => user_proto::UserRole::Admin,
        arcana_domain::UserRole::SuperAdmin => user_proto::UserRole::SuperAdmin,
    }
}
