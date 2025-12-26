//! Repository gRPC service implementation.
//!
//! This service exposes the repository layer via gRPC for distributed deployments.

use crate::proto::{common, repository, user as user_proto};
use arcana_core::{Page, PageRequest, UserId};
use arcana_domain::{Email, User, UserRole, UserStatus};
use arcana_repository::UserRepository;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, error};

/// Repository gRPC service implementation.
pub struct RepositoryGrpcService {
    user_repository: Arc<dyn UserRepository>,
}

impl RepositoryGrpcService {
    /// Creates a new repository gRPC service.
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }
}

#[tonic::async_trait]
impl repository::repository_service_server::RepositoryService for RepositoryGrpcService {
    async fn find_user_by_id(
        &self,
        request: Request<repository::FindUserByIdRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindUserById: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;

        let user = self
            .user_repository
            .find_by_id(user_id)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: user.map(|u| to_proto_user_data(&u)),
        }))
    }

    async fn find_user_by_username(
        &self,
        request: Request<repository::FindUserByUsernameRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindUserByUsername: {}", req.username);

        let user = self
            .user_repository
            .find_by_username(&req.username)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: user.map(|u| to_proto_user_data(&u)),
        }))
    }

    async fn find_user_by_email(
        &self,
        request: Request<repository::FindUserByEmailRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindUserByEmail: {}", req.email);

        let user = self
            .user_repository
            .find_by_email(&req.email)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: user.map(|u| to_proto_user_data(&u)),
        }))
    }

    async fn find_user_by_username_or_email(
        &self,
        request: Request<repository::FindUserByUsernameOrEmailRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindUserByUsernameOrEmail: {}", req.identifier);

        let user = self
            .user_repository
            .find_by_username_or_email(&req.identifier)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: user.map(|u| to_proto_user_data(&u)),
        }))
    }

    async fn exists_by_username(
        &self,
        request: Request<repository::ExistsByUsernameRequest>,
    ) -> Result<Response<repository::ExistsResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC ExistsByUsername: {}", req.username);

        let exists = self
            .user_repository
            .exists_by_username(&req.username)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::ExistsResult { exists }))
    }

    async fn exists_by_email(
        &self,
        request: Request<repository::ExistsByEmailRequest>,
    ) -> Result<Response<repository::ExistsResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC ExistsByEmail: {}", req.email);

        let exists = self
            .user_repository
            .exists_by_email(&req.email)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::ExistsResult { exists }))
    }

    async fn find_all_users(
        &self,
        request: Request<repository::FindAllUsersRequest>,
    ) -> Result<Response<repository::UserListResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindAllUsers");

        let page_request = req.page.map_or_else(
            || PageRequest::default(),
            |p| PageRequest::new(p.page as usize, p.size as usize),
        );

        let page = self
            .user_repository
            .find_all(page_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(to_proto_user_list_result(page)))
    }

    async fn find_users_by_role(
        &self,
        request: Request<repository::FindUsersByRoleRequest>,
    ) -> Result<Response<repository::UserListResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC FindUsersByRole");

        let role = from_proto_role(user_proto::UserRole::try_from(req.role).unwrap_or(user_proto::UserRole::User));
        let page_request = req.page.map_or_else(
            || PageRequest::default(),
            |p| PageRequest::new(p.page as usize, p.size as usize),
        );

        let page = self
            .user_repository
            .find_by_role(role, page_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(to_proto_user_list_result(page)))
    }

    async fn save_user(
        &self,
        request: Request<repository::SaveUserRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC SaveUser");

        let user_data = req.user.ok_or_else(|| Status::invalid_argument("User is required"))?;
        let user = from_proto_user_data(&user_data);

        let saved = self
            .user_repository
            .save(&user)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: Some(to_proto_user_data(&saved)),
        }))
    }

    async fn update_user(
        &self,
        request: Request<repository::UpdateUserDataRequest>,
    ) -> Result<Response<repository::UserResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC UpdateUser");

        let user_data = req.user.ok_or_else(|| Status::invalid_argument("User is required"))?;
        let user = from_proto_user_data(&user_data);

        let updated = self
            .user_repository
            .update(&user)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::UserResult {
            user: Some(to_proto_user_data(&updated)),
        }))
    }

    async fn delete_user(
        &self,
        request: Request<repository::DeleteUserDataRequest>,
    ) -> Result<Response<repository::DeleteResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC DeleteUser: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;

        let deleted = self
            .user_repository
            .delete(user_id)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::DeleteResult { deleted }))
    }

    async fn count_users(
        &self,
        _request: Request<repository::CountUsersRequest>,
    ) -> Result<Response<repository::CountResult>, Status> {
        debug!("gRPC CountUsers");

        let count = self
            .user_repository
            .count()
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::CountResult { count }))
    }

    async fn count_users_by_role(
        &self,
        request: Request<repository::CountUsersByRoleRequest>,
    ) -> Result<Response<repository::CountResult>, Status> {
        let req = request.into_inner();
        debug!("gRPC CountUsersByRole");

        let role = from_proto_role(user_proto::UserRole::try_from(req.role).unwrap_or(user_proto::UserRole::User));

        let count = self
            .user_repository
            .count_by_role(role)
            .await
            .map_err(to_status)?;

        Ok(Response::new(repository::CountResult { count }))
    }
}

// Helper functions

fn parse_user_id(id: &str) -> Result<UserId, Status> {
    uuid::Uuid::parse_str(id)
        .map(UserId::from)
        .map_err(|e| Status::invalid_argument(format!("Invalid user ID: {}", e)))
}

fn to_status(err: arcana_core::ArcanaError) -> Status {
    use arcana_core::ArcanaError;

    error!("gRPC repository error: {:?}", err);

    match err {
        ArcanaError::NotFound { .. } => Status::not_found(err.to_string()),
        ArcanaError::Validation(msg) => Status::invalid_argument(msg),
        ArcanaError::Conflict(msg) => Status::already_exists(msg),
        ArcanaError::Database(msg) => Status::internal(msg),
        _ => Status::internal(err.to_string()),
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

fn to_proto_user_list_result(page: Page<User>) -> repository::UserListResult {
    repository::UserListResult {
        users: page.content.iter().map(to_proto_user_data).collect(),
        page_info: Some(common::PageInfo {
            page: page.info.page as i32,
            size: page.info.size as i32,
            total_elements: page.info.total_elements as i64,
            total_pages: page.info.total_pages as i64,
            first: page.info.first,
            last: page.info.last,
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
