use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JobId(pub String);

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveId(pub String);

impl DriveId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for DriveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaId(pub String);

impl MediaId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPath(pub std::path::PathBuf);

impl OutputPath {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self(path.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipOptions {
    pub verify_checksums: bool,
    pub eject_after_rip: bool,
    pub read_speed: Option<u32>,
    pub retries: u32,
    pub buffer_size: usize,
}

impl Default for RipOptions {
    fn default() -> Self {
        Self {
            verify_checksums: true,
            eject_after_rip: false,
            read_speed: None,
            retries: 3,
            buffer_size: 4 * 1024 * 1024, // 4MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub preserve_timestamps: bool,
    pub preserve_permissions: bool,
    pub overwrite_existing: bool,
    pub extract_path: Option<std::path::PathBuf>,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            preserve_timestamps: true,
            preserve_permissions: true,
            overwrite_existing: false,
            extract_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOptions {
    pub format: ImageFormat,
    pub compression: CompressionLevel,
    pub split_size: Option<u64>,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            format: ImageFormat::Iso,
            compression: CompressionLevel::None,
            split_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageFormat {
    Iso,
    BinCue,
    Img,
    Dmg,
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormat::Iso => write!(f, "ISO"),
            ImageFormat::BinCue => write!(f, "BIN/CUE"),
            ImageFormat::Img => write!(f, "IMG"),
            ImageFormat::Dmg => write!(f, "DMG"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionLevel {
    None,
    Fast,
    Default,
    Best,
}

impl std::fmt::Display for CompressionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionLevel::None => write!(f, "None"),
            CompressionLevel::Fast => write!(f, "Fast"),
            CompressionLevel::Default => write!(f, "Default"),
            CompressionLevel::Best => write!(f, "Best"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub job_id: JobId,
    pub phase: Phase,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub files_processed: u64,
    pub files_total: u64,
    pub speed_bytes_per_sec: f64,
    pub eta_seconds: Option<u64>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProgressInfo {
    pub fn percent(&self) -> f64 {
        if self.bytes_total == 0 {
            // When total is unknown, show bytes processed as rough progress
            // Cap at 99% so it doesn't falsely show completion
            let mb = self.bytes_processed as f64 / 1_000_000.0;
            if mb > 0.0 && mb < 99.0 { mb } else { 99.0 }
        } else {
            (self.bytes_processed as f64 / self.bytes_total as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Detecting,
    Analyzing,
    Reading,
    Extracting,
    Imaging,
    Verifying,
    Finalizing,
    Complete,
    Error,
    Cancelled,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Idle => write!(f, "Idle"),
            Phase::Detecting => write!(f, "Detecting"),
            Phase::Analyzing => write!(f, "Analyzing"),
            Phase::Reading => write!(f, "Reading"),
            Phase::Extracting => write!(f, "Extracting"),
            Phase::Imaging => write!(f, "Imaging"),
            Phase::Verifying => write!(f, "Verifying"),
            Phase::Finalizing => write!(f, "Finalizing"),
            Phase::Complete => write!(f, "Complete"),
            Phase::Error => write!(f, "Error"),
            Phase::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<DateTime<Utc>>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscContent {
    pub files: Vec<FileEntry>,
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub media_types: Vec<super::media::MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumResult {
    pub file_path: String,
    pub expected: String,
    pub actual: String,
    pub valid: bool,
}
