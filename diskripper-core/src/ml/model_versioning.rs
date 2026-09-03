//! Model Versioning and Management.
//!
//! Tracks model versions, rollbacks, and A/B testing.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;

/// Model version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    pub name: String,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub accuracy: f64,
    pub training_samples: usize,
    pub is_active: bool,
}

/// Model version manager
pub struct ModelVersioning {
    model_dir: std::path::PathBuf,
}

impl ModelVersioning {
    pub fn new(model_dir: &Path) -> Result<Self, DiskRipperError> {
        Ok(Self {
            model_dir: model_dir.to_path_buf(),
        })
    }

    /// Save a new model version
    pub fn save_version(
        &self,
        name: &str,
        version: &str,
        accuracy: f64,
        training_samples: usize,
    ) -> Result<(), DiskRipperError> {
        let model_version = ModelVersion {
            name: name.to_string(),
            version: version.to_string(),
            created_at: chrono::Utc::now(),
            accuracy,
            training_samples,
            is_active: true,
        };

        let path = self.model_dir.join(format!("{}_version_{}.json", name, version));
        let json = serde_json::to_string_pretty(&model_version)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize version: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write version: {}", e)))?;

        info!("Saved model {} version {}", name, version);
        Ok(())
    }

    /// Get the active version of a model
    pub fn get_active_version(&self, name: &str) -> Option<ModelVersion> {
        // Find the latest active version
        let mut versions = self.list_versions(name);
        versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        versions.into_iter().find(|v| v.is_active)
    }

    /// List all versions of a model
    pub fn list_versions(&self, name: &str) -> Vec<ModelVersion> {
        let mut versions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.model_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if filename.starts_with(&format!("{}_version_", name))
                        && path.extension().and_then(|e| e.to_str()) == Some("json")
                    {
                        if let Ok(json) = std::fs::read_to_string(&path) {
                            if let Ok(version) = serde_json::from_str::<ModelVersion>(&json) {
                                versions.push(version);
                            }
                        }
                    }
                }
            }
        }

        versions
    }

    /// Rollback to a previous version
    pub fn rollback(&self, name: &str, version: &str) -> Result<(), DiskRipperError> {
        let versions = self.list_versions(name);
        for mut v in versions {
            v.is_active = v.version == version;
            let path = self.model_dir.join(format!("{}_version_{}.json", name, v.version));
            let json = serde_json::to_string_pretty(&v)
                .map_err(|e| DiskRipperError::Io(format!("Failed to serialize: {}", e)))?;
            std::fs::write(&path, json)
                .map_err(|e| DiskRipperError::Io(format!("Failed to write: {}", e)))?;
        }

        info!("Rolled back {} to version {}", name, version);
        Ok(())
    }
}
