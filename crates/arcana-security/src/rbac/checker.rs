//! RBAC permission checker.

use arcana_core::{ArcanaError, ArcanaResult, UserId};
use arcana_core::{Permission, UserRole};
use crate::Claims;

/// Extension trait for Claims to check permissions.
pub trait ClaimsExt {
    /// Requires a specific role.
    fn require_role(&self, role: UserRole) -> ArcanaResult<()>;

    /// Requires a specific permission.
    fn require_permission(&self, permission: Permission) -> ArcanaResult<()>;

    /// Requires either the specified role or being the resource owner.
    fn require_role_or_owner(&self, role: UserRole, resource_owner_id: UserId) -> ArcanaResult<()>;

    /// Checks if the user is the owner of a resource.
    fn is_owner(&self, resource_owner_id: UserId) -> bool;

    /// Requires the user to be an admin.
    fn require_admin(&self) -> ArcanaResult<()>;

    /// Requires the user to be a super admin.
    fn require_super_admin(&self) -> ArcanaResult<()>;
}

impl ClaimsExt for Claims {
    fn require_role(&self, role: UserRole) -> ArcanaResult<()> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err(ArcanaError::Forbidden(format!(
                "Required role: {}, your role: {}",
                role, self.role
            )))
        }
    }

    fn require_permission(&self, permission: Permission) -> ArcanaResult<()> {
        if permission.is_allowed_for(self.role) {
            Ok(())
        } else {
            Err(ArcanaError::Forbidden(format!(
                "Permission denied: {} requires at least {} role",
                permission,
                permission.minimum_role()
            )))
        }
    }

    fn require_role_or_owner(&self, role: UserRole, resource_owner_id: UserId) -> ArcanaResult<()> {
        if self.has_role(role) || self.is_owner(resource_owner_id) {
            Ok(())
        } else {
            Err(ArcanaError::Forbidden(
                "You don't have permission to access this resource".to_string(),
            ))
        }
    }

    fn is_owner(&self, resource_owner_id: UserId) -> bool {
        self.user_id()
            .map(|id| id == resource_owner_id)
            .unwrap_or(false)
    }

    fn require_admin(&self) -> ArcanaResult<()> {
        self.require_role(UserRole::Admin)
    }

    fn require_super_admin(&self) -> ArcanaResult<()> {
        self.require_role(UserRole::SuperAdmin)
    }
}

/// Permission guard for use in middleware.
#[derive(Debug, Clone)]
pub struct PermissionGuard {
    required_role: Option<UserRole>,
    required_permission: Option<Permission>,
    allow_owner: bool,
}

impl PermissionGuard {
    /// Creates a new permission guard.
    #[must_use]
    pub fn new() -> Self {
        Self {
            required_role: None,
            required_permission: None,
            allow_owner: false,
        }
    }

    /// Requires a specific role.
    #[must_use]
    pub fn role(mut self, role: UserRole) -> Self {
        self.required_role = Some(role);
        self
    }

    /// Requires a specific permission.
    #[must_use]
    pub fn permission(mut self, permission: Permission) -> Self {
        self.required_permission = Some(permission);
        self
    }

    /// Allows resource owners to access regardless of role.
    #[must_use]
    pub fn allow_owner(mut self) -> Self {
        self.allow_owner = true;
        self
    }

    /// Checks if the claims satisfy the guard requirements.
    pub fn check(&self, claims: &Claims, resource_owner_id: Option<UserId>) -> ArcanaResult<()> {
        // Check if owner access is allowed and user is owner
        if self.allow_owner {
            if let Some(owner_id) = resource_owner_id {
                if claims.is_owner(owner_id) {
                    return Ok(());
                }
            }
        }

        // Check role requirement
        if let Some(role) = self.required_role {
            claims.require_role(role)?;
        }

        // Check permission requirement
        if let Some(permission) = self.required_permission {
            claims.require_permission(permission)?;
        }

        Ok(())
    }
}

impl Default for PermissionGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Predefined permission guards for common scenarios.
pub mod guards {
    use super::*;

    /// Guard that allows any authenticated user.
    #[must_use]
    pub fn authenticated() -> PermissionGuard {
        PermissionGuard::new()
    }

    /// Guard that requires admin role.
    #[must_use]
    pub fn admin() -> PermissionGuard {
        PermissionGuard::new().role(UserRole::Admin)
    }

    /// Guard that requires super admin role.
    #[must_use]
    pub fn super_admin() -> PermissionGuard {
        PermissionGuard::new().role(UserRole::SuperAdmin)
    }

    /// Guard that requires moderator role.
    #[must_use]
    pub fn moderator() -> PermissionGuard {
        PermissionGuard::new().role(UserRole::Moderator)
    }

    /// Guard that allows owner or admin.
    #[must_use]
    pub fn owner_or_admin() -> PermissionGuard {
        PermissionGuard::new().role(UserRole::Admin).allow_owner()
    }

    /// Guard for user management.
    #[must_use]
    pub fn user_management() -> PermissionGuard {
        PermissionGuard::new().permission(Permission::UserManageRoles)
    }

    /// Guard for plugin management.
    #[must_use]
    pub fn plugin_management() -> PermissionGuard {
        PermissionGuard::new().permission(Permission::PluginInstall)
    }

    /// Guard for system configuration.
    #[must_use]
    pub fn system_config() -> PermissionGuard {
        PermissionGuard::new().permission(Permission::SystemConfig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::Permission;
    use chrono::{Duration, Utc};

    fn create_claims(role: UserRole) -> Claims {
        Claims::new_access(
            UserId::new(),
            "testuser".to_string(),
            "test@example.com".to_string(),
            role,
            "issuer".to_string(),
            "audience".to_string(),
            Utc::now() + Duration::hours(1),
        )
    }

    #[test]
    fn test_require_role() {
        let admin_claims = create_claims(UserRole::Admin);
        let user_claims = create_claims(UserRole::User);

        assert!(admin_claims.require_role(UserRole::User).is_ok());
        assert!(admin_claims.require_role(UserRole::Admin).is_ok());
        assert!(admin_claims.require_role(UserRole::SuperAdmin).is_err());

        assert!(user_claims.require_role(UserRole::User).is_ok());
        assert!(user_claims.require_role(UserRole::Admin).is_err());
    }

    #[test]
    fn test_require_permission() {
        let user_claims = create_claims(UserRole::User);
        let admin_claims = create_claims(UserRole::Admin);

        assert!(user_claims.require_permission(Permission::UserRead).is_ok());
        assert!(user_claims.require_permission(Permission::UserDelete).is_err());
        assert!(admin_claims.require_permission(Permission::UserDelete).is_ok());
    }

    #[test]
    fn test_require_admin() {
        let user_claims = create_claims(UserRole::User);
        let admin_claims = create_claims(UserRole::Admin);
        let superadmin_claims = create_claims(UserRole::SuperAdmin);

        assert!(user_claims.require_admin().is_err());
        assert!(admin_claims.require_admin().is_ok());
        assert!(superadmin_claims.require_admin().is_ok()); // SuperAdmin has admin perms
    }

    #[test]
    fn test_require_super_admin() {
        let admin_claims = create_claims(UserRole::Admin);
        let superadmin_claims = create_claims(UserRole::SuperAdmin);

        assert!(admin_claims.require_super_admin().is_err());
        assert!(superadmin_claims.require_super_admin().is_ok());
    }

    #[test]
    fn test_owner_access() {
        let claims = create_claims(UserRole::User);
        let owner_id = claims.user_id().unwrap();
        let other_id = UserId::new();

        assert!(claims.is_owner(owner_id));
        assert!(!claims.is_owner(other_id));

        assert!(claims.require_role_or_owner(UserRole::Admin, owner_id).is_ok());
        assert!(claims.require_role_or_owner(UserRole::Admin, other_id).is_err());
    }

    #[test]
    fn test_admin_can_access_any_resource() {
        let admin_claims = create_claims(UserRole::Admin);
        let random_owner = UserId::new();

        // Admin can access other users' resources
        assert!(admin_claims.require_role_or_owner(UserRole::Admin, random_owner).is_ok());
    }

    #[test]
    fn test_require_permission_system_config_super_admin_only() {
        let admin_claims = create_claims(UserRole::Admin);
        let superadmin_claims = create_claims(UserRole::SuperAdmin);

        assert!(admin_claims.require_permission(Permission::SystemConfig).is_err());
        assert!(superadmin_claims.require_permission(Permission::SystemConfig).is_ok());
    }

    #[test]
    fn test_permission_guard() {
        let user_claims = create_claims(UserRole::User);
        let admin_claims = create_claims(UserRole::Admin);

        let guard = guards::admin();

        assert!(guard.check(&admin_claims, None).is_ok());
        assert!(guard.check(&user_claims, None).is_err());
    }

    #[test]
    fn test_owner_or_admin_guard() {
        let user_claims = create_claims(UserRole::User);
        let owner_id = user_claims.user_id().unwrap();
        let other_id = UserId::new();

        let guard = guards::owner_or_admin();

        // User can access their own resource
        assert!(guard.check(&user_claims, Some(owner_id)).is_ok());
        // User cannot access others' resources
        assert!(guard.check(&user_claims, Some(other_id)).is_err());
    }

    #[test]
    fn test_moderator_guard() {
        let user_claims = create_claims(UserRole::User);
        let moderator_claims = create_claims(UserRole::Moderator);
        let admin_claims = create_claims(UserRole::Admin);

        let guard = guards::moderator();

        assert!(user_claims.require_role(UserRole::Moderator).is_err());
        assert!(guard.check(&user_claims, None).is_err());
        assert!(guard.check(&moderator_claims, None).is_ok());
        assert!(guard.check(&admin_claims, None).is_ok());
    }

    #[test]
    fn test_permission_guard_with_specific_permission() {
        let user_claims = create_claims(UserRole::User);
        let admin_claims = create_claims(UserRole::Admin);

        let guard = PermissionGuard::new().permission(Permission::UserDelete);

        assert!(guard.check(&admin_claims, None).is_ok());
        assert!(guard.check(&user_claims, None).is_err());
    }
}
