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
        if !source.exists() {
            return Err(DiskRipperError::InvalidPath(self.source_path.clone()));
        }

        let data = tokio::fs::read(source).await?;
        let fs_type = detect_filesystem(&data);

        let files = match fs_type {
            FilesystemType::Iso9660 | FilesystemType::Joliet => {
                let mut reader = Iso9660Reader::new(data)?;
                reader.list_files()?
            }
            FilesystemType::Udf => {
                let mut reader = UdfReader::new(data)?;
                reader.list_files()?
            }
            FilesystemType::Unknown => {
                self.scan_files(source).await?
            }
        };

        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let total_files = files.len() as u64;

        let tracker = ProgressTracker::new(self.job_id.clone(), total_size, total_files);

        for file in &files {
            self.extract_file(source, file, &tracker).await?;
            tracker.add_file();

            if tracker.should_update(100) {
                let snapshot = tracker.snapshot();
                let _ = self.job_manager.update_progress(&self.job_id, snapshot);
            }
        }

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);
        let _ = self.job_manager.set_status(&self.job_id, JobStatus::Completed);

        info!(job_id = %self.job_id, "Extraction complete");
        Ok(())
    }

    async fn scan_files(&self, source: &Path) -> Result<Vec<FileEntry>, DiskRipperError> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(source).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                files.push(FileEntry {
                    path: path.to_string_lossy().to_string(),
                    size: 0,
                    is_dir: true,
                    modified: None,
                    checksum_sha256: None,
                });
            } else {
                files.push(FileEntry {
                    path: path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    is_dir: false,
                    modified: metadata.modified().ok().map(|t| {
                        chrono::DateTime::from(t)
                    }),
                    checksum_sha256: None,
                });
            }
        }

        Ok(files)
    }

    async fn extract_file(
        &self,
        source_root: &Path,
        file: &FileEntry,
        tracker: &ProgressTracker,
    ) -> Result<(), DiskRipperError> {
        if file.is_dir {
            return Ok(());
        }

        let source_path = Path::new(&file.path);
        let relative = match source_path.strip_prefix(source_root) {
            Ok(r) => r,
            Err(_) => {
                match source_path.file_name() {
                    Some(name) => Path::new(name),
                    None => {
                        warn!(path = %file.path, "Cannot determine relative path, skipping");
                        return Ok(());
                    }
                }
            }
        };

        let dest = self.output_path.join(relative);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        if dest.exists() && !self.options.overwrite_existing {
            warn!(path = %dest.display(), "Skipping existing file");
            return Ok(());
        }

        let data = fs::read(&file.path).await?;
        let mut out = fs::File::create(&dest).await?;
        out.write_all(&data).await?;
        out.flush().await?;

        tracker.add_bytes(file.size);

        if self.options.preserve_timestamps {
            if let Some(modified) = file.modified {
                let _ = filetime::set_file_mtime(
                    &dest,
                    filetime::FileTime::from_system_time(modified.into()),
                );
            }
        }

        Ok(())
    }
}
