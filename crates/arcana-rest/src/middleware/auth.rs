//! Authentication middleware.

use arcana_security::{Claims, TokenProvider, TokenProviderInterface};
use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::debug;

/// Authentication middleware state.
#[derive(Clone)]
pub struct AuthMiddlewareState {
    pub token_provider: Arc<dyn TokenProviderInterface>,
}

impl AuthMiddlewareState {
    /// Creates a new auth middleware state from a token provider.
    pub fn new(token_provider: Arc<dyn TokenProviderInterface>) -> Self {
        Self { token_provider }
    }

    /// Creates from a concrete TokenProvider (for backward compatibility).
    pub fn from_provider(provider: Arc<TokenProvider>) -> Self {
        Self {
            token_provider: provider,
        }
    }
}

/// Authentication middleware that validates JWT tokens.
///
/// This middleware extracts the token from the Authorization header,
/// validates it, and adds the claims to the request extensions.
pub async fn auth_middleware(
    State(state): State<AuthMiddlewareState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // Validate token
            match state.token_provider.validate_access_token(token) {
                Ok(claims) => {
                    debug!("Authenticated user: {}", claims.username);
                    request.extensions_mut().insert(claims);
                }
                Err(e) => {
                    debug!("Token validation failed: {}", e);
                    // Don't reject - just don't add claims
                    // The handler can decide if auth is required
                }
            }
        }
    }

    Ok(next.run(request).await)
}

/// Middleware that requires authentication.
///
/// Returns 401 if no valid token is present.
pub async fn require_auth(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check if claims are present in extensions
    if request.extensions().get::<Claims>().is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_config::SecurityConfig;
    use std::sync::Arc;

    #[test]
    fn auth_middleware_state_new_stores_token_provider() {
        let config = Arc::new(SecurityConfig {
            jwt_secret: "test-secret-at-least-32-chars-long!!".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test".to_string(),
            jwt_audience: "test".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        });
        let provider = Arc::new(TokenProvider::new(config));
        let state = AuthMiddlewareState::new(provider as Arc<dyn TokenProviderInterface>);
        // Verify state was created without panicking
        let _ = state.token_provider.clone();
    }

    #[test]
    fn auth_middleware_state_from_provider_creates_state() {
        let config = Arc::new(SecurityConfig {
            jwt_secret: "test-secret-at-least-32-chars-long!!".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test".to_string(),
            jwt_audience: "test".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        });
        let provider = Arc::new(TokenProvider::new(config));
        let state = AuthMiddlewareState::from_provider(provider);
        let _ = state.token_provider.clone();
    }

    #[test]
    fn auth_middleware_state_clone() {
        let config = Arc::new(SecurityConfig {
            jwt_secret: "test-secret-at-least-32-chars-long!!".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test".to_string(),
            jwt_audience: "test".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        });
        let provider = Arc::new(TokenProvider::new(config));
        let state = AuthMiddlewareState::new(provider as Arc<dyn TokenProviderInterface>);
        let _cloned = state.clone();
    }
}
