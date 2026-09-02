use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DiskRipperError {
    #[error("Drive not found: {0}")]
    DriveNotFound(String),
    #[error("No disc in drive: {0}")]
    NoDisc(String),
    #[error("Disc read error: {0}")]
    ReadError(String),
    #[error("Disc type not supported: {0}")]
    UnsupportedDisc(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Job not found: {0}")]
    JobNotFound(String),
    #[error("Job already exists: {0}")]
    JobExists(String),
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
    #[error("Imaging failed: {0}")]
    ImagingFailed(String),
    #[error("Insufficient space: need {need} bytes, have {have} bytes")]
    InsufficientSpace { need: u64, have: u64 },
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("Platform error: {0}")]
    Platform(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for DiskRipperError {
    fn from(e: std::io::Error) -> Self {
        DiskRipperError::Io(e.to_string())
    }
}
