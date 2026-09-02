use std::path::Path;
use std::time::Duration;

use crate::error::DiskRipperError;

/// Configuration for read error recovery
#[derive(Debug, Clone)]
pub struct ReadConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub skip_on_failure: bool,
}

impl Default for ReadConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            skip_on_failure: false,
        }
    }
}

/// Result of a read operation with error recovery
#[derive(Debug)]
pub struct ReadResult {
    pub data: Vec<u8>,
    pub retries_used: u32,
    pub had_errors: bool,
}

/// Read sectors with exponential backoff retry
pub fn read_sectors_with_retry(
    drive_path: &Path,
    start_sector: u64,
    num_sectors: u32,
    sector_size: u32,
    config: &ReadConfig,
) -> Result<ReadResult, DiskRipperError> {
    use crate::filesystem::reader::read_raw_sectors;

    let mut last_error = None;
    let mut delay = config.initial_delay_ms;

    for attempt in 0..config.max_retries {
        match read_raw_sectors(drive_path, start_sector, num_sectors, sector_size) {
            Ok(data) => {
                return Ok(ReadResult {
                    data,
                    retries_used: attempt,
                    had_errors: attempt > 0,
                });
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries - 1 {
                    std::thread::sleep(Duration::from_millis(delay));
                    delay = ((delay as f64) * config.backoff_multiplier) as u64;
                    if delay > config.max_delay_ms {
                        delay = config.max_delay_ms;
                    }
                }
            }
        }
    }

    Err(DiskRipperError::ReadError(format!(
        "Failed after {} retries: {}",
        config.max_retries,
        last_error.unwrap()
    )))
}

/// Read a large range of sectors with per-batch error recovery
pub fn read_large_range(
    drive_path: &Path,
    start_sector: u64,
    total_sectors: u64,
    sector_size: u32,
    batch_size: u32,
    config: &ReadConfig,
) -> Result<Vec<u8>, DiskRipperError> {
    

    let mut all_data = Vec::with_capacity((total_sectors * sector_size as u64) as usize);
    let mut current_sector = start_sector;
    let mut remaining = total_sectors;
    let mut total_retries = 0u32;
    let mut had_errors = false;

    while remaining > 0 {
        let to_read = if remaining > batch_size as u64 {
            batch_size
        } else {
            remaining as u32
        };

        match read_sectors_with_retry(drive_path, current_sector, to_read, sector_size, config) {
            Ok(result) => {
                all_data.extend_from_slice(&result.data);
                total_retries += result.retries_used;
                had_errors |= result.had_errors;
            }
            Err(e) => {
                if config.skip_on_failure {
                    // Fill with zeros for failed sectors
                    let zeros = vec![0u8; (to_read * sector_size) as usize];
                    all_data.extend_from_slice(&zeros);
                    tracing::warn!(
                        "Skipped sectors {}-{} due to read error",
                        current_sector,
                        current_sector + to_read as u64 - 1
                    );
                } else {
                    return Err(e);
                }
            }
        }

        current_sector += to_read as u64;
        remaining -= to_read as u64;
    }

    if had_errors {
        tracing::info!("Read completed with {} retries", total_retries);
    }

    Ok(all_data)
}

/// Verify data integrity using SHA-256
pub fn verify_checksum(data: &[u8], expected: &str) -> bool {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = format!("{:x}", hasher.finalize());
    result == expected
}

/// Calculate SHA-256 checksum of data
pub fn calculate_checksum(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config_default() {
        let config = ReadConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
    }

    #[test]
    fn test_calculate_checksum() {
        let data = b"hello world";
        let checksum = calculate_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA-256 hex string
    }

    #[test]
    fn test_verify_checksum_valid() {
        let data = b"hello world";
        let checksum = calculate_checksum(data);
        assert!(verify_checksum(data, &checksum));
    }

    #[test]
    fn test_verify_checksum_invalid() {
        let data = b"hello world";
        assert!(!verify_checksum(data, "invalid"));
    }
}
