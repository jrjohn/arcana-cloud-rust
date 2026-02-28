//! User status value object.

use serde::{Deserialize, Serialize};
use std::fmt;

/// User account status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// User is pending email verification.
    #[default]
    PendingVerification,
    /// User account is active.
    Active,
    /// User account is suspended.
    Suspended,
    /// User account is locked (too many failed login attempts).
    Locked,
    /// User account is deleted (soft delete).
    Deleted,
}

impl UserStatus {
    /// Checks if the user can perform actions.
    #[must_use]
    pub const fn can_act(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Checks if the user can log in.
    #[must_use]
    pub const fn can_login(&self) -> bool {
        matches!(self, Self::Active | Self::PendingVerification)
    }

    /// Checks if the account is considered active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Checks if the account needs attention.
    #[must_use]
    pub const fn needs_attention(&self) -> bool {
        matches!(self, Self::PendingVerification | Self::Locked)
    }

    /// Returns a human-readable description.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::PendingVerification => "Email verification pending",
            Self::Active => "Account is active",
            Self::Suspended => "Account is suspended",
            Self::Locked => "Account is locked due to security concerns",
            Self::Deleted => "Account has been deleted",
        }
    }

    /// All possible statuses.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::PendingVerification,
            Self::Active,
            Self::Suspended,
            Self::Locked,
            Self::Deleted,
        ]
    }
}

impl fmt::Display for UserStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingVerification => write!(f, "pending_verification"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
            Self::Locked => write!(f, "locked"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_can_login() {
        assert!(UserStatus::Active.can_login());
        assert!(UserStatus::PendingVerification.can_login());
        assert!(!UserStatus::Suspended.can_login());
        assert!(!UserStatus::Locked.can_login());
        assert!(!UserStatus::Deleted.can_login());
    }

    #[test]
    fn test_status_can_act() {
        assert!(UserStatus::Active.can_act());
        assert!(!UserStatus::PendingVerification.can_act());
        assert!(!UserStatus::Suspended.can_act());
        assert!(!UserStatus::Locked.can_act());
        assert!(!UserStatus::Deleted.can_act());
    }

    #[test]
    fn test_status_is_active() {
        assert!(UserStatus::Active.is_active());
        assert!(!UserStatus::PendingVerification.is_active());
        assert!(!UserStatus::Suspended.is_active());
        assert!(!UserStatus::Locked.is_active());
        assert!(!UserStatus::Deleted.is_active());
    }

    #[test]
    fn test_status_needs_attention() {
        assert!(UserStatus::PendingVerification.needs_attention());
        assert!(UserStatus::Locked.needs_attention());
        assert!(!UserStatus::Active.needs_attention());
        assert!(!UserStatus::Suspended.needs_attention());
        assert!(!UserStatus::Deleted.needs_attention());
    }

    #[test]
    fn test_status_description() {
        assert!(!UserStatus::Active.description().is_empty());
        assert!(!UserStatus::PendingVerification.description().is_empty());
        assert!(!UserStatus::Suspended.description().is_empty());
        assert!(!UserStatus::Locked.description().is_empty());
        assert!(!UserStatus::Deleted.description().is_empty());
    }

    #[test]
    fn test_status_description_content() {
        assert!(UserStatus::Active.description().contains("active"));
        assert!(UserStatus::PendingVerification.description().to_lowercase().contains("pending") || 
                UserStatus::PendingVerification.description().to_lowercase().contains("verification"));
        assert!(UserStatus::Suspended.description().to_lowercase().contains("suspend"));
        assert!(UserStatus::Locked.description().to_lowercase().contains("lock"));
        assert!(UserStatus::Deleted.description().to_lowercase().contains("delete"));
    }

    #[test]
    fn test_status_all() {
        let all = UserStatus::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&UserStatus::PendingVerification));
        assert!(all.contains(&UserStatus::Active));
        assert!(all.contains(&UserStatus::Suspended));
        assert!(all.contains(&UserStatus::Locked));
        assert!(all.contains(&UserStatus::Deleted));
    }

    #[test]
    fn test_status_display() {
        assert_eq!(UserStatus::Active.to_string(), "active");
        assert_eq!(UserStatus::PendingVerification.to_string(), "pending_verification");
        assert_eq!(UserStatus::Suspended.to_string(), "suspended");
        assert_eq!(UserStatus::Locked.to_string(), "locked");
        assert_eq!(UserStatus::Deleted.to_string(), "deleted");
    }

    #[test]
    fn test_status_serialization() {
        let status = UserStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");
        let parsed: UserStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UserStatus::Active);
    }

    #[test]
    fn test_status_default() {
        let status = UserStatus::default();
        assert_eq!(status, UserStatus::PendingVerification);
    }

    #[test]
    fn test_status_equality() {
        assert_eq!(UserStatus::Active, UserStatus::Active);
        assert_ne!(UserStatus::Active, UserStatus::Suspended);
    }

    #[test]
    fn test_status_clone() {
        let status = UserStatus::Active;
        let cloned = status;
        assert_eq!(status, cloned);
    }
}
