//! Application configuration structures.

use crate::{CommunicationProtocol, DeploymentLayer, DeploymentMode};
use arcana_core::Interface;
use serde::{Deserialize, Serialize};
use shaku::Component;
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
            url: std::env::var("DATABASE_URL").unwrap_or_default(),
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

/// Interface for security configuration.
///
/// This trait abstracts security configuration for dependency injection.
pub trait SecurityConfigInterface: Interface + Send + Sync {
    /// Returns the JWT secret key.
    fn jwt_secret(&self) -> &str;
    /// Returns the JWT access token expiration in seconds.
    fn jwt_access_expiration_secs(&self) -> u64;
    /// Returns the JWT refresh token expiration in seconds.
    fn jwt_refresh_expiration_secs(&self) -> u64;
    /// Returns the JWT issuer.
    fn jwt_issuer(&self) -> &str;
    /// Returns the JWT audience.
    fn jwt_audience(&self) -> &str;
    /// Returns whether TLS is enabled for gRPC.
    fn grpc_tls_enabled(&self) -> bool;
    /// Returns the TLS certificate path.
    fn tls_cert_path(&self) -> Option<&str>;
    /// Returns the TLS private key path.
    fn tls_key_path(&self) -> Option<&str>;
    /// Returns the password hashing cost.
    fn password_hash_cost(&self) -> u32;
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Component)]
#[shaku(interface = SecurityConfigInterface)]
pub struct SecurityConfig {
    /// JWT secret key.
    #[shaku(default)]
    pub jwt_secret: String,
    /// JWT access token expiration in seconds.
    #[shaku(default)]
    pub jwt_access_expiration_secs: u64,
    /// JWT refresh token expiration in seconds.
    #[shaku(default)]
    pub jwt_refresh_expiration_secs: u64,
    /// JWT issuer.
    #[shaku(default)]
    pub jwt_issuer: String,
    /// JWT audience.
    #[shaku(default)]
    pub jwt_audience: String,
    /// Enable TLS for gRPC.
    #[shaku(default)]
    pub grpc_tls_enabled: bool,
    /// Path to TLS certificate.
    #[shaku(default)]
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key.
    #[shaku(default)]
    pub tls_key_path: Option<String>,
    /// Password hashing cost (Argon2).
    #[shaku(default)]
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

impl SecurityConfigInterface for SecurityConfig {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    fn jwt_access_expiration_secs(&self) -> u64 {
        self.jwt_access_expiration_secs
    }

    fn jwt_refresh_expiration_secs(&self) -> u64 {
        self.jwt_refresh_expiration_secs
    }

    fn jwt_issuer(&self) -> &str {
        &self.jwt_issuer
    }

    fn jwt_audience(&self) -> &str {
        &self.jwt_audience
    }

    fn grpc_tls_enabled(&self) -> bool {
        self.grpc_tls_enabled
    }

    fn tls_cert_path(&self) -> Option<&str> {
        self.tls_cert_path.as_deref()
    }

    fn tls_key_path(&self) -> Option<&str> {
        self.tls_key_path.as_deref()
    }

    fn password_hash_cost(&self) -> u32 {
        self.password_hash_cost
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

    // OpenTelemetry settings
    /// Service name for distributed tracing.
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// OTLP endpoint URL (e.g., "http://localhost:4317").
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    /// Sampling ratio for traces (0.0 to 1.0).
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,
}

fn default_service_name() -> String {
    "arcana-cloud-rust".to_string()
}

fn default_sampling_ratio() -> f64 {
    1.0
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_format: "pretty".to_string(),
            metrics_enabled: true,
            metrics_path: "/metrics".to_string(),
            tracing_enabled: true,
            service_name: default_service_name(),
            otlp_endpoint: None,
            sampling_ratio: default_sampling_ratio(),
        }
    }
}

impl ObservabilityConfig {
    /// Convert to TelemetryConfig for arcana_core::telemetry.
    #[must_use]
    pub fn to_telemetry_config(&self) -> arcana_core::telemetry::TelemetryConfig {
        arcana_core::telemetry::TelemetryConfig {
            enabled: self.tracing_enabled && self.otlp_endpoint.is_some(),
            service_name: self.service_name.clone(),
            otlp_endpoint: self.otlp_endpoint.clone(),
            sampling_ratio: self.sampling_ratio,
            console_output: self.log_format == "pretty",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AppConfig tests
    // =========================================================================

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.app.name, "arcana-cloud");
        assert_eq!(config.app.environment, "development");
        assert_eq!(config.server.rest_port, 8080);
        assert_eq!(config.server.grpc_port, 9090);
        assert!(config.security.jwt_secret.len() > 0);
    }

    #[test]
    fn test_app_config_serialization_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.app.name, parsed.app.name);
        assert_eq!(config.server.rest_port, parsed.server.rest_port);
        assert_eq!(config.database.url, parsed.database.url);
    }

    // =========================================================================
    // AppMetadata tests
    // =========================================================================

    #[test]
    fn test_app_metadata_default() {
        let meta = AppMetadata::default();
        assert_eq!(meta.name, "arcana-cloud");
        assert_eq!(meta.environment, "development");
        assert!(!meta.version.is_empty());
    }

    // =========================================================================
    // ServerConfig tests
    // =========================================================================

    #[test]
    fn test_server_config_rest_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.rest_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_server_config_grpc_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.grpc_addr(), "0.0.0.0:9090");
    }

    #[test]
    fn test_server_config_custom_addr() {
        let config = ServerConfig {
            rest_host: "127.0.0.1".to_string(),
            rest_port: 3000,
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 50051,
            ..ServerConfig::default()
        };
        assert_eq!(config.rest_addr(), "127.0.0.1:3000");
        assert_eq!(config.grpc_addr(), "127.0.0.1:50051");
    }

    #[test]
    fn test_server_config_request_timeout() {
        let config = ServerConfig {
            request_timeout_secs: 60,
            ..ServerConfig::default()
        };
        assert_eq!(config.request_timeout().as_secs(), 60);
    }

    #[test]
    fn test_server_config_default_timeout() {
        let config = ServerConfig::default();
        assert_eq!(config.request_timeout().as_secs(), 30);
    }

    #[test]
    fn test_server_config_default_max_body_size() {
        let config = ServerConfig::default();
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
    }

    // =========================================================================
    // DatabaseConfig tests
    // =========================================================================

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        // URL comes from DATABASE_URL env var, no hardcoded value to assert
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 20);
    }

    #[test]
    fn test_database_config_connect_timeout() {
        let config = DatabaseConfig {
            connect_timeout_secs: 45,
            ..DatabaseConfig::default()
        };
        assert_eq!(config.connect_timeout().as_secs(), 45);
    }

    #[test]
    fn test_database_config_idle_timeout() {
        let config = DatabaseConfig {
            idle_timeout_secs: 300,
            ..DatabaseConfig::default()
        };
        assert_eq!(config.idle_timeout().as_secs(), 300);
    }

    #[test]
    fn test_database_config_default_timeouts() {
        let config = DatabaseConfig::default();
        assert_eq!(config.connect_timeout().as_secs(), 30);
        assert_eq!(config.idle_timeout().as_secs(), 600);
    }

    // =========================================================================
    // SecurityConfig tests
    // =========================================================================

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.jwt_access_expiration_secs, 3600);
        assert_eq!(config.jwt_refresh_expiration_secs, 604800);
        assert_eq!(config.jwt_issuer, "arcana-cloud");
        assert_eq!(config.jwt_audience, "arcana-api");
        assert!(!config.grpc_tls_enabled);
    }

    #[test]
    fn test_security_config_access_token_expiration() {
        let config = SecurityConfig::default();
        assert_eq!(config.access_token_expiration().as_secs(), 3600);
    }

    #[test]
    fn test_security_config_refresh_token_expiration() {
        let config = SecurityConfig::default();
        assert_eq!(config.refresh_token_expiration().as_secs(), 604800);
    }

    #[test]
    fn test_security_config_interface_methods() {
        let config = SecurityConfig::default();
        assert_eq!(config.jwt_secret(), "change-me-in-production");
        assert_eq!(config.jwt_access_expiration_secs(), 3600);
        assert_eq!(config.jwt_refresh_expiration_secs(), 604800);
        assert_eq!(config.jwt_issuer(), "arcana-cloud");
        assert_eq!(config.jwt_audience(), "arcana-api");
        assert!(!config.grpc_tls_enabled());
        assert!(config.tls_cert_path().is_none());
        assert!(config.tls_key_path().is_none());
        assert_eq!(config.password_hash_cost(), 12);
    }

    #[test]
    fn test_security_config_with_tls() {
        let config = SecurityConfig {
            grpc_tls_enabled: true,
            tls_cert_path: Some("/etc/certs/server.crt".to_string()),
            tls_key_path: Some("/etc/certs/server.key".to_string()),
            ..SecurityConfig::default()
        };
        assert!(config.grpc_tls_enabled());
        assert_eq!(config.tls_cert_path(), Some("/etc/certs/server.crt"));
        assert_eq!(config.tls_key_path(), Some("/etc/certs/server.key"));
    }

    // =========================================================================
    // PluginConfig tests
    // =========================================================================

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.enabled);
        assert_eq!(config.directory, "./plugins");
        assert!(config.hot_reload);
        assert_eq!(config.max_memory_bytes, 64 * 1024 * 1024);
    }

    #[test]
    fn test_plugin_config_execution_timeout() {
        let config = PluginConfig {
            execution_timeout_secs: 60,
            ..PluginConfig::default()
        };
        assert_eq!(config.execution_timeout().as_secs(), 60);
    }

    // =========================================================================
    // SsrConfig tests
    // =========================================================================

    #[test]
    fn test_ssr_config_default() {
        let config = SsrConfig::default();
        assert!(config.enabled);
        assert_eq!(config.runtime_pool_size, 4);
        assert!(config.cache_enabled);
        assert_eq!(config.cache_ttl_secs, 300);
    }

    #[test]
    fn test_ssr_config_cache_ttl() {
        let config = SsrConfig::default();
        assert_eq!(config.cache_ttl().as_secs(), 300);
    }

    #[test]
    fn test_ssr_config_render_timeout() {
        let config = SsrConfig {
            render_timeout_ms: 3000,
            ..SsrConfig::default()
        };
        assert_eq!(config.render_timeout().as_millis(), 3000);
    }

    // =========================================================================
    // ObservabilityConfig tests
    // =========================================================================

    #[test]
    fn test_observability_config_default() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.log_format, "pretty");
        assert!(config.metrics_enabled);
        assert_eq!(config.metrics_path, "/metrics");
        assert!(config.tracing_enabled);
        assert_eq!(config.service_name, "arcana-cloud-rust");
        assert!(config.otlp_endpoint.is_none());
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_observability_config_to_telemetry_config_no_endpoint() {
        let config = ObservabilityConfig::default();
        let telemetry = config.to_telemetry_config();
        // Without otlp_endpoint, should be disabled
        assert!(!telemetry.enabled);
        assert_eq!(telemetry.service_name, "arcana-cloud-rust");
    }

    #[test]
    fn test_observability_config_to_telemetry_config_with_endpoint() {
        let config = ObservabilityConfig {
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            ..ObservabilityConfig::default()
        };
        let telemetry = config.to_telemetry_config();
        assert!(telemetry.enabled);
        assert_eq!(telemetry.otlp_endpoint, Some("http://localhost:4317".to_string()));
    }

    #[test]
    fn test_observability_config_json_log_format() {
        let config = ObservabilityConfig {
            log_format: "json".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            ..ObservabilityConfig::default()
        };
        let telemetry = config.to_telemetry_config();
        assert!(!telemetry.console_output);
    }

    #[test]
    fn test_observability_config_pretty_log_format() {
        let config = ObservabilityConfig {
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            ..ObservabilityConfig::default()
        };
        let telemetry = config.to_telemetry_config();
        assert!(telemetry.console_output);
    }

    // =========================================================================
    // RedisConfig tests
    // =========================================================================

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.pool_size, 10);
        assert!(config.enabled);
    }

    // =========================================================================
    // DeploymentConfig tests
    // =========================================================================

    #[test]
    fn test_deployment_config_default() {
        let config = DeploymentConfig::default();
        assert_eq!(config.mode, DeploymentMode::Monolithic);
        assert_eq!(config.layer, DeploymentLayer::All);
        assert_eq!(config.protocol, CommunicationProtocol::Http);
        assert!(config.service_url.is_none());
        assert!(config.repository_url.is_none());
    }
}
