//! Configuration loader with layered sources.

use crate::AppConfig;
use arcana_core::ArcanaError;
use config::{Config, ConfigError, Environment, File};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration loader with runtime refresh support.
#[derive(Clone)]
pub struct ConfigLoader {
    config: Arc<RwLock<AppConfig>>,
    config_dir: String,
}

impl ConfigLoader {
    /// Creates a new configuration loader.
    ///
    /// Configuration is loaded from multiple sources in order:
    /// 1. `config/default.toml` - Default values
    /// 2. `config/{environment}.toml` - Environment-specific overrides
    /// 3. `config/{deployment_mode}.toml` - Deployment mode overrides
    /// 4. Environment variables with `ARCANA_` prefix
    pub fn new(config_dir: impl Into<String>) -> Result<Self, ArcanaError> {
        let config_dir = config_dir.into();
        let config = Self::load_config(&config_dir)?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_dir,
        })
    }

    /// Loads configuration from the default location (`./config`).
    pub fn from_default_location() -> Result<Self, ArcanaError> {
        Self::new("./config")
    }

    /// Returns the current configuration.
    pub async fn get(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// Reloads the configuration from disk.
    pub async fn reload(&self) -> Result<(), ArcanaError> {
        let new_config = Self::load_config(&self.config_dir)?;
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Loads configuration from the specified directory.
    fn load_config(config_dir: &str) -> Result<AppConfig, ArcanaError> {
        // Load .env file if present
        if let Err(e) = dotenvy::dotenv() {
            debug!("No .env file found or error loading it: {}", e);
        }

        let environment = std::env::var("ARCANA_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let deployment_mode = std::env::var("ARCANA_DEPLOYMENT_MODE").unwrap_or_else(|_| "monolithic".to_string());

        info!(
            "Loading configuration for environment: {}, deployment: {}",
            environment, deployment_mode
        );

        let mut builder = Config::builder();

        // 1. Load default configuration
        let default_path = format!("{}/default.toml", config_dir);
        if Path::new(&default_path).exists() {
            debug!("Loading default config from: {}", default_path);
            builder = builder.add_source(File::with_name(&default_path).required(false));
        }

        // 2. Load environment-specific configuration
        let env_path = format!("{}/{}.toml", config_dir, environment);
        if Path::new(&env_path).exists() {
            debug!("Loading environment config from: {}", env_path);
            builder = builder.add_source(File::with_name(&env_path).required(false));
        }

        // 3. Load deployment mode configuration
        let mode_path = format!("{}/{}.toml", config_dir, deployment_mode);
        if Path::new(&mode_path).exists() {
            debug!("Loading deployment mode config from: {}", mode_path);
            builder = builder.add_source(File::with_name(&mode_path).required(false));
        }

        // 4. Load local overrides (not committed to version control)
        let local_path = format!("{}/local.toml", config_dir);
        if Path::new(&local_path).exists() {
            debug!("Loading local config from: {}", local_path);
            builder = builder.add_source(File::with_name(&local_path).required(false));
        }

        // 5. Override with environment variables (ARCANA_ prefix)
        builder = builder.add_source(
            Environment::with_prefix("ARCANA")
                .separator("__")
                .try_parsing(true),
        );

        let config = builder
            .build()
            .map_err(|e| config_error_to_arcana_error(e))?;

        let app_config: AppConfig = config
            .try_deserialize()
            .map_err(|e| config_error_to_arcana_error(e))?;

        // Validate critical configuration
        Self::validate_config(&app_config)?;

        Ok(app_config)
    }

    /// Validates the configuration.
    fn validate_config(config: &AppConfig) -> Result<(), ArcanaError> {
        // Warn about default JWT secret in production
        if config.app.environment == "production" && config.security.jwt_secret == "change-me-in-production" {
            warn!("Using default JWT secret in production! This is a security risk.");
        }

        // Validate database URL
        if config.database.url.is_empty() {
            return Err(ArcanaError::Configuration("Database URL is required".to_string()));
        }

        // Validate layered deployment configuration
        if config.deployment.mode.is_layered() {
            match config.deployment.layer {
                crate::DeploymentLayer::Controller => {
                    if config.deployment.service_url.is_none() {
                        return Err(ArcanaError::Configuration(
                            "Service URL is required for controller layer in layered deployment".to_string(),
                        ));
                    }
                }
                crate::DeploymentLayer::Service => {
                    if config.deployment.repository_url.is_none() {
                        return Err(ArcanaError::Configuration(
                            "Repository URL is required for service layer in layered deployment".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Gets a specific configuration value by key path.
    pub async fn get_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let config = self.config.read().await;
        let json = serde_json::to_value(&*config).ok()?;

        let mut current = &json;
        for part in key.split('.') {
            current = current.get(part)?;
        }

        serde_json::from_value(current.clone()).ok()
    }
}

fn config_error_to_arcana_error(err: ConfigError) -> ArcanaError {
    ArcanaError::Configuration(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerConfig;

    #[tokio::test]
    async fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.rest_port, 8080);
        assert_eq!(config.server.grpc_port, 9090);
        assert!(config.plugins.enabled);
    }

    #[tokio::test]
    async fn test_server_addresses() {
        let config = ServerConfig::default();
        assert_eq!(config.rest_addr(), "0.0.0.0:8080");
        assert_eq!(config.grpc_addr(), "0.0.0.0:9090");
    }
}
