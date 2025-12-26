//! Authentication middleware.

use arcana_security::{Claims, TokenProvider};
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
    pub token_provider: Arc<TokenProvider>,
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
