//! Feature extraction for ML models.
//!
//! Extracts features from audio and video content for classification:
//! - Audio: MFCCs, spectral centroid, zero-crossing rate, chroma
//! - Video: Color histograms, scene changes, motion vectors
//! - Generic: Statistical features, entropy, histograms

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;

/// Extracted features for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    /// Feature vector (flattened)
    pub values: Vec<f32>,
    /// Feature names (for interpretability)
    pub names: Vec<String>,
    /// Source content type
    pub content_type: String,
    /// Duration in seconds (if applicable)
    pub duration: f32,
}

/// Audio feature extractor
pub struct AudioFeatureExtractor;

/// Video feature extractor
pub struct VideoFeatureExtractor;

impl AudioFeatureExtractor {
    /// Extract features from raw audio samples
    pub fn extract(audio_data: &[i16], sample_rate: u32) -> Result<Features, DiskRipperError> {
        if audio_data.is_empty() {
            return Err(DiskRipperError::Io("Empty audio data".to_string()));
        }

        let mut values = Vec::new();
        let mut names: Vec<String> = Vec::new();

        // 1. Basic statistics
        let (mean, std, min, max) = compute_basic_stats(audio_data);
        values.extend_from_slice(&[mean, std, min, max]);
        names.extend(["mean", "std", "min", "max"].iter().map(|&s| s.to_string()));

        // 2. Zero-crossing rate
        let zcr = compute_zero_crossing_rate(audio_data);
        values.push(zcr);
        names.push("zero_crossing_rate".to_string());

        // 3. RMS energy
        let rms = compute_rms(audio_data);
        values.push(rms);
        names.push("rms_energy".to_string());

        // 4. Spectral features (from FFT)
        let spectral = compute_spectral_features(audio_data, sample_rate);
        for (i, &val) in spectral.iter().enumerate() {
            values.push(val);
            names.push(format!("spectral_{}", i));
        }

        // 5. MFCC-like features (simplified)
        let mfcc = compute_mfcc_like(audio_data, sample_rate);
        for (i, &val) in mfcc.iter().enumerate() {
            values.push(val);
            names.push(format!("mfcc_{}", i));
        }

        // 6. Chroma features (12 pitch classes)
        let chroma = compute_chroma(audio_data, sample_rate);
        for (i, &val) in chroma.iter().enumerate() {
            values.push(val);
            names.push(format!("chroma_{}", i));
        }

        Ok(Features {
            values,
            names,
            content_type: "audio".to_string(),
            duration: audio_data.len() as f32 / sample_rate as f32,
        })
    }

    /// Extract features from multiple audio segments (for long audio)
    pub fn extract_segments(
        audio_data: &[i16],
        sample_rate: u32,
        segment_duration_secs: f32,
    ) -> Result<Vec<Features>, DiskRipperError> {
        let samples_per_segment = (sample_rate as f32 * segment_duration_secs) as usize;
        let mut features = Vec::new();

        for (i, chunk) in audio_data.chunks(samples_per_segment).enumerate() {
            if chunk.len() > 1000 {
                let mut feat = Self::extract(chunk, sample_rate)?;
                feat.names.push(format!("segment_{}", i));
                features.push(feat);
            }
        }

        Ok(features)
    }
}

impl VideoFeatureExtractor {
    /// Extract features from video frames (simplified)
    pub fn extract_from_frames(
        frame_data: &[Vec<u8>],
        width: u32,
        height: u32,
    ) -> Result<Features, DiskRipperError> {
        let mut values = Vec::new();
        let mut names: Vec<String> = Vec::new();

        if frame_data.is_empty() {
            return Err(DiskRipperError::Io("No frame data".to_string()));
        }

        // 1. Average brightness per frame
        let brightness: Vec<f32> = frame_data
            .iter()
            .map(|frame| {
                if frame.is_empty() {
                    return 0.0;
                }
                frame.iter().map(|&p| p as f32).sum::<f32>() / frame.len() as f32
            })
            .collect();

        let avg_brightness = brightness.iter().sum::<f32>() / brightness.len() as f32;
        let brightness_std = std_dev(&brightness);
        values.extend_from_slice(&[avg_brightness, brightness_std]);
        names.extend(
            ["avg_brightness", "brightness_std"]
                .iter()
                .map(|&s| s.to_string()),
        );

        // 2. Scene change detection (frame differences)
        let mut scene_changes = 0;
        let mut total_diff = 0.0f32;
        for window in frame_data.windows(2) {
            let diff = compute_frame_diff(&window[0], &window[1]);
            total_diff += diff;
            if diff > 30.0 {
                scene_changes += 1;
            }
        }
        values.push(scene_changes as f32);
        values.push(total_diff / frame_data.len().saturating_sub(1).max(1) as f32);
        names.extend(
            ["scene_changes", "avg_frame_diff"]
                .iter()
                .map(|&s| s.to_string()),
        );

        // 3. Color histogram features (simplified)
        let color_features = compute_color_histogram(&frame_data[0], width, height);
        for (i, &val) in color_features.iter().enumerate() {
            values.push(val);
            names.push(format!("color_{}", i));
        }

        // 4. Motion estimation (simplified)
        let motion = estimate_motion(frame_data);
        values.push(motion);
        names.push("motion_estimate".to_string());

        Ok(Features {
            values,
            names,
            content_type: "video".to_string(),
            duration: frame_data.len() as f32 / 30.0,
        })
    }
}

// === Audio Feature Functions ===

fn compute_basic_stats(audio_data: &[i16]) -> (f32, f32, f32, f32) {
    let n = audio_data.len() as f32;
    let sum: f64 = audio_data.iter().map(|&x| x as f64).sum();
    let mean = sum / n as f64;

    let variance: f64 = audio_data
        .iter()
        .map(|&x| {
            let diff = x as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / n as f64;

    let std = variance.sqrt();
    let min = *audio_data.iter().min().unwrap_or(&0) as f64;
    let max = *audio_data.iter().max().unwrap_or(&0) as f64;

    (mean as f32, std as f32, min as f32, max as f32)
}

fn compute_zero_crossing_rate(audio_data: &[i16]) -> f32 {
    let mut crossings = 0;
    for window in audio_data.windows(2) {
        if (window[0] >= 0) != (window[1] >= 0) {
            crossings += 1;
        }
    }
    crossings as f32 / audio_data.len() as f32
}

fn compute_rms(audio_data: &[i16]) -> f32 {
    let sum_squares: f64 = audio_data
        .iter()
        .map(|&x| {
            let normalized = x as f64 / i16::MAX as f64;
            normalized * normalized
        })
        .sum();
    (sum_squares / audio_data.len() as f64).sqrt() as f32
}

fn compute_spectral_features(audio_data: &[i16], _sample_rate: u32) -> Vec<f32> {
    let window_size = 1024.min(audio_data.len());
    if window_size < 256 {
        return vec![0.0; 8];
    }

    let window = &audio_data[..window_size];
    let magnitudes = compute_fft_magnitudes(window);

    let num_bands = 8;
    let band_size = magnitudes.len() / num_bands;
    let mut features = Vec::with_capacity(num_bands);

    for band in 0..num_bands {
        let start = band * band_size;
        let end = start + band_size;
        let band_energy: f32 = magnitudes[start..end].iter().sum();
        features.push(band_energy);
    }

    let max_energy = features.iter().cloned().fold(0.0f32, f32::max);
    if max_energy > 0.0 {
        for f in &mut features {
            *f /= max_energy;
        }
    }

    features
}

fn compute_fft_magnitudes(window: &[i16]) -> Vec<f32> {
    let n = window.len();
    let mut magnitudes = vec![0.0f32; n / 2];

    for k in 0..n / 2 {
        let mut real = 0.0f64;
        let mut imag = 0.0f64;
        for (i, &sample) in window.iter().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * k as f64 * i as f64 / n as f64;
            let normalized = sample as f64 / i16::MAX as f64;
            real += normalized * angle.cos();
            imag -= normalized * angle.sin();
        }
        magnitudes[k] = ((real * real + imag * imag).sqrt() / n as f64) as f32;
    }

    magnitudes
}

fn compute_mfcc_like(audio_data: &[i16], sample_rate: u32) -> Vec<f32> {
    let spectral = compute_spectral_features(audio_data, sample_rate);
    let mut mfcc = Vec::with_capacity(13);

    for i in 0..13 {
        let mut coeff = 0.0f32;
        for (j, &spec) in spectral.iter().enumerate() {
            let weight = ((i + 1) as f32 * (j + 1) as f32 * std::f32::consts::PI
                / spectral.len() as f32)
                .cos();
            coeff += spec * weight.abs();
        }
        mfcc.push(coeff.ln().max(-10.0));
    }

    mfcc
}

fn compute_chroma(audio_data: &[i16], sample_rate: u32) -> Vec<f32> {
    let magnitudes = compute_fft_magnitudes(audio_data);
    let mut chroma = vec![0.0f32; 12];

    for (k, &mag) in magnitudes.iter().enumerate() {
        if k == 0 {
            continue;
        }
        let freq = k as f32 * sample_rate as f32 / audio_data.len() as f32;
        if freq > 20.0 && freq < 4000.0 {
            let midi_note = 12.0 * (freq / 440.0).log2() + 69.0;
            let pitch_class = (midi_note.round() as i32).rem_euclid(12) as usize;
            chroma[pitch_class] += mag;
        }
    }

    let max_val = chroma.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0 {
        for c in &mut chroma {
            *c /= max_val;
        }
    }

    chroma
}

// === Video Feature Functions ===

fn compute_frame_diff(frame1: &[u8], frame2: &[u8]) -> f32 {
    let len = frame1.len().min(frame2.len());
    if len == 0 {
        return 0.0;
    }

    let diff: u64 = frame1[..len]
        .iter()
        .zip(frame2[..len].iter())
        .map(|(&a, &b)| (a as i16 - b as i16).abs() as u64)
        .sum();

    diff as f32 / len as f32
}

fn compute_color_histogram(frame: &[u8], width: u32, height: u32) -> Vec<f32> {
    let mut histogram = vec![0.0f32; 24];

    let pixel_count = (width * height) as usize;
    let channels = 3;

    for i in 0..pixel_count.min(frame.len() / channels) {
        for c in 0..channels {
            let val = frame[i * channels + c];
            let bin = (val as usize * 8 / 256).min(7);
            histogram[c * 8 + bin] += 1.0;
        }
    }

    let total = pixel_count as f32;
    if total > 0.0 {
        for h in &mut histogram {
            *h /= total;
        }
    }

    histogram
}

fn estimate_motion(frame_data: &[Vec<u8>]) -> f32 {
    if frame_data.len() < 2 {
        return 0.0;
    }

    let mut total_motion = 0.0f32;
    for window in frame_data.windows(2) {
        total_motion += compute_frame_diff(&window[0], &window[1]);
    }

    total_motion / (frame_data.len() - 1) as f32
}

fn std_dev(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f32>()
        / values.len() as f32;
    variance.sqrt()
}
