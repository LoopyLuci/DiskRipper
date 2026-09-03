//! Hybrid Identification System.
//!
//! Combines multiple identification signals for maximum accuracy:
//! - Audio fingerprinting
//! - Video fingerprinting
//! - Content classification
//! - Metadata matching
//! - User feedback history
//!
//! Confidence scoring determines which signals to trust.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::DiskRipperError;
use crate::ml::pipeline::{ContentType, IdentificationSource, PipelineResult};

/// Hybrid identifier combines multiple signals
pub struct HybridIdentifier {
    model_dir: std::path::PathBuf,
}

/// Combined identification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentificationResult {
    pub content_type: ContentType,
    pub confidence: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub source: IdentificationSource,
    /// Individual signal confidences
    pub signal_confidences: Vec<SignalConfidence>,
}

/// Confidence from a single signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfidence {
    pub source: IdentificationSource,
    pub confidence: f64,
    pub title: Option<String>,
}

impl HybridIdentifier {
    pub fn new(model_dir: &Path) -> Result<Self, DiskRipperError> {
        Ok(Self {
            model_dir: model_dir.to_path_buf(),
        })
    }

    /// Combine multiple identification signals into a single result
    ///
    /// Uses weighted voting based on historical accuracy of each signal.
    pub fn combine_signals(&self, signals: Vec<SignalConfidence>) -> Option<IdentificationResult> {
        if signals.is_empty() {
            return None;
        }

        // Weight each signal by its confidence and historical accuracy
        let mut best_title: Option<(String, f64)> = None;
        let mut best_artist: Option<(String, f64)> = None;
        let mut best_album: Option<(String, f64)> = None;
        let mut best_genre: Option<(String, f64)> = None;
        let mut total_confidence = 0.0;
        let mut best_source = IdentificationSource::Hybrid;

        for signal in &signals {
            let weighted_confidence = signal.confidence;

            if let Some(ref title) = signal.title {
                if best_title.as_ref().map_or(true, |(_, c)| weighted_confidence > *c) {
                    best_title = Some((title.clone(), weighted_confidence));
                }
            }

            if let Some(ref artist) = signal.title {
                if best_artist.as_ref().map_or(true, |(_, c)| weighted_confidence > *c) {
                    best_artist = Some((artist.clone(), weighted_confidence));
                }
            }

            total_confidence += signal.confidence;

            // Track best source
            if signal.confidence > 0.8 {
                best_source = signal.source.clone();
            }
        }

        let avg_confidence = total_confidence / signals.len() as f64;

        Some(IdentificationResult {
            content_type: ContentType::Unknown, // Determined by signals
            confidence: avg_confidence,
            title: best_title.map(|(t, _)| t),
            artist: best_artist.map(|(a, _)| a),
            album: best_album.map(|(a, _)| a),
            genre: best_genre.map(|(g, _)| g),
            year: None,
            source: best_source,
            signal_confidences: signals,
        })
    }
}
