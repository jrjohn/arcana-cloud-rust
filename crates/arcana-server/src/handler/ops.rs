//! Operations endpoints (`/health`, `/live`, `/ready`, `/metrics`) served by the
//! background job roles on the configured REST port.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;

/// Shared state for the operations endpoints.
#[derive(Clone)]
pub struct OpsState {
    /// Flipped once the role has finished its startup work.
    ready: Arc<AtomicBool>,
    metrics: PrometheusHandle,
}

impl OpsState {
    /// Creates the operations state from the readiness flag and metrics handle.
    #[must_use]
    pub fn new(ready: Arc<AtomicBool>, metrics: PrometheusHandle) -> Self {
        Self { ready, metrics }
    }
}

/// Builds the operations router.
pub fn ops_router(state: OpsState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/live", get(health))
        .route("/ready", get(readiness))
        .route("/metrics", get(render_metrics))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn readiness(State(state): State<OpsState>) -> impl IntoResponse {
    if state.ready.load(Ordering::SeqCst) {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

async fn render_metrics(State(state): State<OpsState>) -> impl IntoResponse {
    (StatusCode::OK, state.metrics.render())
}
