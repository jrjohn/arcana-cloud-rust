//! User role value object.

use serde::{Deserialize, Serialize};
use std::fmt;

/// User roles with hierarchical permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Regular user with basic permissions.
    #[default]
    User,
    /// Moderator with elevated permissions.
    Moderator,
    /// Administrator with full access.
    Admin,
    /// Super administrator (system owner).
    SuperAdmin,
}

impl UserRole {
    /// Returns the role's permission level (higher = more permissions).
    #[must_use]
    pub const fn level(&self) -> u8 {
        match self {
            Self::User => 1,
            Self::Moderator => 2,
            Self::Admin => 3,
            Self::SuperAdmin => 4,
        }
    }

    /// Checks if this role has at least the permissions of the required role.
    #[must_use]
    pub const fn has_permission(&self, required: Self) -> bool {
        self.level() >= required.level()
    }

    /// Returns all roles at or above this level.
    #[must_use]
    pub fn roles_with_permission(&self) -> Vec<Self> {
        match self {
            Self::User => vec![Self::User, Self::Moderator, Self::Admin, Self::SuperAdmin],
            Self::Moderator => vec![Self::Moderator, Self::Admin, Self::SuperAdmin],
            Self::Admin => vec![Self::Admin, Self::SuperAdmin],
            Self::SuperAdmin => vec![Self::SuperAdmin],
        }
    }

    /// Returns all available roles.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::User, Self::Moderator, Self::Admin, Self::SuperAdmin]
    }

    /// Parses a role from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "user" => Some(Self::User),
            "moderator" | "mod" => Some(Self::Moderator),
            "admin" | "administrator" => Some(Self::Admin),
            "superadmin" | "super_admin" | "superadministrator" => Some(Self::SuperAdmin),
            _ => None,
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Moderator => write!(f, "moderator"),
            Self::Admin => write!(f, "admin"),
            Self::SuperAdmin => write!(f, "superadmin"),
        }
    }
}

/// Permission types for RBAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // User permissions
    UserRead,
    UserCreate,
    UserUpdate,
    UserDelete,
    UserManageRoles,

    // Plugin permissions
    PluginRead,
    PluginInstall,
    PluginUninstall,
    PluginConfigure,

    // System permissions
    SystemConfig,
    SystemMonitor,
    SystemAdmin,

    // Content permissions
    ContentRead,
    ContentCreate,
    ContentUpdate,
    ContentDelete,
    ContentModerate,
}

impl Permission {
    /// Returns the minimum role required for this permission.
    #[must_use]
    pub const fn minimum_role(&self) -> UserRole {
        match self {
            // User permissions
            Self::UserRead => UserRole::User,
            Self::UserCreate | Self::UserUpdate => UserRole::Moderator,
            Self::UserDelete | Self::UserManageRoles => UserRole::Admin,

            // Plugin permissions
            Self::PluginRead => UserRole::User,
            Self::PluginInstall | Self::PluginUninstall | Self::PluginConfigure => UserRole::Admin,

            // System permissions
            Self::SystemMonitor => UserRole::Moderator,
            Self::SystemConfig | Self::SystemAdmin => UserRole::SuperAdmin,

            // Content permissions
            Self::ContentRead | Self::ContentCreate | Self::ContentUpdate => UserRole::User,
            Self::ContentDelete | Self::ContentModerate => UserRole::Moderator,
        }
    }

    /// Checks if the given role has this permission.
    #[must_use]
    pub const fn is_allowed_for(&self, role: UserRole) -> bool {
        role.has_permission(self.minimum_role())
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserRead => write!(f, "user:read"),
            Self::UserCreate => write!(f, "user:create"),
            Self::UserUpdate => write!(f, "user:update"),
            Self::UserDelete => write!(f, "user:delete"),
            Self::UserManageRoles => write!(f, "user:manage_roles"),
            Self::PluginRead => write!(f, "plugin:read"),
            Self::PluginInstall => write!(f, "plugin:install"),
            Self::PluginUninstall => write!(f, "plugin:uninstall"),
            Self::PluginConfigure => write!(f, "plugin:configure"),
            Self::SystemConfig => write!(f, "system:config"),
            Self::SystemMonitor => write!(f, "system:monitor"),
            Self::SystemAdmin => write!(f, "system:admin"),
            Self::ContentRead => write!(f, "content:read"),
            Self::ContentCreate => write!(f, "content:create"),
            Self::ContentUpdate => write!(f, "content:update"),
            Self::ContentDelete => write!(f, "content:delete"),
            Self::ContentModerate => write!(f, "content:moderate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_levels() {
        assert!(UserRole::Admin.level() > UserRole::User.level());
        assert!(UserRole::SuperAdmin.level() > UserRole::Admin.level());
    }

    #[test]
    fn test_role_permissions() {
        assert!(UserRole::Admin.has_permission(UserRole::User));
        assert!(UserRole::Admin.has_permission(UserRole::Moderator));
        assert!(UserRole::Admin.has_permission(UserRole::Admin));
        assert!(!UserRole::Admin.has_permission(UserRole::SuperAdmin));
    }

    #[test]
    fn test_permission_roles() {
        assert!(Permission::UserRead.is_allowed_for(UserRole::User));
        assert!(Permission::UserRead.is_allowed_for(UserRole::Admin));
        assert!(!Permission::UserDelete.is_allowed_for(UserRole::User));
        assert!(Permission::UserDelete.is_allowed_for(UserRole::Admin));
    }
}
