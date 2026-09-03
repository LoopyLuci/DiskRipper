pub mod iso9660;
pub mod udf;
pub mod raw_io;
pub mod reader;
pub mod recovery;
pub mod verify;

pub use iso9660::Iso9660Reader;
pub use udf::UdfReader;
pub use raw_io::{RawDiscReader, RawDiscError};
pub use recovery::{read_sectors_with_retry, read_large_range, verify_checksum, calculate_checksum, ReadConfig, ReadResult};
pub use verify::{verify_file_rip, verify_disc_image, verify_audio_accuracy, VerificationResult};

use crate::error::DiskRipperError;
use crate::types::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemType {
    Iso9660,
    Joliet,
    Udf,
    Hfs,
    Hybrid,
    Unknown,
}

pub trait FilesystemReader {
    fn read_volume(&mut self) -> Result<VolumeInfo, DiskRipperError>;
    fn read_directory(&mut self, path: &str) -> Result<Vec<FileEntry>, DiskRipperError>;
    fn read_file(&mut self, entry: &FileEntry, output_path: &std::path::Path) -> Result<(), DiskRipperError>;
    fn list_files(&mut self) -> Result<Vec<FileEntry>, DiskRipperError>;
}

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub volume_id: String,
    pub system_id: String,
    pub volume_size: u64,
    pub block_size: u32,
    pub files_used: u64,
    pub fs_type: FilesystemType,
}

pub fn detect_filesystem(data: &[u8]) -> FilesystemType {
    // ISO 9660 primary volume descriptor at sector 16 (LBA 16)
    // Offset 0x8000 = sector 16 * 2048 bytes
    if data.len() > 0x8010 {
        let sig = &data[0x8001..0x8006];
        if sig == b"CD001" {
            // Check for Joliet (at 0x8801)
            if data.len() > 0x8810 && &data[0x8801..0x8806] == b"CD001" {
                // Check Joliet escape sequence at 0x882D
                if data.len() > 0x8835 {
                    let escape = &data[0x882D..0x8835];
                    if escape.starts_with(&[0x25, 0x2F]) {
                        return FilesystemType::Joliet;
                    }
                }
            }
            // Check for HFS hybrid
            if data.len() > 0x438 && &data[0x400..0x402] == b"HX" {
                return FilesystemType::Hybrid;
            }
            return FilesystemType::Iso9660;
        }
        
        // UDF signature at sector 16 (NSR02/NSR03)
        if sig == b"NSR02" || sig == b"NSR03" {
            return FilesystemType::Udf;
        }
    }
    
    // HFS signature at sector 1 (offset 0x400) or sector 2 (Apple partition map)
    if data.len() > 0x438 {
        if &data[0x400..0x402] == b"HX" || &data[0x400..0x402] == b"HO" {
            return FilesystemType::Hfs;
        }
    }
    if data.len() > 0x200 && &data[0x200..0x202] == b"PM" {
        return FilesystemType::Hfs;
    }
    
    FilesystemType::Unknown
}
