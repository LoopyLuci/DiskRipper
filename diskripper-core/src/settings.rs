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
    /// Maximum concurrent jobs
    pub max_concurrent_jobs: usize,
    /// Enable parallel processing
    pub enable_parallel: bool,
    /// Theme (dark, light, system)
    pub theme: String,
    /// Language code (en, es, fr, de, ja, zh)
    pub language: String,
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
            max_concurrent_jobs: 2,
            enable_parallel: true,
            theme: "dark".to_string(),
            language: "en".to_string(),
        }
    }
}

impl Settings {
    /// Validate and clamp settings to valid ranges
    pub fn validate(&mut self) {
        // Clamp read_retries to 0-10
        self.read_retries = self.read_retries.min(10);
        
        // Clamp buffer_size_mb to 1-64
        self.buffer_size_mb = self.buffer_size_mb.clamp(1, 64);
        
        // Clamp max_concurrent_jobs to 1-8
        self.max_concurrent_jobs = self.max_concurrent_jobs.clamp(1, 8);
        
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            self.log_level = "info".to_string();
        }
        
        // Validate theme
        let valid_themes = ["dark", "light", "system"];
        if !valid_themes.contains(&self.theme.as_str()) {
            self.theme = "dark".to_string();
        }
        
        // Validate language
        let valid_langs = ["en", "es", "fr", "de", "ja", "zh"];
        if !valid_langs.contains(&self.language.as_str()) {
            self.language = "en".to_string();
        }
        
        // Validate read_speed (must be None or 1-48)
        if let Some(speed) = self.read_speed {
            self.read_speed = Some(speed.clamp(1, 48));
        }
    }
    
    /// Merge with defaults for any missing/invalid fields
    pub fn merge_defaults(&mut self) {
        let defaults = Settings::default();
        
        // If default_output_dir is empty, use default
        if self.default_output_dir.as_os_str().is_empty() {
            self.default_output_dir = defaults.default_output_dir;
        }
        
        // Apply validation
        self.validate();
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
        let mut settings = if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(json) => {
                    info!("Loaded settings from {:?}", settings_path);
                    let mut s: Settings = serde_json::from_str(&json).unwrap_or_default();
                    s.merge_defaults();
                    s
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
        
        // Ensure output directory exists
        if !settings.default_output_dir.exists() {
            let _ = std::fs::create_dir_all(&settings.default_output_dir);
        }
        
        Ok(Self {
            settings_path,
            settings: Mutex::new(settings),
        })
    }
    
    /// Get a copy of current settings
    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
    
    /// Update settings and save to disk atomically
    pub fn update(&self, mut new_settings: Settings) -> Result<(), DiskRipperError> {
        // Validate before saving
        new_settings.merge_defaults();
        
        // Serialize
        let json = serde_json::to_string_pretty(&new_settings)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize settings: {}", e)))?;
        
        // Write atomically: write to temp file, then rename
        let temp_path = self.settings_path.with_extension("tmp");
        std::fs::write(&temp_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write settings: {}", e)))?;
        
        // Atomic rename
        std::fs::rename(&temp_path, &self.settings_path)
            .map_err(|e| DiskRipperError::Io(format!("Failed to save settings: {}", e)))?;
        
        *self.settings.lock().unwrap() = new_settings;
        info!("Settings saved to {:?}", self.settings_path);
        
        Ok(())
    }
    
    /// Update a single setting field
    pub fn update_field<F>(&self, f: F) -> Result<(), DiskRipperError>
    where
        F: FnOnce(&mut Settings),
    {
        let mut settings = self.get();
        f(&mut settings);
        self.update(settings)
    }
    
    /// Reset settings to defaults
    pub fn reset(&self) -> Result<(), DiskRipperError> {
        let defaults = Settings::default();
        self.update(defaults)
    }
    
    /// Get the settings file path
    pub fn path(&self) -> &std::path::Path {
        &self.settings_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.read_retries, 3);
        assert_eq!(settings.buffer_size_mb, 4);
        assert!(settings.verify_checksums);
    }
    
    #[test]
    fn test_settings_validation() {
        let mut settings = Settings::default();
        settings.read_retries = 100; // Should be clamped to 10
        settings.buffer_size_mb = 1000; // Should be clamped to 64
        settings.validate();
        assert_eq!(settings.read_retries, 10);
        assert_eq!(settings.buffer_size_mb, 64);
    }
    
    #[test]
    fn test_settings_merge_defaults() {
        let mut settings = Settings::default();
        settings.default_output_dir = PathBuf::new();
        settings.merge_defaults();
        assert!(!settings.default_output_dir.as_os_str().is_empty());
    }
}
