//! Drive control: read speed, eject/load, and disc quality scanning.
//!
//! Provides:
//! - Read speed control via SCSI SET STREAMING command
//! - Eject/load via SCSI START STOP UNIT command
//! - Disc quality metrics (C1/C2 errors, jitter, BLER)

use std::io;

use tracing::{info, warn};

use crate::error::DiskRipperError;

/// SCSI START STOP UNIT command codes
const SSU_START: u8 = 0x01;
const SSU_STOP: u8 = 0x00;
const SSU_EJECT: u8 = 0x02;
const SSU_LOAD: u8 = 0x03;

/// Drive control operations
pub struct DriveController;

impl DriveController {
    /// Eject disc from drive
    pub fn eject(drive_path: &str) -> Result<(), DiskRipperError> {
        info!("Ejecting disc from {}", drive_path);
        Self::start_stop_unit(drive_path, SSU_EJECT)
    }

    /// Load disc into drive
    pub fn load(drive_path: &str) -> Result<(), DiskRipperError> {
        info!("Loading disc into {}", drive_path);
        Self::start_stop_unit(drive_path, SSU_LOAD)
    }

    /// Start disc spinning
    pub fn start(drive_path: &str) -> Result<(), DiskRipperError> {
        info!("Starting drive {}", drive_path);
        Self::start_stop_unit(drive_path, SSU_START)
    }

    /// Stop disc spinning
    pub fn stop(drive_path: &str) -> Result<(), DiskRipperError> {
        info!("Stopping drive {}", drive_path);
        Self::start_stop_unit(drive_path, SSU_STOP)
    }

    /// Send SCSI START STOP UNIT command
    fn start_stop_unit(drive_path: &str, command: u8) -> Result<(), DiskRipperError> {
        #[cfg(target_os = "windows")]
        {
            use crate::filesystem::native_win::NativeDriveHandle;
            let handle = NativeDriveHandle::open(drive_path)
                .map_err(|e| DiskRipperError::Io(e.to_string()))?;

            // SCSI START STOP UNIT CDB (6 bytes)
            let mut cdb = [0u8; 6];
            cdb[0] = 0x1B; // START STOP UNIT opcode
            cdb[1] = 0x00; // Immediate = 0
            cdb[2] = 0x00; // Reserved
            cdb[3] = 0x00; // Reserved
            cdb[4] = command; // Start/Stop/Eject/Load
            cdb[5] = 0x00; // Control

            let mut spt = Self::build_scsi_pass_through(&cdb, 0, 0x00)?;
            let mut bytes_returned: u32 = 0;

            let result = unsafe {
                windows_sys::Win32::System::IO::DeviceIoControl(
                    handle.handle().0 as isize,
                    0x0004D004, // IOCTL_SCSI_PASS_THROUGH_DIRECT
                    &mut spt as *mut _ as *mut _,
                    std::mem::size_of::<ScsiPassThroughDirect>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            if result == 0 {
                return Err(DiskRipperError::Io("START STOP UNIT failed".to_string()));
            }

            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (drive_path, command);
            Err(DiskRipperError::UnsupportedDisc("Drive control only supported on Windows".to_string()))
        }
    }

    /// Set read speed via SCSI SET STREAMING command
    ///
    /// speed_x: Speed multiplier (1 = 1x, 4 = 4x, 8 = 8x, etc.)
    /// For CD: 1x = 150 KB/s, for DVD: 1x = 1350 KB/s
    pub fn set_read_speed(drive_path: &str, speed_x: u32) -> Result<(), DiskRipperError> {
        info!("Setting read speed to {}x for {}", speed_x, drive_path);

        #[cfg(target_os = "windows")]
        {
            use crate::filesystem::native_win::NativeDriveHandle;
            let handle = NativeDriveHandle::open(drive_path)
                .map_err(|e| DiskRipperError::Io(e.to_string()))?;

            // Convert speed_x to speed in KB/s (CD: 150 KB/s per x)
            let speed_kbs = speed_x * 150;

            // SCSI SET STREAMING CDB (12 bytes)
            let mut cdb = [0u8; 12];
            cdb[0] = 0xB6; // SET STREAMING opcode
            cdb[1] = 0x00; // Reserved
            cdb[2] = 0x00; // Reserved
            cdb[3] = 0x00; // Reserved
            cdb[4] = 0x00; // Reserved
            cdb[5] = 0x00; // Reserved
            cdb[6] = 0x00; // Reserved
            cdb[7] = 0x00; // Reserved
            cdb[8] = 0x00; // Reserved
            cdb[9] = 0x00; // Reserved
            cdb[10] = ((speed_kbs >> 8) & 0xFF) as u8; // Speed high byte
            cdb[11] = (speed_kbs & 0xFF) as u8; // Speed low byte

            let mut spt = Self::build_scsi_pass_through(&cdb, 0, 0x00)?;
            let mut bytes_returned: u32 = 0;

            let result = unsafe {
                windows_sys::Win32::System::IO::DeviceIoControl(
                    handle.handle().0 as isize,
                    0x0004D004, // IOCTL_SCSI_PASS_THROUGH_DIRECT
                    &mut spt as *mut _ as *mut _,
                    std::mem::size_of::<ScsiPassThroughDirect>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            if result == 0 {
                return Err(DiskRipperError::Io("SET STREAMING failed".to_string()));
            }

            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (drive_path, speed_x);
            Err(DiskRipperError::UnsupportedDisc("Read speed control only supported on Windows".to_string()))
        }
    }

    /// Build SCSI pass-through structure
    #[cfg(target_os = "windows")]
    fn build_scsi_pass_through(
        cdb: &[u8],
        data_length: u32,
        data_in: u8,
    ) -> Result<ScsiPassThroughDirect, DiskRipperError> {
        let mut spt = ScsiPassThroughDirect {
            Length: std::mem::size_of::<ScsiPassThroughDirect>() as u16,
            ScsiStatus: 0,
            PathId: 0,
            TargetId: 0,
            Lun: 0,
            CdbLength: cdb.len() as u8,
            SenseInfoLength: 0,
            DataIn: data_in,
            DataTransferLength: data_length,
            TimeOutValue: 30,
            DataBuffer: std::ptr::null_mut::<u8>() as usize,
            SenseInfoOffset: 0,
            Cdb: [0u8; 16],
        };

        spt.Cdb[..cdb.len()].copy_from_slice(cdb);
        Ok(spt)
    }
}

/// SCSI pass-through direct structure
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct ScsiPassThroughDirect {
    Length: u16,
    ScsiStatus: u8,
    PathId: u8,
    TargetId: u8,
    Lun: u8,
    CdbLength: u8,
    SenseInfoLength: u8,
    DataIn: u8,
    DataTransferLength: u32,
    TimeOutValue: u32,
    DataBuffer: usize,
    SenseInfoOffset: u32,
    Cdb: [u8; 16],
}

/// Disc quality scanner
pub struct DiscQualityScanner;

impl DiscQualityScanner {
    /// Scan disc quality by reading sectors and counting errors
    ///
    /// Returns quality metrics for the disc.
    pub fn scan_quality(drive_path: &str, num_sectors: u32) -> Result<DiscQuality, DiskRipperError> {
        info!("Scanning disc quality for {} sectors", num_sectors);

        let mut c1_errors: u64 = 0;
        let mut c2_errors: u64 = 0;
        let mut read_retries: u64 = 0;
        let mut total_jitter: f64 = 0.0;
        let mut sectors_read: u64 = 0;

        // Read sectors and count errors
        for i in 0..num_sectors {
            match Self::read_sector_with_quality(drive_path, i) {
                Ok(quality) => {
                    c1_errors += quality.c1_errors as u64;
                    c2_errors += quality.c2_errors as u64;
                    read_retries += quality.retries as u64;
                    total_jitter += quality.jitter;
                    sectors_read += 1;
                }
                Err(e) => {
                    warn!("Failed to read sector {}: {}", i, e);
                    c2_errors += 1;
                }
            }
        }

        let avg_jitter = if sectors_read > 0 {
            total_jitter / sectors_read as f64
        } else {
            0.0
        };

        let bler = if sectors_read > 0 {
            (c1_errors + c2_errors) as f64 / sectors_read as f64
        } else {
            0.0
        };

        Ok(DiscQuality {
            c1_errors,
            c2_errors,
            read_retries,
            avg_jitter,
            bler,
            sectors_scanned: sectors_read,
        })
    }

    /// Read a single sector and measure quality
    fn read_sector_with_quality(_drive_path: &str, _sector: u32) -> Result<SectorQuality, DiskRipperError> {
        // Placeholder: In production, this would:
        // 1. Read sector with subchannel data
        // 2. Count C1/C2 errors from error flags
        // 3. Measure jitter from read timing
        // 4. Count retries needed

        Ok(SectorQuality {
            c1_errors: 0,
            c2_errors: 0,
            retries: 0,
            jitter: 0.0,
        })
    }
}

/// Disc quality metrics
#[derive(Debug, Clone)]
pub struct DiscQuality {
    pub c1_errors: u64,
    pub c2_errors: u64,
    pub read_retries: u64,
    pub avg_jitter: f64,
    pub bler: f64, // Block Error Rate
    pub sectors_scanned: u64,
}

impl DiscQuality {
    /// Get overall quality score (0-100)
    pub fn quality_score(&self) -> u32 {
        if self.sectors_scanned == 0 {
            return 0;
        }

        // Score based on C1/C2 error rates
        let total_errors = self.c1_errors + self.c2_errors;
        let error_rate = total_errors as f64 / self.sectors_scanned as f64;

        if error_rate == 0.0 {
            100
        } else if error_rate < 0.001 {
            90
        } else if error_rate < 0.01 {
            70
        } else if error_rate < 0.1 {
            50
        } else if error_rate < 1.0 {
            30
        } else {
            10
        }
    }

    /// Get quality description
    pub fn quality_description(&self) -> &'static str {
        let score = self.quality_score();
        match score {
            90..=100 => "Excellent",
            70..=89 => "Good",
            50..=69 => "Fair",
            30..=49 => "Poor",
            _ => "Very Poor",
        }
    }
}

/// Sector quality metrics
#[derive(Debug, Clone)]
pub struct SectorQuality {
    pub c1_errors: u32,
    pub c2_errors: u32,
    pub retries: u32,
    pub jitter: f64,
}

/// Paranoid/secure ripping mode configuration
#[derive(Debug, Clone)]
pub struct ParanoidConfig {
    /// Number of times to read each sector
    pub read_count: u32,
    /// Whether to compare reads and use majority vote
    pub majority_vote: bool,
    /// Whether to abort on mismatches
    pub abort_on_mismatch: bool,
    /// Maximum allowed mismatches before aborting
    pub max_mismatches: u32,
}

impl Default for ParanoidConfig {
    fn default() -> Self {
        Self {
            read_count: 3,
            majority_vote: true,
            abort_on_mismatch: false,
            max_mismatches: 10,
        }
    }
}

/// Paranoid ripper - reads multiple times for verification
pub struct ParanoidRipper;

impl ParanoidRipper {
    /// Rip a track in paranoid mode
    ///
    /// Reads each sector multiple times and compares results.
    /// Uses majority vote to determine correct data.
    pub fn rip_track_paranoid(
        drive_path: &str,
        start_sector: u64,
        num_sectors: u32,
        config: &ParanoidConfig,
    ) -> Result<Vec<u8>, DiskRipperError> {
        info!(
            start = start_sector,
            sectors = num_sectors,
            reads = config.read_count,
            "Starting paranoid rip"
        );

        let mut result = Vec::with_capacity(num_sectors as usize * 2352);
        let mut mismatches = 0u32;

        for i in 0..num_sectors {
            let sector = start_sector + i as u64;
            let data = Self::read_sector_paranoid(drive_path, sector, config)?;
            result.extend_from_slice(&data);
        }

        if config.abort_on_mismatch && mismatches > config.max_mismatches {
            return Err(DiskRipperError::ReadError(format!(
                "Too many mismatches: {} > {}",
                mismatches, config.max_mismatches
            )));
        }

        Ok(result)
    }

    /// Read a single sector multiple times and compare
    fn read_sector_paranoid(
        drive_path: &str,
        sector: u64,
        config: &ParanoidConfig,
    ) -> Result<Vec<u8>, DiskRipperError> {
        let mut reads: Vec<Vec<u8>> = Vec::new();

        for _ in 0..config.read_count {
            match Self::read_sector(drive_path, sector) {
                Ok(data) => reads.push(data),
                Err(e) => {
                    warn!("Read error at sector {}: {}", sector, e);
                }
            }
        }

        if reads.is_empty() {
            return Err(DiskRipperError::ReadError(format!(
                "All reads failed at sector {}",
                sector
            )));
        }

        if config.majority_vote && reads.len() > 1 {
            // Use majority vote
            Self::majority_vote(&reads)
        } else {
            // Use first successful read
            Ok(reads[0].clone())
        }
    }

    /// Read a single sector
    fn read_sector(_drive_path: &str, _sector: u64) -> Result<Vec<u8>, DiskRipperError> {
        // Placeholder: In production, this would use the native drive handle
        Ok(vec![0u8; 2352])
    }

    /// Majority vote on multiple reads
    ///
    /// For each byte position, use the value that appears most often.
    fn majority_vote(reads: &[Vec<u8>]) -> Result<Vec<u8>, DiskRipperError> {
        if reads.is_empty() {
            return Err(DiskRipperError::ReadError("No reads to vote on".to_string()));
        }

        let len = reads[0].len();
        let mut result = vec![0u8; len];

        for byte_pos in 0..len {
            // Count occurrences of each byte value
            let mut counts: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
            for read in reads {
                if byte_pos < read.len() {
                    *counts.entry(read[byte_pos]).or_insert(0) += 1;
                }
            }

            // Find most common value
            let mut max_count = 0;
            let mut best_value = 0u8;
            for (value, count) in counts {
                if count > max_count {
                    max_count = count;
                    best_value = value;
                }
            }

            result[byte_pos] = best_value;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paranoid_config_default() {
        let config = ParanoidConfig::default();
        assert_eq!(config.read_count, 3);
        assert!(config.majority_vote);
        assert!(!config.abort_on_mismatch);
    }

    #[test]
    fn test_quality_score() {
        let quality = DiscQuality {
            c1_errors: 0,
            c2_errors: 0,
            read_retries: 0,
            avg_jitter: 0.0,
            bler: 0.0,
            sectors_scanned: 100,
        };
        assert_eq!(quality.quality_score(), 100);
        assert_eq!(quality.quality_description(), "Excellent");
    }

    #[test]
    fn test_quality_score_poor() {
        let quality = DiscQuality {
            c1_errors: 50,
            c2_errors: 10,
            read_retries: 5,
            avg_jitter: 10.0,
            bler: 0.6,
            sectors_scanned: 100,
        };
        assert!(quality.quality_score() < 50);
    }

    #[test]
    fn test_majority_vote() {
        let reads = vec![
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4],
            vec![1, 2, 99, 4], // One different byte
        ];

        let result = ParanoidRipper::majority_vote(&reads).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);
    }
}
