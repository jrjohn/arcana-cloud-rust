//! Configuration validation module.
//!
//! Provides comprehensive validation for all configuration values,
//! failing fast on invalid configuration rather than at runtime.

use crate::AppConfig;
use std::fmt;
use url::Url;

/// Configuration validation error variants.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValidationError {
    /// JWT secret is too short (minimum 32 characters for security).
    JwtSecretTooShort { actual: usize, minimum: usize },
    /// Port number is invalid (must be 1-65535).
    InvalidPort { name: String, value: u16 },
    /// REST and gRPC ports conflict.
    PortConflict { rest: u16, grpc: u16 },
    /// Pool size configuration is invalid (min must be <= max).
    InvalidPoolSize { min: u32, max: u32 },
    /// Pool size exceeds maximum allowed.
    PoolSizeTooLarge { value: u32, maximum: u32 },
    /// URL format is invalid.
    InvalidUrl { url_type: String, message: String },
    /// TLS certificate path required when TLS is enabled.
    MissingTlsCert,
    /// TLS key path required when TLS is enabled.
    MissingTlsKey,
    /// Sampling ratio must be between 0.0 and 1.0.
    InvalidSamplingRatio { value: f64 },
    /// Timeout value must be positive.
    NonPositiveTimeout { name: String, value: u64 },
    /// Password hash cost is invalid.
    InvalidHashCost { value: u32, minimum: u32, maximum: u32 },
    /// Log level is invalid.
    InvalidLogLevel { value: String },
    /// Service URL required for layered deployment.
    MissingServiceUrl,
    /// Repository URL required for layered deployment.
    MissingRepositoryUrl,
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JwtSecretTooShort { actual, minimum } => {
                write!(
                    f,
                    "JWT secret too short: {} characters (minimum {})",
                    actual, minimum
                )
            }
            Self::InvalidPort { name, value } => {
                write!(f, "Invalid port for {}: {} (must be 1-65535)", name, value)
            }
            Self::PortConflict { rest, grpc } => {
                write!(
                    f,
                    "REST port ({}) and gRPC port ({}) cannot be the same",
                    rest, grpc
                )
            }
            Self::InvalidPoolSize { min, max } => {
                write!(
                    f,
                    "Invalid pool size: min ({}) cannot be greater than max ({})",
                    min, max
                )
            }
            Self::PoolSizeTooLarge { value, maximum } => {
                write!(
                    f,
                    "Pool size {} exceeds maximum allowed ({})",
                    value, maximum
                )
            }
            Self::InvalidUrl { url_type, message } => {
                write!(f, "Invalid {} URL: {}", url_type, message)
            }
            Self::MissingTlsCert => {
                write!(f, "TLS certificate path required when gRPC TLS is enabled")
            }
            Self::MissingTlsKey => {
                write!(f, "TLS key path required when gRPC TLS is enabled")
            }
            Self::InvalidSamplingRatio { value } => {
                write!(
                    f,
                    "Invalid sampling ratio: {} (must be between 0.0 and 1.0)",
                    value
                )
            }
            Self::NonPositiveTimeout { name, value } => {
                write!(f, "Timeout '{}' must be positive, got {}", name, value)
            }
            Self::InvalidHashCost { value, minimum, maximum } => {
                write!(
                    f,
                    "Invalid password hash cost: {} (must be between {} and {})",
                    value, minimum, maximum
                )
            }
            Self::InvalidLogLevel { value } => {
                write!(
                    f,
                    "Invalid log level: '{}' (valid: trace, debug, info, warn, error)",
                    value
                )
            }
            Self::MissingServiceUrl => {
                write!(f, "Service URL required for controller layer in layered deployment")
            }
            Self::MissingRepositoryUrl => {
                write!(f, "Repository URL required for service layer in layered deployment")
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Result of configuration validation containing all errors found.
#[derive(Debug)]
pub struct ValidationResult {
    errors: Vec<ConfigValidationError>,
}

impl ValidationResult {
    /// Creates a new validation result.
    fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Adds an error to the result.
    fn add_error(&mut self, error: ConfigValidationError) {
        self.errors.push(error);
    }

    /// Returns true if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the validation errors.
    pub fn errors(&self) -> &[ConfigValidationError] {
        &self.errors
    }

    /// Converts to Result, returning Err with all errors if any exist.
    pub fn into_result(self) -> Result<(), Vec<ConfigValidationError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }
}

/// Configuration validator.
pub struct ConfigValidator;

impl ConfigValidator {
    /// Minimum JWT secret length for security.
    const MIN_JWT_SECRET_LENGTH: usize = 32;
    /// Maximum connection pool size.
    const MAX_POOL_SIZE: u32 = 1000;
    /// Minimum Argon2 hash cost.
    const MIN_HASH_COST: u32 = 4;
    /// Maximum Argon2 hash cost.
    const MAX_HASH_COST: u32 = 31;
    /// Valid log levels.
    const VALID_LOG_LEVELS: &'static [&'static str] = &["trace", "debug", "info", "warn", "error"];

    /// Validates the entire application configuration.
    ///
    /// Returns Ok(()) if valid, or Err with all validation errors found.
    pub fn validate(config: &AppConfig) -> Result<(), Vec<ConfigValidationError>> {
        let mut result = ValidationResult::new();

        Self::validate_security(&config.security, &mut result);
        Self::validate_server(&config.server, &mut result);
        Self::validate_database(&config.database, &mut result);
        Self::validate_redis(&config.redis, &mut result);
        Self::validate_observability(&config.observability, &mut result);
        Self::validate_deployment(&config.deployment, &mut result);
        Self::validate_ssr(&config.ssr, &mut result);
        Self::validate_plugins(&config.plugins, &mut result);

        result.into_result()
    }

    /// Validates security configuration.
    fn validate_security(config: &crate::SecurityConfig, result: &mut ValidationResult) {
        // JWT secret length
        if config.jwt_secret.len() < Self::MIN_JWT_SECRET_LENGTH {
            result.add_error(ConfigValidationError::JwtSecretTooShort {
                actual: config.jwt_secret.len(),
                minimum: Self::MIN_JWT_SECRET_LENGTH,
            });
        }

        // TLS configuration consistency
        if config.grpc_tls_enabled {
            if config.tls_cert_path.is_none() {
                result.add_error(ConfigValidationError::MissingTlsCert);
            }
            if config.tls_key_path.is_none() {
                result.add_error(ConfigValidationError::MissingTlsKey);
            }
        }

        // Password hash cost
        if config.password_hash_cost < Self::MIN_HASH_COST
            || config.password_hash_cost > Self::MAX_HASH_COST
        {
            result.add_error(ConfigValidationError::InvalidHashCost {
                value: config.password_hash_cost,
                minimum: Self::MIN_HASH_COST,
                maximum: Self::MAX_HASH_COST,
            });
        }

        // Token expiration times
        if config.jwt_access_expiration_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "jwt_access_expiration_secs".to_string(),
                value: 0,
            });
        }
        if config.jwt_refresh_expiration_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "jwt_refresh_expiration_secs".to_string(),
                value: 0,
            });
        }
    }

    /// Validates server configuration.
    fn validate_server(config: &crate::ServerConfig, result: &mut ValidationResult) {
        // Port validation (0 is invalid for binding)
        if config.rest_port == 0 {
            result.add_error(ConfigValidationError::InvalidPort {
                name: "rest_port".to_string(),
                value: config.rest_port,
            });
        }
        if config.grpc_port == 0 {
            result.add_error(ConfigValidationError::InvalidPort {
                name: "grpc_port".to_string(),
                value: config.grpc_port,
            });
        }

        // Port conflict (same host)
        if config.rest_host == config.grpc_host && config.rest_port == config.grpc_port {
            result.add_error(ConfigValidationError::PortConflict {
                rest: config.rest_port,
                grpc: config.grpc_port,
            });
        }

        // Request timeout
        if config.request_timeout_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "request_timeout_secs".to_string(),
                value: 0,
            });
        }
    }

    /// Validates database configuration.
    fn validate_database(config: &crate::DatabaseConfig, result: &mut ValidationResult) {
        // URL format validation
        if config.url.is_empty() {
            result.add_error(ConfigValidationError::InvalidUrl {
                url_type: "database".to_string(),
                message: "URL cannot be empty".to_string(),
            });
        } else if !config.url.starts_with("mysql://")
            && !config.url.starts_with("postgres://")
            && !config.url.starts_with("postgresql://")
            && !config.url.starts_with("sqlite://")
        {
            result.add_error(ConfigValidationError::InvalidUrl {
                url_type: "database".to_string(),
                message: "URL must start with mysql://, postgres://, postgresql://, or sqlite://".to_string(),
            });
        }

        // Pool size validation
        if config.min_connections > config.max_connections {
            result.add_error(ConfigValidationError::InvalidPoolSize {
                min: config.min_connections,
                max: config.max_connections,
            });
        }
        if config.max_connections > Self::MAX_POOL_SIZE {
            result.add_error(ConfigValidationError::PoolSizeTooLarge {
                value: config.max_connections,
                maximum: Self::MAX_POOL_SIZE,
            });
        }

        // Timeouts
        if config.connect_timeout_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "database.connect_timeout_secs".to_string(),
                value: 0,
            });
        }
        if config.idle_timeout_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "database.idle_timeout_secs".to_string(),
                value: 0,
            });
        }
    }

    /// Validates Redis configuration.
    fn validate_redis(config: &crate::RedisConfig, result: &mut ValidationResult) {
        if !config.enabled {
            return;
        }

        // URL format validation
        if !config.url.starts_with("redis://") && !config.url.starts_with("rediss://") {
            result.add_error(ConfigValidationError::InvalidUrl {
                url_type: "redis".to_string(),
                message: "URL must start with redis:// or rediss://".to_string(),
            });
        }

        // Pool size
        if config.pool_size > Self::MAX_POOL_SIZE {
            result.add_error(ConfigValidationError::PoolSizeTooLarge {
                value: config.pool_size,
                maximum: Self::MAX_POOL_SIZE,
            });
        }
    }

    /// Validates observability configuration.
    fn validate_observability(config: &crate::ObservabilityConfig, result: &mut ValidationResult) {
        // Log level
        let level = config.log_level.to_lowercase();
        if !Self::VALID_LOG_LEVELS.contains(&level.as_str()) {
            result.add_error(ConfigValidationError::InvalidLogLevel {
                value: config.log_level.clone(),
            });
        }

        // Sampling ratio
        if !(0.0..=1.0).contains(&config.sampling_ratio) {
            result.add_error(ConfigValidationError::InvalidSamplingRatio {
                value: config.sampling_ratio,
            });
        }

        // OTLP endpoint URL validation (if specified)
        if let Some(ref endpoint) = config.otlp_endpoint {
            if Url::parse(endpoint).is_err() {
                result.add_error(ConfigValidationError::InvalidUrl {
                    url_type: "otlp_endpoint".to_string(),
                    message: format!("Invalid URL format: {}", endpoint),
                });
            }
        }
    }

    /// Validates deployment configuration.
    fn validate_deployment(config: &crate::DeploymentConfig, result: &mut ValidationResult) {
        if !config.mode.is_layered() {
            return;
        }

        match config.layer {
            crate::DeploymentLayer::Controller => {
                if config.service_url.is_none() {
                    result.add_error(ConfigValidationError::MissingServiceUrl);
                } else if let Some(ref url) = config.service_url {
                    if Url::parse(url).is_err() {
                        result.add_error(ConfigValidationError::InvalidUrl {
                            url_type: "service_url".to_string(),
                            message: format!("Invalid URL format: {}", url),
                        });
                    }
                }
            }
            crate::DeploymentLayer::Service => {
                if config.repository_url.is_none() {
                    result.add_error(ConfigValidationError::MissingRepositoryUrl);
                } else if let Some(ref url) = config.repository_url {
                    if Url::parse(url).is_err() {
                        result.add_error(ConfigValidationError::InvalidUrl {
                            url_type: "repository_url".to_string(),
                            message: format!("Invalid URL format: {}", url),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// Validates SSR configuration.
    fn validate_ssr(config: &crate::SsrConfig, result: &mut ValidationResult) {
        if !config.enabled {
            return;
        }

        if config.render_timeout_ms == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "ssr.render_timeout_ms".to_string(),
                value: 0,
            });
        }

        if config.cache_enabled && config.cache_ttl_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "ssr.cache_ttl_secs".to_string(),
                value: 0,
            });
        }
    }

    /// Validates plugin configuration.
    fn validate_plugins(config: &crate::PluginConfig, result: &mut ValidationResult) {
        if !config.enabled {
            return;
        }

        if config.execution_timeout_secs == 0 {
            result.add_error(ConfigValidationError::NonPositiveTimeout {
                name: "plugins.execution_timeout_secs".to_string(),
                value: 0,
            });
        }
    }
}

/// Formats validation errors for display.
pub fn format_validation_errors(errors: &[ConfigValidationError]) -> String {
    let mut output = String::from("Configuration validation failed:\n");
    for (i, error) in errors.iter().enumerate() {
        output.push_str(&format!("  {}. {}\n", i + 1, error));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> AppConfig {
        let mut config = AppConfig::default();
        config.security.jwt_secret = "a".repeat(32); // Valid length
        config
    }

    #[test]
    fn test_valid_config_passes() {
        let config = valid_config();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_jwt_secret_too_short() {
        let mut config = valid_config();
        config.security.jwt_secret = "short".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::JwtSecretTooShort { .. }
        )));
    }

    #[test]
    fn test_invalid_port() {
        let mut config = valid_config();
        config.server.rest_port = 0;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidPort { name, .. } if name == "rest_port"
        )));
    }

    #[test]
    fn test_port_conflict() {
        let mut config = valid_config();
        config.server.rest_port = 8080;
        config.server.grpc_port = 8080;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::PortConflict { .. }
        )));
    }

    #[test]
    fn test_invalid_pool_size() {
        let mut config = valid_config();
        config.database.min_connections = 100;
        config.database.max_connections = 10;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidPoolSize { .. }
        )));
    }

    #[test]
    fn test_pool_size_too_large() {
        let mut config = valid_config();
        config.database.max_connections = 2000;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::PoolSizeTooLarge { .. }
        )));
    }

    #[test]
    fn test_invalid_database_url() {
        let mut config = valid_config();
        config.database.url = "invalid-url".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidUrl { url_type, .. } if url_type == "database"
        )));
    }

    #[test]
    fn test_invalid_redis_url() {
        let mut config = valid_config();
        config.redis.enabled = true;
        config.redis.url = "http://localhost:6379".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidUrl { url_type, .. } if url_type == "redis"
        )));
    }

    #[test]
    fn test_tls_enabled_without_cert() {
        let mut config = valid_config();
        config.security.grpc_tls_enabled = true;
        config.security.tls_cert_path = None;
        config.security.tls_key_path = Some("/path/to/key".to_string());

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::MissingTlsCert
        )));
    }

    #[test]
    fn test_tls_enabled_without_key() {
        let mut config = valid_config();
        config.security.grpc_tls_enabled = true;
        config.security.tls_cert_path = Some("/path/to/cert".to_string());
        config.security.tls_key_path = None;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::MissingTlsKey
        )));
    }

    #[test]
    fn test_invalid_sampling_ratio() {
        let mut config = valid_config();
        config.observability.sampling_ratio = 1.5;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidSamplingRatio { .. }
        )));
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = valid_config();
        config.observability.log_level = "invalid".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidLogLevel { .. }
        )));
    }

    #[test]
    fn test_invalid_hash_cost() {
        let mut config = valid_config();
        config.security.password_hash_cost = 2;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigValidationError::InvalidHashCost { .. }
        )));
    }

    #[test]
    fn test_multiple_errors() {
        let mut config = valid_config();
        config.security.jwt_secret = "short".to_string();
        config.server.rest_port = 0;
        config.database.min_connections = 100;
        config.database.max_connections = 10;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 3);
    }

    #[test]
    fn test_format_validation_errors() {
        let errors = vec![
            ConfigValidationError::JwtSecretTooShort {
                actual: 10,
                minimum: 32,
            },
            ConfigValidationError::InvalidPort {
                name: "rest_port".to_string(),
                value: 0,
            },
        ];

        let output = format_validation_errors(&errors);
        assert!(output.contains("JWT secret too short"));
        assert!(output.contains("Invalid port"));
    }
}
