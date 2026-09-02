use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tracing::{info, error};

use crate::error::DiskRipperError;
use crate::filesystem::reader::{read_raw_sectors, read_raw_cdda};
use crate::job::{JobManager, JobStatus};
use crate::progress::ProgressTracker;
use crate::types::*;

const BUFFER_SIZE: usize = 8 * 1024 * 1024;

pub struct Imager {
    job_manager: Arc<JobManager>,
    job_id: JobId,
    source_path: String,
    output_path: std::path::PathBuf,
    total_size: u64,
    options: ImageOptions,
}

impl Imager {
    pub fn new(
        job_manager: Arc<JobManager>,
        job_id: JobId,
        source_path: String,
        output_path: std::path::PathBuf,
        total_size: u64,
        options: ImageOptions,
    ) -> Self {
        Self { job_manager, job_id, source_path, output_path, total_size, options }
    }

    pub async fn run(&self) -> Result<(), DiskRipperError> {
        info!(job_id = %self.job_id, "Starting disc imaging");

        let source = Path::new(&self.source_path);
        if !source.exists() && !Self::is_drive_path(&self.source_path) {
            return Err(DiskRipperError::InvalidPath(self.source_path.clone()));
        }

        let tracker = ProgressTracker::new(self.job_id.clone(), self.total_size, 1);

        if let Some(parent) = self.output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if Self::is_drive_path(&self.source_path) {
            self.stream_drive_to_disk(&tracker).await?;
        } else {
            self.stream_file_to_disk(source, &tracker).await?;
        }

        let meta_path = self.output_path.with_extension("json");
        let meta = serde_json::json!({
            "source": self.source_path,
            "size": self.total_size,
            "format": self.options.format.to_string(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(&meta_path, meta.to_string()).await?;

        let mut final_snapshot = tracker.snapshot();
        final_snapshot.phase = Phase::Complete;
        let _ = self.job_manager.update_progress(&self.job_id, final_snapshot);
        let _ = self.job_manager.set_status(&self.job_id, JobStatus::Completed);

        info!(job_id = %self.job_id, "Imaging complete");
        Ok(())
    }

    fn is_drive_path(path: &str) -> bool {
        #[cfg(target_os = "windows")]
        { path.len() <= 3 && path.ends_with(':') || path.ends_with('\\') || path.ends_with('/') }
        #[cfg(not(target_os = "windows"))]
        { path.starts_with("/dev/sr") || path.starts_with("/dev/cdrom") || path.starts_with("/dev/disk") }
    }

    async fn stream_drive_to_disk(&self, tracker: &ProgressTracker) -> Result<(), DiskRipperError> {
        let mut writer = fs::File::create(&self.output_path).await?;
        let mut current_sector = 0u32;
        let mut consecutive_errors = 0u32;
        let max_errors = 10;
        let batch_size = 50u32; // Read 50 CDDA sectors at a time

        loop {
            if self.total_size > 0 && (current_sector as u64 * 2352) >= self.total_size {
                break;
            }

            // Try CDDA read first (works for both audio and data CDs when mounted)
            match read_raw_cdda(&self.source_path, current_sector as u64, batch_size) {
                Ok(data) => {
                    if data.is_empty() { 
                        // Fall back to standard sector read
                        match read_raw_sectors(&self.source_path, current_sector as u64, batch_size, 2048) {
                            Ok(sector_data) => {
                                if sector_data.is_empty() { break; }
                                writer.write_all(&sector_data).await?;
                                tracker.add_bytes(sector_data.len() as u64);
                                consecutive_errors = 0;
                                current_sector += batch_size;
                            }
                            Err(_) => break,
                        }
                    } else {
                        writer.write_all(&data).await?;
                        tracker.add_bytes(data.len() as u64);
                        consecutive_errors = 0;
                        current_sector += batch_size;
                    }
                }
                Err(e) => {
                    error!("Failed to read CDDA at sector {}: {}", current_sector, e);
                    // Try standard read
                    match read_raw_sectors(&self.source_path, current_sector as u64, batch_size, 2048) {
                        Ok(sector_data) => {
                            if sector_data.is_empty() { break; }
                            writer.write_all(&sector_data).await?;
                            tracker.add_bytes(sector_data.len() as u64);
                            consecutive_errors = 0;
                            current_sector += batch_size;
                        }
                        Err(e2) => {
                            error!("Failed to read sectors at {}: {}", current_sector, e2);
                            consecutive_errors += 1;
                            if consecutive_errors >= max_errors { break; }
                            current_sector += batch_size;
                        }
                    }
                }
            }

            // Safety limit: stop at 50GB
            if current_sector as u64 * 2352 > 50_000_000_000 { break; }
        }

        writer.flush().await?;
        Ok(())
    }

    async fn stream_file_to_disk(&self, source: &Path, tracker: &ProgressTracker) -> Result<(), DiskRipperError> {
        let mut reader = fs::File::open(source).await?;
        let mut writer = fs::File::create(&self.output_path).await?;
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_read = 0u64;

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 { break; }
            writer.write_all(&buffer[..n]).await?;
            total_read += n as u64;
            tracker.add_bytes(n as u64);
        }

        writer.flush().await?;
        info!("Streamed {} bytes from file", total_read);
        Ok(())
    }
}
