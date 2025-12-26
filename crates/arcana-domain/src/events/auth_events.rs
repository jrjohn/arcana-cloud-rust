//! Authentication-related domain events.

use arcana_core::{ArcanaResult, DomainEvent, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a user successfully logs in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSucceeded {
    pub user_id: UserId,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl LoginSucceeded {
    #[must_use]
    pub fn new(user_id: UserId, ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            user_id,
            ip_address,
            user_agent,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for LoginSucceeded {
    fn event_type(&self) -> &'static str {
        "auth.login_succeeded"
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

/// Event emitted when a login attempt fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFailed {
    pub username_or_email: String,
    pub reason: LoginFailureReason,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl LoginFailed {
    #[must_use]
    pub fn new(
        username_or_email: String,
        reason: LoginFailureReason,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            username_or_email,
            reason,
            ip_address,
            user_agent,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for LoginFailed {
    fn event_type(&self) -> &'static str {
        "auth.login_failed"
    }

    fn aggregate_id(&self) -> String {
        self.username_or_email.clone()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn to_json(&self) -> ArcanaResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Reason for login failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginFailureReason {
    /// User not found.
    UserNotFound,
    /// Invalid password.
    InvalidPassword,
    /// Account is locked.
    AccountLocked,
    /// Account is suspended.
    AccountSuspended,
    /// Account is not verified.
    AccountNotVerified,
    /// Too many failed attempts.
    TooManyAttempts,
}

/// Event emitted when a user logs out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutOccurred {
    pub user_id: UserId,
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl LogoutOccurred {
    #[must_use]
    pub fn new(user_id: UserId, session_id: Option<String>) -> Self {
        Self {
            user_id,
            session_id,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for LogoutOccurred {
    fn event_type(&self) -> &'static str {
        "auth.logout"
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

/// Event emitted when a token is refreshed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshed {
    pub user_id: UserId,
    pub old_token_family: String,
    pub new_token_family: String,
    pub timestamp: DateTime<Utc>,
}

impl TokenRefreshed {
    #[must_use]
    pub fn new(user_id: UserId, old_token_family: String, new_token_family: String) -> Self {
        Self {
            user_id,
            old_token_family,
            new_token_family,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for TokenRefreshed {
    fn event_type(&self) -> &'static str {
        "auth.token_refreshed"
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

/// Event emitted when a password is changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordChanged {
    pub user_id: UserId,
    pub changed_by: UserId,
    pub timestamp: DateTime<Utc>,
}

impl PasswordChanged {
    #[must_use]
    pub fn new(user_id: UserId, changed_by: UserId) -> Self {
        Self {
            user_id,
            changed_by,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for PasswordChanged {
    fn event_type(&self) -> &'static str {
        "auth.password_changed"
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

/// Event emitted when a password reset is requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequested {
    pub user_id: UserId,
    pub email: String,
    pub ip_address: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl PasswordResetRequested {
    #[must_use]
    pub fn new(user_id: UserId, email: String, ip_address: Option<String>) -> Self {
        Self {
            user_id,
            email,
            ip_address,
            timestamp: Utc::now(),
        }
    }
}

impl DomainEvent for PasswordResetRequested {
    fn event_type(&self) -> &'static str {
        "auth.password_reset_requested"
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
    fn test_login_succeeded_event() {
        let user_id = UserId::new();
        let event = LoginSucceeded::new(
            user_id,
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        );

        assert_eq!(event.event_type(), "auth.login_succeeded");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(event.user_agent, Some("Mozilla/5.0".to_string()));

        let json = event.to_json().unwrap();
        assert!(json.contains("192.168.1.1"));
    }

    #[test]
    fn test_login_succeeded_without_optional_fields() {
        let user_id = UserId::new();
        let event = LoginSucceeded::new(user_id, None, None);

        assert_eq!(event.event_type(), "auth.login_succeeded");
        assert!(event.ip_address.is_none());
        assert!(event.user_agent.is_none());
    }

    #[test]
    fn test_login_failed_event() {
        let event = LoginFailed::new(
            "testuser".to_string(),
            LoginFailureReason::InvalidPassword,
            Some("192.168.1.1".to_string()),
            None,
        );

        assert_eq!(event.event_type(), "auth.login_failed");
        assert_eq!(event.aggregate_id(), "testuser");
        assert_eq!(event.reason, LoginFailureReason::InvalidPassword);
    }

    #[test]
    fn test_login_failure_reasons() {
        let reasons = vec![
            LoginFailureReason::UserNotFound,
            LoginFailureReason::InvalidPassword,
            LoginFailureReason::AccountLocked,
            LoginFailureReason::AccountSuspended,
            LoginFailureReason::AccountNotVerified,
            LoginFailureReason::TooManyAttempts,
        ];

        for reason in reasons {
            let event = LoginFailed::new("user".to_string(), reason, None, None);
            assert_eq!(event.reason, reason);
            let json = event.to_json().unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_logout_occurred_event() {
        let user_id = UserId::new();
        let event = LogoutOccurred::new(user_id, Some("session-123".to_string()));

        assert_eq!(event.event_type(), "auth.logout");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_logout_without_session() {
        let user_id = UserId::new();
        let event = LogoutOccurred::new(user_id, None);

        assert_eq!(event.event_type(), "auth.logout");
        assert!(event.session_id.is_none());
    }

    #[test]
    fn test_token_refreshed_event() {
        let user_id = UserId::new();
        let event = TokenRefreshed::new(
            user_id,
            "old-family-123".to_string(),
            "new-family-456".to_string(),
        );

        assert_eq!(event.event_type(), "auth.token_refreshed");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.old_token_family, "old-family-123");
        assert_eq!(event.new_token_family, "new-family-456");
    }

    #[test]
    fn test_password_changed_event() {
        let user_id = UserId::new();
        let changed_by = UserId::new();
        let event = PasswordChanged::new(user_id, changed_by);

        assert_eq!(event.event_type(), "auth.password_changed");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.changed_by, changed_by);
    }

    #[test]
    fn test_password_reset_requested_event() {
        let user_id = UserId::new();
        let event = PasswordResetRequested::new(
            user_id,
            "test@example.com".to_string(),
            Some("10.0.0.1".to_string()),
        );

        assert_eq!(event.event_type(), "auth.password_reset_requested");
        assert_eq!(event.aggregate_id(), user_id.to_string());
        assert_eq!(event.email, "test@example.com");
        assert_eq!(event.ip_address, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let user_id = UserId::new();
        let event = LoginSucceeded::new(user_id, Some("127.0.0.1".to_string()), None);

        let json = event.to_json().unwrap();
        let parsed: LoginSucceeded = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.user_id, event.user_id);
        assert_eq!(parsed.ip_address, event.ip_address);
    }

    #[test]
    fn test_login_failure_reason_serialization() {
        let event = LoginFailed::new(
            "user@example.com".to_string(),
            LoginFailureReason::AccountSuspended,
            None,
            None,
        );

        let json = event.to_json().unwrap();
        assert!(json.contains("account_suspended"));

        let parsed: LoginFailed = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reason, LoginFailureReason::AccountSuspended);
    }
}
