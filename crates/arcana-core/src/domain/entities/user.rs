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
    }
}
