//! Health check controller.

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Creates the health router.
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/live", get(liveness_check))
}

/// Health check endpoint.
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness check endpoint.
async fn readiness_check() -> impl IntoResponse {
    // In a full implementation, check database and other dependencies
    StatusCode::OK
}

/// Liveness check endpoint.
async fn liveness_check() -> impl IntoResponse {
    StatusCode::OK
}
