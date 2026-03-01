//! Audit log entity.

use crate::{AuditLogId, Entity, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Audit log entry for tracking system activities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// Unique identifier for the audit log entry.
    pub id: AuditLogId,

    /// User who performed the action (if applicable).
    pub user_id: Option<UserId>,

    /// Action that was performed.
    pub action: AuditAction,

    /// Resource type that was affected.
    pub resource_type: String,

    /// Resource ID that was affected.
    pub resource_id: Option<String>,

    /// Additional details about the action.
    pub details: Option<JsonValue>,

    /// IP address of the client.
    pub ip_address: Option<String>,

    /// User agent of the client.
    pub user_agent: Option<String>,

    /// Whether the action was successful.
    pub success: bool,

    /// Error message if the action failed.
    pub error_message: Option<String>,

    /// Timestamp of the action.
    pub timestamp: DateTime<Utc>,
}

impl AuditLog {
    /// Creates a new audit log entry.
    #[must_use]
    pub fn new(
        user_id: Option<UserId>,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: Option<String>,
    ) -> Self {
        Self {
            id: AuditLogId::new(),
            user_id,
            action,
            resource_type: resource_type.into(),
            resource_id,
            details: None,
            ip_address: None,
            user_agent: None,
            success: true,
            error_message: None,
            timestamp: Utc::now(),
        }
    }

    /// Creates a success audit log entry.
    #[must_use]
    pub fn success(
        user_id: Option<UserId>,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: Option<String>,
    ) -> Self {
        Self::new(user_id, action, resource_type, resource_id)
    }

    /// Creates a failure audit log entry.
    #[must_use]
    pub fn failure(
        user_id: Option<UserId>,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: Option<String>,
        error_message: impl Into<String>,
    ) -> Self {
        let mut log = Self::new(user_id, action, resource_type, resource_id);
        log.success = false;
        log.error_message = Some(error_message.into());
        log
    }

    /// Sets additional details.
    #[must_use]
    pub fn with_details(mut self, details: JsonValue) -> Self {
        self.details = Some(details);
        self
    }

    /// Sets the IP address.
    #[must_use]
    pub fn with_ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Sets the user agent.
    #[must_use]
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }
}

impl Entity<AuditLogId> for AuditLog {
    fn id(&self) -> &AuditLogId {
        &self.id
    }
}

/// Audit action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    // Authentication actions
    /// User login attempt.
    Login,
    /// User logout.
    Logout,
    /// Token refresh.
    TokenRefresh,
    /// Password change.
    PasswordChange,
    /// Password reset request.
    PasswordResetRequest,
    /// Password reset completion.
    PasswordResetComplete,

    // User management
    /// User creation.
    UserCreate,
    /// User update.
    UserUpdate,
    /// User deletion.
    UserDelete,
    /// User role change.
    UserRoleChange,
    /// User status change.
    UserStatusChange,

    // Plugin actions
    /// Plugin installation.
    PluginInstall,
    /// Plugin uninstallation.
    PluginUninstall,
    /// Plugin enable.
    PluginEnable,
    /// Plugin disable.
    PluginDisable,
    /// Plugin configuration change.
    PluginConfigChange,

    // System actions
    /// Configuration change.
    ConfigChange,
    /// System startup.
    SystemStart,
    /// System shutdown.
    SystemShutdown,

    // Generic CRUD
    /// Resource creation.
    Create,
    /// Resource read.
    Read,
    /// Resource update.
    Update,
    /// Resource deletion.
    Delete,

    // Other
    /// Custom action.
    Custom,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Login => write!(f, "LOGIN"),
            Self::Logout => write!(f, "LOGOUT"),
            Self::TokenRefresh => write!(f, "TOKEN_REFRESH"),
            Self::PasswordChange => write!(f, "PASSWORD_CHANGE"),
            Self::PasswordResetRequest => write!(f, "PASSWORD_RESET_REQUEST"),
            Self::PasswordResetComplete => write!(f, "PASSWORD_RESET_COMPLETE"),
            Self::UserCreate => write!(f, "USER_CREATE"),
            Self::UserUpdate => write!(f, "USER_UPDATE"),
            Self::UserDelete => write!(f, "USER_DELETE"),
            Self::UserRoleChange => write!(f, "USER_ROLE_CHANGE"),
            Self::UserStatusChange => write!(f, "USER_STATUS_CHANGE"),
            Self::PluginInstall => write!(f, "PLUGIN_INSTALL"),
            Self::PluginUninstall => write!(f, "PLUGIN_UNINSTALL"),
            Self::PluginEnable => write!(f, "PLUGIN_ENABLE"),
            Self::PluginDisable => write!(f, "PLUGIN_DISABLE"),
            Self::PluginConfigChange => write!(f, "PLUGIN_CONFIG_CHANGE"),
            Self::ConfigChange => write!(f, "CONFIG_CHANGE"),
            Self::SystemStart => write!(f, "SYSTEM_START"),
            Self::SystemShutdown => write!(f, "SYSTEM_SHUTDOWN"),
            Self::Create => write!(f, "CREATE"),
            Self::Read => write!(f, "READ"),
            Self::Update => write!(f, "UPDATE"),
            Self::Delete => write!(f, "DELETE"),
            Self::Custom => write!(f, "CUSTOM"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_success() {
        let user_id = UserId::new();
        let log = AuditLog::success(
            Some(user_id),
            AuditAction::Login,
            "user",
            Some(user_id.to_string()),
        );

        assert!(log.success);
        assert!(log.error_message.is_none());
    }

    #[test]
    fn test_audit_log_failure() {
        let log = AuditLog::failure(
            None,
            AuditAction::Login,
            "user",
            None,
            "Invalid credentials",
        );

        assert!(!log.success);
        assert_eq!(log.error_message, Some("Invalid credentials".to_string()));
    }

    #[test]
    fn test_audit_log_with_details() {
        use serde_json::json;
        let user_id = UserId::new();
        let log = AuditLog::success(Some(user_id), AuditAction::UserCreate, "user", None)
            .with_details(json!({"role": "admin"}));
        assert!(log.details.is_some());
    }

    #[test]
    fn test_audit_log_with_ip_address() {
        let log = AuditLog::success(None, AuditAction::Login, "user", None)
            .with_ip_address("192.168.1.1");
        assert_eq!(log.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_audit_log_with_user_agent() {
        let log = AuditLog::success(None, AuditAction::Login, "user", None)
            .with_user_agent("Mozilla/5.0");
        assert_eq!(log.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_audit_log_new_direct() {
        let user_id = UserId::new();
        let log = AuditLog::new(Some(user_id), AuditAction::Logout, "session", None);
        assert_eq!(log.action, AuditAction::Logout);
        assert_eq!(log.user_id, Some(user_id));
        assert!(log.resource_type == "session");
    }

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::Login.to_string(), "LOGIN");
        assert_eq!(AuditAction::Logout.to_string(), "LOGOUT");
        assert_eq!(AuditAction::TokenRefresh.to_string(), "TOKEN_REFRESH");
        assert_eq!(AuditAction::PasswordChange.to_string(), "PASSWORD_CHANGE");
        assert_eq!(AuditAction::UserCreate.to_string(), "USER_CREATE");
        assert_eq!(AuditAction::UserUpdate.to_string(), "USER_UPDATE");
        assert_eq!(AuditAction::UserDelete.to_string(), "USER_DELETE");
        assert_eq!(AuditAction::PluginInstall.to_string(), "PLUGIN_INSTALL");
        assert_eq!(AuditAction::PluginUninstall.to_string(), "PLUGIN_UNINSTALL");
        assert_eq!(AuditAction::PluginEnable.to_string(), "PLUGIN_ENABLE");
        assert_eq!(AuditAction::PluginDisable.to_string(), "PLUGIN_DISABLE");
        assert_eq!(AuditAction::PluginConfigChange.to_string(), "PLUGIN_CONFIG_CHANGE");
        assert_eq!(AuditAction::ConfigChange.to_string(), "CONFIG_CHANGE");
        assert_eq!(AuditAction::SystemStart.to_string(), "SYSTEM_START");
        assert_eq!(AuditAction::SystemShutdown.to_string(), "SYSTEM_SHUTDOWN");
        assert_eq!(AuditAction::Create.to_string(), "CREATE");
        assert_eq!(AuditAction::Read.to_string(), "READ");
        assert_eq!(AuditAction::Update.to_string(), "UPDATE");
        assert_eq!(AuditAction::Delete.to_string(), "DELETE");
        assert_eq!(AuditAction::Custom.to_string(), "CUSTOM");
        assert_eq!(AuditAction::UserRoleChange.to_string(), "USER_ROLE_CHANGE");
        assert_eq!(AuditAction::UserStatusChange.to_string(), "USER_STATUS_CHANGE");
        assert_eq!(AuditAction::PasswordResetRequest.to_string(), "PASSWORD_RESET_REQUEST");
        assert_eq!(AuditAction::PasswordResetComplete.to_string(), "PASSWORD_RESET_COMPLETE");
    }
}
