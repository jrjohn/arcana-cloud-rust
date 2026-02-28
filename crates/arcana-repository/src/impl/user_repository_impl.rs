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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::UserDao;
    use crate::traits::UserRepository;
    use arcana_core::{ArcanaResult, Email, Page, PageRequest, User, UserRole, UserId};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // =========================================================================
    // Mock DAO implementation
    // =========================================================================

    struct MockUserDao {
        users: Mutex<HashMap<UserId, User>>,
    }

    impl std::fmt::Debug for MockUserDao {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockUserDao").finish_non_exhaustive()
        }
    }

    impl MockUserDao {
        fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        fn with_user(user: User) -> Self {
            let dao = Self::new();
            dao.users.lock().unwrap().insert(user.id, user);
            dao
        }

        fn with_users(users: Vec<User>) -> Self {
            let dao = Self::new();
            for u in users {
                dao.users.lock().unwrap().insert(u.id, u);
            }
            dao
        }
    }

    #[async_trait]
    impl UserDao for MockUserDao {
        async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().get(&id).cloned())
        }

        async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.username == username)
                .cloned())
        }

        async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.email.as_str() == email)
                .cloned())
        }

        async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>> {
            Ok(self.users.lock().unwrap().values()
                .find(|u| u.username == identifier || u.email.as_str() == identifier)
                .cloned())
        }

        async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values().any(|u| u.username == username))
        }

        async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().values()
                .any(|u| u.email.as_str().to_lowercase() == email.to_lowercase()))
        }

        async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
            let total = users.len() as u64;
            let start = page.offset();
            let end = std::cmp::min(start + page.limit(), users.len());
            let items = if start < users.len() { users[start..end].to_vec() } else { vec![] };
            Ok(Page::new(items, page.page, page.size, total))
        }

        async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>> {
            let users: Vec<User> = self.users.lock().unwrap().values()
                .filter(|u| u.role == role)
                .cloned()
                .collect();
            let total = users.len() as u64;
            let start = page.offset();
            let end = std::cmp::min(start + page.limit(), users.len());
            let items = if start < users.len() { users[start..end].to_vec() } else { vec![] };
            Ok(Page::new(items, page.page, page.size, total))
        }

        async fn save(&self, user: &User) -> ArcanaResult<User> {
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn update(&self, user: &User) -> ArcanaResult<User> {
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
            Ok(self.users.lock().unwrap().remove(&id).is_some())
        }

        async fn count(&self) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().len() as u64)
        }

        async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
            Ok(self.users.lock().unwrap().values()
                .filter(|u| u.role == role)
                .count() as u64)
        }
    }

    // =========================================================================
    // Helper functions
    // =========================================================================

    fn create_test_user(username: &str, email: &str) -> User {
        let mut user = User::new(
            username.to_string(),
            Email::new_unchecked(email.to_string()),
            "hashed_password".to_string(),
            Some("Test".to_string()),
            Some("User".to_string()),
        );
        user.activate();
        user
    }

    fn create_repo(dao: MockUserDao) -> UserRepositoryImpl {
        UserRepositoryImpl::new(Arc::new(dao))
    }

    // =========================================================================
    // UserRepositoryImpl unit tests — verifies delegation to DAO
    // =========================================================================

    #[tokio::test]
    async fn test_find_by_id_delegates_to_dao() {
        let user = create_test_user("alice", "alice@example.com");
        let user_id = user.id;
        let repo = create_repo(MockUserDao::with_user(user));

        let result = repo.find_by_id(user_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "alice");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let repo = create_repo(MockUserDao::new());
        let result = repo.find_by_id(UserId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_username_delegates_to_dao() {
        let user = create_test_user("bob", "bob@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        let result = repo.find_by_username("bob").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email.as_str(), "bob@example.com");
    }

    #[tokio::test]
    async fn test_find_by_username_not_found() {
        let repo = create_repo(MockUserDao::new());
        let result = repo.find_by_username("nobody").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_email_delegates_to_dao() {
        let user = create_test_user("carol", "carol@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        let result = repo.find_by_email("carol@example.com").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "carol");
    }

    #[tokio::test]
    async fn test_find_by_email_not_found() {
        let repo = create_repo(MockUserDao::new());
        let result = repo.find_by_email("ghost@example.com").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_username_or_email_with_username() {
        let user = create_test_user("dave", "dave@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        let result = repo.find_by_username_or_email("dave").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_find_by_username_or_email_with_email() {
        let user = create_test_user("eve", "eve@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        let result = repo.find_by_username_or_email("eve@example.com").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "eve");
    }

    #[tokio::test]
    async fn test_exists_by_username_true() {
        let user = create_test_user("frank", "frank@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        assert!(repo.exists_by_username("frank").await.unwrap());
    }

    #[tokio::test]
    async fn test_exists_by_username_false() {
        let repo = create_repo(MockUserDao::new());
        assert!(!repo.exists_by_username("nobody").await.unwrap());
    }

    #[tokio::test]
    async fn test_exists_by_email_true() {
        let user = create_test_user("grace", "grace@example.com");
        let repo = create_repo(MockUserDao::with_user(user));

        assert!(repo.exists_by_email("grace@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_exists_by_email_false() {
        let repo = create_repo(MockUserDao::new());
        assert!(!repo.exists_by_email("nobody@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_find_all_empty() {
        let repo = create_repo(MockUserDao::new());
        let page = repo.find_all(PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(page.content.len(), 0);
        assert_eq!(page.info.total_elements, 0);
    }

    #[tokio::test]
    async fn test_find_all_with_users() {
        let users = vec![
            create_test_user("u1", "u1@example.com"),
            create_test_user("u2", "u2@example.com"),
        ];
        let repo = create_repo(MockUserDao::with_users(users));

        let page = repo.find_all(PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(page.content.len(), 2);
        assert_eq!(page.info.total_elements, 2);
    }

    #[tokio::test]
    async fn test_find_by_role_delegates_to_dao() {
        let mut admin = create_test_user("admin", "admin@example.com");
        admin.change_role(UserRole::Admin);
        let regular = create_test_user("user", "user@example.com");
        let repo = create_repo(MockUserDao::with_users(vec![admin, regular]));

        let admins = repo.find_by_role(UserRole::Admin, PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(admins.content.len(), 1);
        assert_eq!(admins.content[0].username, "admin");
    }

    #[tokio::test]
    async fn test_save_delegates_to_dao() {
        let user = create_test_user("henry", "henry@example.com");
        let user_id = user.id;
        let repo = create_repo(MockUserDao::new());

        let saved = repo.save(&user).await.unwrap();
        assert_eq!(saved.id, user_id);
        assert_eq!(saved.username, "henry");

        // Verify it's findable
        let found = repo.find_by_id(user_id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_update_delegates_to_dao() {
        let mut user = create_test_user("ivan", "ivan@example.com");
        let user_id = user.id;
        let repo = create_repo(MockUserDao::with_user(user.clone()));

        user.update_profile(Some("Ivan".to_string()), Some("Updated".to_string()), None);
        let updated = repo.update(&user).await.unwrap();
        assert_eq!(updated.first_name, Some("Ivan".to_string()));

        let found = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.first_name, Some("Ivan".to_string()));
    }

    #[tokio::test]
    async fn test_delete_delegates_to_dao() {
        let user = create_test_user("jack", "jack@example.com");
        let user_id = user.id;
        let repo = create_repo(MockUserDao::with_user(user));

        assert!(repo.find_by_id(user_id).await.unwrap().is_some());

        let deleted = repo.delete(user_id).await.unwrap();
        assert!(deleted);

        assert!(repo.find_by_id(user_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let repo = create_repo(MockUserDao::new());
        let deleted = repo.delete(UserId::new()).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_count_delegates_to_dao() {
        let users = vec![
            create_test_user("u1", "u1@example.com"),
            create_test_user("u2", "u2@example.com"),
            create_test_user("u3", "u3@example.com"),
        ];
        let repo = create_repo(MockUserDao::with_users(users));

        assert_eq!(repo.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_count_empty() {
        let repo = create_repo(MockUserDao::new());
        assert_eq!(repo.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_by_role_delegates_to_dao() {
        let mut admin = create_test_user("admin1", "admin1@example.com");
        admin.change_role(UserRole::Admin);
        let regular1 = create_test_user("user1", "user1@example.com");
        let regular2 = create_test_user("user2", "user2@example.com");
        let repo = create_repo(MockUserDao::with_users(vec![admin, regular1, regular2]));

        assert_eq!(repo.count_by_role(UserRole::Admin).await.unwrap(), 1);
        assert_eq!(repo.count_by_role(UserRole::User).await.unwrap(), 2);
        assert_eq!(repo.count_by_role(UserRole::SuperAdmin).await.unwrap(), 0);
    }

    #[test]
    fn test_user_repository_impl_new() {
        let dao = Arc::new(MockUserDao::new());
        let _repo = UserRepositoryImpl::new(dao);
        // Just verifies construction doesn't panic
    }

    #[test]
    fn test_user_repository_impl_debug() {
        let dao = Arc::new(MockUserDao::new());
        let repo = UserRepositoryImpl::new(dao);
        let debug_str = format!("{:?}", repo);
        assert!(debug_str.contains("UserRepositoryImpl"));
    }
}
