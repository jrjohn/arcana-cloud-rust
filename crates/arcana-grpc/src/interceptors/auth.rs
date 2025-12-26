//! Authentication interceptor for gRPC.

use arcana_security::{Claims, TokenProvider};
use std::sync::Arc;
use tonic::{Request, Status};
use tracing::debug;

/// Authentication interceptor that validates JWT tokens.
pub fn auth_interceptor(
    token_provider: Arc<TokenProvider>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |mut request: Request<()>| {
        // Extract authorization header
        let auth_header = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok());

        if let Some(auth_header) = auth_header {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                // Validate token
                match token_provider.validate_access_token(token) {
                    Ok(claims) => {
                        debug!("gRPC: Authenticated user: {}", claims.username);
                        request.extensions_mut().insert(claims);
                    }
                    Err(e) => {
                        debug!("gRPC: Token validation failed: {}", e);
                        // Don't reject - let the service decide if auth is required
                    }
                }
            }
        }

        Ok(request)
    }
}

/// Extracts claims from a gRPC request.
pub fn extract_claims<T>(request: &Request<T>) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}

/// Requires authentication for a gRPC request.
pub fn require_auth<T>(request: &Request<T>) -> Result<&Claims, Status> {
    extract_claims(request).ok_or_else(|| Status::unauthenticated("Authentication required"))
}
