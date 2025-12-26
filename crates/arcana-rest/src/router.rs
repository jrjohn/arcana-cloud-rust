//! Main application router.

use crate::{
    controllers::{auth_controller, health_controller, user_controller},
    middleware::{auth_middleware, logging_middleware, AuthMiddlewareState},
    state::AppState,
};
use arcana_config::ServerConfig;
use arcana_security::TokenProvider;
use axum::{
    middleware,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

/// Creates the main application router.
pub fn create_router(
    state: AppState,
    token_provider: Arc<TokenProvider>,
    server_config: &ServerConfig,
) -> Router {
    // Create CORS layer
    let cors = if server_config.cors_enabled {
        if server_config.cors_origins.contains(&"*".to_string()) {
            CorsLayer::permissive()
        } else {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    } else {
        CorsLayer::new()
    };

    // Create auth middleware state
    let auth_state = AuthMiddlewareState { token_provider };

    // Build the API router with authentication
    let api_router = Router::new()
        .nest("/auth", auth_controller::router())
        .nest("/users", user_controller::router())
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
        .with_state(state.clone());

    let router = Router::new()
        // Health endpoints (no auth required)
        .merge(health_controller::router())
        // API v1
        .nest("/api/v1", api_router)
        // Root endpoint
        .route("/", get(root))
        // Add middleware layers
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(logging_middleware));

    info!("Router created with {} routes", "REST");
    router
}

/// Root endpoint handler.
async fn root() -> &'static str {
    "Arcana Cloud Rust API v1"
}
