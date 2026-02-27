//! UserDao trait — low-level user data access abstraction.
//!
//! This is the DAO (Data Access Object) interface for user data.
//! Implementations connect directly to a single data source
//! (MySQL, PostgreSQL, REST API, gRPC remote, etc.).
//!
//! [`UserRepository`] uses one or more `UserDao` instances to
//! fulfil domain-level operations.
//!
//! [`UserRepository`]: crate::traits::UserRepository

use arcana_core::{ArcanaResult, Interface, Page, PageRequest, UserId};
use arcana_core::{User, UserRole};
use async_trait::async_trait;

/// Low-level user data access object.
///
/// Each implementation targets a single data source.
/// Use [`crate::UserRepositoryImpl`] to coordinate multiple DAOs.
#[async_trait]
pub trait UserDao: Interface + Send + Sync {
    /// Finds a user by ID.
    async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>>;

    /// Finds a user by username.
    async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>>;

    /// Finds a user by email.
    async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>>;

    /// Finds a user by username or email.
    async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>>;

    /// Checks if a username already exists.
    async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool>;

    /// Checks if an email already exists.
    async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool>;

    /// Finds all users with pagination.
    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>>;

    /// Finds users by role.
    async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>>;

    /// Persists a new user.
    async fn save(&self, user: &User) -> ArcanaResult<User>;

    /// Updates an existing user.
    async fn update(&self, user: &User) -> ArcanaResult<User>;

    /// Deletes a user by ID. Returns `true` if deleted.
    async fn delete(&self, id: UserId) -> ArcanaResult<bool>;

    /// Counts all users.
    async fn count(&self) -> ArcanaResult<u64>;

    /// Counts users by role.
    async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64>;
}
