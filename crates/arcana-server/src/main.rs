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
use arcana_repository::create_pool;
use arcana_rest::{create_router, AppState};
use tokio::signal;
use tracing::{error, info};

mod app;
pub mod di;
mod startup;

use di::AppModuleBuilder;

#[tokio::main]
async fn main() {
    // Initialize logging
    init_logging();

    info!("Starting Arcana Cloud Rust Server...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    if let Err(e) = run().await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> ArcanaResult<()> {
    // Load configuration
    let config_loader = ConfigLoader::from_default_location()?;
    let config = config_loader.get().await;

    info!("Environment: {}", config.app.environment);
    info!("Deployment mode: {}", config.deployment.mode);
    info!("Layer: {}", config.deployment.layer);

    // Initialize components based on deployment mode
    match config.deployment.mode {
        DeploymentMode::Monolithic => {
            run_monolithic(config).await?;
        }
        DeploymentMode::LayeredGrpc | DeploymentMode::KubernetesGrpc => {
            run_layered_grpc(config).await?;
        }
        DeploymentMode::LayeredHttp | DeploymentMode::KubernetesHttp => {
            run_layered_http(config).await?;
        }
    }

    Ok(())
}

/// Run in layered mode with gRPC inter-service communication.
async fn run_layered_grpc(config: AppConfig) -> ArcanaResult<()> {
    match config.deployment.layer {
        DeploymentLayer::All => {
            info!("Running all layers (equivalent to monolithic)");
            run_monolithic(config).await
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
    }
}

/// Run in layered mode with HTTP inter-service communication.
async fn run_layered_http(config: AppConfig) -> ArcanaResult<()> {
    match config.deployment.layer {
        DeploymentLayer::All => {
            info!("Running all layers (equivalent to monolithic)");
            run_monolithic(config).await
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
    }
}

async fn run_monolithic(config: AppConfig) -> ArcanaResult<()> {
    // Create database pool
    let db_pool = create_pool(&config.database).await?;

    // Run migrations
    db_pool.run_migrations().await?;

    // Build DI module - centralized dependency injection
    let module = AppModuleBuilder::new()
        .with_database_pool(db_pool)
        .with_security_config(config.security.clone())
        .with_password_hash_cost(config.security.password_hash_cost)
        .build();

    // Resolve services from DI container
    let user_service = module.user_service();
    let auth_service = module.auth_service();
    let token_provider = module.token_provider();

    // Create application state for REST
    let app_state = AppState::new(user_service.clone(), auth_service.clone());

    // Create REST router
    let router = create_router(app_state, token_provider, &config.server);

    // Start REST server
    let rest_addr = config.server.rest_addr();
    info!("Starting REST server on http://{}", rest_addr);

    let listener = tokio::net::TcpListener::bind(&rest_addr)
        .await
        .map_err(|e| arcana_core::ArcanaError::Internal(format!("Failed to bind REST: {}", e)))?;

    // Create gRPC server
    let grpc_server = arcana_grpc::GrpcServer::new(&config.server, user_service, auth_service)?;

    // Run both servers concurrently
    tokio::select! {
        result = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()) => {
            result.map_err(|e| arcana_core::ArcanaError::Internal(format!("REST server error: {}", e)))?;
        }
        result = grpc_server.serve() => {
            result?;
        }
    }

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
    let token_provider = std::sync::Arc::new(token_provider);

    // Create application state for REST
    let app_state = AppState::new(user_service, auth_service);

    // Create REST router
    let router = create_router(app_state, token_provider, &config.server);

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

    // Create remote repository client via gRPC
    let user_repository = std::sync::Arc::new(
        arcana_grpc::RemoteUserRepository::connect(repository_url).await?
    );

    // Create password hasher (local)
    let password_hasher = std::sync::Arc::new(arcana_security::PasswordHasher::with_cost(
        config.security.password_hash_cost,
    ));

    // Create token provider (local)
    let security_config = std::sync::Arc::new(config.security.clone());

    // Create services with remote repository
    let user_service: std::sync::Arc<dyn arcana_service::UserService> =
        std::sync::Arc::new(arcana_service::UserServiceImpl::new(
            user_repository.clone(),
            password_hasher.clone(),
        ));

    let auth_service: std::sync::Arc<dyn arcana_service::AuthService> =
        std::sync::Arc::new(arcana_service::AuthServiceImpl::new(
            user_repository,
            password_hasher,
            security_config,
        ));

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

    // Create database pool
    let db_pool = create_pool(&config.database).await?;

    // Run migrations
    db_pool.run_migrations().await?;

    // Create repository
    let user_repository: std::sync::Arc<dyn arcana_repository::UserRepository> =
        std::sync::Arc::new(arcana_repository::MySqlUserRepository::new(db_pool));

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

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,arcana=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            info!("Received terminate signal, initiating graceful shutdown...");
        }
    }
}
