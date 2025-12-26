//! User-related domain events.

use crate::value_objects::{UserRole, UserStatus};
use arcana_core::{ArcanaResult, DomainEvent, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a new user is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCreated {
    pub user_id: UserId,
    pub username: String,
    pub email: String,
    pub timestamp: DateTime<Utc>,
}

impl UserCreated {
    #[must_use]
    pub fn new(user_id: UserId, username: String, email: String) -> Self {
        Self {
            user_id,
            username,
            email,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserCreated {
    fn event_type(&self) -> &'static str {
        "user.created"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Event emitted when a user's profile is updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdated {
    pub user_id: UserId,
    pub updated_fields: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl UserUpdated {
    #[must_use]
    pub fn new(user_id: UserId, updated_fields: Vec<String>) -> Self {
        Self {
            user_id,
            updated_fields,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserUpdated {
    fn event_type(&self) -> &'static str {
        "user.updated"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Event emitted when a user is deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeleted {
    pub user_id: UserId,
    pub deleted_by: Option<UserId>,
    pub timestamp: DateTime<Utc>,
}

impl UserDeleted {
    #[must_use]
    pub fn new(user_id: UserId, deleted_by: Option<UserId>) -> Self {
        Self {
            user_id,
            deleted_by,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserDeleted {
    fn event_type(&self) -> &'static str {
        "user.deleted"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Event emitted when a user's role is changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleChanged {
    pub user_id: UserId,
    pub old_role: UserRole,
    pub new_role: UserRole,
    pub changed_by: UserId,
    pub timestamp: DateTime<Utc>,
}

impl UserRoleChanged {
    #[must_use]
    pub fn new(user_id: UserId, old_role: UserRole, new_role: UserRole, changed_by: UserId) -> Self {
        Self {
            user_id,
            old_role,
            new_role,
            changed_by,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserRoleChanged {
    fn event_type(&self) -> &'static str {
        "user.role_changed"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Event emitted when a user's status is changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatusChanged {
    pub user_id: UserId,
    pub old_status: UserStatus,
    pub new_status: UserStatus,
    pub reason: Option<String>,
    pub changed_by: Option<UserId>,
    pub timestamp: DateTime<Utc>,
}

impl UserStatusChanged {
    #[must_use]
    pub fn new(
        user_id: UserId,
        old_status: UserStatus,
        new_status: UserStatus,
        reason: Option<String>,
        changed_by: Option<UserId>,
    ) -> Self {
        Self {
            user_id,
            old_status,
            new_status,
            reason,
            changed_by,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserStatusChanged {
    fn event_type(&self) -> &'static str {
        "user.status_changed"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Event emitted when a user's email is verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailVerified {
    pub user_id: UserId,
    pub email: String,
    pub timestamp: DateTime<Utc>,
}

impl UserEmailVerified {
    #[must_use]
    pub fn new(user_id: UserId, email: String) -> Self {
        Self {
            user_id,
            email,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for UserEmailVerified {
    fn event_type(&self) -> &'static str {
        "user.email_verified"
    }

    fn aggregate_id(&self) -> String {
        self.user_id.to_string()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::DomainEvent;

    #[test]
    fn test_user_created_event() {
        let user_id = UserId::new();
        let event = UserCreated::new(user_id, "testuser".to_string(), "test@example.com".to_string());

        assert_eq!(event.event_type(), "user.created");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.username, "testuser");
        assert_eq!(event.email, "test@example.com");

        let json = event.to_json().unwrap();
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_user_updated_event() {
        let user_id = UserId::new();
        let event = UserUpdated::new(user_id, vec!["first_name".to_string(), "last_name".to_string()]);

        assert_eq!(event.event_type(), "user.updated");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.updated_fields.len(), 2);
    }

    #[test]
    fn test_user_deleted_event() {
        let user_id = UserId::new();
        let deleted_by = UserId::new();
        let event = UserDeleted::new(user_id, Some(deleted_by));

        assert_eq!(event.event_type(), "user.deleted");
        assert_eq!(event.deleted_by, Some(deleted_by));
    }

    #[test]
    fn test_user_role_changed_event() {
        let user_id = UserId::new();
        let changed_by = UserId::new();
        let event = UserRoleChanged::new(user_id, UserRole::User, UserRole::Admin, changed_by);

        assert_eq!(event.event_type(), "user.role_changed");
        assert_eq!(event.old_role, UserRole::User);
        assert_eq!(event.new_role, UserRole::Admin);
    }

    #[test]
    fn test_user_status_changed_event() {
        let user_id = UserId::new();
        let event = UserStatusChanged::new(
            user_id,
            UserStatus::Active,
            UserStatus::Suspended,
            Some("Violation".to_string()),
            None,
        );

        assert_eq!(event.event_type(), "user.status_changed");
        assert_eq!(event.old_status, UserStatus::Active);
        assert_eq!(event.new_status, UserStatus::Suspended);
        assert_eq!(event.reason, Some("Violation".to_string()));
    }

    #[test]
    fn test_user_email_verified_event() {
        let user_id = UserId::new();
        let event = UserEmailVerified::new(user_id, "test@example.com".to_string());

        assert_eq!(event.event_type(), "user.email_verified");
        assert_eq!(event.email, "test@example.com");
    }

    #[test]
    fn test_event_serialization() {
        let user_id = UserId::new();
        let event = UserCreated::new(user_id, "testuser".to_string(), "test@example.com".to_string());

        let json = event.to_json().unwrap();
        let parsed: UserCreated = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.username, event.username);
        assert_eq!(parsed.email, event.email);
    }
}
