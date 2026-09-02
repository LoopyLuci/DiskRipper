use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::error::DiskRipperError;
use crate::filesystem::{detect_filesystem, FilesystemReader, FilesystemType, Iso9660Reader, UdfReader};
use crate::job::{JobManager, JobStatus};
use crate::progress::ProgressTracker;
use crate::types::*;

pub struct Extractor {
    job_manager: Arc<JobManager>,
    job_id: JobId,
    source_path: String,
    output_path: std::path::PathBuf,
    options: ExtractOptions,
}

impl Extractor {
    pub fn new(
        job_manager: Arc<JobManager>,
        job_id: JobId,
        source_path: String,
        output_path: std::path::PathBuf,
        options: ExtractOptions,
    ) -> Self {
        Self {
            job_manager,
            job_id,
            source_path,
            output_path,
            options,
        }
    }

    pub async fn run(&self) -> Result<(), DiskRipperError> {
        info!(job_id = %self.job_id, "Starting extraction");

        let source = Path::new(&self.source_path);
        
        // Check if source is a drive path (e.g., "D:\")
        let is_drive = Self::is_drive_path(&self.source_path);
        
        if is_drive {
            // For drive paths, read directly from the mounted filesystem
            info!(job_id = %self.job_id, "Source is a drive, reading from mounted filesystem");
            self.extract_from_mounted_drive(source).await?;
        } else if source.exists() {
            // For file paths, read the file and parse as ISO/UDF
            let data = tokio::fs::read(source).await?;
            let fs_type = detect_filesystem(&data);

            match fs_type {
                FilesystemType::Iso9660 | FilesystemType::Joliet => {
                    self.extract_iso9660(data).await?;
                }
                FilesystemType::Udf => {
                    self.extract_udf(data).await?;
                }
                FilesystemType::Unknown => {
                    return Err(DiskRipperError::UnsupportedDisc(
                        "Unknown filesystem on disc image".to_string(),
                    ));
                }
            }
        } else {
            return Err(DiskRipperError::InvalidPath(self.source_path.clone()));
        };

        let _ = self.job_manager.set_status(&self.job_id, JobStatus::Completed);
        info!(job_id = %self.job_id, "Extraction complete");
        Ok(())
    }

    /// Check if path looks like an optical drive
    fn is_drive_path(path: &str) -> bool {
        let path = path.trim_end_matches('\\');
        (path.len() == 2 && path.ends_with(':')) || path.len() == 1
    }

    /// Extract files from a mounted drive (e.g., D:\)
    async fn extract_from_mounted_drive(&self, drive_path: &Path) -> Result<(), DiskRipperError> {
        // Scan directory synchronously (it's fast for mounted drives)
        let mut entries = Vec::new();
        Self::scan_directory_sync(drive_path, "", &mut entries)?;
        
        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, entries.len() as u64);

        for entry in &entries {
            if entry.is_dir {
                continue;
            }

            let dest = self.output_path.join(&entry.path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }

            if dest.exists() && !self.options.overwrite_existing {
                warn!(path = %dest.display(), "Skipping existing file");
                continue;
            }

            // Copy file from drive
            if let Err(e) = fs::copy(&entry.full_path, &dest).await {
                warn!(path = %entry.full_path.display(), error = %e, "Failed to copy file");
                continue;
            }

            tracker.add_bytes(entry.size);

            if self.options.preserve_timestamps {
                if let Some(modified) = entry.modified {
                    let _ = filetime::set_file_mtime(
                        &dest,
                        filetime::FileTime::from_system_time(modified.into()),
                    );
                }
            }

            if tracker.should_update(100) {
                let snapshot = tracker.snapshot();
                let _ = self.job_manager.update_progress(&self.job_id, snapshot);
            }
        }

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);

        Ok(())
    }

    /// Recursively scan a directory (synchronous version)
    fn scan_directory_sync(
        dir: &Path,
        prefix: &str,
        entries: &mut Vec<DirEntry>,
    ) -> Result<(), DiskRipperError> {
        for entry in std::fs::read_dir(dir).map_err(|e| DiskRipperError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| DiskRipperError::Io(e.to_string()))?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|e| DiskRipperError::Io(e.to_string()))?;
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            
            let relative_path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };

            if metadata.is_dir() {
                entries.push(DirEntry {
                    path: relative_path.clone(),
                    full_path: path.clone(),
                    size: 0,
                    is_dir: true,
                    modified: None,
                });
                Self::scan_directory_sync(&path, &relative_path, entries)?;
            } else {
                let modified = metadata.modified().ok().map(|t| {
                    chrono::DateTime::from(t)
                });
                entries.push(DirEntry {
                    path: relative_path,
                    full_path: path,
                    size: metadata.len(),
                    is_dir: false,
                    modified,
                });
            }
        }

        Ok(())
    }

    /// Extract files from ISO 9660 image
    async fn extract_iso9660(&self, data: Vec<u8>) -> Result<(), DiskRipperError> {
        let mut reader = Iso9660Reader::new(data)?;
        let files = reader.list_files()?;
        
        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, files.len() as u64);

        for file in &files {
            if file.is_dir {
                continue;
            }

            let relative = file.path.trim_start_matches('/');
            let dest = self.output_path.join(relative);

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }

            if dest.exists() && !self.options.overwrite_existing {
                warn!(path = %dest.display(), "Skipping existing file");
                continue;
            }

            if let Err(e) = reader.read_file(file, &dest) {
                warn!(path = %file.path, error = %e, "Failed to extract file");
                continue;
            }

            tracker.add_bytes(file.size);

            if self.options.preserve_timestamps {
                if let Some(modified) = file.modified {
                    let _ = filetime::set_file_mtime(
                        &dest,
                        filetime::FileTime::from_system_time(modified.into()),
                    );
                }
            }

            if tracker.should_update(100) {
                let snapshot = tracker.snapshot();
                let _ = self.job_manager.update_progress(&self.job_id, snapshot);
            }
        }

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);

        Ok(())
    }

    /// Extract files from UDF image
    async fn extract_udf(&self, data: Vec<u8>) -> Result<(), DiskRipperError> {
        let mut reader = UdfReader::new(data)?;
        let files = reader.list_files()?;
        
        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, files.len() as u64);

        for file in &files {
            if file.is_dir {
                continue;
            }

            let relative = file.path.trim_start_matches('/');
            let dest = self.output_path.join(relative);

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }

            if dest.exists() && !self.options.overwrite_existing {
                warn!(path = %dest.display(), "Skipping existing file");
                continue;
            }

            if let Err(e) = reader.read_file(file, &dest) {
                warn!(path = %file.path, error = %e, "Failed to extract file");
                continue;
            }

            tracker.add_bytes(file.size);

            if self.options.preserve_timestamps {
                if let Some(modified) = file.modified {
                    let _ = filetime::set_file_mtime(
                        &dest,
                        filetime::FileTime::from_system_time(modified.into()),
                    );
                }
            }

            if tracker.should_update(100) {
                let snapshot = tracker.snapshot();
                let _ = self.job_manager.update_progress(&self.job_id, snapshot);
            }
        }

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);

        Ok(())
    }
}

/// Directory entry for mounted drive scanning
struct DirEntry {
    path: String,
    full_path: std::path::PathBuf,
    size: u64,
    is_dir: bool,
    modified: Option<chrono::DateTime<chrono::Utc>>,
}
