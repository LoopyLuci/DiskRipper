//! Crash reporting and error tracking.

use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::error::DiskRipperError;

/// Crash reporter
pub struct CrashReporter {
    crash_dir: std::path::PathBuf,
    enabled: bool,
}

/// Crash report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error_message: String,
    pub component: String,
    pub os_name: String,
    pub architecture: String,
    pub num_cpus: usize,
    pub app_version: String,
}

/// Error statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total_crashes: u64,
    pub last_crash: Option<chrono::DateTime<chrono::Utc>>,
    pub most_common_error: Option<String>,
    pub errors_by_component: std::collections::HashMap<String, u64>,
}

impl CrashReporter {
    pub fn new(crash_dir: &Path) -> Result<Self, DiskRipperError> {
        std::fs::create_dir_all(crash_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create crash dir: {}", e)))?;
        Ok(Self { crash_dir: crash_dir.to_path_buf(), enabled: true })
    }

    pub fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    pub fn is_enabled(&self) -> bool { self.enabled }

    pub fn capture_crash(&self, error: &DiskRipperError, component: &str) -> Result<CrashReport, DiskRipperError> {
        if !self.enabled {
            return Err(DiskRipperError::Io("Crash reporting disabled".to_string()));
        }

        let report = CrashReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            error_message: error.to_string(),
            component: component.to_string(),
            os_name: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            num_cpus: num_cpus::get(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let path = self.crash_dir.join(format!("crash_{}.json", report.id));
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write: {}", e)))?;

        info!("Crash report saved: {}", report.id);
        Ok(report)
    }

    pub fn get_stats(&self) -> Result<ErrorStats, DiskRipperError> {
        let stats_path = self.crash_dir.join("error_stats.json");
        if stats_path.exists() {
            let json = std::fs::read_to_string(&stats_path)
                .map_err(|e| DiskRipperError::Io(format!("Failed to read stats: {}", e)))?;
            serde_json::from_str(&json)
                .map_err(|e| DiskRipperError::Io(format!("Failed to parse stats: {}", e)))
        } else {
            Ok(ErrorStats { total_crashes: 0, last_crash: None, most_common_error: None, errors_by_component: std::collections::HashMap::new() })
        }
    }

    pub fn get_reports(&self) -> Result<Vec<CrashReport>, DiskRipperError> {
        let mut reports = Vec::new();
        for entry in std::fs::read_dir(&self.crash_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read dir: {}", e)))?
        {
            if let Ok(entry) = entry {
                if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("crash_") && name.ends_with(".json") {
                        if let Ok(json) = std::fs::read_to_string(entry.path()) {
                            if let Ok(report) = serde_json::from_str::<CrashReport>(&json) {
                                reports.push(report);
                            }
                        }
                    }
                }
            }
        }
        reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(reports)
    }

    pub fn clear_reports(&self) -> Result<(), DiskRipperError> {
        for entry in std::fs::read_dir(&self.crash_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read dir: {}", e)))?
        {
            if let Ok(entry) = entry {
                if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("crash_") {
                        std::fs::remove_file(entry.path())
                            .map_err(|e| DiskRipperError::Io(format!("Failed to remove: {}", e)))?;
                    }
                }
            }
        }
        Ok(())
    }
}
