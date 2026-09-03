//! Music Identification Model.
//!
//! Hybrid approach combining:
//! - Audio fingerprinting (local)
//! - MusicBrainz metadata (when available)
//! - User feedback learning
//! - Confidence scoring

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;

/// Music identifier
pub struct MusicIdentifier {
    model_dir: std::path::PathBuf,
    /// Database of known music tracks
    music_db: HashMap<u64, MusicEntry>,
}

/// Music entry in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicEntry {
    pub id: u64,
    pub fingerprint_hash: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u8>,
    pub duration: f32,
    /// Number of confirmations from user feedback
    pub confirmations: u32,
}

/// Identification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicResult {
    pub confidence: f64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u8>,
}

impl MusicIdentifier {
    pub fn new(model_dir: &Path) -> Result<Self, DiskRipperError> {
        let db_path = model_dir.join("music_database.json");
        let music_db = if db_path.exists() {
            let json = std::fs::read_to_string(&db_path)
                .map_err(|e| DiskRipperError::Io(format!("Failed to read music db: {}", e)))?;
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            music_db,
        })
    }

    /// Identify music from fingerprint hash
    pub fn identify(&self, fingerprint_hash: u64) -> Option<MusicResult> {
        self.music_db.get(&fingerprint_hash).map(|entry| {
            let confidence = if entry.confirmations > 0 {
                0.9 + (entry.confirmations as f64 * 0.01).min(0.1)
            } else {
                0.7
            };

            MusicResult {
                confidence,
                title: entry.title.clone(),
                artist: entry.artist.clone(),
                album: entry.album.clone(),
                genre: entry.genre.clone(),
                year: entry.year,
                track_number: entry.track_number,
            }
        })
    }

    /// Add music entry to database
    pub fn add_entry(
        &mut self,
        fingerprint_hash: u64,
        title: &str,
        artist: &str,
        album: &str,
        genre: Option<&str>,
        year: Option<u32>,
        track_number: Option<u8>,
        duration: f32,
    ) -> Result<(), DiskRipperError> {
        let entry = MusicEntry {
            id: fingerprint_hash,
            fingerprint_hash,
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: genre.map(|s| s.to_string()),
            year,
            track_number,
            duration,
            confirmations: 1,
        };

        self.music_db.insert(fingerprint_hash, entry);
        self.save_database()?;

        info!("Added music entry: {} - {}", artist, title);
        Ok(())
    }

    /// Confirm a music identification (increases confidence)
    pub fn confirm(&mut self, fingerprint_hash: u64) -> Result<(), DiskRipperError> {
        if let Some(entry) = self.music_db.get_mut(&fingerprint_hash) {
            entry.confirmations += 1;
            self.save_database()?;
        }
        Ok(())
    }

    /// Save database to disk
    fn save_database(&self) -> Result<(), DiskRipperError> {
        let db_path = self.model_dir.join("music_database.json");
        let json = serde_json::to_string_pretty(&self.music_db)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize music db: {}", e)))?;
        std::fs::write(&db_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write music db: {}", e)))?;
        Ok(())
    }
}
