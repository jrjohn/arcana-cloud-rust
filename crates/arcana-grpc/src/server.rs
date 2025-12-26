//! gRPC server setup.

use crate::proto::{auth, health, repository, user};
use crate::services::{AuthGrpcService, HealthServiceImpl, RepositoryGrpcService, UserGrpcService};
use arcana_config::ServerConfig;
use arcana_core::ArcanaResult;
use arcana_repository::UserRepository;
use arcana_service::{AuthService, UserService};
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

/// gRPC server builder for service layer (exposes UserService and AuthService).
pub struct GrpcServer {
    addr: SocketAddr,
    user_service: Arc<dyn UserService>,
    auth_service: Arc<dyn AuthService>,
}

impl GrpcServer {
    /// Creates a new gRPC server.
    pub fn new(
        config: &ServerConfig,
        user_service: Arc<dyn UserService>,
        auth_service: Arc<dyn AuthService>,
    ) -> ArcanaResult<Self> {
        let addr = config.grpc_addr().parse().map_err(|e| {
            arcana_core::ArcanaError::Configuration(format!("Invalid gRPC address: {}", e))
        })?;

        Ok(Self {
            addr,
            user_service,
            auth_service,
        })
    }

    /// Starts the gRPC server.
    pub async fn serve(self) -> ArcanaResult<()> {
        info!("Starting gRPC server on {}", self.addr);

        // Create service implementations
        let health_service = HealthServiceImpl::new();
        let user_grpc_service = UserGrpcService::new(self.user_service);
        let auth_grpc_service = AuthGrpcService::new(self.auth_service);

        Server::builder()
            .add_service(health::health_server::HealthServer::new(health_service))
            .add_service(user::user_service_server::UserServiceServer::new(user_grpc_service))
            .add_service(auth::auth_service_server::AuthServiceServer::new(auth_grpc_service))
            .serve(self.addr)
            .await
            .map_err(|e| arcana_core::ArcanaError::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}

/// gRPC server for repository layer (exposes UserRepository).
pub struct RepositoryGrpcServer {
    addr: SocketAddr,
    user_repository: Arc<dyn UserRepository>,
}

impl RepositoryGrpcServer {
    /// Creates a new repository gRPC server.
    pub fn new(
        config: &ServerConfig,
        user_repository: Arc<dyn UserRepository>,
    ) -> ArcanaResult<Self> {
        let addr = config.grpc_addr().parse().map_err(|e| {
            arcana_core::ArcanaError::Configuration(format!("Invalid gRPC address: {}", e))
        })?;

        Ok(Self {
            addr,
            user_repository,
        })
    }

    /// Starts the repository gRPC server.
    pub async fn serve(self) -> ArcanaResult<()> {
        info!("Starting Repository gRPC server on {}", self.addr);

        let health_service = HealthServiceImpl::new();
        let repository_service = RepositoryGrpcService::new(self.user_repository);

        Server::builder()
            .add_service(health::health_server::HealthServer::new(health_service))
            .add_service(
                repository::repository_service_server::RepositoryServiceServer::new(
                    repository_service,
                ),
            )
            .serve(self.addr)
            .await
            .map_err(|e| arcana_core::ArcanaError::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}

/// Simple gRPC server without service injection (for testing/health only).
pub struct SimpleGrpcServer {
    addr: SocketAddr,
}

impl SimpleGrpcServer {
    /// Creates a new simple gRPC server.
    pub fn new(config: &ServerConfig) -> ArcanaResult<Self> {
        let addr = config.grpc_addr().parse().map_err(|e| {
            arcana_core::ArcanaError::Configuration(format!("Invalid gRPC address: {}", e))
        })?;

        Ok(Self { addr })
    }

    /// Starts the gRPC server with only health service.
    pub async fn serve(self) -> ArcanaResult<()> {
        info!("Starting gRPC server on {}", self.addr);

        let health_service = HealthServiceImpl::new();

        Server::builder()
            .add_service(health::health_server::HealthServer::new(health_service))
            .serve(self.addr)
            .await
            .map_err(|e| arcana_core::ArcanaError::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}
