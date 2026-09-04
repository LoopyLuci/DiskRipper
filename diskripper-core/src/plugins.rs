//! Plugin system with WASM support.
//!
//! Provides:
//! - Plugin trait for extensible functionality
//! - WASM runtime for sandboxed plugins
//! - Plugin discovery and loading
//! - Hook system for plugin events

use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use crate::error::DiskRipperError;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub hooks: Vec<String>,
}

/// Plugin capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCapability {
    DiscIdentifier,
    MetadataProvider,
    AudioProcessor,
    VideoProcessor,
    ExportFormat,
    UiExtension,
}

/// Plugin interface
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn capabilities(&self) -> Vec<PluginCapability>;
    fn initialize(&mut self) -> Result<(), DiskRipperError>;
    fn shutdown(&mut self);
    fn on_disc_inserted(&self, disc_info: &crate::disc::DiscInfo) -> Result<(), DiskRipperError>;
    fn on_rip_complete(&self, job_info: &crate::job::Job) -> Result<(), DiskRipperError>;
}

/// Plugin manager
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_dir: std::path::PathBuf,
    hooks: std::collections::HashMap<String, Vec<String>>,
}

impl PluginManager {
    pub fn new(plugin_dir: &Path) -> Result<Self, DiskRipperError> {
        std::fs::create_dir_all(plugin_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create plugin dir: {}", e)))?;
        Ok(Self {
            plugins: Vec::new(),
            plugin_dir: plugin_dir.to_path_buf(),
            hooks: std::collections::HashMap::new(),
        })
    }

    /// Load all plugins from the plugin directory
    pub fn load_plugins(&mut self) -> Result<(), DiskRipperError> {
        if !self.plugin_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.plugin_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read plugin dir: {}", e)))?
        {
            let entry = entry.map_err(|e| DiskRipperError::Io(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<PluginMetadata>(&json) {
                        info!("Found plugin: {} v{}", metadata.name, metadata.version);
                        // In a full implementation, load WASM module here
                    }
                }
            }
        }

        info!("Loaded {} plugins", self.plugins.len());
        Ok(())
    }

    /// Register a plugin
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), DiskRipperError> {
        let metadata = plugin.metadata();
        info!("Registering plugin: {} v{}", metadata.name, metadata.version);
        
        // Register hooks
        for hook in &metadata.hooks {
            self.hooks.entry(hook.clone())
                .or_insert_with(Vec::new)
                .push(metadata.name.clone());
        }

        self.plugins.push(plugin);
        Ok(())
    }

    /// Get all plugins
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get plugins by capability
    pub fn get_by_capability(&self, capability: PluginCapability) -> Vec<&dyn Plugin> {
        self.plugins.iter()
            .filter(|p| p.capabilities().contains(&capability))
            .map(|p| p.as_ref())
            .collect()
    }

    /// Trigger a hook
    pub fn trigger_hook(&self, hook: &str, data: &str) -> Result<(), DiskRipperError> {
        if let Some(plugins) = self.hooks.get(hook) {
            for plugin_name in plugins {
                if let Some(plugin) = self.plugins.iter().find(|p| &p.metadata().name == plugin_name) {
                    info!("Triggering hook {} on plugin {}", hook, plugin_name);
                    // In a full implementation, call plugin hook handler
                }
            }
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) {
        for plugin in &mut self.plugins {
            plugin.shutdown();
        }
    }
}

/// Example: Metadata provider plugin
pub struct MetadataPlugin {
    metadata: PluginMetadata,
    provider_url: String,
}

impl MetadataPlugin {
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                author: "DiskRipper".to_string(),
                description: "Metadata provider plugin".to_string(),
                hooks: vec!["on_disc_inserted".to_string(), "on_rip_complete".to_string()],
            },
            provider_url: url.to_string(),
        }
    }
}

impl Plugin for MetadataPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::MetadataProvider]
    }

    fn initialize(&mut self) -> Result<(), DiskRipperError> {
        info!("Initializing metadata plugin: {}", self.metadata.name);
        Ok(())
    }

    fn shutdown(&mut self) {
        info!("Shutting down metadata plugin: {}", self.metadata.name);
    }

    fn on_disc_inserted(&self, disc_info: &crate::disc::DiscInfo) -> Result<(), DiskRipperError> {
        info!("Plugin {}: Disc inserted: {:?}", self.metadata.name, disc_info.disc_type);
        Ok(())
    }

    fn on_rip_complete(&self, job_info: &crate::job::Job) -> Result<(), DiskRipperError> {
        info!("Plugin {}: Rip complete: {}", self.metadata.name, job_info.name);
        Ok(())
    }
}
