//! Application configuration structures.

use crate::{CommunicationProtocol, DeploymentLayer, DeploymentMode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name and metadata.
    #[serde(default)]
    pub app: AppMetadata,

    /// Server configuration.
    #[serde(default)]
    pub server: ServerConfig,

    /// Deployment configuration.
    #[serde(default)]
    pub deployment: DeploymentConfig,

    /// Database configuration.
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Redis configuration.
    #[serde(default)]
    pub redis: RedisConfig,

    /// JWT/Security configuration.
    #[serde(default)]
    pub security: SecurityConfig,

    /// Plugin configuration.
    #[serde(default)]
    pub plugins: PluginConfig,

    /// SSR configuration.
    #[serde(default)]
    pub ssr: SsrConfig,

    /// Observability configuration.
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppMetadata::default(),
            server: ServerConfig::default(),
            deployment: DeploymentConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            security: SecurityConfig::default(),
            plugins: PluginConfig::default(),
            ssr: SsrConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Application metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Environment (development, staging, production).
    pub environment: String,
}

impl Default for AppMetadata {
    fn default() -> Self {
        Self {
            name: "arcana-cloud".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
        }
    }
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// REST server host.
    pub rest_host: String,
    /// REST server port.
    pub rest_port: u16,
    /// gRPC server host.
    pub grpc_host: String,
    /// gRPC server port.
    pub grpc_port: u16,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Enable CORS.
    pub cors_enabled: bool,
    /// CORS allowed origins.
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            rest_host: "0.0.0.0".to_string(),
            rest_port: 8080,
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 9090,
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

impl ServerConfig {
    /// Returns the REST server address.
    #[must_use]
    pub fn rest_addr(&self) -> String {
        format!("{}:{}", self.rest_host, self.rest_port)
    }

    /// Returns the gRPC server address.
    #[must_use]
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }

    /// Returns the request timeout as a Duration.
    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }
}

/// Deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeploymentConfig {
    /// Deployment mode.
    pub mode: DeploymentMode,
    /// Current layer for layered deployments.
    pub layer: DeploymentLayer,
    /// Communication protocol for inter-service communication.
    pub protocol: CommunicationProtocol,
    /// Service layer URL (for controller layer in layered mode).
    pub service_url: Option<String>,
    /// Repository layer URL (for service layer in layered mode).
    pub repository_url: Option<String>,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL.
    pub url: String,
    /// Minimum connection pool size.
    pub min_connections: u32,
    /// Maximum connection pool size.
    pub max_connections: u32,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Idle timeout in seconds.
    pub idle_timeout_secs: u64,
    /// Enable SQL query logging.
    pub log_queries: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "mysql://arcana:arcana@localhost:3306/arcana".to_string(),
            min_connections: 5,
            max_connections: 20,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
            log_queries: false,
        }
    }
}

impl DatabaseConfig {
    /// Returns the connect timeout as a Duration.
    #[must_use]
    pub const fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs)
    }

    /// Returns the idle timeout as a Duration.
    #[must_use]
    pub const fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout_secs)
    }
}

/// Redis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL.
    pub url: String,
    /// Connection pool size.
    pub pool_size: u32,
    /// Enable Redis (can be disabled for local development).
    pub enabled: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            enabled: true,
        }
    }
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret key.
    pub jwt_secret: String,
    /// JWT access token expiration in seconds.
    pub jwt_access_expiration_secs: u64,
    /// JWT refresh token expiration in seconds.
    pub jwt_refresh_expiration_secs: u64,
    /// JWT issuer.
    pub jwt_issuer: String,
    /// JWT audience.
    pub jwt_audience: String,
    /// Enable TLS for gRPC.
    pub grpc_tls_enabled: bool,
    /// Path to TLS certificate.
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key.
    pub tls_key_path: Option<String>,
    /// Password hashing cost (Argon2).
    pub password_hash_cost: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".to_string(),
            jwt_access_expiration_secs: 3600,      // 1 hour
            jwt_refresh_expiration_secs: 604800,   // 7 days
            jwt_issuer: "arcana-cloud".to_string(),
            jwt_audience: "arcana-api".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 12,
        }
    }
}

impl SecurityConfig {
    /// Returns the access token expiration as a Duration.
    #[must_use]
    pub const fn access_token_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_access_expiration_secs)
    }

    /// Returns the refresh token expiration as a Duration.
    #[must_use]
    pub const fn refresh_token_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_refresh_expiration_secs)
    }
}

/// Plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Enable plugin system.
    pub enabled: bool,
    /// Plugin directory path.
    pub directory: String,
    /// Enable plugin hot reload.
    pub hot_reload: bool,
    /// Plugin execution timeout in seconds.
    pub execution_timeout_secs: u64,
    /// Maximum plugin memory in bytes.
    pub max_memory_bytes: u64,
    /// Enable plugin signature verification.
    pub verify_signatures: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: "./plugins".to_string(),
            hot_reload: true,
            execution_timeout_secs: 30,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            verify_signatures: true,
        }
    }
}

impl PluginConfig {
    /// Returns the execution timeout as a Duration.
    #[must_use]
    pub const fn execution_timeout(&self) -> Duration {
        Duration::from_secs(self.execution_timeout_secs)
    }
}

/// SSR (Server-Side Rendering) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrConfig {
    /// Enable SSR.
    pub enabled: bool,
    /// JavaScript runtime pool size.
    pub runtime_pool_size: usize,
    /// SSR cache enabled.
    pub cache_enabled: bool,
    /// SSR cache TTL in seconds.
    pub cache_ttl_secs: u64,
    /// Maximum render time in milliseconds.
    pub render_timeout_ms: u64,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime_pool_size: 4,
            cache_enabled: true,
            cache_ttl_secs: 300, // 5 minutes
            render_timeout_ms: 5000,
        }
    }
}

impl SsrConfig {
    /// Returns the cache TTL as a Duration.
    #[must_use]
    pub const fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }

    /// Returns the render timeout as a Duration.
    #[must_use]
    pub const fn render_timeout(&self) -> Duration {
        Duration::from_millis(self.render_timeout_ms)
    }
}

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
    /// Log format (json, pretty).
    pub log_format: String,
    /// Enable metrics.
    pub metrics_enabled: bool,
    /// Metrics endpoint path.
    pub metrics_path: String,
    /// Enable request tracing.
    pub tracing_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_format: "pretty".to_string(),
            metrics_enabled: true,
            metrics_path: "/metrics".to_string(),
            tracing_enabled: true,
        }
    }
}
