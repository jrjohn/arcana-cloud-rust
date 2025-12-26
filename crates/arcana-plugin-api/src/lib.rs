//! # Arcana Plugin API
//!
//! Plugin API for Arcana Cloud Rust.
//! Defines the interface for WASM plugins to interact with the platform.
//!
//! This crate can be compiled for both host (native) and WASM targets.

pub mod extensions;

use serde::{Deserialize, Serialize};

/// Plugin descriptor containing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    /// Unique plugin key.
    pub key: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version (semver).
    pub version: String,
    /// Plugin description.
    pub description: String,
    /// Plugin author.
    pub author: String,
    /// Minimum platform version required.
    pub min_platform_version: String,
}

/// Plugin lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Plugin is installed but not resolved.
    Installed,
    /// Plugin dependencies are resolved.
    Resolved,
    /// Plugin is starting.
    Starting,
    /// Plugin is active and running.
    Active,
    /// Plugin is stopping.
    Stopping,
    /// Plugin is uninstalled.
    Uninstalled,
}

/// Plugin trait that all plugins must implement.
pub trait Plugin: Send + Sync {
    /// Returns the plugin descriptor.
    fn descriptor(&self) -> &PluginDescriptor;

    /// Called when the plugin is installed.
    fn on_install(&mut self) -> Result<(), String>;

    /// Called when the plugin is enabled.
    fn on_enable(&mut self) -> Result<(), String>;

    /// Called when the plugin is disabled.
    fn on_disable(&mut self) -> Result<(), String>;

    /// Called when the plugin is uninstalled.
    fn on_uninstall(&mut self) -> Result<(), String>;
}
