//! Remote repository client via gRPC.

use crate::proto::{common, repository, user as user_proto};
use arcana_core::{ArcanaError, ArcanaResult, Page, PageRequest, UserId};
use arcana_domain::{Email, User, UserRole, UserStatus};
use arcana_repository::UserRepository;
use async_trait::async_trait;
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::debug;

/// Remote user repository client that communicates via gRPC.
pub struct RemoteUserRepository {
    client: repository::repository_service_client::RepositoryServiceClient<Channel>,
}

impl RemoteUserRepository {
    /// Creates a new remote repository client.
    pub async fn connect(addr: &str) -> ArcanaResult<Self> {
        let client = repository::repository_service_client::RepositoryServiceClient::connect(addr.to_string())
            .await
            .map_err(|e| ArcanaError::Internal(format!("Failed to connect to repository service: {}", e)))?;

        Ok(Self { client })
    }

    /// Creates from an existing channel.
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            client: repository::repository_service_client::RepositoryServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl UserRepository for RemoteUserRepository {
    async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>> {
        debug!("Remote FindUserById: {}", id);

        let response = self
            .client
            .clone()
            .find_user_by_id(repository::FindUserByIdRequest {
                user_id: id.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().user.map(|u| from_proto_user_data(&u)))
    }

    async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>> {
        debug!("Remote FindUserByUsername: {}", username);

        let response = self
            .client
            .clone()
            .find_user_by_username(repository::FindUserByUsernameRequest {
                username: username.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().user.map(|u| from_proto_user_data(&u)))
    }

    async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>> {
        debug!("Remote FindUserByEmail: {}", email);

        let response = self
            .client
            .clone()
            .find_user_by_email(repository::FindUserByEmailRequest {
                email: email.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().user.map(|u| from_proto_user_data(&u)))
    }

    async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>> {
        debug!("Remote FindUserByUsernameOrEmail: {}", identifier);

        let response = self
            .client
            .clone()
            .find_user_by_username_or_email(repository::FindUserByUsernameOrEmailRequest {
                identifier: identifier.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().user.map(|u| from_proto_user_data(&u)))
    }

    async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool> {
        debug!("Remote ExistsByUsername: {}", username);

        let response = self
            .client
            .clone()
            .exists_by_username(repository::ExistsByUsernameRequest {
                username: username.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().exists)
    }

    async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
        debug!("Remote ExistsByEmail: {}", email);

        let response = self
            .client
            .clone()
            .exists_by_email(repository::ExistsByEmailRequest {
                email: email.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().exists)
    }

    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Remote FindAllUsers");

        let response = self
            .client
            .clone()
            .find_all_users(repository::FindAllUsersRequest {
                page: Some(common::PageRequest {
                    page: page.page as i32,
                    size: page.size as i32,
                }),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let inner = response.into_inner();
        let page_info = inner.page_info.unwrap_or_default();
        let users: Vec<User> = inner.users.iter().map(from_proto_user_data).collect();

        Ok(Page::new(
            users,
            page_info.page as usize,
            page_info.size as usize,
            page_info.total_elements as u64,
        ))
    }

    async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Remote FindUsersByRole: {:?}", role);

        let response = self
            .client
            .clone()
            .find_users_by_role(repository::FindUsersByRoleRequest {
                role: to_proto_role(role) as i32,
                page: Some(common::PageRequest {
                    page: page.page as i32,
                    size: page.size as i32,
                }),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let inner = response.into_inner();
        let page_info = inner.page_info.unwrap_or_default();
        let users: Vec<User> = inner.users.iter().map(from_proto_user_data).collect();

        Ok(Page::new(
            users,
            page_info.page as usize,
            page_info.size as usize,
            page_info.total_elements as u64,
        ))
    }

    async fn save(&self, user: &User) -> ArcanaResult<User> {
        debug!("Remote SaveUser: {}", user.id);

        let response = self
            .client
            .clone()
            .save_user(repository::SaveUserRequest {
                user: Some(to_proto_user_data(user)),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let saved = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::Internal("No user in save response".to_string()))?;

        Ok(from_proto_user_data(&saved))
    }

    async fn update(&self, user: &User) -> ArcanaResult<User> {
        debug!("Remote UpdateUser: {}", user.id);

        let response = self
            .client
            .clone()
            .update_user(repository::UpdateUserDataRequest {
                user: Some(to_proto_user_data(user)),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        let updated = response
            .into_inner()
            .user
            .ok_or_else(|| ArcanaError::Internal("No user in update response".to_string()))?;

        Ok(from_proto_user_data(&updated))
    }

    async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
        debug!("Remote DeleteUser: {}", id);

        let response = self
            .client
            .clone()
            .delete_user(repository::DeleteUserDataRequest {
                user_id: id.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().deleted)
    }

    async fn count(&self) -> ArcanaResult<u64> {
        debug!("Remote CountUsers");

        let response = self
            .client
            .clone()
            .count_users(repository::CountUsersRequest {})
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().count)
    }

    async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
        debug!("Remote CountUsersByRole: {:?}", role);

        let response = self
            .client
            .clone()
            .count_users_by_role(repository::CountUsersByRoleRequest {
                role: to_proto_role(role) as i32,
            })
            .await
            .map_err(|e| map_grpc_error(e))?;

        Ok(response.into_inner().count)
    }
}

/// Creates a shareable remote user repository.
pub async fn create_remote_user_repository(addr: &str) -> ArcanaResult<Arc<dyn UserRepository>> {
    let client = RemoteUserRepository::connect(addr).await?;
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

fn from_proto_user_data(user: &repository::UserData) -> User {
    let id = UserId::parse(&user.id).unwrap_or_else(|_| UserId::new());
    let email = Email::new_unchecked(user.email.clone());
    let role = from_proto_role(user_proto::UserRole::try_from(user.role).unwrap_or(user_proto::UserRole::User));
    let status = from_proto_status(user_proto::UserStatus::try_from(user.status).unwrap_or(user_proto::UserStatus::Active));

    let created_at = user
        .created_at
        .as_ref()
        .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .unwrap_or_else(|| chrono::Utc::now());

    let updated_at = user
        .updated_at
        .as_ref()
        .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .unwrap_or_else(|| chrono::Utc::now());

    let last_login_at = user
        .last_login_at
        .as_ref()
        .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32));

    User {
        id,
        username: user.username.clone(),
        email,
        password_hash: user.password_hash.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        role,
        status,
        email_verified: user.email_verified,
        avatar_url: user.avatar_url.clone(),
        last_login_at,
        created_at,
        updated_at,
    }
}

fn to_proto_user_data(user: &User) -> repository::UserData {
    repository::UserData {
        id: user.id.to_string(),
        username: user.username.clone(),
        email: user.email.as_str().to_string(),
        password_hash: user.password_hash.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        role: to_proto_role(user.role) as i32,
        status: to_proto_status(user.status) as i32,
        email_verified: user.email_verified,
        avatar_url: user.avatar_url.clone(),
        last_login_at: user.last_login_at.map(|dt| common::Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }),
        created_at: Some(common::Timestamp {
            seconds: user.created_at.timestamp(),
            nanos: user.created_at.timestamp_subsec_nanos() as i32,
        }),
        updated_at: Some(common::Timestamp {
            seconds: user.updated_at.timestamp(),
            nanos: user.updated_at.timestamp_subsec_nanos() as i32,
        }),
    }
}

fn to_proto_role(role: UserRole) -> user_proto::UserRole {
    match role {
        UserRole::User => user_proto::UserRole::User,
        UserRole::Moderator => user_proto::UserRole::Moderator,
        UserRole::Admin => user_proto::UserRole::Admin,
        UserRole::SuperAdmin => user_proto::UserRole::SuperAdmin,
    }
}

fn from_proto_role(role: user_proto::UserRole) -> UserRole {
    match role {
        user_proto::UserRole::User | user_proto::UserRole::Unspecified => UserRole::User,
        user_proto::UserRole::Moderator => UserRole::Moderator,
        user_proto::UserRole::Admin => UserRole::Admin,
        user_proto::UserRole::SuperAdmin => UserRole::SuperAdmin,
    }
}

fn to_proto_status(status: UserStatus) -> user_proto::UserStatus {
    match status {
        UserStatus::PendingVerification => user_proto::UserStatus::PendingVerification,
        UserStatus::Active => user_proto::UserStatus::Active,
        UserStatus::Suspended => user_proto::UserStatus::Suspended,
        UserStatus::Locked => user_proto::UserStatus::Locked,
        UserStatus::Deleted => user_proto::UserStatus::Deleted,
    }
}

fn from_proto_status(status: user_proto::UserStatus) -> UserStatus {
    match status {
        user_proto::UserStatus::Unspecified | user_proto::UserStatus::PendingVerification => {
            UserStatus::PendingVerification
        }
        user_proto::UserStatus::Active => UserStatus::Active,
        user_proto::UserStatus::Suspended => UserStatus::Suspended,
        user_proto::UserStatus::Locked => UserStatus::Locked,
        user_proto::UserStatus::Deleted => UserStatus::Deleted,
    }
}
