//! User gRPC service implementation.

use crate::proto::{common, user};
use arcana_core::{PageRequest, UserId};
use arcana_service::dto::{CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest, UpdateUserStatusRequest};
use arcana_service::UserService;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, error};
use uuid::Uuid;

/// User gRPC service implementation.
pub struct UserGrpcService {
    user_service: Arc<dyn UserService>,
}

impl UserGrpcService {
    /// Creates a new user gRPC service.
    pub fn new(user_service: Arc<dyn UserService>) -> Self {
        Self { user_service }
    }
}

#[tonic::async_trait]
impl user::user_service_server::UserService for UserGrpcService {
    async fn get_user(
        &self,
        request: Request<user::GetUserRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC GetUser: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;

        let response = self
            .user_service
            .get_user(user_id)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn get_user_by_username(
        &self,
        request: Request<user::GetUserByUsernameRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC GetUserByUsername: {}", req.username);

        let response = self
            .user_service
            .get_user_by_username(&req.username)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn list_users(
        &self,
        request: Request<user::ListUsersRequest>,
    ) -> Result<Response<user::ListUsersResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC ListUsers");

        let page_request = req.page.map_or_else(
            || PageRequest::default(),
            |p| PageRequest::new(p.page as usize, p.size as usize),
        );

        let response = self
            .user_service
            .list_users(page_request)
            .await
            .map_err(to_status)?;

        let is_last = response.page >= response.total_pages.saturating_sub(1) as usize;

        Ok(Response::new(user::ListUsersResponse {
            users: response.users.iter().map(to_proto_user).collect(),
            page_info: Some(common::PageInfo {
                page: response.page as i32,
                size: response.size as i32,
                total_elements: response.total_elements as i64,
                total_pages: response.total_pages as i64,
                first: response.page == 0,
                last: is_last,
            }),
        }))
    }

    async fn create_user(
        &self,
        request: Request<user::CreateUserRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC CreateUser: {}", req.username);

        let create_request = CreateUserRequest {
            username: req.username,
            email: req.email,
            password: req.password,
            first_name: req.first_name.filter(|s| !s.is_empty()),
            last_name: req.last_name.filter(|s| !s.is_empty()),
        };

        let response = self
            .user_service
            .create_user(create_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn update_user(
        &self,
        request: Request<user::UpdateUserRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC UpdateUser: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;

        let update_request = UpdateUserRequest {
            first_name: req.first_name.filter(|s| !s.is_empty()),
            last_name: req.last_name.filter(|s| !s.is_empty()),
            avatar_url: req.avatar_url.filter(|s| !s.is_empty()),
        };

        let response = self
            .user_service
            .update_user(user_id, update_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn delete_user(
        &self,
        request: Request<user::DeleteUserRequest>,
    ) -> Result<Response<common::Empty>, Status> {
        let req = request.into_inner();
        debug!("gRPC DeleteUser: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;

        self.user_service
            .delete_user(user_id)
            .await
            .map_err(to_status)?;

        Ok(Response::new(common::Empty {}))
    }

    async fn update_user_role(
        &self,
        request: Request<user::UpdateUserRoleRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC UpdateUserRole: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;
        let role = from_proto_role(req.role());

        let update_request = UpdateUserRoleRequest { role };

        let response = self
            .user_service
            .update_user_role(user_id, update_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn update_user_status(
        &self,
        request: Request<user::UpdateUserStatusRequest>,
    ) -> Result<Response<user::UserResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC UpdateUserStatus: {}", req.user_id);

        let user_id = parse_user_id(&req.user_id)?;
        let status = from_proto_status(req.status());

        let update_request = UpdateUserStatusRequest {
            status,
            reason: req.reason.filter(|s| !s.is_empty()),
        };

        let response = self
            .user_service
            .update_user_status(user_id, update_request)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::UserResponse {
            user: Some(to_proto_user(&response)),
        }))
    }

    async fn username_exists(
        &self,
        request: Request<user::UsernameExistsRequest>,
    ) -> Result<Response<user::ExistsResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC UsernameExists: {}", req.username);

        let exists = self
            .user_service
            .username_exists(&req.username)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::ExistsResponse { exists }))
    }

    async fn email_exists(
        &self,
        request: Request<user::EmailExistsRequest>,
    ) -> Result<Response<user::ExistsResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC EmailExists: {}", req.email);

        let exists = self
            .user_service
            .email_exists(&req.email)
            .await
            .map_err(to_status)?;

        Ok(Response::new(user::ExistsResponse { exists }))
    }
}

// Helper functions

fn parse_user_id(id: &str) -> Result<UserId, Status> {
    Uuid::parse_str(id)
        .map(UserId::from)
        .map_err(|e| Status::invalid_argument(format!("Invalid user ID: {}", e)))
}

fn to_status(err: arcana_core::ArcanaError) -> Status {
    use arcana_core::ArcanaError;

    error!("gRPC error: {:?}", err);

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

fn to_proto_user(user: &arcana_service::dto::UserResponse) -> user::User {
    user::User {
        id: user.id.to_string(),
        username: user.username.clone(),
        email: user.email.clone(),
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
            seconds: user.created_at.timestamp(), // Using created_at as fallback
            nanos: user.created_at.timestamp_subsec_nanos() as i32,
        }),
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
