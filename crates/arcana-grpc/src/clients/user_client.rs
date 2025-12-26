//! Remote user service client via gRPC.

use crate::proto::{common, user};
use arcana_core::{ArcanaError, ArcanaResult, PageRequest, UserId};
use arcana_service::dto::{
    ChangePasswordRequest, CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest,
    UpdateUserStatusRequest, UserListResponse, UserResponse,
};
use arcana_service::UserService;
use async_trait::async_trait;
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::debug;

/// Remote user service client that communicates via gRPC.
pub struct RemoteUserServiceClient {
    client: user::user_service_client::UserServiceClient<Channel>,
}

impl RemoteUserServiceClient {
    /// Creates a new remote user service client.
    pub async fn connect(addr: &str) -> ArcanaResult<Self> {
        let client = user::user_service_client::UserServiceClient::connect(addr.to_string())
            .await
            .map_err(|e| ArcanaError::Internal(format!("Failed to connect to user service: {}", e)))?;

        Ok(Self { client })
    }

    /// Creates from an existing channel.
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            client: user::user_service_client::UserServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl UserService for RemoteUserServiceClient {
    async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Remote CreateUser: {}", request.username);

        let proto_request = user::CreateUserRequest {
            username: request.username,
            email: request.email,
            password: request.password,
            first_name: request.first_name,
            last_name: request.last_name,
        };

        let response = self
            .client
            .clone()
            .create_user(proto_request)
            .await
            .map_err(|e| ArcanaError::Internal(format!("gRPC error: {}", e)))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::Internal("No user in response".to_string()))?;

        Ok(from_proto_user(&user))
    }

    async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse> {
        debug!("Remote GetUser: {}", id);

        let response = self
            .client
            .clone()
            .get_user(user::GetUserRequest {
                user_id: id.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        Ok(from_proto_user(&user))
    }

    async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse> {
        debug!("Remote GetUserByUsername: {}", username);

        let response = self
            .client
            .clone()
            .get_user_by_username(user::GetUserByUsernameRequest {
                username: username.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::not_found("User", username))?;

        Ok(from_proto_user(&user))
    }

    async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse> {
        debug!("Remote ListUsers");

        let response = self
            .client
            .clone()
            .list_users(user::ListUsersRequest {
                page: Some(common::PageRequest {
                    page: page.page as i32,
                    size: page.size as i32,
                }),
                role_filter: None,
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let inner = response.into_inner();
        let page_info = inner.page_info.unwrap_or_default();

        Ok(UserListResponse {
            users: inner.users.iter().map(from_proto_user).collect(),
            page: page_info.page as usize,
            size: page_info.size as usize,
            total_elements: page_info.total_elements as u64,
            total_pages: page_info.total_pages as u64,
        })
    }

    async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Remote UpdateUser: {}", id);

        let response = self
            .client
            .clone()
            .update_user(user::UpdateUserRequest {
                user_id: id.to_string(),
                first_name: request.first_name,
                last_name: request.last_name,
                avatar_url: request.avatar_url,
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        Ok(from_proto_user(&user))
    }

    async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse> {
        debug!("Remote UpdateUserRole: {}", id);

        let response = self
            .client
            .clone()
            .update_user_role(user::UpdateUserRoleRequest {
                user_id: id.to_string(),
                role: to_proto_role(request.role) as i32,
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        Ok(from_proto_user(&user))
    }

    async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse> {
        debug!("Remote UpdateUserStatus: {}", id);

        let response = self
            .client
            .clone()
            .update_user_status(user::UpdateUserStatusRequest {
                user_id: id.to_string(),
                status: to_proto_status(request.status) as i32,
                reason: request.reason,
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let user = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        Ok(from_proto_user(&user))
    }

    async fn change_password(&self, _id: UserId, _request: ChangePasswordRequest) -> ArcanaResult<()> {
        // Password changes should be handled by auth service
        Err(ArcanaError::Internal("Password change not supported via remote client".to_string()))
    }

    async fn delete_user(&self, id: UserId) -> ArcanaResult<()> {
        debug!("Remote DeleteUser: {}", id);

        self.client
            .clone()
            .delete_user(user::DeleteUserRequest {
                user_id: id.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(())
    }

    async fn username_exists(&self, username: &str) -> ArcanaResult<bool> {
        let response = self
            .client
            .clone()
            .username_exists(user::UsernameExistsRequest {
                username: username.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().exists)
    }

    async fn email_exists(&self, email: &str) -> ArcanaResult<bool> {
        let response = self
            .client
            .clone()
            .email_exists(user::EmailExistsRequest {
                email: email.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().exists)
    }
}

/// Creates a shareable user service client.
pub async fn create_remote_user_service(addr: &str) -> ArcanaResult<Arc<dyn UserService>> {
    let client = RemoteUserServiceClient::connect(addr).await?;
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

fn from_proto_user(user: &user::User) -> UserResponse {
    UserResponse {
        id: UserId::parse(&user.id).unwrap_or_else(|_| UserId::new()),
        username: user.username.clone(),
        email: user.email.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        role: from_proto_role(user.role()),
        status: from_proto_status(user.status()),
        email_verified: user.email_verified,
        avatar_url: user.avatar_url.clone(),
        last_login_at: user.last_login_at.as_ref().map(|t| {
            chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                .unwrap_or_else(|| chrono::Utc::now())
        }),
        created_at: user
            .created_at
            .as_ref()
            .map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                    .unwrap_or_else(|| chrono::Utc::now())
            })
            .unwrap_or_else(|| chrono::Utc::now()),
    }
}

fn to_proto_role(role: arcana_domain::UserRole) -> user::UserRole {
    match role {
        arcana_domain::UserRole::User => user::UserRole::User,
        arcana_domain::UserRole::Moderator => user::UserRole::Moderator,
        arcana_domain::UserRole::Admin => user::UserRole::Admin,
        arcana_domain::UserRole::SuperAdmin => user::UserRole::SuperAdmin,
    }
}

fn from_proto_role(role: user::UserRole) -> arcana_domain::UserRole {
    match role {
        user::UserRole::User | user::UserRole::Unspecified => arcana_domain::UserRole::User,
        user::UserRole::Moderator => arcana_domain::UserRole::Moderator,
        user::UserRole::Admin => arcana_domain::UserRole::Admin,
        user::UserRole::SuperAdmin => arcana_domain::UserRole::SuperAdmin,
    }
}

fn to_proto_status(status: arcana_domain::UserStatus) -> user::UserStatus {
    match status {
        arcana_domain::UserStatus::PendingVerification => user::UserStatus::PendingVerification,
        arcana_domain::UserStatus::Active => user::UserStatus::Active,
        arcana_domain::UserStatus::Suspended => user::UserStatus::Suspended,
        arcana_domain::UserStatus::Locked => user::UserStatus::Locked,
        arcana_domain::UserStatus::Deleted => user::UserStatus::Deleted,
    }
}

fn from_proto_status(status: user::UserStatus) -> arcana_domain::UserStatus {
    match status {
        user::UserStatus::Unspecified | user::UserStatus::PendingVerification => {
            arcana_domain::UserStatus::PendingVerification
        }
        user::UserStatus::Active => arcana_domain::UserStatus::Active,
        user::UserStatus::Suspended => arcana_domain::UserStatus::Suspended,
        user::UserStatus::Locked => arcana_domain::UserStatus::Locked,
        user::UserStatus::Deleted => arcana_domain::UserStatus::Deleted,
    }
}
