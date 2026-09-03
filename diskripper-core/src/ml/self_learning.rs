//! Self-Learning Pipeline.
//!
//! Enables models to improve from user feedback:
//! - Collects corrections from users
//! - Retrains models periodically
//! - Tracks model accuracy over time
//! - A/B testing for model improvements

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::DiskRipperError;

/// Self-learning manager
pub struct SelfLearning {
    model_dir: std::path::PathBuf,
    feedback_dir: std::path::PathBuf,
}

/// Feedback entry for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub id: u64,
    pub original_prediction: String,
    pub corrected_title: String,
    pub corrected_artist: Option<String>,
    pub corrected_album: Option<String>,
    pub corrected_genre: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this feedback has been used for training
    pub used_for_training: bool,
}

/// Training batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    pub id: u64,
    pub entries: Vec<FeedbackEntry>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub model_version: String,
}

/// Model accuracy metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    pub model_name: String,
    pub total_predictions: u64,
    pub correct_predictions: u64,
    pub accuracy: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl SelfLearning {
    pub fn new(model_dir: &Path, feedback_dir: &Path) -> Result<Self, DiskRipperError> {
        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            feedback_dir: feedback_dir.to_path_buf(),
        })
    }

    /// Load all feedback entries
    pub fn load_feedback(&self) -> Result<Vec<FeedbackEntry>, DiskRipperError> {
        let mut entries = Vec::new();

        if !self.feedback_dir.exists() {
            return Ok(entries);
        }

        for entry in std::fs::read_dir(&self.feedback_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read feedback dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| DiskRipperError::Io(format!("Failed to read entry: {}", e)))?;

            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let json = std::fs::read_to_string(entry.path())
                    .map_err(|e| DiskRipperError::Io(format!("Failed to read feedback: {}", e)))?;

                if let Ok(feedback) = serde_json::from_str::<FeedbackEntry>(&json) {
                    entries.push(feedback);
                }
            }
        }

        Ok(entries)
    }

    /// Load unused feedback for training
    pub fn load_unused_feedback(&self) -> Result<Vec<FeedbackEntry>, DiskRipperError> {
        let all_feedback = self.load_feedback()?;
        Ok(all_feedback
            .into_iter()
            .filter(|f| !f.used_for_training)
            .collect())
    }

    /// Mark feedback as used for training
    pub fn mark_used(&self, feedback_id: u64) -> Result<(), DiskRipperError> {
        let entries = self.load_feedback()?;
        for entry in &entries {
            if entry.id == feedback_id {
                let mut updated = entry.clone();
                updated.used_for_training = true;

                let path = self.feedback_dir.join(format!("feedback_{}.json", entry.id));
                let json = serde_json::to_string_pretty(&updated)
                    .map_err(|e| DiskRipperError::Io(format!("Failed to serialize: {}", e)))?;
                std::fs::write(&path, json)
                    .map_err(|e| DiskRipperError::Io(format!("Failed to write: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Create a training batch from unused feedback
    pub fn create_training_batch(&self) -> Result<Option<TrainingBatch>, DiskRipperError> {
        let unused = self.load_unused_feedback()?;

        if unused.len() < 10 {
            info!("Not enough feedback for training batch (need 10, have {})", unused.len());
            return Ok(None);
        }

        let batch = TrainingBatch {
            id: chrono::Utc::now().timestamp() as u64,
            entries: unused,
            created_at: chrono::Utc::now(),
            model_version: "v1".to_string(),
        };

        info!("Created training batch with {} entries", batch.entries.len());
        Ok(Some(batch))
    }

    /// Get accuracy metrics for a model
    pub fn get_accuracy(&self, model_name: &str) -> Option<AccuracyMetrics> {
        let metrics_path = self.model_dir.join(format!("{}_metrics.json", model_name));
        if metrics_path.exists() {
            let json = std::fs::read_to_string(&metrics_path).ok()?;
            serde_json::from_str(&json).ok()
        } else {
            None
        }
    }

    /// Update accuracy metrics
    pub fn update_accuracy(
        &self,
        model_name: &str,
        correct: bool,
    ) -> Result<(), DiskRipperError> {
        let mut metrics = self.get_accuracy(model_name).unwrap_or(AccuracyMetrics {
            model_name: model_name.to_string(),
            total_predictions: 0,
            correct_predictions: 0,
            accuracy: 0.0,
            last_updated: chrono::Utc::now(),
        });

        metrics.total_predictions += 1;
        if correct {
            metrics.correct_predictions += 1;
        }
        metrics.accuracy = metrics.correct_predictions as f64 / metrics.total_predictions as f64;
        metrics.last_updated = chrono::Utc::now();

        let metrics_path = self.model_dir.join(format!("{}_metrics.json", model_name));
        let json = serde_json::to_string_pretty(&metrics)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize metrics: {}", e)))?;
        std::fs::write(&metrics_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write metrics: {}", e)))?;

        Ok(())
    }
}
