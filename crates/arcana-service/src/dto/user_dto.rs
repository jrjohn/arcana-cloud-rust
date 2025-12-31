//! User-related DTOs.

use arcana_core::UserId;
use arcana_core::{User, UserRole, UserStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to create a new user.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
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

/// Request to update user profile.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRequest {
    #[validate(length(max = 64))]
    pub first_name: Option<String>,

    #[validate(length(max = 64))]
    pub last_name: Option<String>,

    #[validate(url(message = "Invalid avatar URL"))]
    pub avatar_url: Option<String>,
}

/// Request to update user role (admin only).
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRoleRequest {
    pub role: UserRole,
}

/// Request to update user status (admin only).
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserStatusRequest {
    pub status: UserStatus,
    #[validate(length(max = 500, message = "Reason cannot exceed 500 characters"))]
    pub reason: Option<String>,
}

/// Request to change password.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,

    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,
}

/// User response DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email.to_string(),
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
            status: user.status,
            email_verified: user.email_verified,
            avatar_url: user.avatar_url,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.to_string(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: user.role,
            status: user.status,
            email_verified: user.email_verified,
            avatar_url: user.avatar_url.clone(),
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

/// User list response with pagination.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
    pub page: usize,
    pub size: usize,
    pub total_elements: u64,
    pub total_pages: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::Email;
    use validator::Validate;

    fn create_test_user() -> User {
        User::new(
            "testuser".to_string(),
            Email::new("test@example.com").unwrap(),
            "hashedpassword".to_string(),
            None,
            None,
        )
    }

    #[test]
    fn test_create_user_request_valid() {
        let request = CreateUserRequest {
            username: "validuser".to_string(),
            email: "valid@example.com".to_string(),
            password: "password123".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_create_user_request_invalid_username_short() {
        let request = CreateUserRequest {
            username: "ab".to_string(),
            email: "valid@example.com".to_string(),
            password: "password123".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_user_request_invalid_email() {
        let request = CreateUserRequest {
            username: "validuser".to_string(),
            email: "not-an-email".to_string(),
            password: "password123".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_user_request_password_too_short() {
        let request = CreateUserRequest {
            username: "validuser".to_string(),
            email: "valid@example.com".to_string(),
            password: "short".to_string(),
            first_name: None,
            last_name: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_user_request_valid() {
        let request = UpdateUserRequest {
            first_name: Some("Jane".to_string()),
            last_name: Some("Smith".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_user_request_invalid_avatar_url() {
        let request = UpdateUserRequest {
            first_name: None,
            last_name: None,
            avatar_url: Some("not-a-url".to_string()),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_change_password_request_valid() {
        let request = ChangePasswordRequest {
            current_password: "oldpassword".to_string(),
            new_password: "newpassword123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_change_password_request_new_password_too_short() {
        let request = ChangePasswordRequest {
            current_password: "oldpassword".to_string(),
            new_password: "short".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_user_response_from_user() {
        let user = create_test_user();
        let response: UserResponse = user.clone().into();

        assert_eq!(response.id, user.id);
        assert_eq!(response.username, user.username);
        assert_eq!(response.email, user.email.to_string());
        assert_eq!(response.role, user.role);
        assert_eq!(response.status, user.status);
    }

    #[test]
    fn test_user_response_from_user_ref() {
        let user = create_test_user();
        let response: UserResponse = (&user).into();

        assert_eq!(response.id, user.id);
        assert_eq!(response.username, user.username);
    }

    #[test]
    fn test_user_list_response() {
        let user = create_test_user();
        let response = UserListResponse {
            users: vec![user.into()],
            page: 0,
            size: 10,
            total_elements: 1,
            total_pages: 1,
        };

        assert_eq!(response.users.len(), 1);
        assert_eq!(response.page, 0);
        assert_eq!(response.total_elements, 1);
    }

    #[test]
    fn test_update_user_role_request() {
        let request = UpdateUserRoleRequest { role: UserRole::Admin };
        assert_eq!(request.role, UserRole::Admin);
    }

    #[test]
    fn test_update_user_status_request() {
        let request = UpdateUserStatusRequest {
            status: UserStatus::Suspended,
            reason: Some("Policy violation".to_string()),
        };

        assert_eq!(request.status, UserStatus::Suspended);
        assert_eq!(request.reason, Some("Policy violation".to_string()));
    }

    #[test]
    fn test_dto_serialization() {
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            first_name: Some("Test".to_string()),
            last_name: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: CreateUserRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.username, request.username);
        assert_eq!(parsed.email, request.email);
    }
}
