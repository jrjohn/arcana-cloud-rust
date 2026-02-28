//! Unified error types for all layers of the application.

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// Unified error type for all layers of Arcana Cloud.
///
/// This enum provides a comprehensive set of error variants that cover
/// domain, application, infrastructure, and presentation layer errors.
#[derive(Error, Debug)]
pub enum ArcanaError {
    // ============ Domain Errors ============
    /// Resource not found
    #[error("Resource not found: {resource_type} with id {id}")]
    NotFound {
        resource_type: &'static str,
        id: String,
    },

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Conflict error (e.g., duplicate entry)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Business rule violation
    #[error("Business rule violation: {0}")]
    BusinessRule(String),

    // ============ Authentication/Authorization Errors ============
    /// Unauthorized access
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Forbidden access
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Invalid token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    // ============ Infrastructure Errors ============
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// External service error
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    /// Redis/Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    // ============ Plugin Errors ============
    /// Plugin error
    #[error("Plugin error: {plugin_key} - {message}")]
    Plugin { plugin_key: String, message: String },

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    /// Plugin loading error
    #[error("Plugin loading error: {0}")]
    PluginLoading(String),

    /// Plugin execution error
    #[error("Plugin execution error: {plugin_key} - {message}")]
    PluginExecution { plugin_key: String, message: String },

    // ============ Resilience Errors ============
    /// Circuit breaker open
    #[error("Service unavailable: circuit breaker open for {0}")]
    CircuitBreakerOpen(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    // ============ SSR Errors ============
    /// SSR rendering error
    #[error("SSR rendering error: {0}")]
    SsrRendering(String),

    /// JavaScript runtime error
    #[error("JavaScript runtime error: {0}")]
    JsRuntime(String),

    // ============ Internal Errors ============
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Generic error wrapper
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ArcanaError {
    /// Returns the HTTP status code for this error.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 404,
            Self::Validation(_) | Self::BusinessRule(_) => 400,
            Self::Conflict(_) => 409,
            Self::Unauthorized(_) | Self::InvalidToken(_) | Self::TokenExpired | Self::InvalidCredentials => 401,
            Self::Forbidden(_) => 403,
            Self::CircuitBreakerOpen(_) | Self::Timeout(_) => 503,
            Self::RateLimitExceeded => 429,
            Self::ExternalService { .. } => 502,
            Self::Database(_)
            | Self::Configuration(_)
            | Self::Cache(_)
            | Self::Plugin { .. }
            | Self::PluginNotFound(_)
            | Self::PluginLoading(_)
            | Self::PluginExecution { .. }
            | Self::SsrRendering(_)
            | Self::JsRuntime(_)
            | Self::Internal(_)
            | Self::Other(_) => 500,
        }
    }

    /// Returns a machine-readable error code.
    #[must_use]
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Conflict(_) => "CONFLICT",
            Self::BusinessRule(_) => "BUSINESS_RULE_VIOLATION",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::InvalidToken(_) => "INVALID_TOKEN",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Configuration(_) => "CONFIGURATION_ERROR",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Cache(_) => "CACHE_ERROR",
            Self::Plugin { .. } => "PLUGIN_ERROR",
            Self::PluginNotFound(_) => "PLUGIN_NOT_FOUND",
            Self::PluginLoading(_) => "PLUGIN_LOADING_ERROR",
            Self::PluginExecution { .. } => "PLUGIN_EXECUTION_ERROR",
            Self::CircuitBreakerOpen(_) => "CIRCUIT_BREAKER_OPEN",
            Self::Timeout(_) => "TIMEOUT",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::SsrRendering(_) => "SSR_RENDERING_ERROR",
            Self::JsRuntime(_) => "JS_RUNTIME_ERROR",
            Self::Internal(_) | Self::Other(_) => "INTERNAL_ERROR",
        }
    }

    /// Creates a not found error for a resource.
    #[must_use]
    pub fn not_found<T: ToString>(resource_type: &'static str, id: T) -> Self {
        Self::NotFound {
            resource_type,
            id: id.to_string(),
        }
    }

    /// Creates a validation error.
    #[must_use]
    pub fn validation<T: Into<String>>(message: T) -> Self {
        Self::Validation(message.into())
    }

    /// Creates a conflict error.
    #[must_use]
    pub fn conflict<T: Into<String>>(message: T) -> Self {
        Self::Conflict(message.into())
    }

    /// Creates an unauthorized error.
    #[must_use]
    pub fn unauthorized<T: Into<String>>(message: T) -> Self {
        Self::Unauthorized(message.into())
    }

    /// Creates a forbidden error.
    #[must_use]
    pub fn forbidden<T: Into<String>>(message: T) -> Self {
        Self::Forbidden(message.into())
    }

    /// Creates an internal error.
    #[must_use]
    pub fn internal<T: Into<String>>(message: T) -> Self {
        Self::Internal(message.into())
    }

    /// Checks if this error is retriable.
    #[must_use]
    pub const fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::Database(_)
                | Self::ExternalService { .. }
                | Self::Cache(_)
                | Self::CircuitBreakerOpen(_)
                | Self::Timeout(_)
        )
    }

    /// Checks if this error should trigger circuit breaker.
    #[must_use]
    pub const fn should_trip_circuit_breaker(&self) -> bool {
        matches!(
            self,
            Self::Database(_) | Self::ExternalService { .. } | Self::Timeout(_)
        )
    }
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for ArcanaError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound {
                resource_type: "database_row",
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                // Check for unique constraint violation
                if let Some(code) = db_err.code() {
                    if code == "23505" || code == "1062" {
                        // PostgreSQL / MySQL unique violation
                        return Self::Conflict(db_err.message().to_string());
                    }
                }
                Self::Database(err.to_string())
            }
            _ => Self::Database(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for ArcanaError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON serialization error: {}", err))
    }
}

/// Serializable error response for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional field-level errors for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
    /// Request trace ID for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Field-level validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FieldError {
    /// Field name
    pub field: String,
    /// Error message
    pub message: String,
    /// Error code
    pub code: String,
}

impl ErrorResponse {
    /// Creates a new error response from an `ArcanaError`.
    #[must_use]
    pub fn from_error(error: &ArcanaError) -> Self {
        Self {
            code: error.error_code().to_string(),
            message: error.to_string(),
            details: None,
            trace_id: None,
        }
    }

    /// Sets the trace ID.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Sets field-level validation errors.
    #[must_use]
    pub fn with_details(mut self, details: Vec<FieldError>) -> Self {
        self.details = Some(details);
        self
    }
}

impl From<&ArcanaError> for ErrorResponse {
    fn from(error: &ArcanaError) -> Self {
        Self::from_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(ArcanaError::not_found("User", 1).status_code(), 404);
        assert_eq!(ArcanaError::validation("invalid email").status_code(), 400);
        assert_eq!(ArcanaError::unauthorized("not logged in").status_code(), 401);
        assert_eq!(ArcanaError::forbidden("no permission").status_code(), 403);
        assert_eq!(ArcanaError::conflict("duplicate").status_code(), 409);
        assert_eq!(ArcanaError::RateLimitExceeded.status_code(), 429);
    }

    #[test]
    fn test_error_status_codes_extended() {
        assert_eq!(ArcanaError::InvalidToken("bad".to_string()).status_code(), 401);
        assert_eq!(ArcanaError::TokenExpired.status_code(), 401);
        assert_eq!(ArcanaError::InvalidCredentials.status_code(), 401);
        assert_eq!(ArcanaError::Database("db error".to_string()).status_code(), 500);
        assert_eq!(ArcanaError::Internal("oops".to_string()).status_code(), 500);
        assert_eq!(ArcanaError::Timeout("timed out".to_string()).status_code(), 503);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ArcanaError::not_found("User", 1).error_code(), "NOT_FOUND");
        assert_eq!(ArcanaError::TokenExpired.error_code(), "TOKEN_EXPIRED");
    }

    #[test]
    fn test_error_codes_extended() {
        assert_eq!(ArcanaError::validation("bad input").error_code(), "VALIDATION_ERROR");
        assert_eq!(ArcanaError::conflict("duplicate").error_code(), "CONFLICT");
        assert_eq!(ArcanaError::unauthorized("no auth").error_code(), "UNAUTHORIZED");
        assert_eq!(ArcanaError::forbidden("no perm").error_code(), "FORBIDDEN");
        assert_eq!(ArcanaError::InvalidToken("bad".to_string()).error_code(), "INVALID_TOKEN");
        assert_eq!(ArcanaError::InvalidCredentials.error_code(), "INVALID_CREDENTIALS");
        assert_eq!(ArcanaError::Database("db".to_string()).error_code(), "DATABASE_ERROR");
        assert_eq!(ArcanaError::internal("err").error_code(), "INTERNAL_SERVER_ERROR");
        assert_eq!(ArcanaError::Timeout("t".to_string()).error_code(), "TIMEOUT");
        assert_eq!(ArcanaError::RateLimitExceeded.error_code(), "RATE_LIMIT_EXCEEDED");
    }

    #[test]
    fn test_retriable_errors() {
        assert!(ArcanaError::Database("connection lost".to_string()).is_retriable());
        assert!(ArcanaError::Timeout("request timed out".to_string()).is_retriable());
        assert!(!ArcanaError::not_found("User", 1).is_retriable());
    }

    #[test]
    fn test_non_retriable_errors() {
        assert!(!ArcanaError::validation("bad input").is_retriable());
        assert!(!ArcanaError::forbidden("no perm").is_retriable());
        assert!(!ArcanaError::unauthorized("no auth").is_retriable());
        assert!(!ArcanaError::conflict("dup").is_retriable());
        assert!(!ArcanaError::InvalidCredentials.is_retriable());
        assert!(!ArcanaError::RateLimitExceeded.is_retriable());
    }

    #[test]
    fn test_circuit_breaker_errors() {
        assert!(ArcanaError::Database("db error".to_string()).should_trip_circuit_breaker());
        assert!(ArcanaError::Timeout("timeout".to_string()).should_trip_circuit_breaker());
        assert!(!ArcanaError::validation("bad input").should_trip_circuit_breaker());
        assert!(!ArcanaError::not_found("User", 1).should_trip_circuit_breaker());
    }

    #[test]
    fn test_error_constructors() {
        let not_found = ArcanaError::not_found("User", "123");
        assert!(not_found.to_string().contains("User"));

        let validation = ArcanaError::validation("invalid field");
        assert!(validation.to_string().contains("invalid field"));

        let conflict = ArcanaError::conflict("duplicate entry");
        assert!(conflict.to_string().contains("duplicate entry"));

        let unauthorized = ArcanaError::unauthorized("no token");
        assert!(unauthorized.to_string().contains("no token"));

        let forbidden = ArcanaError::forbidden("no perms");
        assert!(forbidden.to_string().contains("no perms"));

        let internal = ArcanaError::internal("panic");
        assert!(internal.to_string().contains("panic"));
    }

    #[test]
    fn test_error_display() {
        let err = ArcanaError::TokenExpired;
        assert!(err.to_string().contains("Token expired") || !err.to_string().is_empty());
    }

    #[test]
    fn test_error_response_from_error() {
        let err = ArcanaError::not_found("User", 1);
        let response = ErrorResponse::from_error(&err);
        assert_eq!(response.code, "NOT_FOUND");
        assert!(!response.message.is_empty());
        assert!(response.details.is_none());
        assert!(response.trace_id.is_none());
    }

    #[test]
    fn test_error_response_with_trace_id() {
        let err = ArcanaError::not_found("User", 1);
        let response = ErrorResponse::from_error(&err)
            .with_trace_id("trace-123");
        assert_eq!(response.trace_id, Some("trace-123".to_string()));
    }

    #[test]
    fn test_error_response_with_details() {
        let err = ArcanaError::validation("bad input");
        let details = vec![FieldError {
            field: "email".to_string(),
            message: "Invalid email".to_string(),
            code: "INVALID_EMAIL".to_string(),
        }];
        let response = ErrorResponse::from_error(&err).with_details(details);
        assert!(response.details.is_some());
        assert_eq!(response.details.unwrap().len(), 1);
    }

    #[test]
    fn test_error_response_from_ref() {
        let err = ArcanaError::not_found("User", 42);
        let response: ErrorResponse = ErrorResponse::from(&err);
        assert_eq!(response.code, "NOT_FOUND");
    }
}
