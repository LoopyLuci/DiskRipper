use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::info;

use crate::error::DiskRipperError;
use crate::types::*;
pub use crate::types::JobId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub status: JobStatus,
    pub progress: ProgressInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "Queued"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Paused => write!(f, "Paused"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
            JobStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

pub type JobUpdateSender = broadcast::Sender<ProgressInfo>;
pub type JobUpdateReceiver = broadcast::Receiver<ProgressInfo>;

/// Event emitted when job status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvent {
    pub job_id: String,
    pub event_type: JobEventType,
    pub progress: Option<ProgressInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobEventType {
    Started,
    Progress,
    Completed,
    Failed,
    Cancelled,
}

pub struct JobManager {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
    update_channels: Arc<Mutex<HashMap<JobId, JobUpdateSender>>>,
    cancellation_tokens: Arc<Mutex<HashMap<JobId, tokio_util::sync::CancellationToken>>>,
    event_sender: broadcast::Sender<JobEvent>,
}

impl JobManager {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(256);
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            update_channels: Arc::new(Mutex::new(HashMap::new())),
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
        }
    }

    pub fn create_job(&self, name: impl Into<String>) -> JobId {
        let id = JobId::new();
        let now = chrono::Utc::now();
        let job = Job {
            id: id.clone(),
            name: name.into(),
            status: JobStatus::Queued,
            progress: ProgressInfo {
                job_id: id.clone(),
                phase: Phase::Idle,
                bytes_processed: 0,
                bytes_total: 0,
                files_processed: 0,
                files_total: 0,
                speed_bytes_per_sec: 0.0,
                eta_seconds: None,
                started_at: now,
                updated_at: now,
            },
            created_at: now,
            updated_at: now,
            error: None,
        };

        let (tx, _) = broadcast::channel(256);
        let cancel_token = tokio_util::sync::CancellationToken::new();
        
        self.jobs.lock().unwrap().insert(id.clone(), job);
        self.update_channels.lock().unwrap().insert(id.clone(), tx);
        self.cancellation_tokens.lock().unwrap().insert(id.clone(), cancel_token);
        
        info!(job_id = %id, "Created job");
        id
    }

    pub fn get_job(&self, id: &JobId) -> Option<Job> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    pub fn list_jobs(&self) -> Vec<Job> {
        let mut jobs: Vec<_> = self.jobs.lock().unwrap().values().cloned().collect();
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        jobs
    }

    pub fn update_progress(
        &self,
        id: &JobId,
        progress: ProgressInfo,
    ) -> Result<(), DiskRipperError> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs
            .get_mut(id)
            .ok_or_else(|| DiskRipperError::JobNotFound(id.to_string()))?;
        job.progress = progress.clone();
        job.updated_at = chrono::Utc::now();

        if let Some(tx) = self.update_channels.lock().unwrap().get(id) {
            let _ = tx.send(progress.clone());
        }

        // Emit event for frontend
        let _ = self.event_sender.send(JobEvent {
            job_id: id.to_string(),
            event_type: JobEventType::Progress,
            progress: Some(progress),
            error: None,
        });

        Ok(())
    }

    pub fn set_status(&self, id: &JobId, status: JobStatus) -> Result<(), DiskRipperError> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs
            .get_mut(id)
            .ok_or_else(|| DiskRipperError::JobNotFound(id.to_string()))?;
        job.status = status.clone();
        job.updated_at = chrono::Utc::now();

        // Emit event for frontend
        let event_type = match status {
            JobStatus::Running => JobEventType::Started,
            JobStatus::Completed => JobEventType::Completed,
            JobStatus::Failed => JobEventType::Failed,
            JobStatus::Cancelled => JobEventType::Cancelled,
            _ => return Ok(()),
        };

        let _ = self.event_sender.send(JobEvent {
            job_id: id.to_string(),
            event_type,
            progress: Some(job.progress.clone()),
            error: job.error.clone(),
        });

        Ok(())
    }

    pub fn set_error(&self, id: &JobId, error: impl Into<String>) -> Result<(), DiskRipperError> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs
            .get_mut(id)
            .ok_or_else(|| DiskRipperError::JobNotFound(id.to_string()))?;
        job.error = Some(error.into());
        job.status = JobStatus::Failed;
        job.updated_at = chrono::Utc::now();

        // Emit event for frontend
        let _ = self.event_sender.send(JobEvent {
            job_id: id.to_string(),
            event_type: JobEventType::Failed,
            progress: Some(job.progress.clone()),
            error: job.error.clone(),
        });

        Ok(())
    }

    pub fn subscribe(&self, id: &JobId) -> Option<JobUpdateReceiver> {
        self.update_channels
            .lock()
            .unwrap()
            .get(id)
            .map(|tx| tx.subscribe())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<JobEvent> {
        self.event_sender.subscribe()
    }

    pub fn remove_job(&self, id: &JobId) -> Result<(), DiskRipperError> {
        self.jobs
            .lock()
            .unwrap()
            .remove(id)
            .ok_or_else(|| DiskRipperError::JobNotFound(id.to_string()))?;
        self.update_channels.lock().unwrap().remove(id);
        self.cancellation_tokens.lock().unwrap().remove(id);
        Ok(())
    }

    pub fn cancel_job(&self, id: &JobId) -> Result<(), DiskRipperError> {
        if let Some(token) = self.cancellation_tokens.lock().unwrap().get(id) {
            token.cancel();
        }
        self.set_status(id, JobStatus::Cancelled)
    }

    pub fn get_cancellation_token(&self, id: &JobId) -> Option<tokio_util::sync::CancellationToken> {
        self.cancellation_tokens.lock().unwrap().get(id).cloned()
    }

    pub fn is_cancelled(&self, id: &JobId) -> bool {
        self.cancellation_tokens
            .lock()
            .unwrap()
            .get(id)
            .map(|t| t.is_cancelled())
            .unwrap_or(true)
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}
