//! User service implementations.

use crate::cache::{cache_keys, CacheExt, CacheInterface, DEFAULT_TTL, SHORT_TTL};
use crate::dto::{
    ChangePasswordRequest, CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest,
    UpdateUserStatusRequest, UserListResponse, UserResponse,
};
use crate::user_service::UserService;
use arcana_core::{ArcanaError, ArcanaResult, Interface, PageRequest, UserId, ValidateExt};
use arcana_core::{Email, User};
use arcana_repository::UserRepository;
use arcana_security::{PasswordHasher, PasswordHasherInterface};
use async_trait::async_trait;
use shaku::Component;
use std::sync::Arc;
use tracing::{debug, info};

/// Generic user service implementation (non-DI).
pub struct UserServiceImpl<R: UserRepository> {
    user_repository: Arc<R>,
    password_hasher: Arc<PasswordHasher>,
}

impl<R: UserRepository> UserServiceImpl<R> {
    /// Creates a new user service.
    pub fn new(user_repository: Arc<R>, password_hasher: Arc<PasswordHasher>) -> Self {
        Self {
            user_repository,
            password_hasher,
        }
    }
}

#[async_trait]
impl<R: UserRepository + 'static> UserService for UserServiceImpl<R> {
    async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Creating user: {}", request.username);

        // Validate request
        request.validate_request()?;

        // Check for existing username
        if self.user_repository.exists_by_username(&request.username).await? {
            return Err(ArcanaError::Conflict(format!(
                "Username '{}' already exists",
                request.username
            )));
        }

        // Check for existing email
        if self.user_repository.exists_by_email(&request.email).await? {
            return Err(ArcanaError::Conflict(format!(
                "Email '{}' already exists",
                request.email
            )));
        }

        // Parse and validate email
        let email = Email::new(&request.email)
            .map_err(|e| ArcanaError::Validation(e.to_string()))?;

        // Hash password
        let password_hash = self.password_hasher.hash(&request.password)?;

        // Create user entity
        let user = User::new(
            request.username,
            email,
            password_hash,
            request.first_name,
            request.last_name,
        );

        // Save user
        let saved_user = self.user_repository.save(&user).await?;

        info!("User created: {}", saved_user.id);
        Ok(UserResponse::from(saved_user))
    }

    async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse> {
        debug!("Getting user: {}", id);

        let user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        Ok(UserResponse::from(user))
    }

    async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse> {
        debug!("Getting user by username: {}", username);

        let user = self
            .user_repository
            .find_by_username(username)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", username))?;

        Ok(UserResponse::from(user))
    }

    async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse> {
        debug!("Listing users, page: {}, size: {}", page.page, page.size);

        let users = self.user_repository.find_all(page).await?;
        Ok(UserListResponse::from(users))
    }

    async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user: {}", id);

        request.validate_request()?;

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        user.update_profile(request.first_name, request.last_name, request.avatar_url);

        let updated_user = self.user_repository.update(&user).await?;

        info!("User updated: {}", id);
        Ok(UserResponse::from(updated_user))
    }

    async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user role: {} -> {:?}", id, request.role);

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        user.change_role(request.role);

        let updated_user = self.user_repository.update(&user).await?;

        info!("User role updated: {} -> {:?}", id, request.role);
        Ok(UserResponse::from(updated_user))
    }

    async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user status: {} -> {:?}", id, request.status);

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        user.status = request.status;
        user.updated_at = chrono::Utc::now();

        let updated_user = self.user_repository.update(&user).await?;

        info!("User status updated: {} -> {:?}", id, request.status);
        Ok(UserResponse::from(updated_user))
    }

    async fn change_password(&self, id: UserId, request: ChangePasswordRequest) -> ArcanaResult<()> {
        debug!("Changing password for user: {}", id);

        request.validate_request()?;

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        // Verify current password
        if !self.password_hasher.verify(&request.current_password, &user.password_hash)? {
            return Err(ArcanaError::InvalidCredentials);
        }

        // Hash new password
        let new_hash = self.password_hasher.hash(&request.new_password)?;
        user.update_password(new_hash);

        self.user_repository.update(&user).await?;

        info!("Password changed for user: {}", id);
        Ok(())
    }

    async fn delete_user(&self, id: UserId) -> ArcanaResult<()> {
        debug!("Deleting user: {}", id);

        let deleted = self.user_repository.delete(id).await?;

        if !deleted {
            return Err(ArcanaError::not_found("User", id));
        }

        info!("User deleted: {}", id);
        Ok(())
    }

    async fn username_exists(&self, username: &str) -> ArcanaResult<bool> {
        self.user_repository.exists_by_username(username).await
    }

    async fn email_exists(&self, email: &str) -> ArcanaResult<bool> {
        self.user_repository.exists_by_email(email).await
    }
}

impl<R: UserRepository> std::fmt::Debug for UserServiceImpl<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserServiceImpl").finish_non_exhaustive()
    }
}

/// Concrete user service component for Shaku DI.
///
/// This component uses dependency injection to receive its dependencies,
/// providing compile-time verified DI through Shaku.
#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceComponent {
    #[shaku(inject)]
    user_repository: Arc<dyn UserRepository>,
    #[shaku(inject)]
    password_hasher: Arc<dyn PasswordHasherInterface>,
    #[shaku(inject)]
    cache: Arc<dyn CacheInterface>,
}

#[async_trait]
impl UserService for UserServiceComponent {
    async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Creating user: {}", request.username);

        request.validate_request()?;

        if self.user_repository.exists_by_username(&request.username).await? {
            return Err(ArcanaError::Conflict(format!(
                "Username '{}' already exists",
                request.username
            )));
        }

        if self.user_repository.exists_by_email(&request.email).await? {
            return Err(ArcanaError::Conflict(format!(
                "Email '{}' already exists",
                request.email
            )));
        }

        let email = Email::new(&request.email)
            .map_err(|e| ArcanaError::Validation(e.to_string()))?;

        let password_hash = self.password_hasher.hash(&request.password)?;

        let user = User::new(
            request.username,
            email,
            password_hash,
            request.first_name,
            request.last_name,
        );

        let saved_user = self.user_repository.save(&user).await?;

        info!("User created: {}", saved_user.id);
        Ok(UserResponse::from(saved_user))
    }

    async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse> {
        debug!("Getting user: {}", id);

        let cache_key = cache_keys::user_by_id(id);

        // Try cache first
        if let Some(cached) = self.cache.get::<UserResponse>(&cache_key).await? {
            debug!("Cache hit for user: {}", id);
            return Ok(cached);
        }

        let user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        let response = UserResponse::from(user);

        // Cache the result
        let _ = self.cache.set(&cache_key, &response, DEFAULT_TTL).await;

        Ok(response)
    }

    async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse> {
        debug!("Getting user by username: {}", username);

        let cache_key = cache_keys::user_by_username(username);

        // Try cache first
        if let Some(cached) = self.cache.get::<UserResponse>(&cache_key).await? {
            debug!("Cache hit for username: {}", username);
            return Ok(cached);
        }

        let user = self
            .user_repository
            .find_by_username(username)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", username))?;

        let response = UserResponse::from(user);

        // Cache the result (also cache by ID for cross-lookup)
        let _ = self.cache.set(&cache_key, &response, DEFAULT_TTL).await;
        let _ = self
            .cache
            .set(&cache_keys::user_by_id(response.id), &response, DEFAULT_TTL)
            .await;

        Ok(response)
    }

    async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse> {
        debug!("Listing users, page: {}, size: {}", page.page, page.size);

        let users = self.user_repository.find_all(page).await?;
        Ok(UserListResponse::from(users))
    }

    async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user: {}", id);

        request.validate_request()?;

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        // Capture username before update for cache invalidation
        let username = user.username.clone();

        user.update_profile(request.first_name, request.last_name, request.avatar_url);

        let updated_user = self.user_repository.update(&user).await?;

        // Invalidate cache entries
        let _ = self.cache.delete(&cache_keys::user_by_id(id)).await;
        let _ = self.cache.delete(&cache_keys::user_by_username(&username)).await;

        info!("User updated: {}", id);
        Ok(UserResponse::from(updated_user))
    }

    async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user role: {} -> {:?}", id, request.role);

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        let username = user.username.clone();
        user.change_role(request.role);

        let updated_user = self.user_repository.update(&user).await?;

        // Invalidate cache entries
        let _ = self.cache.delete(&cache_keys::user_by_id(id)).await;
        let _ = self.cache.delete(&cache_keys::user_by_username(&username)).await;

        info!("User role updated: {} -> {:?}", id, request.role);
        Ok(UserResponse::from(updated_user))
    }

    async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse> {
        debug!("Updating user status: {} -> {:?}", id, request.status);

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        let username = user.username.clone();
        user.status = request.status;
        user.updated_at = chrono::Utc::now();

        let updated_user = self.user_repository.update(&user).await?;

        // Invalidate cache entries
        let _ = self.cache.delete(&cache_keys::user_by_id(id)).await;
        let _ = self.cache.delete(&cache_keys::user_by_username(&username)).await;

        info!("User status updated: {} -> {:?}", id, request.status);
        Ok(UserResponse::from(updated_user))
    }

    async fn change_password(&self, id: UserId, request: ChangePasswordRequest) -> ArcanaResult<()> {
        debug!("Changing password for user: {}", id);

        request.validate_request()?;

        let mut user = self
            .user_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ArcanaError::not_found("User", id))?;

        if !self.password_hasher.verify(&request.current_password, &user.password_hash)? {
            return Err(ArcanaError::InvalidCredentials);
        }

        let new_hash = self.password_hasher.hash(&request.new_password)?;
        user.update_password(new_hash);

        self.user_repository.update(&user).await?;

        info!("Password changed for user: {}", id);
        Ok(())
    }

    async fn delete_user(&self, id: UserId) -> ArcanaResult<()> {
        debug!("Deleting user: {}", id);

        // Get user info before deletion for cache invalidation
        let user = self.user_repository.find_by_id(id).await?;

        let deleted = self.user_repository.delete(id).await?;

        if !deleted {
            return Err(ArcanaError::not_found("User", id));
        }

        // Invalidate cache entries
        let _ = self.cache.delete(&cache_keys::user_by_id(id)).await;
        if let Some(user) = user {
            let _ = self.cache.delete(&cache_keys::user_by_username(&user.username)).await;
        }

        info!("User deleted: {}", id);
        Ok(())
    }

    async fn username_exists(&self, username: &str) -> ArcanaResult<bool> {
        let cache_key = cache_keys::username_exists(username);

        // Try cache first
        if let Some(cached) = self.cache.get::<bool>(&cache_key).await? {
            return Ok(cached);
        }

        let exists = self.user_repository.exists_by_username(username).await?;

        // Cache the result with short TTL
        let _ = self.cache.set(&cache_key, &exists, SHORT_TTL).await;

        Ok(exists)
    }

    async fn email_exists(&self, email: &str) -> ArcanaResult<bool> {
        let cache_key = cache_keys::email_exists(email);

        // Try cache first
        if let Some(cached) = self.cache.get::<bool>(&cache_key).await? {
            return Ok(cached);
        }

        let exists = self.user_repository.exists_by_email(email).await?;

        // Cache the result with short TTL
        let _ = self.cache.set(&cache_key, &exists, SHORT_TTL).await;

        Ok(exists)
    }
}

impl std::fmt::Debug for UserServiceComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserServiceComponent").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::Page;
    use arcana_core::{Email, User, UserRole, UserStatus};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock user repository for testing.
    struct MockUserRepository {
        users: Mutex<HashMap<UserId, User>>,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        fn with_user(user: User) -> Self {
            let repo = Self::new();
            repo.users.lock().unwrap().insert(user.id, user);
            repo
        }

        fn add_user(&self, user: User) {
            self.users.lock().unwrap().insert(user.id, user);
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
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
            Ok(self.users.lock().unwrap().values()
                .any(|u| u.username == username))
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
            let items = if start < users.len() {
                users[start..end].to_vec()
            } else {
                vec![]
            };
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
            let items = if start < users.len() {
                users[start..end].to_vec()
            } else {
                vec![]
            };
            Ok(Page::new(items, page.page, page.size, total))
        }

        async fn save(&self, user: &User) -> ArcanaResult<User> {
            let mut users = self.users.lock().unwrap();
            users.insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn update(&self, user: &User) -> ArcanaResult<User> {
            let mut users = self.users.lock().unwrap();
            users.insert(user.id, user.clone());
            Ok(user.clone())
        }

        async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
            let mut users = self.users.lock().unwrap();
            Ok(users.remove(&id).is_some())
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

    fn create_test_user() -> User {
        User::new(
            "testuser".to_string(),
            Email::new_unchecked("test@example.com".to_string()),
            "hashed_password".to_string(),
            Some("Test".to_string()),
            Some("User".to_string()),
        )
    }

    fn create_user_service(repo: MockUserRepository) -> UserServiceImpl<MockUserRepository> {
        UserServiceImpl::new(
            Arc::new(repo),
            Arc::new(PasswordHasher::new()),
        )
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let request = CreateUserRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
        };

        let result = service.create_user(request).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "newuser");
        assert_eq!(user.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_create_user_duplicate_username() {
        let existing_user = create_test_user();
        let repo = MockUserRepository::with_user(existing_user);
        let service = create_user_service(repo);

        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "other@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.create_user(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Conflict(msg) => assert!(msg.contains("Username")),
            _ => panic!("Expected Conflict error"),
        }
    }

    #[tokio::test]
    async fn test_create_user_duplicate_email() {
        let existing_user = create_test_user();
        let repo = MockUserRepository::with_user(existing_user);
        let service = create_user_service(repo);

        let request = CreateUserRequest {
            username: "otheruser".to_string(),
            email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.create_user(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::Conflict(msg) => assert!(msg.contains("Email")),
            _ => panic!("Expected Conflict error"),
        }
    }

    #[tokio::test]
    async fn test_create_user_invalid_email() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let request = CreateUserRequest {
            username: "newuser".to_string(),
            email: "invalid-email".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.create_user(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_user_success() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let result = service.get_user(user_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, user_id);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let result = service.get_user(UserId::new()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::NotFound { .. } => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_user_by_username_success() {
        let user = create_test_user();
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let result = service.get_user_by_username("testuser").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_get_user_by_username_not_found() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let result = service.get_user_by_username("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_users() {
        let user = create_test_user();
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let result = service.list_users(PageRequest::default()).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total_elements, 1);
        assert_eq!(response.users.len(), 1);
    }

    #[tokio::test]
    async fn test_update_user_success() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = UpdateUserRequest {
            first_name: Some("Updated".to_string()),
            last_name: Some("Name".to_string()),
            avatar_url: None,
        };

        let result = service.update_user(user_id, request).await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.first_name, Some("Updated".to_string()));
        assert_eq!(updated.last_name, Some("Name".to_string()));
    }

    #[tokio::test]
    async fn test_update_user_not_found() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let request = UpdateUserRequest {
            first_name: Some("Updated".to_string()),
            last_name: None,
            avatar_url: None,
        };

        let result = service.update_user(UserId::new(), request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_user_role() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = UpdateUserRoleRequest { role: UserRole::Admin };
        let result = service.update_user_role(user_id, request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_update_user_status() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = UpdateUserStatusRequest {
            status: UserStatus::Suspended,
            reason: Some("Test suspension".to_string()),
        };

        let result = service.update_user_status(user_id, request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, UserStatus::Suspended);
    }

    #[tokio::test]
    async fn test_delete_user_success() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let result = service.delete_user(user_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let result = service.delete_user(UserId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_username_exists() {
        let user = create_test_user();
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        assert!(service.username_exists("testuser").await.unwrap());
        assert!(!service.username_exists("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_email_exists() {
        let user = create_test_user();
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        assert!(service.email_exists("test@example.com").await.unwrap());
        assert!(!service.email_exists("other@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_change_password_success() {
        let mut user = create_test_user();
        let user_id = user.id;
        let hasher = PasswordHasher::new();
        user.password_hash = hasher.hash("OldPassword123").unwrap();

        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = ChangePasswordRequest {
            current_password: "OldPassword123".to_string(),
            new_password: "NewPassword456".to_string(),
        };

        let result = service.change_password(user_id, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_change_password_wrong_current() {
        let mut user = create_test_user();
        let user_id = user.id;
        let hasher = PasswordHasher::new();
        user.password_hash = hasher.hash("OldPassword123").unwrap();

        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = ChangePasswordRequest {
            current_password: "WrongPassword".to_string(),
            new_password: "NewPassword456".to_string(),
        };

        let result = service.change_password(user_id, request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArcanaError::InvalidCredentials => {}
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    // =========================================================================
    // Additional Edge Case Tests
    // =========================================================================

    #[tokio::test]
    async fn test_change_password_user_not_found() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let request = ChangePasswordRequest {
            current_password: "OldPassword123".to_string(),
            new_password: "NewPassword456".to_string(),
        };

        let result = service.change_password(UserId::new(), request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_users_empty_repository() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        let page = PageRequest::new(0, 10);
        let result = service.list_users(page).await.unwrap();

        assert!(result.users.is_empty());
        assert_eq!(result.total_elements, 0);
    }

    #[tokio::test]
    async fn test_list_users_pagination_beyond_total() {
        let user1 = create_test_user_with_name("user1", "user1@example.com");
        let user2 = create_test_user_with_name("user2", "user2@example.com");
        let repo = MockUserRepository::new();
        repo.add_user(user1);
        repo.add_user(user2);
        let service = create_user_service(repo);

        // Request page beyond available data
        let page = PageRequest::new(5, 10); // Page 5 with only 2 users
        let result = service.list_users(page).await.unwrap();

        assert!(result.users.is_empty());
        assert_eq!(result.total_elements, 2);
    }

    #[tokio::test]
    async fn test_update_user_email_to_existing_email() {
        let user1 = create_test_user_with_name("user1", "user1@example.com");
        let user2 = create_test_user_with_name("user2", "user2@example.com");
        let user1_id = user1.id;
        let repo = MockUserRepository::new();
        repo.add_user(user1);
        repo.add_user(user2);
        let service = create_user_service(repo);

        // Try to update user1's email to user2's email
        let request = UpdateUserRequest {
            first_name: None,
            last_name: None,
            avatar_url: None,
        };

        // This should succeed since UpdateUserRequest doesn't change email
        let result = service.update_user(user1_id, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_with_short_username() {
        let repo = MockUserRepository::new();
        let service = create_user_service(repo);

        // Username too short (minimum 3 characters per validation)
        let request = CreateUserRequest {
            username: "ab".to_string(), // Only 2 characters
            email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
        };

        let result = service.create_user(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_user_role_to_super_admin() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = UpdateUserRoleRequest {
            role: UserRole::SuperAdmin,
        };

        let result = service.update_user_role(user_id, request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().role, UserRole::SuperAdmin);
    }

    #[tokio::test]
    async fn test_update_user_status_to_deleted() {
        let user = create_test_user();
        let user_id = user.id;
        let repo = MockUserRepository::with_user(user);
        let service = create_user_service(repo);

        let request = UpdateUserStatusRequest {
            status: UserStatus::Deleted,
            reason: Some("Account deleted by admin".to_string()),
        };

        let result = service.update_user_status(user_id, request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, UserStatus::Deleted);
    }

    fn create_test_user_with_name(username: &str, email: &str) -> User {
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
}
