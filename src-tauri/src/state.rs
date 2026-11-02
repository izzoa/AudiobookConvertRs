//! Application state management for AudiobookConvertRs
//!
//! Provides thread-safe state management for:
//! - Current audiobook being edited
//! - Conversion job queue
//! - FFmpeg availability status

use crate::config::UserSettings;
use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, ConversionJob, JobStatus};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Thread-safe application state
pub struct AppState {
    inner: Arc<RwLock<AppStateInner>>,
}

/// Inner state data
struct AppStateInner {
    /// Currently loaded audiobooks (by ID)
    audiobooks: HashMap<String, AudioBook>,

    /// Active audiobook ID (the one being edited)
    active_audiobook_id: Option<String>,

    /// Conversion job queue
    conversion_queue: Vec<ConversionJob>,

    /// Currently running job ID
    active_job_id: Option<String>,

    /// Whether FFmpeg is available on the system
    ffmpeg_available: bool,

    /// FFmpeg version string (if available)
    ffmpeg_version: Option<String>,

    /// Whether the state has unsaved changes
    is_dirty: bool,
}

impl Default for AppStateInner {
    fn default() -> Self {
        Self {
            audiobooks: HashMap::new(),
            active_audiobook_id: None,
            conversion_queue: Vec::new(),
            active_job_id: None,
            ffmpeg_available: false,
            ffmpeg_version: None,
            is_dirty: false,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AppStateInner::default())),
        }
    }

    // ============================================
    // Audiobook Management
    // ============================================

    /// Add an audiobook to the state
    /// Store a book without making it the active one, for books loaded in the
    /// background (a batch import) while another book is open.
    pub fn insert_audiobook(&self, audiobook: AudioBook) -> String {
        let id = audiobook.id.clone();
        self.inner.write().audiobooks.insert(id.clone(), audiobook);
        info!("Stored audiobook: {}", id);
        id
    }

    pub fn add_audiobook(&self, audiobook: AudioBook) -> String {
        let id = audiobook.id.clone();
        let mut state = self.inner.write();
        state.audiobooks.insert(id.clone(), audiobook);
        state.active_audiobook_id = Some(id.clone());
        state.is_dirty = true;
        info!("Added audiobook: {}", id);
        id
    }

    /// Get an audiobook by ID
    pub fn get_audiobook(&self, id: &str) -> Option<AudioBook> {
        self.inner.read().audiobooks.get(id).cloned()
    }

    /// Get the active audiobook
    pub fn get_active_audiobook(&self) -> Option<AudioBook> {
        let state = self.inner.read();
        state
            .active_audiobook_id
            .as_ref()
            .and_then(|id| state.audiobooks.get(id))
            .cloned()
    }

    /// Get the active audiobook ID
    pub fn get_active_audiobook_id(&self) -> Option<String> {
        self.inner.read().active_audiobook_id.clone()
    }

    /// Set the active audiobook
    pub fn set_active_audiobook(&self, id: Option<String>) -> AppResult<()> {
        let mut state = self.inner.write();

        if let Some(ref id) = id {
            if !state.audiobooks.contains_key(id) {
                return Err(AppError::InvalidAudiobookId(id.clone()));
            }
        }

        state.active_audiobook_id = id;
        Ok(())
    }

    /// Update an audiobook
    pub fn update_audiobook(&self, audiobook: AudioBook) -> AppResult<()> {
        let mut state = self.inner.write();
        let id = audiobook.id.clone();

        if !state.audiobooks.contains_key(&id) {
            return Err(AppError::InvalidAudiobookId(id));
        }

        state.audiobooks.insert(id, audiobook);
        state.is_dirty = true;
        Ok(())
    }

    /// Update an audiobook using a closure
    pub fn update_audiobook_with<F>(&self, id: &str, updater: F) -> AppResult<()>
    where
        F: FnOnce(&mut AudioBook),
    {
        let mut state = self.inner.write();

        let audiobook = state
            .audiobooks
            .get_mut(id)
            .ok_or_else(|| AppError::InvalidAudiobookId(id.to_string()))?;

        updater(audiobook);
        audiobook.mark_modified();
        state.is_dirty = true;

        Ok(())
    }

    /// Remove an audiobook
    pub fn remove_audiobook(&self, id: &str) -> Option<AudioBook> {
        let mut state = self.inner.write();

        // Clear active if this is the active audiobook
        if state.active_audiobook_id.as_ref() == Some(&id.to_string()) {
            state.active_audiobook_id = None;
        }

        let removed = state.audiobooks.remove(id);
        if removed.is_some() {
            state.is_dirty = true;
            info!("Removed audiobook: {}", id);
        }
        removed
    }

    /// Get all audiobook IDs
    pub fn get_audiobook_ids(&self) -> Vec<String> {
        self.inner.read().audiobooks.keys().cloned().collect()
    }

    /// Clear all audiobooks
    pub fn clear_audiobooks(&self) {
        let mut state = self.inner.write();
        state.audiobooks.clear();
        state.active_audiobook_id = None;
        state.is_dirty = true;
        info!("Cleared all audiobooks");
    }

    // ============================================
    // Job Queue Management
    // ============================================

    /// Add a job to the queue
    pub fn add_job(&self, job: ConversionJob) -> String {
        let id = job.id.clone();
        let mut state = self.inner.write();
        state.conversion_queue.push(job);
        info!("Added job to queue: {}", id);
        id
    }

    /// Get a job by ID
    pub fn get_job(&self, id: &str) -> Option<ConversionJob> {
        self.inner
            .read()
            .conversion_queue
            .iter()
            .find(|j| j.id == id)
            .cloned()
    }

    /// Get the next pending job
    pub fn get_next_pending_job(&self) -> Option<ConversionJob> {
        self.inner
            .read()
            .conversion_queue
            .iter()
            .find(|j| j.status == JobStatus::Queued)
            .cloned()
    }

    /// Get the active job
    pub fn get_active_job(&self) -> Option<ConversionJob> {
        let state = self.inner.read();
        state
            .active_job_id
            .as_ref()
            .and_then(|id| state.conversion_queue.iter().find(|j| j.id == *id))
            .cloned()
    }

    /// Update a job's status
    pub fn update_job(&self, job: ConversionJob) -> AppResult<()> {
        let mut state = self.inner.write();

        let existing = state
            .conversion_queue
            .iter_mut()
            .find(|j| j.id == job.id)
            .ok_or_else(|| AppError::JobNotFound(job.id.clone()))?;

        *existing = job;
        Ok(())
    }

    /// Update a job using a closure
    pub fn update_job_with<F>(&self, id: &str, updater: F) -> AppResult<()>
    where
        F: FnOnce(&mut ConversionJob),
    {
        let mut state = self.inner.write();

        let job = state
            .conversion_queue
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| AppError::JobNotFound(id.to_string()))?;

        updater(job);
        Ok(())
    }

    /// Set the active job
    pub fn set_active_job(&self, id: Option<String>) {
        self.inner.write().active_job_id = id;
    }

    /// Cancel a job
    pub fn cancel_job(&self, id: &str) -> AppResult<()> {
        self.update_job_with(id, |job| {
            job.cancel();
        })?;

        let mut state = self.inner.write();
        if state.active_job_id.as_ref() == Some(&id.to_string()) {
            state.active_job_id = None;
        }

        info!("Cancelled job: {}", id);
        Ok(())
    }

    /// Pause a job
    pub fn pause_job(&self, id: &str) -> AppResult<bool> {
        let mut paused = false;
        self.update_job_with(id, |job| {
            paused = job.pause();
        })?;

        if paused {
            info!("Paused job: {}", id);
        } else {
            warn!("Cannot pause job {} - not running", id);
        }
        Ok(paused)
    }

    /// Resume a job
    pub fn resume_job(&self, id: &str) -> AppResult<bool> {
        let mut resumed = false;
        self.update_job_with(id, |job| {
            resumed = job.resume();
        })?;

        if resumed {
            info!("Resumed job: {}", id);
        } else {
            warn!("Cannot resume job {} - not paused", id);
        }
        Ok(resumed)
    }

    /// Remove completed/cancelled jobs from queue
    pub fn cleanup_completed_jobs(&self) {
        let mut state = self.inner.write();
        let before_count = state.conversion_queue.len();
        state
            .conversion_queue
            .retain(|j| !j.status.is_terminal());
        let removed = before_count - state.conversion_queue.len();
        if removed > 0 {
            debug!("Cleaned up {} completed jobs", removed);
        }
    }

    /// Get all jobs
    pub fn get_all_jobs(&self) -> Vec<ConversionJob> {
        self.inner.read().conversion_queue.clone()
    }

    /// Get pending jobs count
    pub fn pending_jobs_count(&self) -> usize {
        self.inner
            .read()
            .conversion_queue
            .iter()
            .filter(|j| j.status == JobStatus::Queued)
            .count()
    }

    // ============================================
    // FFmpeg Status
    // ============================================

    /// Set FFmpeg availability status
    pub fn set_ffmpeg_status(&self, available: bool, version: Option<String>) {
        let mut state = self.inner.write();
        state.ffmpeg_available = available;
        state.ffmpeg_version = version;

        if available {
            info!(
                "FFmpeg is available: {}",
                state.ffmpeg_version.as_deref().unwrap_or("unknown version")
            );
        } else {
            warn!("FFmpeg is not available");
        }
    }

    /// Check if FFmpeg is available
    pub fn is_ffmpeg_available(&self) -> bool {
        self.inner.read().ffmpeg_available
    }

    /// Get FFmpeg version
    pub fn get_ffmpeg_version(&self) -> Option<String> {
        self.inner.read().ffmpeg_version.clone()
    }

    // ============================================
    // State Status
    // ============================================

    /// Check if state has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.inner.read().is_dirty
    }

    /// Mark state as clean (saved)
    pub fn mark_clean(&self) {
        self.inner.write().is_dirty = false;
    }

    /// Reset all state
    pub fn reset(&self) {
        let mut state = self.inner.write();
        *state = AppStateInner::default();
        info!("Application state reset");
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ConversionSettings;

    fn create_test_audiobook() -> AudioBook {
        AudioBook::new("Test Audiobook".to_string())
    }

    fn create_test_job(audiobook: &AudioBook) -> ConversionJob {
        let mut settings = ConversionSettings::default();
        settings.output_path = "/output/test.m4b".into();
        ConversionJob::new(audiobook.clone(), settings)
    }

    #[test]
    fn test_add_and_get_audiobook() {
        let state = AppState::new();
        let audiobook = create_test_audiobook();
        let id = audiobook.id.clone();

        state.add_audiobook(audiobook);

        let retrieved = state.get_audiobook(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Audiobook");
    }

    #[test]
    fn test_active_audiobook() {
        let state = AppState::new();
        let audiobook = create_test_audiobook();
        let id = audiobook.id.clone();

        state.add_audiobook(audiobook);

        let active = state.get_active_audiobook();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, id);
    }

    #[test]
    fn test_update_audiobook() {
        let state = AppState::new();
        let audiobook = create_test_audiobook();
        let id = audiobook.id.clone();

        state.add_audiobook(audiobook);

        state
            .update_audiobook_with(&id, |ab| {
                ab.title = "Updated Title".to_string();
            })
            .unwrap();

        let updated = state.get_audiobook(&id).unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert!(updated.is_dirty);
    }

    #[test]
    fn test_job_queue() {
        let state = AppState::new();
        let audiobook = create_test_audiobook();
        let job = create_test_job(&audiobook);
        let job_id = job.id.clone();

        state.add_audiobook(audiobook);
        state.add_job(job);

        assert_eq!(state.pending_jobs_count(), 1);

        let retrieved = state.get_job(&job_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().status, JobStatus::Queued);
    }

    #[test]
    fn test_cancel_job() {
        let state = AppState::new();
        let audiobook = create_test_audiobook();
        let job = create_test_job(&audiobook);
        let job_id = job.id.clone();

        state.add_audiobook(audiobook);
        state.add_job(job);

        state.cancel_job(&job_id).unwrap();

        let cancelled = state.get_job(&job_id).unwrap();
        assert_eq!(cancelled.status, JobStatus::Cancelled);
    }

    #[test]
    fn test_ffmpeg_status() {
        let state = AppState::new();
        assert!(!state.is_ffmpeg_available());

        state.set_ffmpeg_status(true, Some("5.0".to_string()));

        assert!(state.is_ffmpeg_available());
        assert_eq!(state.get_ffmpeg_version(), Some("5.0".to_string()));
    }
}

