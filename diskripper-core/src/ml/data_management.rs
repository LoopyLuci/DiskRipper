//! Data Management for ML Training.
//!
//! Handles:
//! - Training data collection and storage
//! - Data augmentation for audio/video
//! - Dataset versioning
//! - Data quality validation

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;

/// Data manager for ML training datasets
pub struct DataManager {
    data_dir: std::path::PathBuf,
}

/// Training sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub id: u64,
    pub features: Vec<f64>,
    pub label: String,
    pub content_type: String,
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Dataset statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_samples: usize,
    pub num_classes: usize,
    pub class_distribution: std::collections::HashMap<String, usize>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl DataManager {
    pub fn new(data_dir: &Path) -> Result<Self, DiskRipperError> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create data dir: {}", e)))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Add a training sample
    pub fn add_sample(
        &self,
        features: Vec<f64>,
        label: &str,
        content_type: &str,
        source: &str,
    ) -> Result<(), DiskRipperError> {
        let sample = TrainingSample {
            id: chrono::Utc::now().timestamp() as u64,
            features,
            label: label.to_string(),
            content_type: content_type.to_string(),
            source: source.to_string(),
            timestamp: chrono::Utc::now(),
        };

        let path = self.data_dir.join(format!("sample_{}.json", sample.id));
        let json = serde_json::to_string_pretty(&sample)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize sample: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write sample: {}", e)))?;

        Ok(())
    }

    /// Load all training samples
    pub fn load_samples(&self) -> Result<Vec<TrainingSample>, DiskRipperError> {
        let mut samples = Vec::new();

        for entry in std::fs::read_dir(&self.data_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read data dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| DiskRipperError::Io(format!("Failed to read entry: {}", e)))?;

            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let json = std::fs::read_to_string(entry.path())
                    .map_err(|e| DiskRipperError::Io(format!("Failed to read sample: {}", e)))?;

                if let Ok(sample) = serde_json::from_str::<TrainingSample>(&json) {
                    samples.push(sample);
                }
            }
        }

        Ok(samples)
    }

    /// Get dataset statistics
    pub fn get_stats(&self) -> Result<DatasetStats, DiskRipperError> {
        let samples = self.load_samples()?;
        let mut class_distribution = std::collections::HashMap::new();

        for sample in &samples {
            *class_distribution.entry(sample.label.clone()).or_insert(0) += 1;
        }

        Ok(DatasetStats {
            total_samples: samples.len(),
            num_classes: class_distribution.len(),
            class_distribution,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Augment audio data with various transformations
    pub fn augment_audio(&self, audio_data: &[i16], sample_rate: u32) -> Vec<Vec<i16>> {
        let mut augmented = Vec::new();

        // Original
        augmented.push(audio_data.to_vec());

        // Pitch shift (simple resampling)
        let pitch_shifted: Vec<i16> = audio_data.iter().step_by(2).copied().collect();
        augmented.push(pitch_shifted);

        // Time stretch (skip every Nth sample)
        let time_stretched: Vec<i16> = audio_data.iter().step_by(3).copied().collect();
        augmented.push(time_stretched);

        // Add noise (small random variations)
        let noisy: Vec<i16> = audio_data
            .iter()
            .map(|&s| {
                let noise = (rand::random::<f64>() - 0.5) * 100.0;
                (s as f64 + noise) as i16
            })
            .collect();
        augmented.push(noisy);

        augmented
    }
}
