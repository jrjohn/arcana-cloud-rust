//! OAuth token entity.

use crate::{Entity, OAuthTokenId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OAuth token entity for managing refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Unique identifier for the token.
    pub id: OAuthTokenId,

    /// User this token belongs to.
    pub user_id: UserId,

    /// The refresh token value (hashed).
    #[serde(skip_serializing)]
    pub token_hash: String,

    /// Token family for rotation detection.
    pub family_id: String,

    /// Device/client identifier.
    pub device_id: Option<String>,

    /// User agent of the client.
    pub user_agent: Option<String>,

    /// IP address of the client.
    pub ip_address: Option<String>,

    /// Token expiration timestamp.
    pub expires_at: DateTime<Utc>,

    /// Whether the token has been revoked.
    pub revoked: bool,

    /// When the token was revoked.
    pub revoked_at: Option<DateTime<Utc>>,

    /// Token creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last used timestamp.
    pub last_used_at: Option<DateTime<Utc>>,
}

impl OAuthToken {
    /// Creates a new OAuth token.
    #[must_use]
    pub fn new(
        user_id: UserId,
        token_hash: String,
        family_id: String,
        expires_at: DateTime<Utc>,
        device_id: Option<String>,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            id: OAuthTokenId::new(),
            user_id,
            token_hash,
            family_id,
            device_id,
            user_agent,
            ip_address,
            expires_at,
            revoked: false,
            revoked_at: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    /// Checks if the token is expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Checks if the token is valid (not expired and not revoked).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.revoked
    }

    /// Revokes the token.
    pub fn revoke(&mut self) {
        self.revoked = true;
        self.revoked_at = Some(Utc::now());
    }

    /// Records a token usage.
    pub fn record_usage(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

impl Entity<OAuthTokenId> for OAuthToken {
    fn id(&self) -> &OAuthTokenId {
        &self.id
    }
}

/// Token type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// Access token (short-lived).
    Access,
    /// Refresh token (long-lived).
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
    fn test_token_creation() {
        let user_id = UserId::new();
        let expires = Utc::now() + Duration::hours(1);
        let token = OAuthToken::new(
            user_id,
            "hash".to_string(),
            "family".to_string(),
            expires,
            None,
            None,
            None,
        );

        assert!(!token.is_expired());
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_revocation() {
        let user_id = UserId::new();
        let expires = Utc::now() + Duration::hours(1);
        let mut token = OAuthToken::new(
            user_id,
            "hash".to_string(),
            "family".to_string(),
            expires,
            None,
            None,
            None,
        );

        assert!(token.is_valid());
        token.revoke();
        assert!(!token.is_valid());
        assert!(token.revoked);
    }

    #[test]
    fn test_expired_token() {
        let user_id = UserId::new();
        let expires = Utc::now() - Duration::hours(1);
        let token = OAuthToken::new(
            user_id,
            "hash".to_string(),
            "family".to_string(),
            expires,
            None,
            None,
            None,
        );

        assert!(token.is_expired());
        assert!(!token.is_valid());
    }
}
