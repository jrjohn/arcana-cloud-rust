//! Application builder.

use arcana_config::AppConfig;
use arcana_core::ArcanaResult;

/// Application builder for constructing the server.
pub struct AppBuilder {
    config: Option<AppConfig>,
}

impl AppBuilder {
    /// Creates a new application builder.
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Sets the configuration.
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Builds and runs the application.
    pub async fn run(self) -> ArcanaResult<()> {
        let _config = self.config.unwrap_or_default();
        // Application startup logic would go here
        Ok(())
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_builder_new() {
        let builder = AppBuilder::new();
        assert!(builder.config.is_none());
    }

    #[test]
    fn test_app_builder_default() {
        let builder = AppBuilder::default();
        assert!(builder.config.is_none());
    }

    #[test]
    fn test_app_builder_with_config() {
        let config = AppConfig::default();
        let builder = AppBuilder::new().with_config(config);
        assert!(builder.config.is_some());
    }

    #[tokio::test]
    async fn test_app_builder_run_with_default_config() {
        let builder = AppBuilder::new();
        let result = builder.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_app_builder_run_with_custom_config() {
        let config = AppConfig::default();
        let builder = AppBuilder::new().with_config(config);
        let result = builder.run().await;
        assert!(result.is_ok());
    }
}
