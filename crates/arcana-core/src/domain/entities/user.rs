//! User entity.

use super::super::value_objects::{Email, UserRole, UserStatus};
use crate::{Entity, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// User entity representing an authenticated user in the system.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct User {
    /// Unique identifier for the user.
    pub id: UserId,

    /// Unique username.
    #[validate(length(min = 3, max = 32))]
    pub username: String,

    /// User's email address.
    pub email: Email,

    /// Hashed password (never exposed via API).
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// User's first name.
    #[validate(length(max = 64))]
    pub first_name: Option<String>,

    /// User's last name.
    #[validate(length(max = 64))]
    pub last_name: Option<String>,

    /// User's role.
    pub role: UserRole,

    /// User's status.
    pub status: UserStatus,

    /// Whether the user's email is verified.
    pub email_verified: bool,

    /// Profile picture URL.
    pub avatar_url: Option<String>,

    /// Last login timestamp.
    pub last_login_at: Option<DateTime<Utc>>,

    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user with the given details.
    #[must_use]
    pub fn new(
        username: String,
        email: Email,
        password_hash: String,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(),
            username,
            email,
            password_hash,
            first_name,
            last_name,
            role: UserRole::User,
            status: UserStatus::PendingVerification,
            email_verified: false,
            avatar_url: None,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new admin user.
    #[must_use]
    pub fn new_admin(
        username: String,
        email: Email,
        password_hash: String,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Self {
        let mut user = Self::new(username, email, password_hash, first_name, last_name);
        user.role = UserRole::Admin;
        user.status = UserStatus::Active;
        user.email_verified = true;
        user
    }

    /// Returns the user's full name.
    #[must_use]
    pub fn full_name(&self) -> Option<String> {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            (Some(first), None) => Some(first.clone()),
            (None, Some(last)) => Some(last.clone()),
            (None, None) => None,
        }
    }

    /// Returns the display name (full name or username).
    #[must_use]
    pub fn display_name(&self) -> String {
        self.full_name().unwrap_or_else(|| self.username.clone())
    }

    /// Checks if the user is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    /// Checks if the user can log in.
    #[must_use]
    pub const fn can_login(&self) -> bool {
        self.is_active()
    }

    /// Checks if the user is an admin.
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    /// Checks if the user has the specified role or higher.
    #[must_use]
    pub const fn has_role(&self, required_role: UserRole) -> bool {
        self.role.has_permission(required_role)
    }

    /// Activates the user account.
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
        self.email_verified = true;
        self.updated_at = Utc::now();
    }

    /// Suspends the user account.
    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Records a successful login.
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Updates the user's password hash.
    pub fn update_password(&mut self, password_hash: String) {
        self.password_hash = password_hash;
        self.updated_at = Utc::now();
    }

    /// Updates the user's profile.
    pub fn update_profile(&mut self, first_name: Option<String>, last_name: Option<String>, avatar_url: Option<String>) {
        self.first_name = first_name;
        self.last_name = last_name;
        self.avatar_url = avatar_url;
        self.updated_at = Utc::now();
    }

    /// Changes the user's role.
    pub fn change_role(&mut self, role: UserRole) {
        self.role = role;
        self.updated_at = Utc::now();
    }
}

impl Entity<UserId> for User {
    fn id(&self) -> &UserId {
        &self.id
    }
}

/// Builder for creating User instances.
#[derive(Debug, Default)]
pub struct UserBuilder {
    username: Option<String>,
    email: Option<Email>,
    password_hash: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    role: Option<UserRole>,
    status: Option<UserStatus>,
}

impl UserBuilder {
    /// Creates a new user builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the username.
    #[must_use]
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the email.
    #[must_use]
    pub fn email(mut self, email: Email) -> Self {
        self.email = Some(email);
        self
    }

    /// Sets the password hash.
    #[must_use]
    pub fn password_hash(mut self, hash: impl Into<String>) -> Self {
        self.password_hash = Some(hash.into());
        self
    }

    /// Sets the first name.
    #[must_use]
    pub fn first_name(mut self, name: impl Into<String>) -> Self {
        self.first_name = Some(name.into());
        self
    }

    /// Sets the last name.
    #[must_use]
    pub fn last_name(mut self, name: impl Into<String>) -> Self {
        self.last_name = Some(name.into());
        self
    }

    /// Sets the role.
    #[must_use]
    pub fn role(mut self, role: UserRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Sets the status.
    #[must_use]
    pub fn status(mut self, status: UserStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Builds the User instance.
    ///
    /// # Panics
    ///
    /// Panics if username, email, or password_hash are not set.
    #[must_use]
    pub fn build(self) -> User {
        let mut user = User::new(
            self.username.expect("username is required"),
            self.email.expect("email is required"),
            self.password_hash.expect("password_hash is required"),
            self.first_name,
            self.last_name,
        );

        if let Some(role) = self.role {
            user.role = role;
        }
        if let Some(status) = self.status {
            user.status = status;
        }

        user
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_user(username: &str) -> User {
        User::new(
            username.to_string(),
            Email::new(&format!("{}@example.com", username)).unwrap(),
            "hashed_password".to_string(),
            None,
            None,
        )
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "johndoe".to_string(),
            Email::new("john@example.com").unwrap(),
            "hashed_password".to_string(),
            Some("John".to_string()),
            Some("Doe".to_string()),
        );

        assert_eq!(user.username, "johndoe");
        assert_eq!(user.full_name(), Some("John Doe".to_string()));
        assert!(!user.is_active());
        assert!(!user.is_admin());
        assert_eq!(user.role, UserRole::User);
        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(!user.email_verified);
        assert!(user.avatar_url.is_none());
        assert!(user.last_login_at.is_none());
    }

    #[test]
    fn test_user_initial_status_is_pending_verification() {
        let user = create_user("newuser");
        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(!user.is_active());
    }

    #[test]
    fn test_user_builder() {
        let user = UserBuilder::new()
            .username("janedoe")
            .email(Email::new("jane@example.com").unwrap())
            .password_hash("hashed")
            .first_name("Jane")
            .role(UserRole::Admin)
            .status(UserStatus::Active)
            .build();

        assert_eq!(user.username, "janedoe");
        assert!(user.is_admin());
        assert!(user.is_active());
        assert_eq!(user.first_name, Some("Jane".to_string()));
    }

    #[test]
    fn test_user_builder_with_last_name() {
        let user = UserBuilder::new()
            .username("testuser")
            .email(Email::new("test@example.com").unwrap())
            .password_hash("hashed")
            .last_name("Smith")
            .build();

        assert_eq!(user.last_name, Some("Smith".to_string()));
    }

    #[test]
    fn test_user_activation() {
        let mut user = User::new(
            "test".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            None,
            None,
        );

        assert!(!user.is_active());
        user.activate();
        assert!(user.is_active());
        assert!(user.email_verified);
        assert_eq!(user.status, UserStatus::Active);
    }

    #[test]
    fn test_user_suspension() {
        let mut user = create_user("testuser");
        user.activate();
        assert!(user.is_active());
        user.suspend();
        assert!(!user.is_active());
        assert_eq!(user.status, UserStatus::Suspended);
    }

    #[test]
    fn test_user_record_login() {
        let mut user = create_user("testuser");
        assert!(user.last_login_at.is_none());
        user.record_login();
        assert!(user.last_login_at.is_some());
    }

    #[test]
    fn test_user_update_password() {
        let mut user = create_user("testuser");
        let old_hash = user.password_hash.clone();
        user.update_password("new_hash_value".to_string());
        assert_ne!(user.password_hash, old_hash);
        assert_eq!(user.password_hash, "new_hash_value");
    }

    #[test]
    fn test_user_update_profile() {
        let mut user = create_user("testuser");
        user.update_profile(
            Some("John".to_string()),
            Some("Doe".to_string()),
            Some("https://example.com/avatar.png".to_string()),
        );
        assert_eq!(user.first_name, Some("John".to_string()));
        assert_eq!(user.last_name, Some("Doe".to_string()));
        assert_eq!(user.avatar_url, Some("https://example.com/avatar.png".to_string()));
    }

    #[test]
    fn test_user_update_profile_clear_fields() {
        let mut user = User::new(
            "test".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            Some("Old".to_string()),
            Some("Name".to_string()),
        );
        user.update_profile(None, None, None);
        assert!(user.first_name.is_none());
        assert!(user.last_name.is_none());
        assert!(user.avatar_url.is_none());
    }

    #[test]
    fn test_user_change_role() {
        let mut user = create_user("testuser");
        assert_eq!(user.role, UserRole::User);
        user.change_role(UserRole::Admin);
        assert_eq!(user.role, UserRole::Admin);
        assert!(user.is_admin());
    }

    #[test]
    fn test_user_change_role_to_moderator() {
        let mut user = create_user("testuser");
        user.change_role(UserRole::Moderator);
        assert_eq!(user.role, UserRole::Moderator);
        assert!(!user.is_admin());
    }

    #[test]
    fn test_user_has_role() {
        let mut user = create_user("testuser");
        user.change_role(UserRole::Admin);

        assert!(user.has_role(UserRole::User));
        assert!(user.has_role(UserRole::Moderator));
        assert!(user.has_role(UserRole::Admin));
        assert!(!user.has_role(UserRole::SuperAdmin));
    }

    #[test]
    fn test_user_full_name_both_names() {
        let user = User::new(
            "test".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            Some("John".to_string()),
            Some("Doe".to_string()),
        );
        assert_eq!(user.full_name(), Some("John Doe".to_string()));
    }

    #[test]
    fn test_user_full_name_first_only() {
        let user = User::new(
            "test".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            Some("John".to_string()),
            None,
        );
        assert_eq!(user.full_name(), Some("John".to_string()));
    }

    #[test]
    fn test_user_full_name_last_only() {
        let user = User::new(
            "test".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            None,
            Some("Doe".to_string()),
        );
        assert_eq!(user.full_name(), Some("Doe".to_string()));
    }

    #[test]
    fn test_user_full_name_neither() {
        let user = create_user("testuser");
        assert_eq!(user.full_name(), None);
    }

    #[test]
    fn test_user_display_name_with_full_name() {
        let user = User::new(
            "testuser".to_string(),
            Email::new("test@example.com").unwrap(),
            "hash".to_string(),
            Some("John".to_string()),
            Some("Doe".to_string()),
        );
        assert_eq!(user.display_name(), "John Doe");
    }

    #[test]
    fn test_user_display_name_without_name() {
        let user = create_user("testuser");
        assert_eq!(user.display_name(), "testuser");
    }

    #[test]
    fn test_can_login_active_user() {
        let mut user = create_user("testuser");
        user.activate();
        assert!(user.can_login());
    }

    #[test]
    fn test_cannot_login_suspended_user() {
        let mut user = create_user("testuser");
        user.activate();
        user.suspend();
        assert!(!user.can_login());
    }

    #[test]
    fn test_new_admin_user() {
        let user = User::new_admin(
            "adminuser".to_string(),
            Email::new("admin@example.com").unwrap(),
            "hash".to_string(),
            Some("Admin".to_string()),
            None,
        );
        assert!(user.is_admin());
        assert!(user.is_active());
        assert!(user.email_verified);
        assert_eq!(user.role, UserRole::Admin);
        assert_eq!(user.status, UserStatus::Active);
    }

    #[test]
    fn test_user_clone() {
        let user = create_user("testuser");
        let cloned = user.clone();
        assert_eq!(cloned.id, user.id);
        assert_eq!(cloned.username, user.username);
    }

    #[test]
    fn test_user_serialize_does_not_expose_password() {
        let user = create_user("testuser");
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("hashed_password"));
    }

    #[test]
    fn test_user_id_is_unique() {
        let user1 = create_user("user1");
        let user2 = create_user("user2");
        assert_ne!(user1.id, user2.id);
    }
}
