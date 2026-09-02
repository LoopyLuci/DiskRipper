use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

use crate::disc::{DiscAnalyzer, DiscInfo, PlatformDiscAnalyzer};
use crate::drive::{DriveInfo, DriveScanner, PlatformDriveScanner};
use crate::error::DiskRipperError;
use crate::extract::Extractor;
use crate::image::Imager;
use crate::job::{JobManager, JobStatus};
use crate::types::*;

pub struct RipEngine {
    drive_scanner: PlatformDriveScanner,
    disc_analyzer: PlatformDiscAnalyzer,
    job_manager: Arc<JobManager>,
    active_jobs: Arc<Mutex<Vec<JobId>>>,
}

impl RipEngine {
    pub fn new() -> Self {
        Self {
            drive_scanner: PlatformDriveScanner::new(),
            disc_analyzer: PlatformDiscAnalyzer::new(),
            job_manager: Arc::new(JobManager::new()),
            active_jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn drives(&self) -> Vec<DriveInfo> {
        self.drive_scanner.scan_drives()
    }

    pub fn analyze_drive(&self, drive_id: &str) -> Result<DiscInfo, DiskRipperError> {
        let drive = self.drive_scanner.get_drive(drive_id)
            .ok_or_else(|| DiskRipperError::DriveNotFound(drive_id.to_string()))?;
        self.disc_analyzer.analyze(&drive.path)
    }

    pub fn job_manager(&self) -> Arc<JobManager> {
        self.job_manager.clone()
    }

    pub async fn start_image_rip(
        &self,
        drive_id: &str,
        output_path: &Path,
        options: ImageOptions,
    ) -> Result<JobId, DiskRipperError> {
        let drive = self.drive_scanner.get_drive(drive_id)
            .ok_or_else(|| DiskRipperError::DriveNotFound(drive_id.to_string()))?;
        let mut disc_info = self.disc_analyzer.analyze(&drive.path)?;
        
        // Try to get actual disc size if available
        if let Some(actual_size) = self.disc_analyzer.get_disc_size(&drive.path) {
            if actual_size > 0 {
                disc_info.total_size = actual_size;
                disc_info.free_size = actual_size;
            }
        }

        let job_id = self.job_manager.create_job(format!("Rip {} to {}", drive_id, output_path.display()));
        self.job_manager.set_status(&job_id, JobStatus::Running)?;

        let jm = self.job_manager.clone();
        let jid = job_id.clone();
        let active = self.active_jobs.clone();
        let drive_path = drive.path.clone();
        let out = output_path.to_path_buf();
        let total = disc_info.total_size;
        let cancel_token = jm.get_cancellation_token(&job_id).unwrap_or_default();

        tokio::spawn(async move {
            active.lock().await.push(jid.clone());
            
            if cancel_token.is_cancelled() {
                let _ = jm.set_status(&jid, JobStatus::Cancelled);
                active.lock().await.retain(|id| id != &jid);
                return;
            }
            
            let engine = Imager::new(jm.clone(), jid.clone(), drive_path, out, total, options);
            if let Err(e) = engine.run().await {
                error!(job_id = %jid, error = %e, "Image rip failed");
                let _ = jm.set_error(&jid, e.to_string());
            }
            active.lock().await.retain(|id| id != &jid);
        });

        Ok(job_id)
    }

    pub async fn start_extraction(
        &self,
        drive_id: &str,
        output_path: &Path,
        options: ExtractOptions,
    ) -> Result<JobId, DiskRipperError> {
        let drive = self.drive_scanner.get_drive(drive_id)
            .ok_or_else(|| DiskRipperError::DriveNotFound(drive_id.to_string()))?;

        let job_id = self.job_manager.create_job(format!("Extract {} to {}", drive_id, output_path.display()));
        self.job_manager.set_status(&job_id, JobStatus::Running)?;

        let jm = self.job_manager.clone();
        let jid = job_id.clone();
        let active = self.active_jobs.clone();
        let drive_path = drive.path.clone();
        let out = output_path.to_path_buf();
        let cancel_token = jm.get_cancellation_token(&job_id).unwrap_or_default();

        tokio::spawn(async move {
            active.lock().await.push(jid.clone());
            
            if cancel_token.is_cancelled() {
                let _ = jm.set_status(&jid, JobStatus::Cancelled);
                active.lock().await.retain(|id| id != &jid);
                return;
            }
            
            let engine = Extractor::new(jm.clone(), jid.clone(), drive_path, out, options);
            if let Err(e) = engine.run().await {
                error!(job_id = %jid, error = %e, "Extraction failed");
                let _ = jm.set_error(&jid, e.to_string());
            }
            active.lock().await.retain(|id| id != &jid);
        });

        Ok(job_id)
    }

    pub async fn cancel_job(&self, job_id: &JobId) -> Result<(), DiskRipperError> {
        self.job_manager.cancel_job(job_id)
    }
}

impl Default for RipEngine {
    fn default() -> Self {
        Self::new()
    }
}
