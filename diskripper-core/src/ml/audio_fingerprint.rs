//! Audio Fingerprinting System (AcoustID Replacement).
//!
//! Custom audio fingerprinting that works entirely locally:
//! - Generates compact fingerprints from audio data
//! - Matches fingerprints against a local database
//! - Self-improving through user feedback
//! - No external API dependencies

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::DiskRipperError;

/// Audio fingerprint (compact representation of audio)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFingerprint {
    /// Spectral peaks (frequency, time)
    pub peaks: Vec<(u32, u32)>,
    /// Hash of peaks for fast matching
    pub hash: u64,
    /// Duration in seconds
    pub duration: f32,
    /// Sample rate
    pub sample_rate: u32,
}

/// Match result from fingerprint comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintMatch {
    pub confidence: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub fingerprint_id: u64,
}

/// Audio fingerprinter
pub struct AudioFingerprinter {
    model_dir: std::path::PathBuf,
    /// Database of known fingerprints
    fingerprint_db: HashMap<u64, KnownFingerprint>,
}

/// Known fingerprint in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownFingerprint {
    pub id: u64,
    pub peaks: Vec<(u32, u32)>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    /// Number of times this fingerprint has been confirmed
    pub confirmations: u32,
}

impl AudioFingerprinter {
    pub fn new(model_dir: &Path) -> Result<Self, DiskRipperError> {
        let db_path = model_dir.join("audio_fingerprints.json");

        let fingerprint_db = if db_path.exists() {
            let json = std::fs::read_to_string(&db_path)
                .map_err(|e| DiskRipperError::Io(format!("Failed to read fingerprint db: {}", e)))?;
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            fingerprint_db,
        })
    }

    /// Generate fingerprint from audio data
    ///
    /// Algorithm:
    /// 1. Apply FFT to get spectrogram
    /// 2. Find spectral peaks (local maxima)
    /// 3. Create combinatorial hash from peak pairs
    pub fn generate(&self, audio_data: &[i16], sample_rate: u32) -> Result<AudioFingerprint, DiskRipperError> {
        // Step 1: Compute spectrogram using FFT
        let spectrogram = self.compute_spectrogram(audio_data, sample_rate);

        // Step 2: Find spectral peaks
        let peaks = self.find_spectral_peaks(&spectrogram, sample_rate);

        // Step 3: Generate hash from peaks
        let hash = self.hash_peaks(&peaks);

        let duration = audio_data.len() as f32 / sample_rate as f32;

        Ok(AudioFingerprint {
            peaks: peaks.clone(),
            hash,
            duration,
            sample_rate,
        })
    }

    /// Match fingerprint against database
    pub fn match_fingerprint(&self, fingerprint: &AudioFingerprint) -> Option<FingerprintMatch> {
        let mut best_match: Option<FingerprintMatch> = None;
        let mut best_score: f64 = 0.0;

        for (id, known) in &self.fingerprint_db {
            let score = self.compare_fingerprints(fingerprint, known);

            if score > best_score && score > 0.3 {
                best_score = score;
                best_match = Some(FingerprintMatch {
                    confidence: score,
                    title: known.title.clone(),
                    artist: known.artist.clone(),
                    album: known.album.clone(),
                    fingerprint_id: *id,
                });
            }
        }

        best_match
    }

    /// Add fingerprint to database
    pub fn add_fingerprint(
        &mut self,
        fingerprint: &AudioFingerprint,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
    ) -> Result<(), DiskRipperError> {
        let id = fingerprint.hash;

        let known = KnownFingerprint {
            id,
            peaks: fingerprint.peaks.clone(),
            title: title.map(|s| s.to_string()),
            artist: artist.map(|s| s.to_string()),
            album: album.map(|s| s.to_string()),
            genre: None,
            year: None,
            confirmations: 1,
        };

        self.fingerprint_db.insert(id, known);
        self.save_database()?;

        info!("Added fingerprint {} to database", id);
        Ok(())
    }

    /// Confirm a fingerprint match (increases confidence)
    pub fn confirm_fingerprint(&mut self, id: u64) -> Result<(), DiskRipperError> {
        if let Some(fingerprint) = self.fingerprint_db.get_mut(&id) {
            fingerprint.confirmations += 1;
            self.save_database()?;
        }
        Ok(())
    }

    /// Save fingerprint database to disk
    fn save_database(&self) -> Result<(), DiskRipperError> {
        let db_path = self.model_dir.join("audio_fingerprints.json");
        let json = serde_json::to_string_pretty(&self.fingerprint_db)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize fingerprint db: {}", e)))?;
        std::fs::write(&db_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write fingerprint db: {}", e)))?;
        Ok(())
    }

    /// Compute spectrogram using FFT
    fn compute_spectrogram(&self, audio_data: &[i16], sample_rate: u32) -> Vec<Vec<f64>> {
        let window_size = 2048;
        let hop_size = 512;
        let num_windows = (audio_data.len() - window_size) / hop_size;

        let mut spectrogram = Vec::new();

        for i in 0..num_windows {
            let start = i * hop_size;
            let window = &audio_data[start..start + window_size];

            // Apply Hann window
            let windowed: Vec<f64> = window.iter().enumerate().map(|(j, &sample)| {
                let hann = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * j as f64 / window_size as f64).cos());
                sample as f64 * hann
            }).collect();

            // Compute FFT magnitude
            let spectrum = self.fft_magnitude(&windowed);
            spectrogram.push(spectrum);
        }

        spectrogram
    }

    /// Compute FFT magnitude spectrum
    fn fft_magnitude(&self, signal: &[f64]) -> Vec<f64> {
        let n = signal.len();
        let mut magnitudes = vec![0.0; n / 2];

        // Simple DFT for now — in production use FFTW or rustfft
        for k in 0..n / 2 {
            let mut real = 0.0;
            let mut imag = 0.0;

            for (t, &sample) in signal.iter().enumerate() {
                let angle = -2.0 * std::f64::consts::PI * k as f64 * t as f64 / n as f64;
                real += sample * angle.cos();
                imag += sample * angle.sin();
            }

            magnitudes[k] = (real * real + imag * imag).sqrt();
        }

        magnitudes
    }

    /// Find spectral peaks (local maxima in time-frequency space)
    fn find_spectral_peaks(&self, spectrogram: &[Vec<f64>], _sample_rate: u32) -> Vec<(u32, u32)> {
        let mut peaks = Vec::new();
        let threshold = 0.5; // Minimum magnitude threshold

        for (t, spectrum) in spectrogram.iter().enumerate() {
            for (f, &magnitude) in spectrum.iter().enumerate() {
                if magnitude > threshold {
                    // Check if local maximum
                    let is_max = (f == 0 || magnitude > spectrum[f - 1])
                        && (f + 1 >= spectrum.len() || magnitude > spectrum[f + 1]);

                    if is_max {
                        peaks.push((f as u32, t as u32));
                    }
                }
            }
        }

        // Keep only top N peaks for compactness
        peaks.sort_by(|a, b| {
            let mag_a = spectrogram[a.1 as usize][a.0 as usize];
            let mag_b = spectrogram[b.1 as usize][b.0 as usize];
            mag_b.partial_cmp(&mag_a).unwrap()
        });
        peaks.truncate(100);

        peaks
    }

    /// Generate hash from peaks using combinatorial hashing
    fn hash_peaks(&self, peaks: &[(u32, u32)]) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Create pairs of peaks and hash them
        for i in 0..peaks.len().min(50) {
            for j in (i + 1)..peaks.len().min(50) {
                let (f1, t1) = peaks[i];
                let (f2, t2) = peaks[j];

                let hash_input = (f1, f2, t2.wrapping_sub(t1));
                hash_input.hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Compare two fingerprints
    fn compare_fingerprints(&self, fp1: &AudioFingerprint, fp2: &KnownFingerprint) -> f64 {
        // Compare peak sets using Jaccard similarity
        let set1: std::collections::HashSet<_> = fp1.peaks.iter().collect();
        let set2: std::collections::HashSet<_> = fp2.peaks.iter().collect();

        let intersection: std::collections::HashSet<_> = set1.intersection(&set2).collect();
        let union: std::collections::HashSet<_> = set1.union(&set2).collect();

        if union.is_empty() {
            return 0.0;
        }

        let jaccard = intersection.len() as f64 / union.len() as f64;

        // Also compare duration similarity
        let duration_sim = 1.0 - ((fp1.duration as f64 - fp2.peaks.len() as f64 * 0.01).abs()
            / fp1.duration.max(1.0) as f64);

        // Weighted combination
        jaccard * 0.7 + duration_sim * 0.3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_generation() {
        // Generate simple sine wave
        let sample_rate = 44100;
        let duration_secs = 1.0;
        let num_samples = (sample_rate as f64 * duration_secs) as usize;
        let frequency = 440.0;

        let audio: Vec<i16> = (0..num_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
                (sample * i16::MAX as f64) as i16
            })
            .collect();

        let fingerprinter = AudioFingerprinter {
            model_dir: std::path::PathBuf::from("."),
            fingerprint_db: HashMap::new(),
        };

        let fingerprint = fingerprinter.generate(&audio, sample_rate).unwrap();
        assert!(!fingerprint.peaks.is_empty());
        assert!(fingerprint.duration > 0.0);
    }

    #[test]
    fn test_fingerprint_matching() {
        let mut fingerprinter = AudioFingerprinter {
            model_dir: std::path::PathBuf::from("."),
            fingerprint_db: HashMap::new(),
        };

        let peaks = vec![(100, 1), (200, 2), (300, 3)];
        let hash = 12345;

        let fingerprint = AudioFingerprint {
            peaks: peaks.clone(),
            hash,
            duration: 60.0,
            sample_rate: 44100,
        };

        let known = KnownFingerprint {
            id: hash,
            peaks,
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            genre: None,
            year: None,
            confirmations: 1,
        };

        fingerprinter.fingerprint_db.insert(hash, known);

        let result = fingerprinter.match_fingerprint(&fingerprint);
        assert!(result.is_some());

        let match_result = result.unwrap();
        assert_eq!(match_result.title, Some("Test Song".to_string()));
        assert!(match_result.confidence > 0.0);
    }
}
