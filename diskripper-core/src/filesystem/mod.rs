pub mod iso9660;
pub mod udf;
pub mod reader;
pub mod recovery;
pub mod verify;

pub use iso9660::Iso9660Reader;
pub use udf::UdfReader;
pub use recovery::{read_sectors_with_retry, read_large_range, verify_checksum, calculate_checksum, ReadConfig, ReadResult};
pub use verify::{verify_file_rip, verify_disc_image, verify_audio_accuracy, VerificationResult};

use crate::error::DiskRipperError;
use crate::types::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemType {
    Iso9660,
    Joliet,
    Udf,
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
    if data.len() > 0x8010 && &data[0x8001..0x8006] == b"CD001" {
        if data.len() > 0x8810 && &data[0x8801..0x8806] == b"CD001" {
            return FilesystemType::Joliet;
        }
        return FilesystemType::Iso9660;
    }
    if data.len() > 0x8010 && (&data[0x8001..0x8006] == b"NSR02" || &data[0x8001..0x8006] == b"NSR03") {
        return FilesystemType::Udf;
    }
    FilesystemType::Unknown
}
