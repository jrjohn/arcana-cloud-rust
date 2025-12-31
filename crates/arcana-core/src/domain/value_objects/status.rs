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
    }

    #[test]
    fn test_status_can_act() {
        assert!(UserStatus::Active.can_act());
        assert!(!UserStatus::PendingVerification.can_act());
        assert!(!UserStatus::Suspended.can_act());
    }
}
