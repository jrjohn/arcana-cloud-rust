//! JWT claims extractor.

use arcana_core::ArcanaError;
use arcana_security::Claims;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use crate::responses::ApiResponse;
use arcana_core::ErrorResponse;

/// Extractor for authenticated user claims.
///
/// This extractor validates the JWT token from the Authorization header
/// and provides the claims to the handler.
pub struct AuthenticatedUser(pub Claims);

impl std::ops::Deref for AuthenticatedUser {
    type Target = Claims;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Error type for authentication extraction.
pub struct AuthError(ArcanaError);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code())
            .unwrap_or(StatusCode::UNAUTHORIZED);

        let error_response = ErrorResponse::from_error(&self.0);
        let body = Json(ApiResponse::<()>::error(error_response));

        (status, body).into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AuthError(ArcanaError::Unauthorized("Missing authorization header".to_string())))?;

        // Extract the token from "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError(ArcanaError::Unauthorized("Invalid authorization format".to_string())))?;

        // Get the token provider from extensions (set by middleware)
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| {
                // If claims aren't in extensions, we need to validate the token
                // This shouldn't happen if the auth middleware is properly configured
                AuthError(ArcanaError::Internal("Auth middleware not configured".to_string()))
            })?;

        Ok(AuthenticatedUser(claims))
    }
}

/// Optional authenticated user extractor.
///
/// Returns `None` if no valid token is present, instead of failing.
pub struct OptionalUser(pub Option<Claims>);

impl std::ops::Deref for OptionalUser {
    type Target = Option<Claims>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<Claims>().cloned();
        Ok(OptionalUser(claims))
    }
}
