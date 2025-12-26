//! Authentication-related DTOs.

use arcana_core::UserId;
use arcana_domain::UserRole;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Login request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Username or email is required"))]
    pub username_or_email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,

    /// Optional device identifier for token tracking.
    pub device_id: Option<String>,
}

/// Registration request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 32, message = "Username must be 3-32 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(max = 64))]
    pub first_name: Option<String>,

    #[validate(length(max = 64))]
    pub last_name: Option<String>,
}

/// Token refresh request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Authentication response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: AuthUserInfo,
}

/// User info included in auth response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUserInfo {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Password reset request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

/// Password reset confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PasswordResetConfirmRequest {
    pub token: String,

    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,
}

/// Logout request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// If true, invalidates all sessions for the user.
    #[serde(default)]
    pub all_sessions: bool,
}

/// Simple message response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

impl MessageResponse {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_login_request_valid() {
        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "password123".to_string(),
            device_id: Some("device-123".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_login_request_empty_username() {
        let request = LoginRequest {
            username_or_email: "".to_string(),
            password: "password123".to_string(),
            device_id: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_login_request_empty_password() {
        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "".to_string(),
            device_id: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_register_request_valid() {
        let request = RegisterRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "password123".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_register_request_invalid_username() {
        let request = RegisterRequest {
            username: "ab".to_string(), // Too short
            email: "valid@example.com".to_string(),
            password: "password123".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_register_request_invalid_email() {
        let request = RegisterRequest {
            username: "validuser".to_string(),
            email: "not-an-email".to_string(),
            password: "password123".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_register_request_password_too_short() {
        let request = RegisterRequest {
            username: "validuser".to_string(),
            email: "valid@example.com".to_string(),
            password: "short".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_password_reset_request_valid() {
        let request = PasswordResetRequest {
            email: "test@example.com".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_password_reset_request_invalid_email() {
        let request = PasswordResetRequest {
            email: "not-valid".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_password_reset_confirm_request_valid() {
        let request = PasswordResetConfirmRequest {
            token: "reset-token-123".to_string(),
            new_password: "newpassword123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_password_reset_confirm_password_too_short() {
        let request = PasswordResetConfirmRequest {
            token: "reset-token-123".to_string(),
            new_password: "short".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_refresh_token_request() {
        let request = RefreshTokenRequest {
            refresh_token: "refresh-token-123".to_string(),
        };

        assert_eq!(request.refresh_token, "refresh-token-123");
    }

    #[test]
    fn test_logout_request_default() {
        let json = r#"{}"#;
        let request: LogoutRequest = serde_json::from_str(json).unwrap();

        assert!(!request.all_sessions);
    }

    #[test]
    fn test_logout_request_all_sessions() {
        let request = LogoutRequest { all_sessions: true };

        assert!(request.all_sessions);
    }

    #[test]
    fn test_message_response() {
        let response = MessageResponse::new("Success");

        assert_eq!(response.message, "Success");
    }

    #[test]
    fn test_message_response_from_string() {
        let response = MessageResponse::new("Operation completed".to_string());

        assert_eq!(response.message, "Operation completed");
    }

    #[test]
    fn test_auth_response_structure() {
        let user_info = AuthUserInfo {
            id: UserId::new(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            role: UserRole::User,
            first_name: Some("Test".to_string()),
            last_name: None,
        };

        let response = AuthResponse {
            access_token: "access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            user: user_info,
        };

        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.user.username, "testuser");
    }

    #[test]
    fn test_auth_dto_serialization() {
        let request = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "password123".to_string(),
            device_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: LoginRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.username_or_email, request.username_or_email);
    }
}
