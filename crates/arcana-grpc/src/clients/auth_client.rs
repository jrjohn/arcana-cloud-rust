//! Remote auth service client via gRPC.

use crate::proto::{auth, user as user_proto};
use arcana_core::{ArcanaError, ArcanaResult, UserId};
use arcana_security::Claims;
use arcana_service::dto::{
    AuthResponse, AuthUserInfo, LoginRequest, MessageResponse, RefreshTokenRequest, RegisterRequest,
};
use arcana_service::AuthService;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::debug;

/// Remote auth service client that communicates via gRPC.
pub struct RemoteAuthServiceClient {
    client: auth::auth_service_client::AuthServiceClient<Channel>,
}

impl RemoteAuthServiceClient {
    /// Creates a new remote auth service client.
    pub async fn connect(addr: &str) -> ArcanaResult<Self> {
        let client = auth::auth_service_client::AuthServiceClient::connect(addr.to_string())
            .await
            .map_err(|e| ArcanaError::Internal(format!("Failed to connect to auth service: {}", e)))?;

        Ok(Self { client })
    }

    /// Creates from an existing channel.
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            client: auth::auth_service_client::AuthServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl AuthService for RemoteAuthServiceClient {
    async fn register(&self, request: RegisterRequest) -> ArcanaResult<AuthResponse> {
        debug!("Remote Register: {}", request.username);

        let proto_request = auth::RegisterRequest {
            username: request.username,
            email: request.email,
            password: request.password,
            first_name: request.first_name,
            last_name: request.last_name,
        };

        let response = self
            .client
            .clone()
            .register(proto_request)
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(from_proto_auth_response(response.into_inner()))
    }

    async fn login(&self, request: LoginRequest) -> ArcanaResult<AuthResponse> {
        debug!("Remote Login: {}", request.username_or_email);

        let proto_request = auth::LoginRequest {
            username_or_email: request.username_or_email,
            password: request.password,
            device_id: request.device_id,
        };

        let response = self
            .client
            .clone()
            .login(proto_request)
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(from_proto_auth_response(response.into_inner()))
    }

    async fn refresh_token(&self, request: RefreshTokenRequest) -> ArcanaResult<AuthResponse> {
        debug!("Remote RefreshToken");

        let proto_request = auth::RefreshTokenRequest {
            refresh_token: request.refresh_token,
        };

        let response = self
            .client
            .clone()
            .refresh_token(proto_request)
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(from_proto_auth_response(response.into_inner()))
    }

    async fn validate_token(&self, token: &str) -> ArcanaResult<Claims> {
        debug!("Remote ValidateToken");

        let proto_request = auth::ValidateTokenRequest {
            token: token.to_string(),
        };

        let response = self
            .client
            .clone()
            .validate_token(proto_request)
            .await
            .map_err(|e| map_grpc_error(e))?;

        let inner = response.into_inner();

        if !inner.valid {
            return Err(ArcanaError::InvalidToken("Token is not valid".to_string()));
        }

        // Reconstruct claims from response
        let user_id = inner
            .user_id
            .as_ref()
            .and_then(|id| UserId::parse(id).ok())
            .ok_or_else(|| ArcanaError::InvalidToken("Missing user ID in token".to_string()))?;

        let username = inner.username.unwrap_or_default();
        // Email is not in ValidateTokenResponse, use empty string (will be filled by caller if needed)
        let email = String::new();
        let role = inner
            .role
            .map(|r| from_proto_role(user_proto::UserRole::try_from(r).unwrap_or(user_proto::UserRole::User)))
            .unwrap_or(arcana_core::UserRole::User);
        let expires_at = inner.expires_at.unwrap_or(0);

        Ok(Claims::new_access(
            user_id,
            username,
            email,
            role,
            "arcana-cloud".to_string(),
            "arcana-api".to_string(),
            Utc::now() + Duration::seconds(expires_at),
        ))
    }

    async fn logout(&self, user_id: UserId) -> ArcanaResult<MessageResponse> {
        debug!("Remote Logout: {}", user_id);

        let proto_request = auth::LogoutRequest { all_sessions: false };

        self.client
            .clone()
            .logout(proto_request)
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(MessageResponse::new("Successfully logged out"))
    }

    async fn get_current_user(&self, claims: &Claims) -> ArcanaResult<AuthUserInfo> {
        debug!("Remote GetCurrentUser");

        // Get user ID from claims
        let user_id = claims.user_id().ok_or_else(|| {
            ArcanaError::InvalidToken("Invalid token: missing user ID".to_string())
        })?;

        // We can construct the AuthUserInfo from claims without calling the server
        Ok(AuthUserInfo {
            id: user_id,
            username: claims.username.clone(),
            email: claims.email.clone(),
            role: claims.role,
            first_name: None,
            last_name: None,
        })
    }
}

/// Creates a shareable auth service client.
pub async fn create_remote_auth_service(addr: &str) -> ArcanaResult<Arc<dyn AuthService>> {
    let client = RemoteAuthServiceClient::connect(addr).await?;
    Ok(Arc::new(client))
}

// Helper functions

fn map_grpc_error(status: tonic::Status) -> ArcanaError {
    match status.code() {
        tonic::Code::NotFound => ArcanaError::NotFound {
            resource_type: "Resource",
            id: status.message().to_string(),
        },
        tonic::Code::InvalidArgument => ArcanaError::Validation(status.message().to_string()),
        tonic::Code::AlreadyExists => ArcanaError::Conflict(status.message().to_string()),
        tonic::Code::Unauthenticated => ArcanaError::InvalidCredentials,
        tonic::Code::PermissionDenied => ArcanaError::Forbidden(status.message().to_string()),
        _ => ArcanaError::Internal(format!("gRPC error: {}", status.message())),
    }
}

fn from_proto_auth_response(response: auth::AuthResponse) -> AuthResponse {
    let user_info = response.user.map(|u| AuthUserInfo {
        id: UserId::parse(&u.id).unwrap_or_else(|_| UserId::new()),
        username: u.username,
        email: u.email,
        role: from_proto_role(user_proto::UserRole::try_from(u.role).unwrap_or(user_proto::UserRole::User)),
        first_name: u.first_name,
        last_name: u.last_name,
    });

    AuthResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        token_type: response.token_type,
        expires_in: response.expires_in,
        user: user_info.unwrap_or_else(|| AuthUserInfo {
            id: UserId::new(),
            username: String::new(),
            email: String::new(),
            role: arcana_core::UserRole::User,
            first_name: None,
            last_name: None,
        }),
    }
}

fn from_proto_role(role: user_proto::UserRole) -> arcana_core::UserRole {
    match role {
        user_proto::UserRole::User | user_proto::UserRole::Unspecified => arcana_core::UserRole::User,
        user_proto::UserRole::Moderator => arcana_core::UserRole::Moderator,
        user_proto::UserRole::Admin => arcana_core::UserRole::Admin,
        user_proto::UserRole::SuperAdmin => arcana_core::UserRole::SuperAdmin,
    }
}
