//! `UserRepositoryImpl` — Repository layer implementation.
//!
//! Implements the [`UserRepository`] domain interface by coordinating
//! one or more [`UserDao`] instances.
//!
//! In the 4-layer hierarchy this sits between Service and DAO:
//!
//! ```text
//! Service
//!   ↓ Arc<dyn UserRepository>
//! UserRepositoryImpl          ← coordinates DAOs, applies domain logic
//!   ↓ Arc<dyn UserDao>
//! MySqlUserDaoImpl / RemoteUserDaoImpl / …
//!   ↓
//! MySQL / gRPC / REST / …
//! ```
//!
//! [`UserRepository`]: crate::traits::UserRepository
//! [`UserDao`]: crate::dao::UserDao

use crate::{dao::UserDao, traits::UserRepository};
use arcana_core::{ArcanaResult, Interface, Page, PageRequest, UserId};
use arcana_core::{User, UserRole};
use async_trait::async_trait;
use shaku::Component;
use std::sync::Arc;
use tracing::debug;

/// Repository implementation that orchestrates [`UserDao`] access.
///
/// To use multiple DAOs (e.g. primary + read replica, or local + remote
/// fallback), inject them here and coordinate reads/writes as needed.
///
/// [`UserDao`]: crate::dao::UserDao
#[derive(Component)]
#[shaku(interface = UserRepository)]
pub struct UserRepositoryImpl {
    /// Primary data access object.
    #[shaku(inject)]
    user_dao: Arc<dyn UserDao>,
}

impl UserRepositoryImpl {
    /// Creates a new `UserRepositoryImpl` with the given DAO.
    #[must_use]
    pub fn new(user_dao: Arc<dyn UserDao>) -> Self {
        Self { user_dao }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>> {
        debug!("Repository: find_by_id {}", id);
        self.user_dao.find_by_id(id).await
    }

    async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>> {
        debug!("Repository: find_by_username {}", username);
        self.user_dao.find_by_username(username).await
    }

    async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>> {
        debug!("Repository: find_by_email {}", email);
        self.user_dao.find_by_email(email).await
    }

    async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>> {
        debug!("Repository: find_by_username_or_email {}", identifier);
        self.user_dao.find_by_username_or_email(identifier).await
    }

    async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool> {
        self.user_dao.exists_by_username(username).await
    }

    async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
        self.user_dao.exists_by_email(email).await
    }

    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Repository: find_all page={}", page.page);
        self.user_dao.find_all(page).await
    }

    async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Repository: find_by_role {:?}", role);
        self.user_dao.find_by_role(role, page).await
    }

    async fn save(&self, user: &User) -> ArcanaResult<User> {
        debug!("Repository: save user {}", user.username);
        self.user_dao.save(user).await
    }

    async fn update(&self, user: &User) -> ArcanaResult<User> {
        debug!("Repository: update user {}", user.id);
        self.user_dao.update(user).await
    }

    async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
        debug!("Repository: delete user {}", id);
        self.user_dao.delete(id).await
    }

    async fn count(&self) -> ArcanaResult<u64> {
        self.user_dao.count().await
    }

    async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
        self.user_dao.count_by_role(role).await
    }
}

impl std::fmt::Debug for UserRepositoryImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserRepositoryImpl").finish_non_exhaustive()
    }
}
