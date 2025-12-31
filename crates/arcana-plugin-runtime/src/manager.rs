//! Plugin manager for loading and managing WASM plugins.

use arcana_config::PluginConfig;
use arcana_core::{ArcanaError, ArcanaResult, PluginId};
use arcana_plugin_api::{PluginDescriptor, PluginState};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Plugin manager for loading and managing WASM plugins.
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<PluginId, LoadedPlugin>>>,
    config: PluginConfig,
}

/// A loaded plugin instance.
pub struct LoadedPlugin {
    pub descriptor: PluginDescriptor,
    pub state: PluginState,
    // In a full implementation, this would contain the Wasmtime instance
}

impl PluginManager {
    /// Creates a new plugin manager.
    pub fn new(config: PluginConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initializes the plugin manager and loads plugins from the configured directory.
    pub async fn initialize(&self) -> ArcanaResult<()> {
        if !self.config.enabled {
            info!("Plugin system is disabled");
            return Ok(());
        }

        let plugin_dir = Path::new(&self.config.directory);
        if !plugin_dir.exists() {
            info!("Plugin directory does not exist, creating: {}", self.config.directory);
            std::fs::create_dir_all(plugin_dir).map_err(|e| {
                ArcanaError::PluginLoading(format!("Failed to create plugin directory: {}", e))
            })?;
        }

        // Scan for plugins
        self.scan_plugins().await?;

        info!("Plugin manager initialized");
        Ok(())
    }

    /// Scans the plugin directory for plugins.
    async fn scan_plugins(&self) -> ArcanaResult<()> {
        let plugin_dir = Path::new(&self.config.directory);

        if !plugin_dir.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(plugin_dir).map_err(|e| {
            ArcanaError::PluginLoading(format!("Failed to read plugin directory: {}", e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wasm") {
                debug!("Found plugin: {:?}", path);
                // In a full implementation, load the plugin here
            }
        }

        Ok(())
    }

    /// Installs a plugin from WASM bytes.
    pub async fn install_plugin(&self, wasm_bytes: &[u8]) -> ArcanaResult<PluginId> {
        info!("Installing plugin from WASM bytes ({} bytes)", wasm_bytes.len());

        // In a full implementation:
        // 1. Compile the WASM module with Wasmtime
        // 2. Instantiate the module
        // 3. Call the plugin's get_descriptor function
        // 4. Store the plugin instance

        // For now, return a placeholder
        let plugin_id = PluginId::new("placeholder-plugin");

        let loaded = LoadedPlugin {
            descriptor: PluginDescriptor {
                key: plugin_id.as_str().to_string(),
                name: "Placeholder Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "A placeholder plugin".to_string(),
                author: "Arcana".to_string(),
                min_platform_version: "0.1.0".to_string(),
            },
            state: PluginState::Installed,
        };

        self.plugins.write().await.insert(plugin_id.clone(), loaded);

        Ok(plugin_id)
    }

    /// Enables a plugin.
    pub async fn enable_plugin(&self, plugin_id: &PluginId) -> ArcanaResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id).ok_or_else(|| {
            ArcanaError::PluginNotFound(plugin_id.as_str().to_string())
        })?;

        info!("Enabling plugin: {}", plugin_id);
        plugin.state = PluginState::Active;

        Ok(())
    }

    /// Disables a plugin.
    pub async fn disable_plugin(&self, plugin_id: &PluginId) -> ArcanaResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id).ok_or_else(|| {
            ArcanaError::PluginNotFound(plugin_id.as_str().to_string())
        })?;

        info!("Disabling plugin: {}", plugin_id);
        plugin.state = PluginState::Resolved;

        Ok(())
    }

    /// Uninstalls a plugin.
    pub async fn uninstall_plugin(&self, plugin_id: &PluginId) -> ArcanaResult<()> {
        let mut plugins = self.plugins.write().await;

        if plugins.remove(plugin_id).is_none() {
            return Err(ArcanaError::PluginNotFound(plugin_id.as_str().to_string()));
        }

        info!("Uninstalled plugin: {}", plugin_id);
        Ok(())
    }

    /// Lists all installed plugins.
    pub async fn list_plugins(&self) -> Vec<(PluginId, PluginDescriptor, PluginState)> {
        let plugins = self.plugins.read().await;
        plugins
            .iter()
            .map(|(id, p)| (id.clone(), p.descriptor.clone(), p.state))
            .collect()
    }

    /// Gets a plugin by ID.
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Option<(PluginDescriptor, PluginState)> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|p| (p.descriptor.clone(), p.state))
    }
}

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager")
            .field("enabled", &self.config.enabled)
            .field("directory", &self.config.directory)
            .finish()
    }
}
