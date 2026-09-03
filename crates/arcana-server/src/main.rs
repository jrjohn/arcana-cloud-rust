//! # Arcana Cloud Rust Server
//!
//! Main entry point for the Arcana Cloud Rust application.
//!
//! Supports multiple deployment modes:
//! - **Monolithic**: All layers in a single process
//! - **LayeredGrpc**: Distributed layers with gRPC communication
//! - **LayeredHttp**: Distributed layers with HTTP communication

use arcana_config::{AppConfig, ConfigLoader, DeploymentLayer, DeploymentMode};
use arcana_core::ArcanaResult;
use arcana_jobs::JobsConfig;
use arcana_rest::create_router;
use tracing::{error, info};

use arcana_server::di::{
    build_distributed_service_module, build_monolithic_module, build_repository_module,
    DatabaseResolver, RepositoryResolver, ServiceResolver,
};
use arcana_server::jobs_runtime;
use arcana_server::shutdown::wait_for_signal as shutdown_signal;

#[tokio::main]
async fn main() {
    // Load configuration first (needed for telemetry setup)
    let (config, jobs_config) = match load_config().await {
        Ok(loaded) => loaded,
        Err(e) => {
            // Fall back to basic logging if config fails
            init_basic_logging();
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize telemetry/logging based on config
    if let Err(e) = init_telemetry(&config) {
        init_basic_logging();
        eprintln!("Failed to initialize telemetry: {}", e);
    }

    info!("Starting Arcana Cloud Rust Server...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    if let Err(e) = run_with_config(config, jobs_config).await {
        error!("Application error: {}", e);
        arcana_core::telemetry::shutdown_telemetry();
        std::process::exit(1);
    }

    arcana_core::telemetry::shutdown_telemetry();
}

/// Loads the application configuration plus the `[jobs]` section.
///
/// `arcana-jobs` depends on `arcana-config`, so `JobsConfig` cannot live inside
/// `AppConfig`. It is read through the same loader instead of a second set of
/// rules, so file layering and `ARCANA_` env overrides behave identically.
async fn load_config() -> ArcanaResult<(AppConfig, JobsConfig)> {
    let config_loader = ConfigLoader::from_default_location()?;
    let app_config = config_loader.get().await;
    let jobs_config = config_loader
        .section::<JobsConfig>("jobs")
        .await?
        .unwrap_or_default();
    Ok((app_config, jobs_config))
}

fn init_telemetry(config: &AppConfig) -> ArcanaResult<()> {
    let telemetry_config = config.observability.to_telemetry_config();
    arcana_core::telemetry::init_telemetry(&telemetry_config)
}

fn init_basic_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,arcana=debug"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .try_init();
}

async fn run_with_config(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    info!("Environment: {}", config.app.environment);
    info!("Deployment mode: {}", config.deployment.mode);
    info!("Layer: {}", config.deployment.layer);

    // Background job roles serve no API and open no database pool, so they are
    // dispatched on the layer alone -- they behave the same under every
    // deployment mode, including plain Docker Compose.
    if config.deployment.layer.is_job_role() {
        return match config.deployment.layer {
            DeploymentLayer::Worker => jobs_runtime::run_worker(config, jobs_config).await,
            DeploymentLayer::Scheduler => jobs_runtime::run_scheduler(config, jobs_config).await,
            other => Err(arcana_core::ArcanaError::Configuration(format!(
                "layer '{other}' is not a job role"
            ))),
        };
    }

    // Initialize components based on deployment mode
    match config.deployment.mode {
        DeploymentMode::Monolithic => {
            run_monolithic(config, jobs_config).await?;
        }
        DeploymentMode::LayeredGrpc | DeploymentMode::KubernetesGrpc => {
            run_layered_grpc(config, jobs_config).await?;
        }
        DeploymentMode::LayeredHttp | DeploymentMode::KubernetesHttp => {
            run_layered_http(config, jobs_config).await?;
        }
    }

    Ok(())
}

/// Run in layered mode with gRPC inter-service communication.
async fn run_layered_grpc(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    match config.deployment.layer {
        DeploymentLayer::All => {
            info!("Running all layers (equivalent to monolithic)");
            run_monolithic(config, jobs_config).await
        }
        DeploymentLayer::Controller => {
            run_controller_layer_grpc(config).await
        }
        DeploymentLayer::Service => {
            run_service_layer_grpc(config).await
        }
        DeploymentLayer::Repository => {
            run_repository_layer(config).await
        }
        DeploymentLayer::Worker | DeploymentLayer::Scheduler => {
            // Unreachable: job roles are dispatched before the mode branch.
            unreachable!("job roles are dispatched in run_with_config")
        }
    }
}

/// Run in layered mode with HTTP inter-service communication.
async fn run_layered_http(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    match config.deployment.layer {
        DeploymentLayer::All => {
            info!("Running all layers (equivalent to monolithic)");
            run_monolithic(config, jobs_config).await
        }
        DeploymentLayer::Controller => {
            run_controller_layer_http(config).await
        }
        DeploymentLayer::Service => {
            run_service_layer_http(config).await
        }
        DeploymentLayer::Repository => {
            run_repository_layer(config).await
        }
        DeploymentLayer::Worker | DeploymentLayer::Scheduler => {
            // Unreachable: job roles are dispatched before the mode branch.
            unreachable!("job roles are dispatched in run_with_config")
        }
    }
}

async fn run_monolithic(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    // Build Shaku DI module with all components
    let module = build_monolithic_module(&config.database, &config.redis, config.security.clone()).await?;

    // Run migrations using the resolved database pool
    module.database_pool().run_migrations().await?;

    // Create REST router from module
    let router = create_router(module.as_ref(), &config.server);

    // Resolve services for gRPC server
    let user_service = module.user_service();
    let auth_service = module.auth_service();

    // Start REST server
    let rest_addr = config.server.rest_addr();
    info!("Starting REST server on http://{}", rest_addr);

    let listener = tokio::net::TcpListener::bind(&rest_addr)
        .await
        .map_err(|e| arcana_core::ArcanaError::Internal(format!("Failed to bind REST: {}", e)))?;

    // Create gRPC server
    let grpc_server = arcana_grpc::GrpcServer::new(&config.server, user_service, auth_service)?;

    // A monolithic deployment is one container that does everything, so it
    // hosts the job runtime in-process rather than in separate pods.
    let embedded_jobs = jobs_runtime::spawn_embedded(&jobs_config).await?;

    // Run both servers concurrently
    let result = tokio::select! {
        result = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()) => {
            result.map_err(|e| arcana_core::ArcanaError::Internal(format!("REST server error: {}", e)))
        }
        result = grpc_server.serve() => {
            result
        }
    };

    if let Some(jobs) = embedded_jobs {
        jobs.shutdown().await;
    }
    result?;

    info!("Server shutdown complete");
    Ok(())
}

/// Controller layer: Exposes REST API, calls Service layer via gRPC.
async fn run_controller_layer_grpc(config: AppConfig) -> ArcanaResult<()> {
    info!("Starting Controller Layer (gRPC upstream)");

    let service_url = config.deployment.service_url.as_ref().ok_or_else(|| {
        arcana_core::ArcanaError::Configuration(
            "service_url is required for controller layer".to_string(),
        )
    })?;

    info!("Connecting to service layer at: {}", service_url);

    // Create remote service clients via gRPC
    let user_service = arcana_grpc::create_remote_user_service(service_url).await?;
    let auth_service = arcana_grpc::create_remote_auth_service(service_url).await?;

    // Create token provider for JWT validation (still local)
    let token_provider = arcana_security::TokenProvider::new(std::sync::Arc::new(config.security.clone()));
    let token_provider: std::sync::Arc<dyn arcana_security::TokenProviderInterface> =
        std::sync::Arc::new(token_provider);

    // Create application state for REST
    let app_state = arcana_rest::AppState::new(user_service, auth_service);

    // Create REST router with state and token provider
    // Note: For controller layer, we use the legacy AppState approach
    // since we're using remote services rather than a Shaku module
    let router = create_router_legacy(app_state, token_provider, &config.server);

    // Start REST server only (controller doesn't expose gRPC)
    let rest_addr = config.server.rest_addr();
    info!("Starting REST server on http://{}", rest_addr);

    let listener = tokio::net::TcpListener::bind(&rest_addr)
        .await
        .map_err(|e| arcana_core::ArcanaError::Internal(format!("Failed to bind REST: {}", e)))?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| arcana_core::ArcanaError::Internal(format!("REST server error: {}", e)))?;

    info!("Controller layer shutdown complete");
    Ok(())
}

/// Controller layer: Exposes REST API, calls Service layer via HTTP.
async fn run_controller_layer_http(config: AppConfig) -> ArcanaResult<()> {
    info!("Starting Controller Layer (HTTP upstream)");

    // TODO: Implement HTTP client for service layer
    // For now, fall back to gRPC
    info!("HTTP upstream not yet implemented, using gRPC");
    run_controller_layer_grpc(config).await
}

/// Service layer: Exposes gRPC services, calls Repository layer via gRPC.
async fn run_service_layer_grpc(config: AppConfig) -> ArcanaResult<()> {
    info!("Starting Service Layer (gRPC upstream to repository)");

    let repository_url = config.deployment.repository_url.as_ref().ok_or_else(|| {
        arcana_core::ArcanaError::Configuration(
            "repository_url is required for service layer".to_string(),
        )
    })?;

    info!("Connecting to repository layer at: {}", repository_url);

    // Build Shaku distributed service module
    let module = build_distributed_service_module(repository_url, &config.redis, config.security.clone()).await?;

    // Resolve services from module
    let user_service = module.user_service();
    let auth_service = module.auth_service();

    // Create gRPC server to expose services
    let grpc_server =
        arcana_grpc::GrpcServer::new(&config.server, user_service.clone(), auth_service.clone())?;

    info!(
        "Starting gRPC server on {}",
        config.server.grpc_addr()
    );

    grpc_server.serve().await?;

    info!("Service layer shutdown complete");
    Ok(())
}

/// Service layer: Exposes gRPC services, calls Repository layer via HTTP.
async fn run_service_layer_http(config: AppConfig) -> ArcanaResult<()> {
    info!("Starting Service Layer (HTTP upstream to repository)");

    // TODO: Implement HTTP client for repository layer
    // For now, fall back to gRPC
    info!("HTTP upstream not yet implemented, using gRPC");
    run_service_layer_grpc(config).await
}

/// Repository layer: Connects to database, exposes repository via gRPC.
async fn run_repository_layer(config: AppConfig) -> ArcanaResult<()> {
    info!("Starting Repository Layer");

    // Build Shaku repository module
    let module = build_repository_module(&config.database).await?;

    // Run migrations
    module.database_pool().run_migrations().await?;

    // Resolve repository from module
    let user_repository = RepositoryResolver::user_repository(module.as_ref());

    // Create gRPC server to expose repository
    let grpc_server = arcana_grpc::RepositoryGrpcServer::new(&config.server, user_repository)?;

    info!(
        "Starting Repository gRPC server on {}",
        config.server.grpc_addr()
    );

    grpc_server.serve().await?;

    info!("Repository layer shutdown complete");
    Ok(())
}

/// Creates a REST router using legacy AppState (for controller layer with remote services).
fn create_router_legacy(
    state: arcana_rest::AppState,
    token_provider: std::sync::Arc<dyn arcana_security::TokenProviderInterface>,
    server_config: &arcana_config::ServerConfig,
) -> axum::Router {
    use arcana_rest::middleware::{auth_middleware, logging_middleware, AuthMiddlewareState};
    use axum::{middleware, routing::get, Router};
    use tower_http::{
        compression::CompressionLayer,
        cors::{Any, CorsLayer},
        trace::TraceLayer,
    };

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
    let auth_state = AuthMiddlewareState::new(token_provider);

    // Build the API router with authentication
    let api_router = Router::new()
        .nest("/auth", arcana_rest::controllers::auth_controller::router())
        .nest("/users", arcana_rest::controllers::user_controller::router())
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
        .with_state(state.clone());

    Router::new()
        // Health endpoints (no auth required)
        .merge(arcana_rest::controllers::health_controller::router())
        // API v1
        .nest("/api/v1", api_router)
        // Root endpoint
        .route("/", get(|| async { "Arcana Cloud Rust API v1" }))
        // Add middleware layers
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(logging_middleware))
}
