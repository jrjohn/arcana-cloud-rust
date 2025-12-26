//! JWT token provider for creating and validating tokens.

use super::{Claims, TokenType};
use arcana_config::SecurityConfig;
use arcana_core::{ArcanaError, ArcanaResult, UserId};
use arcana_domain::UserRole;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::sync::Arc;
use tracing::{debug, warn};

/// Token pair containing access and refresh tokens.
#[derive(Debug, Clone)]
pub struct TokenPair {
    /// Access token (short-lived).
    pub access_token: String,
    /// Refresh token (long-lived).
    pub refresh_token: String,
    /// Access token expiration timestamp.
    pub access_expires_at: i64,
    /// Refresh token expiration timestamp.
    pub refresh_expires_at: i64,
    /// Token type (always "Bearer").
    pub token_type: String,
}

/// JWT token provider service.
#[derive(Clone)]
pub struct TokenProvider {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    config: Arc<SecurityConfig>,
    validation: Validation,
}

impl TokenProvider {
    /// Creates a new token provider.
    #[must_use]
    pub fn new(config: Arc<SecurityConfig>) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&config.jwt_issuer]);
        validation.set_audience(&[&config.jwt_audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            encoding_key,
            decoding_key,
            config,
            validation,
        }
    }

    /// Generates a token pair for a user.
    pub fn generate_tokens(
        &self,
        user_id: UserId,
        username: &str,
        email: &str,
        role: UserRole,
    ) -> ArcanaResult<TokenPair> {
        let session_id = uuid::Uuid::now_v7().to_string();

        let access_token = self.generate_access_token(user_id, username, email, role)?;
        let refresh_token = self.generate_refresh_token(user_id, username, email, role, &session_id)?;

        let access_expires_at = (Utc::now() + Duration::seconds(self.config.jwt_access_expiration_secs as i64)).timestamp();
        let refresh_expires_at = (Utc::now() + Duration::seconds(self.config.jwt_refresh_expiration_secs as i64)).timestamp();

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_expires_at,
            refresh_expires_at,
            token_type: "Bearer".to_string(),
        })
    }

    /// Generates an access token.
    pub fn generate_access_token(
        &self,
        user_id: UserId,
        username: &str,
        email: &str,
        role: UserRole,
    ) -> ArcanaResult<String> {
        let expires_at = Utc::now() + Duration::seconds(self.config.jwt_access_expiration_secs as i64);

        let claims = Claims::new_access(
            user_id,
            username.to_string(),
            email.to_string(),
            role,
            self.config.jwt_issuer.clone(),
            self.config.jwt_audience.clone(),
            expires_at,
        );

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| ArcanaError::Internal(format!("Failed to generate access token: {}", e)))?;

        debug!("Generated access token for user {}", user_id);
        Ok(token)
    }

    /// Generates a refresh token.
    pub fn generate_refresh_token(
        &self,
        user_id: UserId,
        username: &str,
        email: &str,
        role: UserRole,
        session_id: &str,
    ) -> ArcanaResult<String> {
        let expires_at = Utc::now() + Duration::seconds(self.config.jwt_refresh_expiration_secs as i64);

        let claims = Claims::new_refresh(
            user_id,
            username.to_string(),
            email.to_string(),
            role,
            self.config.jwt_issuer.clone(),
            self.config.jwt_audience.clone(),
            expires_at,
            session_id.to_string(),
        );

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| ArcanaError::Internal(format!("Failed to generate refresh token: {}", e)))?;

        debug!("Generated refresh token for user {}", user_id);
        Ok(token)
    }

    /// Validates a token and returns the claims.
    pub fn validate_token(&self, token: &str) -> ArcanaResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                warn!("Token validation failed: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => ArcanaError::TokenExpired,
                    jsonwebtoken::errors::ErrorKind::InvalidToken
                    | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        ArcanaError::InvalidToken("Invalid token signature".to_string())
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                        ArcanaError::InvalidToken("Invalid token issuer".to_string())
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                        ArcanaError::InvalidToken("Invalid token audience".to_string())
                    }
                    _ => ArcanaError::InvalidToken(e.to_string()),
                }
            })?;

        Ok(token_data.claims)
    }

    /// Validates an access token specifically.
    pub fn validate_access_token(&self, token: &str) -> ArcanaResult<Claims> {
        let claims = self.validate_token(token)?;

        if !claims.is_access_token() {
            return Err(ArcanaError::InvalidToken("Expected access token".to_string()));
        }

        Ok(claims)
    }

    /// Validates a refresh token specifically.
    pub fn validate_refresh_token(&self, token: &str) -> ArcanaResult<Claims> {
        let claims = self.validate_token(token)?;

        if !claims.is_refresh_token() {
            return Err(ArcanaError::InvalidToken("Expected refresh token".to_string()));
        }

        Ok(claims)
    }

    /// Refreshes a token pair using a refresh token.
    pub fn refresh_tokens(&self, refresh_token: &str) -> ArcanaResult<TokenPair> {
        let claims = self.validate_refresh_token(refresh_token)?;

        let user_id = claims.user_id().ok_or_else(|| {
            ArcanaError::InvalidToken("Refresh token missing user ID".to_string())
        })?;

        self.generate_tokens(user_id, &claims.username, &claims.email, claims.role)
    }

    /// Decodes a token without validation (for inspection).
    pub fn decode_without_validation(&self, token: &str) -> ArcanaResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| ArcanaError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }
}

impl std::fmt::Debug for TokenProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenProvider")
            .field("issuer", &self.config.jwt_issuer)
            .field("audience", &self.config.jwt_audience)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider() -> TokenProvider {
        let config = SecurityConfig {
            jwt_secret: "test-secret-key-for-testing-only".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 86400,
            jwt_issuer: "test-issuer".to_string(),
            jwt_audience: "test-audience".to_string(),
            ..Default::default()
        };
        TokenProvider::new(Arc::new(config))
    }

    #[test]
    fn test_generate_and_validate_tokens() {
        let provider = create_test_provider();
        let user_id = UserId::new();

        let tokens = provider
            .generate_tokens(user_id, "testuser", "test@example.com", UserRole::User)
            .unwrap();

        let claims = provider.validate_access_token(&tokens.access_token).unwrap();
        assert_eq!(claims.username, "testuser");
        assert!(claims.is_access_token());

        let refresh_claims = provider.validate_refresh_token(&tokens.refresh_token).unwrap();
        assert!(refresh_claims.is_refresh_token());
    }

    #[test]
    fn test_refresh_tokens() {
        let provider = create_test_provider();
        let user_id = UserId::new();

        let tokens = provider
            .generate_tokens(user_id, "testuser", "test@example.com", UserRole::User)
            .unwrap();

        let new_tokens = provider.refresh_tokens(&tokens.refresh_token).unwrap();
        assert_ne!(tokens.access_token, new_tokens.access_token);
    }

    #[test]
    fn test_invalid_token() {
        let provider = create_test_provider();
        let result = provider.validate_token("invalid-token");
        assert!(result.is_err());
    }
}
