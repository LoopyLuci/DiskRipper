use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use crate::error::DiskRipperError;
use crate::filesystem::recovery::{calculate_checksum, verify_checksum};
use crate::filesystem::reader::read_raw_sectors;
use crate::job::JobManager;
use crate::progress::ProgressTracker;
use crate::types::*;

/// Verification result for a ripped file or disc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub file_path: String,
    pub source_checksum: String,
    pub dest_checksum: String,
    pub valid: bool,
    pub bytes_verified: u64,
}

/// Verify a ripped file against its source on disc
pub fn verify_file_rip(
    source_drive: &Path,
    dest_path: &Path,
    file_lba: u32,
    file_size: u32,
    sector_size: u32,
) -> Result<VerificationResult, DiskRipperError> {
    let num_sectors = (file_size + sector_size - 1) / sector_size;
    let source_data = read_raw_sectors(source_drive, file_lba as u64, num_sectors, sector_size)
        .map_err(|e| DiskRipperError::ReadError(format!("Source read failed: {}", e)))?;

    let source_data = &source_data[..file_size as usize];
    let source_checksum = calculate_checksum(source_data);

    let dest_data = std::fs::read(dest_path)
        .map_err(|e| DiskRipperError::Io(format!("Destination read failed: {}", e)))?;
    let dest_checksum = calculate_checksum(&dest_data);

    let valid = source_checksum == dest_checksum;

    Ok(VerificationResult {
        file_path: dest_path.to_string_lossy().to_string(),
        source_checksum,
        dest_checksum,
        valid,
        bytes_verified: file_size as u64,
    })
}

/// Verify an entire disc image against the source disc
pub fn verify_disc_image(
    job_manager: Arc<JobManager>,
    job_id: JobId,
    source_drive: &Path,
    image_path: &Path,
    total_size: u64,
) -> Result<Vec<VerificationResult>, DiskRipperError> {
    let sector_size = 2048u32;
    let batch_size = 100u32;
    let total_sectors = (total_size / sector_size as u64) as u32;

    let tracker = ProgressTracker::new(job_id.clone(), total_size, 1);
    let mut results = Vec::new();

    let image_data = std::fs::read(image_path)
        .map_err(|e| DiskRipperError::Io(format!("Failed to read image: {}", e)))?;

    let mut current_sector = 0u32;
    while current_sector < total_sectors {
        let to_read = if current_sector + batch_size > total_sectors {
            total_sectors - current_sector
        } else {
            batch_size
        };

        match read_raw_sectors(source_drive, current_sector as u64, to_read, sector_size) {
            Ok(source_data) => {
                let offset = (current_sector as usize) * (sector_size as usize);
                let end = offset + (to_read as usize) * (sector_size as usize);
                let image_slice = &image_data[offset..end.min(image_data.len())];

                let source_checksum = calculate_checksum(&source_data);
                let image_checksum = calculate_checksum(image_slice);
                let valid = source_checksum == image_checksum;

                results.push(VerificationResult {
                    file_path: format!("Sectors {}-{}", current_sector, current_sector + to_read - 1),
                    source_checksum,
                    dest_checksum: image_checksum,
                    valid,
                    bytes_verified: (to_read * sector_size) as u64,
                });
            }
            Err(e) => {
                results.push(VerificationResult {
                    file_path: format!("Sectors {}-{}", current_sector, current_sector + to_read - 1),
                    source_checksum: String::new(),
                    dest_checksum: String::new(),
                    valid: false,
                    bytes_verified: 0,
                });
                tracing::warn!("Failed to verify sectors {}-{}: {}", current_sector, current_sector + to_read - 1, e);
            }
        }

        current_sector += to_read;
        tracker.add_bytes((to_read * sector_size) as u64);

        if tracker.should_update(100) {
            let snapshot = tracker.snapshot();
            let _ = job_manager.update_progress(&job_id, snapshot);
        }
    }

    let mut final_snapshot = tracker.snapshot();
    final_snapshot.phase = Phase::Complete;
    let _ = job_manager.update_progress(&job_id, final_snapshot);

    Ok(results)
}

/// Verify using AccurateRip-style checksums (for audio CDs)
pub fn verify_audio_accuracy(track_data: &[u8], expected_checksum: &str) -> bool {
    verify_checksum(track_data, expected_checksum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_valid() {
        let result = VerificationResult {
            file_path: "test.iso".to_string(),
            source_checksum: "abc123".to_string(),
            dest_checksum: "abc123".to_string(),
            valid: true,
            bytes_verified: 1024,
        };
        assert!(result.valid);
    }

    #[test]
    fn test_verification_result_invalid() {
        let result = VerificationResult {
            file_path: "test.iso".to_string(),
            source_checksum: "abc123".to_string(),
            dest_checksum: "def456".to_string(),
            valid: false,
            bytes_verified: 1024,
        };
        assert!(!result.valid);
    }
}
