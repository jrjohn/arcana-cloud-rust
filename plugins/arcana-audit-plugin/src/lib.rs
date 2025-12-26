//! # Arcana Audit Plugin
//!
//! Sample audit plugin demonstrating the plugin API.
//! This plugin logs all platform events for auditing purposes.

use arcana_plugin_api::{
    extensions::{EventListenerExtension, EventSubscription, PluginEvent},
    Plugin, PluginDescriptor,
};

/// Audit plugin implementation.
pub struct AuditPlugin {
    descriptor: PluginDescriptor,
    enabled: bool,
}

impl AuditPlugin {
    /// Creates a new audit plugin.
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                key: "arcana-audit-plugin".to_string(),
                name: "Audit Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "Logs all platform events for auditing purposes".to_string(),
                author: "Arcana Team".to_string(),
                min_platform_version: "0.1.0".to_string(),
            },
            enabled: false,
        }
    }
}

impl Default for AuditPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for AuditPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn on_install(&mut self) -> Result<(), String> {
        println!("Audit plugin installed");
        Ok(())
    }

    fn on_enable(&mut self) -> Result<(), String> {
        self.enabled = true;
        println!("Audit plugin enabled");
        Ok(())
    }

    fn on_disable(&mut self) -> Result<(), String> {
        self.enabled = false;
        println!("Audit plugin disabled");
        Ok(())
    }

    fn on_uninstall(&mut self) -> Result<(), String> {
        println!("Audit plugin uninstalled");
        Ok(())
    }
}

impl EventListenerExtension for AuditPlugin {
    fn subscriptions(&self) -> EventSubscription {
        EventSubscription {
            event_types: vec!["*".to_string()], // Subscribe to all events
            order: 1000,                        // Low priority (run last)
            async_handling: true,
        }
    }

    fn handle_event(&self, event: PluginEvent) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        // Log the event
        println!(
            "[AUDIT] Event: {} at {} from {:?}",
            event.event_type,
            event.timestamp,
            event.source_plugin
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_lifecycle() {
        let mut plugin = AuditPlugin::new();

        assert_eq!(plugin.descriptor().key, "arcana-audit-plugin");

        plugin.on_install().unwrap();
        plugin.on_enable().unwrap();
        assert!(plugin.enabled);

        plugin.on_disable().unwrap();
        assert!(!plugin.enabled);

        plugin.on_uninstall().unwrap();
    }
}
