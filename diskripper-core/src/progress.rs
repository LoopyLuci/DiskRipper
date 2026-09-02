use crate::types::*;


pub struct ProgressTracker {
    job_id: JobId,
    bytes_total: u64,
    files_total: u64,
    bytes_processed: std::sync::atomic::AtomicU64,
    files_processed: std::sync::atomic::AtomicU64,
    start_time: std::time::Instant,
    last_update: std::sync::Mutex<std::time::Instant>,
}

impl ProgressTracker {
    pub fn new(job_id: JobId, bytes_total: u64, files_total: u64) -> Self {
        Self {
            job_id,
            bytes_total,
            files_total,
            bytes_processed: std::sync::atomic::AtomicU64::new(0),
            files_processed: std::sync::atomic::AtomicU64::new(0),
            start_time: std::time::Instant::now(),
            last_update: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub fn add_bytes(&self, bytes: u64) {
        self.bytes_processed
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn add_file(&self) {
        self.files_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> ProgressInfo {
        let bytes_processed = self
            .bytes_processed
            .load(std::sync::atomic::Ordering::Relaxed);
        let files_processed = self
            .files_processed
            .load(std::sync::atomic::Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            bytes_processed as f64 / elapsed
        } else {
            0.0
        };
        let eta = if speed > 0.0 && bytes_processed < self.bytes_total {
            Some(((self.bytes_total - bytes_processed) as f64 / speed) as u64)
        } else {
            None
        };

        ProgressInfo {
            job_id: self.job_id.clone(),
            phase: Phase::Reading,
            bytes_processed,
            bytes_total: self.bytes_total,
            files_processed,
            files_total: self.files_total,
            speed_bytes_per_sec: speed,
            eta_seconds: eta,
            started_at: chrono::Utc::now()
                - chrono::Duration::from_std(self.start_time.elapsed()).unwrap_or_default(),
            updated_at: chrono::Utc::now(),
        }
    }

    pub fn should_update(&self, interval_ms: u64) -> bool {
        let mut last = self.last_update.lock().unwrap();
        if last.elapsed().as_millis() as u64 >= interval_ms {
            *last = std::time::Instant::now();
            true
        } else {
            false
        }
    }
}
