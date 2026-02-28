//! # Arcana Repository
//!
//! Four-layer data access hierarchy:
//!
//! ```text
//! Service
//!   ↓  Arc<dyn UserRepository>  (domain interface)
//! UserRepositoryImpl            (repository impl — coordinates DAOs)
//!   ↓  Arc<dyn UserDao>         (DAO interface)
//! MySqlUserDaoImpl              (DAO impl — MySQL / SQLx)
//!   ↓
//! MySQL
//! ```
//!
//! ## Structure
//!
//! ```text
//! src/
//!   traits.rs                    ← UserRepository trait
//!   impl/
//!     mod.rs
//!     user_repository_impl.rs    ← UserRepositoryImpl
//!   dao/
//!     user_dao.rs                ← UserDao trait
//!     impl/
//!       mod.rs
//!       mysql/
//!         user_dao_impl.rs       ← MySqlUserDaoImpl
//! ```
//!
//! The existing [`MySqlUserRepository`] is retained for backward
//! compatibility and for direct use in the distributed gRPC module.

pub mod dao;
pub mod pool;
pub mod mysql;
pub mod traits;
pub mod r#impl;

pub use dao::UserDao;
pub use pool::*;
pub use traits::*;
pub use r#impl::UserRepositoryImpl;

// Re-export DAO and MySQL implementations for convenience
pub use dao::MySqlUserDaoImpl;
pub use mysql::*;

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::{ArcanaResult, Page, PageRequest, UserId};
    use arcana_core::{Email, User, UserRole, UserStatus};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory mock repository for testing.
    struct InMemoryUserRepository {
        users: Mutex<HashMap<UserId, User>>,
    }

    impl InMemoryUserRepository {
        fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        fn with_users(users: Vec<User>) -> Self {
            let repo = Self::new();
            for user in users {
                repo.users.lock().unwrap().insert(user.id, user);
            }
            repo
        }
    }

    #[async_trait]
    impl UserRepository for InMemoryUserRepository {
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

    // =============================================================================
    // UserRepository Tests
    // =============================================================================

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = InMemoryUserRepository::new();
        let user = create_test_user("testuser", "test@example.com");
        let user_id = user.id;

        repo.save(&user).await.unwrap();

        let found = repo.find_by_id(user_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let repo = InMemoryUserRepository::new();
        let result = repo.find_by_id(UserId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_username() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        let found = repo.find_by_username("testuser").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email.as_str(), "test@example.com");
    }

    #[tokio::test]
    async fn test_find_by_username_not_found() {
        let repo = InMemoryUserRepository::new();
        let result = repo.find_by_username("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_email() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        let found = repo.find_by_email("test@example.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_username_or_email_with_username() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        let found = repo.find_by_username_or_email("testuser").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_find_by_username_or_email_with_email() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        let found = repo.find_by_username_or_email("test@example.com").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_exists_by_username() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        assert!(repo.exists_by_username("testuser").await.unwrap());
        assert!(!repo.exists_by_username("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_exists_by_email() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        assert!(repo.exists_by_email("test@example.com").await.unwrap());
        assert!(!repo.exists_by_email("nonexistent@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_exists_by_email_case_insensitive() {
        let user = create_test_user("testuser", "test@example.com");
        let repo = InMemoryUserRepository::with_users(vec![user]);

        assert!(repo.exists_by_email("TEST@EXAMPLE.COM").await.unwrap());
    }

    #[tokio::test]
    async fn test_find_all_empty() {
        let repo = InMemoryUserRepository::new();
        let page = repo.find_all(PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(page.content.len(), 0);
        assert_eq!(page.info.total_elements, 0);
    }

    #[tokio::test]
    async fn test_find_all_with_users() {
        let users = vec![
            create_test_user("user1", "user1@example.com"),
            create_test_user("user2", "user2@example.com"),
            create_test_user("user3", "user3@example.com"),
        ];
        let repo = InMemoryUserRepository::with_users(users);

        let page = repo.find_all(PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(page.content.len(), 3);
        assert_eq!(page.info.total_elements, 3);
    }

    #[tokio::test]
    async fn test_find_all_with_pagination() {
        let users = vec![
            create_test_user("user1", "user1@example.com"),
            create_test_user("user2", "user2@example.com"),
            create_test_user("user3", "user3@example.com"),
        ];
        let repo = InMemoryUserRepository::with_users(users);

        let page = repo.find_all(PageRequest::new(0, 2)).await.unwrap();
        assert_eq!(page.content.len(), 2);
        assert_eq!(page.info.total_elements, 3);

        let page2 = repo.find_all(PageRequest::new(1, 2)).await.unwrap();
        assert_eq!(page2.content.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_role() {
        let mut admin = create_test_user("admin", "admin@example.com");
        admin.change_role(UserRole::Admin);
        let user = create_test_user("user", "user@example.com");
        let repo = InMemoryUserRepository::with_users(vec![admin, user]);

        let admins = repo.find_by_role(UserRole::Admin, PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(admins.content.len(), 1);
        assert_eq!(admins.content[0].username, "admin");

        let users = repo.find_by_role(UserRole::User, PageRequest::new(0, 10)).await.unwrap();
        assert_eq!(users.content.len(), 1);
        assert_eq!(users.content[0].username, "user");
    }

    #[tokio::test]
    async fn test_update_user() {
        let mut user = create_test_user("testuser", "test@example.com");
        let user_id = user.id;
        let repo = InMemoryUserRepository::with_users(vec![user.clone()]);

        user.update_profile(Some("Updated".to_string()), Some("Name".to_string()), None);
        repo.update(&user).await.unwrap();

        let found = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.first_name, Some("Updated".to_string()));
        assert_eq!(found.last_name, Some("Name".to_string()));
    }

    #[tokio::test]
    async fn test_delete_user() {
        let user = create_test_user("testuser", "test@example.com");
        let user_id = user.id;
        let repo = InMemoryUserRepository::with_users(vec![user]);

        assert!(repo.find_by_id(user_id).await.unwrap().is_some());

        let deleted = repo.delete(user_id).await.unwrap();
        assert!(deleted);

        assert!(repo.find_by_id(user_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let repo = InMemoryUserRepository::new();
        let deleted = repo.delete(UserId::new()).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_count_users() {
        let users = vec![
            create_test_user("user1", "user1@example.com"),
            create_test_user("user2", "user2@example.com"),
        ];
        let repo = InMemoryUserRepository::with_users(users);

        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_count_by_role() {
        let mut admin1 = create_test_user("admin1", "admin1@example.com");
        admin1.change_role(UserRole::Admin);
        let mut admin2 = create_test_user("admin2", "admin2@example.com");
        admin2.change_role(UserRole::Admin);
        let user = create_test_user("user", "user@example.com");
        let repo = InMemoryUserRepository::with_users(vec![admin1, admin2, user]);

        assert_eq!(repo.count_by_role(UserRole::Admin).await.unwrap(), 2);
        assert_eq!(repo.count_by_role(UserRole::User).await.unwrap(), 1);
        assert_eq!(repo.count_by_role(UserRole::SuperAdmin).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_user_status_changes() {
        let mut user = create_test_user("testuser", "test@example.com");
        assert_eq!(user.status, UserStatus::Active);

        user.suspend();
        assert_eq!(user.status, UserStatus::Suspended);

        user.status = UserStatus::Locked;
        assert_eq!(user.status, UserStatus::Locked);

        user.activate();
        assert_eq!(user.status, UserStatus::Active);
    }

    #[tokio::test]
    async fn test_user_role_changes() {
        let mut user = create_test_user("testuser", "test@example.com");
        assert_eq!(user.role, UserRole::User);

        user.change_role(UserRole::Moderator);
        assert_eq!(user.role, UserRole::Moderator);

        user.change_role(UserRole::Admin);
        assert_eq!(user.role, UserRole::Admin);
    }
}
