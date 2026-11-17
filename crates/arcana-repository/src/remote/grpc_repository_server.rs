//! gRPC server for the repository tier (exposes [`UserRepository`] to remote
//! service tiers).

use super::RepositoryGrpcService;
use crate::UserRepository;
use arcana_config::{SecurityConfig, ServerConfig};
use arcana_core::ArcanaResult;
use arcana_proto::health::health_server::{Health, HealthServer};
use arcana_proto::repository::repository_service_server::RepositoryServiceServer;
use arcana_proto::TlsConfigBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::{Server, ServerTlsConfig};
use tracing::info;

/// gRPC server for repository layer (exposes `UserRepository`).
pub struct RepositoryGrpcServer {
    addr: SocketAddr,
    user_repository: Arc<dyn UserRepository>,
    tls_config: Option<ServerTlsConfig>,
}

impl RepositoryGrpcServer {
    /// Creates a new repository gRPC server.
    ///
    /// # Errors
    ///
    /// Returns `ArcanaError::Configuration` if `config.grpc_addr()` is not a valid socket address.
    pub fn new(
        config: &ServerConfig,
        user_repository: Arc<dyn UserRepository>,
    ) -> ArcanaResult<Self> {
        let addr = config.grpc_addr().parse().map_err(|e| {
            arcana_core::ArcanaError::Configuration(format!("Invalid gRPC address: {e}"))
        })?;

        Ok(Self {
            addr,
            user_repository,
            tls_config: None,
        })
    }

    /// Creates a new repository gRPC server with TLS configuration.
    ///
    /// # Errors
    ///
    /// Returns `ArcanaError::Configuration` if `config.grpc_addr()` is not a valid socket address,
    /// if TLS is enabled but the certificate or key path is missing, or if the certificate,
    /// key or CA file cannot be read.
    pub fn with_tls(
        config: &ServerConfig,
        security_config: &SecurityConfig,
        user_repository: Arc<dyn UserRepository>,
    ) -> ArcanaResult<Self> {
        let addr = config.grpc_addr().parse().map_err(|e| {
            arcana_core::ArcanaError::Configuration(format!("Invalid gRPC address: {e}"))
        })?;

        let tls_config = TlsConfigBuilder::from_security_config(security_config)?
            .map(|builder| builder.build_server_config())
            .transpose()?;

        Ok(Self {
            addr,
            user_repository,
            tls_config,
        })
    }

    /// Starts the repository gRPC server, hosting `health` next to the
    /// repository service.
    ///
    /// # Errors
    ///
    /// Returns `ArcanaError::Configuration` if the TLS configuration cannot be applied, and
    /// `ArcanaError::Internal` if the server fails to bind or terminates with a transport error.
    pub async fn serve<H: Health>(self, health: H) -> ArcanaResult<()> {
        let tls_status = if self.tls_config.is_some() { "with TLS" } else { "without TLS" };
        info!("Starting Repository gRPC server on {} {}", self.addr, tls_status);

        let repository_service = RepositoryGrpcService::new(self.user_repository);

        let mut builder = Server::builder();

        // Apply TLS if configured
        if let Some(tls_config) = self.tls_config {
            builder = builder.tls_config(tls_config).map_err(|e| {
                arcana_core::ArcanaError::Configuration(format!("Failed to configure TLS: {e}"))
            })?;
        }

        builder
            .add_service(HealthServer::new(health))
            .add_service(RepositoryServiceServer::new(repository_service))
            .serve(self.addr)
            .await
            .map_err(|e| arcana_core::ArcanaError::Internal(format!("gRPC server error: {e}")))?;

        Ok(())
    }
}
