//! Conversion job queue module
//!
//! Provides thread-safe job queue management for conversion jobs,
//! including persistence to disk for resuming interrupted conversions.

use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, ConversionJob, ConversionSettings, JobStatus};
use camino::{Utf8Path, Utf8PathBuf};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Persistent queue state for saving/loading
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueState {
    /// All jobs in the queue
    jobs: Vec<ConversionJob>,
    /// Currently active job ID
    active_job_id: Option<String>,
}

/// Thread-safe conversion job queue
#[derive(Debug)]
pub struct ConversionQueue {
    /// Jobs waiting to be processed
    pending: RwLock<VecDeque<ConversionJob>>,
    /// Jobs currently being processed
    active: RwLock<Option<ConversionJob>>,
    /// Completed jobs (kept for history)
    completed: RwLock<Vec<ConversionJob>>,
    /// Path to persistence file
    persistence_path: Option<Utf8PathBuf>,
    /// Maximum number of completed jobs to keep
    max_completed: usize,
}

impl ConversionQueue {
    /// Create a new empty queue
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(VecDeque::new()),
            active: RwLock::new(None),
            completed: RwLock::new(Vec::new()),
            persistence_path: None,
            max_completed: 50,
        }
    }

    /// Create a queue with persistence to disk
    pub fn with_persistence(path: Utf8PathBuf) -> AppResult<Self> {
        let mut queue = Self::new();
        queue.persistence_path = Some(path.clone());

        // Try to load existing queue state
        if path.exists() {
            match queue.load_from_disk() {
                Ok(()) => info!("Loaded queue state from {}", path),
                Err(e) => warn!("Failed to load queue state: {}", e),
            }
        }

        Ok(queue)
    }

    /// Add a new job to the queue
    pub fn add_job(&self, job: ConversionJob) -> AppResult<()> {
        info!("Adding job {} to queue", job.id);
        
        let mut pending = self.pending.write();
        pending.push_back(job);
        
        drop(pending);
        self.save_to_disk()?;
        
        Ok(())
    }

    /// Create and add a job from an audiobook and settings
    pub fn queue_conversion(
        &self,
        audiobook: AudioBook,
        settings: ConversionSettings,
    ) -> AppResult<String> {
        let job = ConversionJob::new(audiobook, settings);
        let job_id = job.id.clone();
        self.add_job(job)?;
        Ok(job_id)
    }

    /// Get the next pending job (does not remove from queue)
    pub fn peek_next_job(&self) -> Option<ConversionJob> {
        let pending = self.pending.read();
        pending.front().cloned()
    }

    /// Get the next job and mark it as active
    pub fn get_next_job(&self) -> Option<ConversionJob> {
        // Check if there's already an active job
        {
            let active = self.active.read();
            if active.is_some() {
                debug!("Cannot get next job: there's already an active job");
                return None;
            }
        }

        let mut pending = self.pending.write();
        if let Some(mut job) = pending.pop_front() {
            job.start();
            
            let mut active = self.active.write();
            *active = Some(job.clone());
            
            drop(pending);
            drop(active);
            
            if let Err(e) = self.save_to_disk() {
                error!("Failed to save queue state: {}", e);
            }
            
            Some(job)
        } else {
            None
        }
    }

    /// Update the status of a job
    pub fn update_job_status(&self, job_id: &str, status: JobStatus) -> AppResult<()> {
        debug!("Updating job {} status to {:?}", job_id, status);

        // Check active job first
        {
            let mut active = self.active.write();
            if let Some(ref mut job) = *active {
                if job.id == job_id {
                    job.status = status;
                    
                    // If job is terminal, move to completed
                    if status.is_terminal() {
                        let completed_job = active.take().unwrap();
                        drop(active);
                        
                        let mut completed = self.completed.write();
                        completed.push(completed_job);
                        
                        // Trim completed list if too long
                        while completed.len() > self.max_completed {
                            completed.remove(0);
                        }
                    }
                    
                    self.save_to_disk()?;
                    return Ok(());
                }
            }
        }

        // Check pending jobs
        {
            let mut pending = self.pending.write();
            for job in pending.iter_mut() {
                if job.id == job_id {
                    job.status = status;
                    self.save_to_disk()?;
                    return Ok(());
                }
            }
        }

        Err(AppError::StateError(format!(
            "Job not found: {}",
            job_id
        )))
    }

    /// Update job progress
    pub fn update_job_progress(&self, job_id: &str, progress: f32, file_index: usize) {
        let mut active = self.active.write();
        if let Some(ref mut job) = *active {
            if job.id == job_id {
                job.progress_percent = progress;
                job.current_file_index = file_index;
            }
        }
    }

    /// Mark a job as complete
    pub fn complete_job(&self, job_id: &str) -> AppResult<()> {
        self.update_job_status(job_id, JobStatus::Complete)
    }

    /// Mark a job as failed
    pub fn fail_job(&self, job_id: &str, error: &str) -> AppResult<()> {
        // Update error message
        {
            let mut active = self.active.write();
            if let Some(ref mut job) = *active {
                if job.id == job_id {
                    job.error_message = Some(error.to_string());
                }
            }
        }
        
        self.update_job_status(job_id, JobStatus::Error)
    }

    /// Cancel a job
    pub fn cancel_job(&self, job_id: &str) -> AppResult<()> {
        info!("Cancelling job {}", job_id);

        // Check if it's the active job
        {
            let active = self.active.read();
            if let Some(ref job) = *active {
                if job.id == job_id {
                    drop(active);
                    return self.update_job_status(job_id, JobStatus::Cancelled);
                }
            }
        }

        // Remove from pending queue
        {
            let mut pending = self.pending.write();
            if let Some(pos) = pending.iter().position(|j| j.id == job_id) {
                let mut job = pending.remove(pos).unwrap();
                job.cancel();
                
                let mut completed = self.completed.write();
                completed.push(job);
                
                drop(pending);
                drop(completed);
                
                self.save_to_disk()?;
                return Ok(());
            }
        }

        Err(AppError::StateError(format!(
            "Job not found: {}",
            job_id
        )))
    }

    /// Get all jobs (pending + active + completed)
    pub fn get_all_jobs(&self) -> Vec<ConversionJob> {
        let mut jobs = Vec::new();

        // Add pending jobs
        {
            let pending = self.pending.read();
            jobs.extend(pending.iter().cloned());
        }

        // Add active job
        {
            let active = self.active.read();
            if let Some(ref job) = *active {
                jobs.push(job.clone());
            }
        }

        // Add completed jobs
        {
            let completed = self.completed.read();
            jobs.extend(completed.iter().cloned());
        }

        jobs
    }

    /// Get pending jobs only
    pub fn get_pending_jobs(&self) -> Vec<ConversionJob> {
        let pending = self.pending.read();
        pending.iter().cloned().collect()
    }

    /// Get the currently active job
    pub fn get_active_job(&self) -> Option<ConversionJob> {
        let active = self.active.read();
        active.clone()
    }

    /// Get completed jobs
    pub fn get_completed_jobs(&self) -> Vec<ConversionJob> {
        let completed = self.completed.read();
        completed.clone()
    }

    /// Get a specific job by ID
    pub fn get_job(&self, job_id: &str) -> Option<ConversionJob> {
        // Check active
        {
            let active = self.active.read();
            if let Some(ref job) = *active {
                if job.id == job_id {
                    return Some(job.clone());
                }
            }
        }

        // Check pending
        {
            let pending = self.pending.read();
            if let Some(job) = pending.iter().find(|j| j.id == job_id) {
                return Some(job.clone());
            }
        }

        // Check completed
        {
            let completed = self.completed.read();
            if let Some(job) = completed.iter().find(|j| j.id == job_id) {
                return Some(job.clone());
            }
        }

        None
    }

    /// Get queue length (pending jobs only)
    pub fn len(&self) -> usize {
        let pending = self.pending.read();
        pending.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        let pending = self.pending.read();
        pending.is_empty()
    }

    /// Check if there's an active job
    pub fn has_active_job(&self) -> bool {
        let active = self.active.read();
        active.is_some()
    }

    /// Clear all completed jobs
    pub fn clear_completed(&self) -> AppResult<()> {
        {
            let mut completed = self.completed.write();
            completed.clear();
        }
        self.save_to_disk()
    }

    /// Save queue state to disk
    fn save_to_disk(&self) -> AppResult<()> {
        let Some(ref path) = self.persistence_path else {
            return Ok(());
        };

        let state = {
            let pending = self.pending.read();
            let active = self.active.read();
            let completed = self.completed.read();

            let mut jobs: Vec<ConversionJob> = pending.iter().cloned().collect();
            let active_id = active.as_ref().map(|j| {
                jobs.push(j.clone());
                j.id.clone()
            });
            jobs.extend(completed.iter().cloned());

            QueueState {
                jobs,
                active_job_id: active_id,
            }
        };

        let json = serde_json::to_string_pretty(&state).map_err(|e| {
            AppError::StateError(format!("Failed to serialize queue state: {}", e))
        })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::FileWriteError {
                path: parent.to_path_buf().into(),
                reason: format!("Failed to create directory: {}", e),
            })?;
        }

        fs::write(path, json).map_err(|e| AppError::FileWriteError {
            path: path.clone().into(),
            reason: format!("Failed to write queue state: {}", e),
        })?;

        debug!("Saved queue state to {}", path);
        Ok(())
    }

    /// Load queue state from disk
    fn load_from_disk(&self) -> AppResult<()> {
        let Some(ref path) = self.persistence_path else {
            return Ok(());
        };

        let content = fs::read_to_string(path).map_err(|e| AppError::FileReadError {
            path: path.clone().into(),
            reason: format!("Failed to read queue state: {}", e),
        })?;

        let state: QueueState = serde_json::from_str(&content).map_err(|e| {
            AppError::StateError(format!("Failed to parse queue state: {}", e))
        })?;

        let mut pending = self.pending.write();
        let mut active = self.active.write();
        let mut completed = self.completed.write();

        pending.clear();
        *active = None;
        completed.clear();

        for job in state.jobs {
            if state.active_job_id.as_ref() == Some(&job.id) && job.status == JobStatus::Running {
                // Resume active job as queued (it was interrupted)
                let mut resumed_job = job;
                resumed_job.status = JobStatus::Queued;
                pending.push_back(resumed_job);
            } else if job.status.is_terminal() {
                completed.push(job);
            } else {
                pending.push_back(job);
            }
        }

        Ok(())
    }
}

impl Default for ConversionQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe handle to a conversion queue
pub type SharedQueue = Arc<ConversionQueue>;

/// Create a shared queue instance
pub fn create_shared_queue() -> SharedQueue {
    Arc::new(ConversionQueue::new())
}

/// Create a shared queue with persistence
pub fn create_shared_queue_with_persistence(path: Utf8PathBuf) -> AppResult<SharedQueue> {
    Ok(Arc::new(ConversionQueue::with_persistence(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_audiobook() -> AudioBook {
        AudioBook::new("Test Audiobook".to_string())
    }

    #[test]
    fn test_queue_creation() {
        let queue = ConversionQueue::new();
        assert!(queue.is_empty());
        assert!(!queue.has_active_job());
    }

    #[test]
    fn test_queue_add_job() {
        let queue = ConversionQueue::new();
        let audiobook = create_test_audiobook();
        let settings = ConversionSettings::default();
        
        let job_id = queue.queue_conversion(audiobook, settings).expect("Should add job");
        
        assert_eq!(queue.len(), 1);
        assert!(queue.get_job(&job_id).is_some());
    }

    #[test]
    fn test_queue_get_next_job() {
        let queue = ConversionQueue::new();
        let audiobook = create_test_audiobook();
        let settings = ConversionSettings::default();
        
        let job_id = queue.queue_conversion(audiobook, settings).expect("Should add job");
        
        let job = queue.get_next_job().expect("Should get job");
        assert_eq!(job.id, job_id);
        assert_eq!(job.status, JobStatus::Running);
        assert!(queue.is_empty()); // Pending is empty
        assert!(queue.has_active_job());
    }

    #[test]
    fn test_queue_complete_job() {
        let queue = ConversionQueue::new();
        let audiobook = create_test_audiobook();
        let settings = ConversionSettings::default();
        
        let job_id = queue.queue_conversion(audiobook, settings).expect("Should add job");
        let _ = queue.get_next_job();
        
        queue.complete_job(&job_id).expect("Should complete");
        
        assert!(!queue.has_active_job());
        assert_eq!(queue.get_completed_jobs().len(), 1);
    }

    #[test]
    fn test_queue_cancel_pending_job() {
        let queue = ConversionQueue::new();
        let audiobook = create_test_audiobook();
        let settings = ConversionSettings::default();
        
        let job_id = queue.queue_conversion(audiobook, settings).expect("Should add job");
        
        queue.cancel_job(&job_id).expect("Should cancel");
        
        assert!(queue.is_empty());
        assert_eq!(queue.get_completed_jobs().len(), 1);
        assert_eq!(queue.get_completed_jobs()[0].status, JobStatus::Cancelled);
    }

    #[test]
    fn test_queue_multiple_jobs() {
        let queue = ConversionQueue::new();
        let settings = ConversionSettings::default();
        
        queue.queue_conversion(create_test_audiobook(), settings.clone()).unwrap();
        queue.queue_conversion(create_test_audiobook(), settings.clone()).unwrap();
        queue.queue_conversion(create_test_audiobook(), settings).unwrap();
        
        assert_eq!(queue.len(), 3);
        
        // Get first job
        let _ = queue.get_next_job();
        assert_eq!(queue.len(), 2);
        assert!(queue.has_active_job());
        
        // Can't get another while one is active
        assert!(queue.get_next_job().is_none());
    }
}
