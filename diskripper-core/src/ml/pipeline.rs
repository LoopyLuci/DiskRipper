//! Core ML Pipeline.
//!
//! Provides the main interface for training and inference across all ML models.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::DiskRipperError;

/// ML Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Directory for model storage
    pub model_dir: std::path::PathBuf,
    /// Directory for training data
    pub data_dir: std::path::PathBuf,
    /// Directory for user feedback
    pub feedback_dir: std::path::PathBuf,
    /// Enable self-learning from user feedback
    pub enable_self_learning: bool,
    /// Minimum confidence threshold for predictions
    pub confidence_threshold: f64,
    /// Maximum batch size for inference
    pub max_batch_size: usize,
    /// Enable GPU acceleration if available
    pub enable_gpu: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            model_dir: std::path::PathBuf::from("./models"),
            data_dir: std::path::PathBuf::from("./data"),
            feedback_dir: std::path::PathBuf::from("./feedback"),
            enable_self_learning: true,
            confidence_threshold: 0.7,
            max_batch_size: 32,
            enable_gpu: false,
        }
    }
}

/// Main ML Pipeline
pub struct MlPipeline {
    pub config: PipelineConfig,
    // Sub-models
    pub audio_fingerprinter: crate::ml::audio_fingerprint::AudioFingerprinter,
    pub music_identifier: crate::ml::music_identification::MusicIdentifier,
    pub video_fingerprinter: crate::ml::video_fingerprint::VideoFingerprinter,
    pub content_classifier: crate::ml::content_classifier::ContentClassifier,
    pub hybrid_identifier: crate::ml::hybrid_identifier::HybridIdentifier,
}

/// Result from ML pipeline inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Identified content type
    pub content_type: ContentType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Identified title
    pub title: Option<String>,
    /// Identified artist/creator
    pub artist: Option<String>,
    /// Identified album/series
    pub album: Option<String>,
    /// Genre classification
    pub genre: Option<String>,
    /// Year of release
    pub year: Option<u32>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
    /// Source of identification (which model)
    pub source: IdentificationSource,
}

/// Content type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Music,
    Movie,
    TvShow,
    Software,
    Game,
    Data,
    Unknown,
}

/// Source of identification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IdentificationSource {
    AudioFingerprint,
    MusicBrainz,
    VideoFingerprint,
    Tmdb,
    ContentClassifier,
    Hybrid,
    UserProvided,
}

impl MlPipeline {
    /// Initialize the ML pipeline
    pub fn new(config: PipelineConfig) -> Result<Self, DiskRipperError> {
        info!("Initializing ML pipeline");

        // Create directories
        std::fs::create_dir_all(&config.model_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create model dir: {}", e)))?;
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create data dir: {}", e)))?;
        std::fs::create_dir_all(&config.feedback_dir)
            .map_err(|e| DiskRipperError::Io(format!("Failed to create feedback dir: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            audio_fingerprinter: crate::ml::audio_fingerprint::AudioFingerprinter::new(&config.model_dir)?,
            music_identifier: crate::ml::music_identification::MusicIdentifier::new(&config.model_dir)?,
            video_fingerprinter: crate::ml::video_fingerprint::VideoFingerprinter::new(&config.model_dir)?,
            content_classifier: crate::ml::content_classifier::ContentClassifier::new(&config.model_dir)?,
            hybrid_identifier: crate::ml::hybrid_identifier::HybridIdentifier::new(&config.model_dir)?,
        })
    }

    /// Identify audio content
    pub fn identify_audio(&self, audio_data: &[i16], sample_rate: u32) -> Result<PipelineResult, DiskRipperError> {
        // Step 1: Generate audio fingerprint
        let fingerprint = self.audio_fingerprinter.generate(audio_data, sample_rate)?;

        // Step 2: Try to match against known fingerprints
        if let Some(match_result) = self.audio_fingerprinter.match_fingerprint(&fingerprint) {
            if match_result.confidence >= self.config.confidence_threshold {
                return Ok(PipelineResult {
                    content_type: ContentType::Music,
                    confidence: match_result.confidence,
                    title: match_result.title,
                    artist: match_result.artist,
                    album: match_result.album,
                    genre: None,
                    year: None,
                    metadata: std::collections::HashMap::new(),
                    source: IdentificationSource::AudioFingerprint,
                });
            }
        }

        // Step 3: Fall back to content classifier
        let classification = self.content_classifier.classify_audio(audio_data, sample_rate)?;

        Ok(PipelineResult {
            content_type: ContentType::Music,
            confidence: classification.confidence,
            title: None,
            artist: None,
            album: None,
            genre: classification.genre,
            year: None,
            metadata: std::collections::HashMap::new(),
            source: IdentificationSource::ContentClassifier,
        })
    }

    /// Identify video content
    pub fn identify_video(&self, video_path: &Path) -> Result<PipelineResult, DiskRipperError> {
        // Step 1: Generate video fingerprint
        let fingerprint = self.video_fingerprinter.generate(video_path)?;

        // Step 2: Try to match against known fingerprints
        if let Some(match_result) = self.video_fingerprinter.match_fingerprint(&fingerprint) {
            if match_result.confidence >= self.config.confidence_threshold {
                return Ok(PipelineResult {
                    content_type: ContentType::Movie,
                    confidence: match_result.confidence,
                    title: match_result.title,
                    artist: None,
                    album: None,
                    genre: None,
                    year: None,
                    metadata: std::collections::HashMap::new(),
                    source: IdentificationSource::VideoFingerprint,
                });
            }
        }

        // Step 3: Fall back to content classifier
        let classification = self.content_classifier.classify_video(video_path)?;

        Ok(PipelineResult {
            content_type: classification.content_type,
            confidence: classification.confidence,
            title: None,
            artist: None,
            album: None,
            genre: classification.genre,
            year: None,
            metadata: std::collections::HashMap::new(),
            source: IdentificationSource::ContentClassifier,
        })
    }

    /// Provide user feedback to improve models
    pub fn provide_feedback(
        &self,
        result: &PipelineResult,
        correct_title: &str,
        correct_artist: Option<&str>,
    ) -> Result<(), DiskRipperError> {
        if !self.config.enable_self_learning {
            return Ok(());
        }

        let feedback = UserFeedback {
            original_result: result.clone(),
            correct_title: correct_title.to_string(),
            correct_artist: correct_artist.map(|s| s.to_string()),
            timestamp: chrono::Utc::now(),
        };

        // Save feedback for future training
        let feedback_path = self.config.feedback_dir.join(format!(
            "feedback_{}.json",
            chrono::Utc::now().timestamp()
        ));

        let json = serde_json::to_string_pretty(&feedback)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize feedback: {}", e)))?;

        std::fs::write(&feedback_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to save feedback: {}", e)))?;

        info!("Saved user feedback to {:?}", feedback_path);
        Ok(())
    }
}

/// User feedback for self-learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub original_result: PipelineResult,
    pub correct_title: String,
    pub correct_artist: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
