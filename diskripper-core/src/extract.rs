use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use rayon::prelude::*;
use tracing::{info, warn, debug};

use crate::error::DiskRipperError;
use crate::filesystem::{detect_filesystem, FilesystemReader, FilesystemType, Iso9660Reader, UdfReader};
use crate::job::{JobManager, JobStatus};
use crate::progress::ProgressTracker;
use crate::types::*;

/// Hardware resource information
struct HardwareInfo {
    num_cpus: usize,
}

impl HardwareInfo {
    fn discover() -> Self {
        Self { num_cpus: num_cpus::get() }
    }
}

pub struct Extractor {
    job_manager: Arc<JobManager>,
    job_id: JobId,
    source_path: String,
    output_path: std::path::PathBuf,
    options: ExtractOptions,
    hardware: HardwareInfo,
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
            hardware: HardwareInfo::discover(),
        }
    }

    pub async fn run(&self) -> Result<(), DiskRipperError> {
        info!(job_id = %self.job_id, "Starting extraction");
        debug!(
            job_id = %self.job_id,
            num_cpus = self.hardware.num_cpus,
            "Hardware resources for extraction"
        );

        let source = Path::new(&self.source_path);
        let is_drive = Self::is_drive_path(&self.source_path);

        if is_drive {
            info!(job_id = %self.job_id, "Source is a drive, reading from mounted filesystem");
            self.extract_from_mounted_drive(source).await?;
        } else if source.exists() {
            let data = tokio::fs::read(source).await?;
            let fs_type = detect_filesystem(&data);

            match fs_type {
                FilesystemType::Iso9660 | FilesystemType::Joliet => {
                    self.extract_iso9660(&data).await?;
                }
                FilesystemType::Udf => {
                    self.extract_udf(&data).await?;
                }
                FilesystemType::Hfs | FilesystemType::Hybrid => {
                    return Err(DiskRipperError::UnsupportedDisc(
                        "HFS/Hybrid extraction not yet supported".to_string(),
                    ));
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

    fn is_drive_path(path: &str) -> bool {
        let path = path.trim_end_matches('\\');
        (path.len() == 2 && path.ends_with(':')) || path.len() == 1
    }

    /// Extract files from a mounted drive using parallel file copying
    async fn extract_from_mounted_drive(&self, drive_path: &Path) -> Result<(), DiskRipperError> {
        let mut entries = Vec::new();
        Self::scan_directory_sync(drive_path, "", &mut entries)?;

        let file_entries: Vec<_> = entries.into_iter().filter(|e| !e.is_dir).collect();
        let total_size: u64 = file_entries.iter().map(|e| e.size).sum();
        let processed_bytes = Arc::new(AtomicU64::new(0));
        let processed_files = Arc::new(AtomicU64::new(0));

        let output_path = self.output_path.clone();
        let overwrite = self.options.overwrite_existing;
        let preserve_ts = self.options.preserve_timestamps;
        let pb = processed_bytes.clone();
        let pf = processed_files.clone();

        debug!(
            job_id = %self.job_id,
            file_count = file_entries.len(),
            total_size = total_size,
            "Starting parallel extraction with rayon"
        );

        // Spawn blocking for parallel file copy
        let result = tokio::task::spawn_blocking(move || {
            file_entries.par_iter().for_each(|entry| {
                let dest = output_path.join(&entry.path);

                if let Some(parent) = dest.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        warn!(error = %e, "Failed to create parent directory");
                        return;
                    }
                }

                if dest.exists() && !overwrite {
                    warn!(path = %dest.display(), "Skipping existing file");
                    return;
                }

                match std::fs::copy(&entry.full_path, &dest) {
                    Ok(bytes) => {
                        pb.fetch_add(bytes, Ordering::Relaxed);
                        pf.fetch_add(1, Ordering::Relaxed);

                        if preserve_ts {
                            if let Some(modified) = &entry.modified {
                                let _ = filetime::set_file_mtime(
                                    &dest,
                                    filetime::FileTime::from_unix_time(modified.timestamp(), 0),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(path = %entry.path, error = %e, "Failed to copy file");
                    }
                }
            });
        }).await;

        if let Err(e) = result {
            return Err(DiskRipperError::Io(format!("Join error: {}", e)));
        }

        let final_bytes = processed_bytes.load(Ordering::Relaxed);
        let final_files = processed_files.load(Ordering::Relaxed);
        info!(
            job_id = %self.job_id,
            files = final_files,
            bytes = final_bytes,
            total = total_size,
            "Parallel extraction complete"
        );

        Ok(())
    }

    /// Recursively scan a directory (synchronous)
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
                let modified = metadata.modified().ok().map(|t| chrono::DateTime::from(t));
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

    /// Extract files from ISO 9660 image (parallel)
    async fn extract_iso9660(&self, data: &[u8]) -> Result<(), DiskRipperError> {
        let reader = std::sync::Mutex::new(Iso9660Reader::new(data.to_vec())?);
        let files = reader.lock().unwrap().list_files()?;

        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, files.len() as u64);

        // Extract in parallel using rayon
        let results: Vec<(String, Result<(), DiskRipperError>)> = files
            .par_iter()
            .filter(|f| !f.is_dir)
            .map(|file| {
                let relative = file.path.trim_start_matches('/');
                let dest = self.output_path.join(relative);

                if let Some(parent) = dest.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return (file.path.clone(), Err(DiskRipperError::Io(e.to_string())));
                    }
                }

                if dest.exists() && !self.options.overwrite_existing {
                    return (file.path.clone(), Ok(()));
                }

                let result = reader.lock().unwrap().read_file(file, &dest)
                    .map_err(|e| DiskRipperError::Io(format!("Failed to extract {}: {}", file.path, e)));

                (file.path.clone(), result)
            })
            .collect();

        let mut errors = Vec::new();
        for (path, result) in &results {
            tracker.add_bytes(1);
            if let Err(e) = result {
                errors.push(format!("{}: {}", path, e));
            }
        }

        if !errors.is_empty() {
            warn!(errors = errors.len(), "Some files failed to extract");
        }

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);

        Ok(())
    }

    /// Extract files from UDF image (parallel)
    async fn extract_udf(&self, data: &[u8]) -> Result<(), DiskRipperError> {
        let reader = std::sync::Mutex::new(UdfReader::new(data.to_vec())?);
        let files = reader.lock().unwrap().list_files()?;

        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, files.len() as u64);

        // Extract in parallel using rayon
        let results: Vec<(String, Result<(), DiskRipperError>)> = files
            .par_iter()
            .filter(|f| !f.is_dir)
            .map(|file| {
                let relative = file.path.trim_start_matches('/');
                let dest = self.output_path.join(relative);

                if let Some(parent) = dest.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return (file.path.clone(), Err(DiskRipperError::Io(e.to_string())));
                    }
                }

                if dest.exists() && !self.options.overwrite_existing {
                    return (file.path.clone(), Ok(()));
                }

                let result = reader.lock().unwrap().read_file(file, &dest)
                    .map_err(|e| DiskRipperError::Io(format!("Failed to extract {}: {}", file.path, e)));

                (file.path.clone(), result)
            })
            .collect();

        let mut errors = Vec::new();
        for (path, result) in &results {
            tracker.add_bytes(1);
            if let Err(e) = result {
                errors.push(format!("{}: {}", path, e));
            }
        }

        if !errors.is_empty() {
            warn!(errors = errors.len(), "Some files failed to extract");
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
