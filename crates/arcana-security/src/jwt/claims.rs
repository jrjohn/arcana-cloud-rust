//! JWT claims structure.

use arcana_core::UserId;
use arcana_core::UserRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,

    /// User ID as UUID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,

    /// Username.
    pub username: String,

    /// User's email.
    pub email: String,

    /// User's role.
    pub role: UserRole,

    /// Token type (access or refresh).
    pub token_type: TokenType,

    /// Issued at timestamp.
    pub iat: i64,

    /// Expiration timestamp.
    pub exp: i64,

    /// Not before timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,

    /// Issuer.
    pub iss: String,

    /// Audience.
    pub aud: String,

    /// JWT ID (unique identifier for this token).
    pub jti: String,

    /// Session ID for refresh token invalidation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl Claims {
    /// Creates new access token claims.
    #[must_use]
    pub fn new_access(
        user_id: UserId,
        username: String,
        email: String,
        role: UserRole,
        issuer: String,
        audience: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            user_id: Some(user_id.into_inner()),
            username,
            email,
            role,
            token_type: TokenType::Access,
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            nbf: Some(now.timestamp()),
            iss: issuer,
            aud: audience,
            jti: Uuid::now_v7().to_string(),
            session_id: None,
        }
    }

    /// Creates new refresh token claims.
    #[must_use]
    pub fn new_refresh(
        user_id: UserId,
        username: String,
        email: String,
        role: UserRole,
        issuer: String,
        audience: String,
        expires_at: DateTime<Utc>,
        session_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            user_id: Some(user_id.into_inner()),
            username,
            email,
            role,
            token_type: TokenType::Refresh,
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            nbf: Some(now.timestamp()),
            iss: issuer,
            aud: audience,
            jti: Uuid::now_v7().to_string(),
            session_id: Some(session_id),
        }
    }

    /// Returns the user ID.
    #[must_use]
    pub fn user_id(&self) -> Option<UserId> {
        self.user_id.map(UserId::from_uuid)
    }

    /// Checks if the token is expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Returns the expiration time.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(Utc::now)
    }

    /// Checks if the user has the required role.
    #[must_use]
    pub const fn has_role(&self, required: UserRole) -> bool {
        self.role.has_permission(required)
    }

    /// Checks if this is an access token.
    #[must_use]
    pub const fn is_access_token(&self) -> bool {
        matches!(self.token_type, TokenType::Access)
    }

    /// Checks if this is a refresh token.
    #[must_use]
    pub const fn is_refresh_token(&self) -> bool {
        matches!(self.token_type, TokenType::Refresh)
    }
}

/// Token type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// Access token (short-lived, used for API requests).
    Access,
    /// Refresh token (long-lived, used to obtain new access tokens).
    Refresh,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access => write!(f, "access"),
            Self::Refresh => write!(f, "refresh"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_access_token_claims() {
        let user_id = UserId::new();
        let expires = Utc::now() + Duration::hours(1);
        let claims = Claims::new_access(
            user_id,
            "testuser".to_string(),
            "test@example.com".to_string(),
            UserRole::User,
            "issuer".to_string(),
            "audience".to_string(),
            expires,
        );

        assert!(claims.is_access_token());
        assert!(!claims.is_refresh_token());
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_role_check() {
        let user_id = UserId::new();
        let expires = Utc::now() + Duration::hours(1);
        let claims = Claims::new_access(
            user_id,
            "admin".to_string(),
            "admin@example.com".to_string(),
            UserRole::Admin,
            "issuer".to_string(),
            "audience".to_string(),
            expires,
        );

        assert!(claims.has_role(UserRole::User));
        assert!(claims.has_role(UserRole::Admin));
        assert!(!claims.has_role(UserRole::SuperAdmin));
    }
}
