//! Configuration loader with layered sources.

use crate::validation::{format_validation_errors, ConfigValidator};
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
    /// The merged, untyped source tree behind `config`.
    ///
    /// Kept so crates that own their own configuration section (for example
    /// `arcana-jobs`, which cannot be a dependency of this crate) deserialize
    /// it through the *same* layered rules instead of re-implementing them.
    raw: Arc<RwLock<Config>>,
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
        let (config, raw) = Self::load_config(&config_dir)?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            raw: Arc::new(RwLock::new(raw)),
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

    /// Deserializes one top-level configuration section by key.
    ///
    /// Returns `Ok(None)` when the section is absent, so a caller can fall back
    /// to its own defaults. Any other deserialization failure is an error --
    /// a malformed section must never be silently replaced by defaults.
    pub async fn section<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, ArcanaError> {
        let raw = self.raw.read().await;
        match raw.get::<T>(key) {
            Ok(value) => Ok(Some(value)),
            Err(ConfigError::NotFound(_)) => Ok(None),
            Err(e) => Err(config_error_to_arcana_error(e)),
        }
    }

    /// Reloads the configuration from disk.
    pub async fn reload(&self) -> Result<(), ArcanaError> {
        let (new_config, new_raw) = Self::load_config(&self.config_dir)?;
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }
        {
            let mut raw = self.raw.write().await;
            *raw = new_raw;
        }
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Loads configuration from the specified directory.
    fn load_config(config_dir: &str) -> Result<(AppConfig, Config), ArcanaError> {
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
        //
        // `with_list_parse_key` is opt-in per key: comma splitting is applied
        // only where a list is expected, so a value that legitimately contains
        // a comma elsewhere is left intact.
        builder = builder.add_source(
            Environment::with_prefix("ARCANA")
                // Without this, `config` falls back to using `separator` as the
                // prefix separator too, so it would look for `ARCANA__FOO__BAR`
                // and ignore every `ARCANA_FOO__BAR` variable the deployment
                // manifests, Dockerfiles and Compose files actually set.
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true)
                .list_separator(",")
                .with_list_parse_key("jobs.worker.queues"),
        );

        let config = builder
            .build()
            .map_err(config_error_to_arcana_error)?;

        let app_config: AppConfig = config
            .clone()
            .try_deserialize()
            .map_err(config_error_to_arcana_error)?;

        // Validate critical configuration
        Self::validate_config(&app_config)?;

        Ok((app_config, config))
    }

    /// Validates the configuration using comprehensive validation rules.
    fn validate_config(config: &AppConfig) -> Result<(), ArcanaError> {
        // Run comprehensive validation
        if let Err(errors) = ConfigValidator::validate(config) {
            return Err(ArcanaError::Configuration(format_validation_errors(&errors)));
        }

        // Production-specific warnings (non-fatal)
        if config.app.environment == "production" {
            if config.security.jwt_secret.starts_with("change-me") {
                warn!("Using default JWT secret in production! This is a security risk.");
            }
            if !config.security.grpc_tls_enabled {
                warn!("gRPC TLS is disabled in production. Consider enabling it for security.");
            }
            if config.observability.sampling_ratio < 0.01 {
                warn!("Very low trace sampling ratio in production. Consider increasing for better observability.");
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
    use crate::{AppConfig, DatabaseConfig, PluginConfig, SecurityConfig, ServerConfig};

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

    #[test]
    fn test_server_config_custom_ports() {
        let mut config = ServerConfig::default();
        config.rest_port = 3000;
        config.grpc_port = 4000;
        assert_eq!(config.rest_addr(), "0.0.0.0:3000");
        assert_eq!(config.grpc_addr(), "0.0.0.0:4000");
    }

    #[test]
    fn test_server_config_custom_host() {
        let config = ServerConfig {
            rest_host: "127.0.0.1".to_string(),
            grpc_host: "127.0.0.1".to_string(),
            rest_port: 8080,
            grpc_port: 9090,
            ..Default::default()
        };
        assert_eq!(config.rest_addr(), "127.0.0.1:8080");
        assert_eq!(config.grpc_addr(), "127.0.0.1:9090");
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert!(config.max_connections > 0);
        assert!(config.min_connections <= config.max_connections);
    }

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(!config.jwt_secret.is_empty());
        assert!(!config.jwt_issuer.is_empty());
        assert!(!config.jwt_audience.is_empty());
        assert!(config.jwt_access_expiration_secs > 0);
        assert!(config.jwt_refresh_expiration_secs > 0);
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_app_config_default_environment() {
        let config = AppConfig::default();
        assert_eq!(config.app.environment, "development");
    }

    #[test]
    fn test_app_config_default_name() {
        let config = AppConfig::default();
        assert!(!config.app.name.is_empty());
    }

    // =========================================================================
    // Environment override contract
    //
    // Every deployment manifest, Dockerfile and Compose file in this repo
    // configures the binary through `ARCANA_<SECTION>__<FIELD>`. These tests
    // pin that spelling: `config` silently ignores an unmatched variable, so a
    // regression here does not fail loudly -- it just makes every override
    // vanish and the process run on file defaults.
    // =========================================================================

    /// Serialises the tests below: environment variables are process-global.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Copies the shipped `config/default.toml` into a scratch directory.
    ///
    /// Using the real file means these tests also fail if the defaults stop
    /// parsing, and `local.toml` supplies only the one value that would
    /// otherwise trip validation.
    fn scratch_config_dir(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "arcana-config-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let shipped = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/default.toml");
        std::fs::copy(&shipped, dir.join("default.toml"))
            .unwrap_or_else(|e| panic!("copying {}: {e}", shipped.display()));

        std::fs::write(
            dir.join("local.toml"),
            "[security]\njwt_secret = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        )
        .unwrap();

        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn test_single_underscore_after_prefix_overrides_config() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_config_dir("single");

        std::env::set_var("ARCANA_DEPLOYMENT__LAYER", "worker");
        std::env::set_var("ARCANA_APP__ENVIRONMENT", "staging");

        let loader = ConfigLoader::new(&dir).expect("config should load");
        let config = futures_lite_block_on(loader.get());

        std::env::remove_var("ARCANA_DEPLOYMENT__LAYER");
        std::env::remove_var("ARCANA_APP__ENVIRONMENT");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(config.deployment.layer, crate::DeploymentLayer::Worker);
        assert_eq!(config.app.environment, "staging");
    }

    #[test]
    fn test_double_underscore_after_prefix_is_not_an_override() {
        // The opposite spelling must stay inert -- otherwise both forms
        // "work" and neither manifest convention is authoritative.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_config_dir("double");

        std::env::set_var("ARCANA__DEPLOYMENT__LAYER", "worker");

        let loader = ConfigLoader::new(&dir).expect("config should load");
        let config = futures_lite_block_on(loader.get());

        std::env::remove_var("ARCANA__DEPLOYMENT__LAYER");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(config.deployment.layer, crate::DeploymentLayer::All);
    }

    #[test]
    fn test_section_reads_a_foreign_config_block() {
        // `JobsConfig` lives in a crate that depends on this one, so the
        // section is deserialized by the caller through the same loader.
        #[derive(serde::Deserialize)]
        struct WorkerSection {
            queues: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct JobsSection {
            worker: WorkerSection,
        }

        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_config_dir("section");

        std::env::set_var("ARCANA_JOBS__WORKER__QUEUES", "critical,high");

        let loader = ConfigLoader::new(&dir).expect("config should load");
        let jobs = futures_lite_block_on(loader.section::<JobsSection>("jobs"))
            .expect("section should deserialize")
            .expect("jobs section should be present");

        std::env::remove_var("ARCANA_JOBS__WORKER__QUEUES");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(jobs.worker.queues, vec!["critical", "high"]);
    }

    #[test]
    fn test_section_absent_returns_none() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_config_dir("absent");

        let loader = ConfigLoader::new(&dir).expect("config should load");
        let missing =
            futures_lite_block_on(loader.section::<std::collections::HashMap<String, String>>(
                "not_a_real_section",
            ))
            .expect("absent section is not an error");

        let _ = std::fs::remove_dir_all(&dir);
        assert!(missing.is_none());
    }

    /// Minimal blocking executor so these tests need no async runtime.
    fn futures_lite_block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn test_config_loader_invalid_dir() {
        let result = ConfigLoader::new("/nonexistent/path/config");
        // Should either succeed (file not required) or fail with a meaningful error
        // The important thing is it doesn't panic
        let _ = result;
    }
}
