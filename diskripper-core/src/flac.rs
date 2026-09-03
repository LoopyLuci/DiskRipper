//! FLAC compression support for audio CD ripping.
//!
//! Provides lossless audio compression using the flac command-line encoder.
//! Falls back to WAV if flac is not available.

use std::path::Path;
use std::process::Command;

use tracing::{info, warn};

use crate::error::DiskRipperError;

/// FLAC compression level (0-8)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlacCompression {
    Fast = 0,
    Default = 5,
    Best = 8,
}

impl From<u8> for FlacCompression {
    fn from(level: u8) -> Self {
        match level {
            0..=2 => FlacCompression::Fast,
            3..=6 => FlacCompression::Default,
            _ => FlacCompression::Best,
        }
    }
}

/// FLAC encoder using external flac command
pub struct FlacEncoder;

impl FlacEncoder {
    /// Check if flac command is available
    pub fn is_available() -> bool {
        Command::new("flac")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Compress a WAV file to FLAC
    pub fn compress(
        wav_path: &Path,
        flac_path: &Path,
        compression: FlacCompression,
    ) -> Result<(), DiskRipperError> {
        info!(
            input = %wav_path.display(),
            output = %flac_path.display(),
            "Compressing WAV to FLAC"
        );

        let output = Command::new("flac")
            .arg(format!("--compression-level-{}", compression as u8))
            .arg("--force") // Overwrite existing
            .arg("--quiet")
            .arg("-o")
            .arg(flac_path)
            .arg(wav_path)
            .output()
            .map_err(|e| DiskRipperError::Io(format!("Failed to run flac: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DiskRipperError::Io(format!("flac failed: {}", stderr)));
        }

        info!(output = %flac_path.display(), "FLAC compression complete");
        Ok(())
    }

    /// Compress WAV to FLAC and remove the original WAV
    pub fn compress_and_remove(
        wav_path: &Path,
        flac_path: &Path,
        compression: FlacCompression,
    ) -> Result<(), DiskRipperError> {
        Self::compress(wav_path, flac_path, compression)?;

        // Remove the WAV file
        std::fs::remove_file(wav_path)
            .map_err(|e| DiskRipperError::Io(format!("Failed to remove WAV: {}", e)))?;

        Ok(())
    }

    /// Get FLAC version
    pub fn version() -> Option<String> {
        Command::new("flac")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }
}

/// Encode audio data to FLAC format
pub fn encode_to_flac(
    samples: &[i16],
    sample_rate: u32,
    channels: u8,
    output_path: &Path,
) -> Result<(), DiskRipperError> {
    // First write to a temporary WAV file
    let temp_wav = output_path.with_extension("wav.tmp");
    
    // Write WAV header and data
    {
        let mut file = std::fs::File::create(&temp_wav)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        let data_len = samples.len() * 2; // 2 bytes per sample
        let file_size = data_len + 36;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;
        
        // RIFF header
        file.write_all(b"RIFF").map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&(file_size as u32).to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(b"WAVE").map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        // fmt chunk
        file.write_all(b"fmt ").map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&16u32.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&1u16.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&channels.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&sample_rate.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&byte_rate.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&block_align.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&16u16.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        // data chunk
        file.write_all(b"data").map_err(|e| DiskRipperError::Io(e.to_string()))?;
        file.write_all(&(data_len as u32).to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        // Write samples
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).map_err(|e| DiskRipperError::Io(e.to_string()))?;
        }
    }
    
    // Compress to FLAC
    if FlacEncoder::is_available() {
        FlacEncoder::compress_and_remove(&temp_wav, output_path, FlacCompression::Default)?;
    } else {
        warn!("flac not available, keeping WAV format");
        std::fs::rename(&temp_wav, output_path)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
    }
    
    Ok(())
}

use std::io::Write;
