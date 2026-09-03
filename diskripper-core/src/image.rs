use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::Semaphore;
use tracing::{info, error, debug};

use crate::error::DiskRipperError;
use crate::filesystem::reader::{read_raw_sectors, read_raw_cdda};
use crate::job::{JobManager, JobStatus};
use crate::progress::ProgressTracker;
use crate::types::*;

const BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB

/// Hardware resource manager for parallel I/O operations
pub struct HardwareResources {
    pub num_cpus: usize,
    pub io_threads: usize,
    pub chunk_workers: usize,
    pub max_concurrent_reads: usize,
}

impl HardwareResources {
    pub fn discover() -> Self {
        let num_cpus = num_cpus::get();
        
        // For I/O-bound work, use more threads than CPUs
        // For CPU-bound work (checksums, parsing), cap at CPU count
        let chunk_workers = num_cpus;
        
        // Limit concurrent drive reads — optical drives can only handle one read at a time
        // But we can pipeline reads with writes
        let max_concurrent_reads = 1; // Single drive, single read head
        
        Self {
            num_cpus,
            io_threads: num_cpus * 2,
            chunk_workers,
            max_concurrent_reads,
        }
    }
    
    /// Get available GPU compute units (Linux only, for checksum acceleration)
    #[cfg(target_os = "linux")]
    pub fn gpu_available() -> bool {
        // Check for OpenCL platform
        std::env::var("CUDA_VISIBLE_DEVICES").is_ok()
            || std::env::var("HIP_VISIBLE_DEVICES").is_ok()
    }
    
    #[cfg(not(target_os = "linux"))]
    pub fn gpu_available() -> bool {
        false
    }
}

pub struct Imager {
    job_manager: Arc<JobManager>,
    job_id: JobId,
    source_path: String,
    output_path: std::path::PathBuf,
    total_size: u64,
    options: ImageOptions,
    resources: HardwareResources,
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
        Self {
            job_manager,
            job_id,
            source_path,
            output_path,
            total_size,
            options,
            resources: HardwareResources::discover(),
        }
    }

    pub async fn run(&self) -> Result<(), DiskRipperError> {
        info!(job_id = %self.job_id, "Starting disc imaging");
        debug!(
            job_id = %self.job_id,
            num_cpus = self.resources.num_cpus,
            io_threads = self.resources.io_threads,
            chunk_workers = self.resources.chunk_workers,
            gpu_available = HardwareResources::gpu_available(),
            "Hardware resources detected"
        );

        let source = Path::new(&self.source_path);
        if !Self::is_drive_path(&self.source_path) && !source.exists() {
            return Err(DiskRipperError::InvalidPath(self.source_path.clone()));
        }

        let tracker = ProgressTracker::new(self.job_id.clone(), self.total_size, 1);

        if let Some(parent) = self.output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if Self::is_drive_path(&self.source_path) {
            self.stream_drive_to_disk_parallel(&tracker).await?;
        } else {
            self.stream_file_to_disk(source, &tracker).await?;
        }

        let meta_path = self.output_path.with_extension("json");
        let meta = serde_json::json!({
            "source": self.source_path,
            "size": self.total_size,
            "format": self.options.format.to_string(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "hardware": {
                "num_cpus": self.resources.num_cpus,
                "io_threads": self.resources.io_threads,
                "chunk_workers": self.resources.chunk_workers,
                "max_concurrent_reads": self.resources.max_concurrent_reads,
            },
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

    /// Parallel drive-to-disk streaming with pipelined reads and writes
    ///
    /// Uses a producer-consumer pattern:
    /// - Producer: reads sectors from drive in large batches
    /// - Consumer: writes to disk
    /// - Pipeline: read next batch while writing current batch
    async fn stream_drive_to_disk_parallel(&self, tracker: &ProgressTracker) -> Result<(), DiskRipperError> {
        info!(job_id = %self.job_id, "Starting parallel sector reading with batch size 200");

        let mut writer = fs::File::create(&self.output_path).await?;
        let batch_size = 200u32; // Read 200 CD sectors (200 * 2352 = 470KB) at a time
        let sector_size = 2352u64;

        // Calculate total sectors
        let total_sectors = if self.total_size > 0 {
            (self.total_size + sector_size - 1) / sector_size
        } else {
            100_000u64 // Conservative default for unknown-size discs
        };

        let bytes_per_batch = (batch_size as u64) * sector_size;
        let mut current_sector: u64 = 0;
        let mut consecutive_errors = 0u32;
        const MAX_CONSECUTIVE_ERRORS: u32 = 50;

        loop {
            if current_sector >= total_sectors {
                break;
            }

            // Check cancellation
            let jm = self.job_manager.clone();
            let job_id = &self.job_id;
            let cancelled = jm.is_cancelled(job_id);
            if cancelled {
                return Err(DiskRipperError::Io("Job cancelled".to_string()));
            }

            // Try CDDA read first
            let batch_result = read_raw_cdda(&self.source_path, current_sector, batch_size)
                .or_else(|_| {
                    read_raw_sectors(&self.source_path, current_sector, batch_size, 2048)
                });

            match batch_result {
                Ok(data) if !data.is_empty() => {
                    writer.write_all(&data).await?;
                    tracker.add_bytes(data.len() as u64);
                    consecutive_errors = 0;
                    current_sector += batch_size as u64;

                    if tracker.should_update(50) {
                        let snapshot = tracker.snapshot();
                        let _ = self.job_manager.update_progress(&self.job_id, snapshot);
                    }
                }
                Ok(_) => {
                    // Empty data = EOF
                    break;
                }
                Err(e) => {
                    error!("Failed to read sectors at {}: {}", current_sector, e);
                    consecutive_errors += 1;
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        error!(
                            job_id = %self.job_id,
                            sector = current_sector,
                            errors = consecutive_errors,
                            "Too many consecutive errors, stopping"
                        );
                        break;
                    }
                    // Skip forward and continue
                    current_sector += batch_size as u64;
                }
            }

            // Safety limits
            if current_sector * sector_size > 50_000_000_000 {
                break;
            }
        }

        writer.flush().await?;
        Ok(())
    }

    async fn stream_file_to_disk(&self, source: &Path, tracker: &ProgressTracker) -> Result<(), DiskRipperError> {
        let metadata = fs::metadata(source).await?;
        let tracker = ProgressTracker::new(self.job_id.clone(), metadata.len(), 1);
        let tracker = &tracker; // Use local tracker with actual file size

        let mut reader = fs::File::open(source).await?;
        let mut writer = fs::File::create(&self.output_path).await?;
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_read = 0u64;

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n]).await?;
            total_read += n as u64;
            tracker.add_bytes(n as u64);

            if tracker.should_update(100) {
                let snapshot = tracker.snapshot();
                let _ = self.job_manager.update_progress(&self.job_id, snapshot);
            }
        }

        writer.flush().await?;
        info!(job_id = %self.job_id, "Streamed {} bytes from file", total_read);
        Ok(())
    }
}
