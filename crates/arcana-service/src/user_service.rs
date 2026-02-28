//! User service trait definition.

use crate::dto::{
    ChangePasswordRequest, CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest,
    UpdateUserStatusRequest, UserListResponse, UserResponse,
};
use arcana_core::{ArcanaResult, Interface, PageRequest, UserId};
use async_trait::async_trait;

/// User service trait.
#[async_trait]
pub trait UserService: Interface + Send + Sync {
    /// Creates a new user.
    async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse>;

    /// Gets a user by ID.
    async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse>;

    /// Gets a user by username.
    async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse>;

    /// Lists all users with pagination.
    async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse>;

    /// Updates a user's profile.
    async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse>;

    /// Updates a user's role.
    async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse>;

    /// Updates a user's status.
    async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse>;

    /// Changes a user's password.
    async fn change_password(&self, id: UserId, request: ChangePasswordRequest) -> ArcanaResult<()>;

    /// Deletes a user.
    async fn delete_user(&self, id: UserId) -> ArcanaResult<()>;

    /// Checks if a username exists.
    async fn username_exists(&self, username: &str) -> ArcanaResult<bool>;

    /// Checks if an email exists.
    async fn email_exists(&self, email: &str) -> ArcanaResult<bool>;
}
