use std::io;
use std::path::Path;
use tracing::debug;

use crate::error::DiskRipperError;

/// Raw sector reader using PowerShell helper (cross-platform safe).
/// The PowerShell approach spawns a process per batch, which is slower than native,
/// but works reliably on all Windows configurations.
/// All methods are synchronous and should be wrapped in `tokio::task::spawn_blocking`.
pub struct RawDiscReader {
    drive_path: String,
    sector_size: u32,
}

impl RawDiscReader {
    pub fn new<P: AsRef<Path>>(drive_path: P, sector_size: u32) -> Self {
        Self {
            drive_path: drive_path.as_ref().to_string_lossy().to_string(),
            sector_size,
        }
    }

    /// Read sectors synchronously (blocking)
    pub fn read_sectors(
        &self,
        start_sector: u64,
        num_sectors: u32,
    ) -> Result<Vec<u8>, DiskRipperError> {
        let result = crate::filesystem::reader::read_raw_sectors(
            &self.drive_path,
            start_sector,
            num_sectors,
            self.sector_size,
        );
        result.map_err(|e| DiskRipperError::Io(e.to_string()))
    }

    /// Read CDDA sectors synchronously (blocking)
    pub fn read_cdda(
        &self,
        start_sector: u64,
        num_sectors: u32,
    ) -> Result<Vec<u8>, DiskRipperError> {
        let result = crate::filesystem::reader::read_raw_cdda(&self.drive_path, start_sector, num_sectors);
        result.map_err(|e| DiskRipperError::Io(e.to_string()))
    }

    /// Get disc size synchronously (blocking)
    pub fn get_disc_size(&self) -> Result<u64, DiskRipperError> {
        let result = crate::filesystem::reader::get_disc_size(&self.drive_path);
        result.map_err(|e| DiskRipperError::Io(e.to_string()))
    }
}

/// Error type for raw disc operations
#[derive(Debug, thiserror::Error)]
pub enum RawDiscError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("Invalid sector range: start={start}, count={count}")]
    InvalidRange { start: u64, count: u32 },
    
    #[error("Disc ejected during operation")]
    DiscEjected,
    
    #[error("Bad sector at LBA {lba}")]
    BadSector { lba: u64 },
}

impl From<RawDiscError> for DiskRipperError {
    fn from(e: RawDiscError) -> Self {
        match e {
            RawDiscError::Io(io) => DiskRipperError::Io(io.to_string()),
            RawDiscError::Cancelled => DiskRipperError::Io("Operation cancelled".to_string()),
            RawDiscError::InvalidRange { start, count } => DiskRipperError::Io(format!(
                "Invalid sector range: start={}, count={}",
                start, count
            )),
            RawDiscError::DiscEjected => DiskRipperError::NoDisc("Disc ejected".to_string()),
            RawDiscError::BadSector { lba } => {
                DiskRipperError::ReadError(format!("Bad sector at LBA {}", lba))
            }
        }
    }
}
