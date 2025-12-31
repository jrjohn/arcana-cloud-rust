//! Main application router.

use crate::{
    controllers::{auth_controller, health_controller, jobs_controller, user_controller},
    middleware::{auth_middleware, logging_middleware, AuthMiddlewareState},
    openapi::ApiDoc,
    state::AppState,
};
use arcana_config::ServerConfig;
use arcana_security::TokenProviderInterface;
use arcana_service::{AuthService, UserService};
use axum::{
    middleware,
    routing::get,
    Router,
};
use shaku::{HasComponent, Module};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Creates the main application router from a Shaku module.
///
/// This is the preferred way to create the router, using Shaku for dependency injection.
/// The module must provide UserService, AuthService, and TokenProviderInterface components.
pub fn create_router<M>(module: &M, server_config: &ServerConfig) -> Router
where
    M: Module
        + HasComponent<dyn UserService>
        + HasComponent<dyn AuthService>
        + HasComponent<dyn TokenProviderInterface>,
{
    // Create CORS layer
    let cors = create_cors_layer(server_config);

    // Get token provider from module for auth middleware
    let token_provider: Arc<dyn TokenProviderInterface> = module.resolve();
    let auth_state = AuthMiddlewareState::new(token_provider);

    // Create app state by resolving services from module
    let state = AppState::from_module(module);

    // Build the API router with authentication
    let api_router = Router::new()
        .nest("/auth", auth_controller::router())
        .nest("/users", user_controller::router())
        .nest("/jobs", jobs_controller::router())
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
        .with_state(state.clone());

    let router = Router::new()
        // Health endpoints (no auth required)
        .merge(health_controller::router())
        // API v1
        .nest("/api/v1", api_router)
        // Swagger UI and OpenAPI spec
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Root endpoint
        .route("/", get(root))
        // Add middleware layers
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(logging_middleware));

    info!("Router created with REST endpoints and Swagger UI at /swagger-ui");
    router
}

/// Creates a CORS layer based on server configuration.
fn create_cors_layer(server_config: &ServerConfig) -> CorsLayer {
    if server_config.cors_enabled {
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
    }
}

/// Root endpoint handler.
async fn root() -> &'static str {
    "Arcana Cloud Rust API v1"
}
