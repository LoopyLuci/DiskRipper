use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{info, warn};

use crate::error::DiskRipperError;

/// Application settings that persist across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default output directory for ripped files
    pub default_output_dir: PathBuf,
    /// Default read speed (None = maximum)
    pub read_speed: Option<u32>,
    /// Verify checksums after rip
    pub verify_checksums: bool,
    /// Eject disc after completion
    pub eject_after_rip: bool,
    /// Number of retries on read error
    pub read_retries: u32,
    /// Buffer size in MB for read operations
    pub buffer_size_mb: usize,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Enable audio CD extraction
    pub enable_audio_cd: bool,
    /// Audio CD jitter correction
    pub jitter_correction: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            default_output_dir: home.join("DiskRipper"),
            read_speed: None,
            verify_checksums: true,
            eject_after_rip: false,
            read_retries: 3,
            buffer_size_mb: 4,
            log_level: "info".to_string(),
            enable_audio_cd: true,
            jitter_correction: true,
        }
    }
}

/// Manages loading and saving settings to disk
pub struct SettingsManager {
    settings_path: PathBuf,
    settings: Mutex<Settings>,
}

impl SettingsManager {
    pub fn new(app_dir: PathBuf) -> Result<Self, DiskRipperError> {
        let settings_path = app_dir.join("settings.json");
        
        // Load existing settings or create defaults
        let settings = if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(json) => {
                    info!("Loaded settings from {:?}", settings_path);
                    serde_json::from_str(&json).unwrap_or_default()
                }
                Err(e) => {
                    warn!("Failed to read settings: {}, using defaults", e);
                    Settings::default()
                }
            }
        } else {
            info!("No settings file found, using defaults");
            Settings::default()
        };

        Ok(Self {
            settings_path,
            settings: Mutex::new(settings),
        })
    }

    /// Get a copy of current settings
    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Update settings and save to disk
    pub fn update(&self, new_settings: Settings) -> Result<(), DiskRipperError> {
        let json = serde_json::to_string_pretty(&new_settings)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize settings: {}", e)))?;
        
        std::fs::write(&self.settings_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write settings: {}", e)))?;
        
        *self.settings.lock().unwrap() = new_settings;
        info!("Settings saved to {:?}", self.settings_path);
        
        Ok(())
    }

    /// Update a single setting field
    pub fn update_field<F>(&self, updater: F) -> Result<(), DiskRipperError>
    where
        F: FnOnce(&mut Settings),
    {
        let mut settings = self.get();
        updater(&mut settings);
        self.update(settings)
    }

    /// Reset to default settings
    pub fn reset_to_defaults(&self) -> Result<(), DiskRipperError> {
        self.update(Settings::default())
    }

    /// Get the settings file path
    pub fn path(&self) -> &PathBuf {
        &self.settings_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.verify_checksums);
        assert!(!settings.eject_after_rip);
        assert_eq!(settings.read_retries, 3);
    }

    #[test]
    fn test_settings_serialize() {
        let settings = Settings::default();
        let json = serde_json::to_string_pretty(&settings);
        assert!(json.is_ok());
    }

    #[test]
    fn test_settings_deserialize() {
        let json = r#"{
            "default_output_dir": "/tmp/test",
            "read_speed": null,
            "verify_checksums": true,
            "eject_after_rip": false,
            "read_retries": 3,
            "buffer_size_mb": 4,
            "log_level": "info",
            "enable_audio_cd": true,
            "jitter_correction": true
        }"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.read_retries, 3);
    }
}
