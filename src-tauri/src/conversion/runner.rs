//! Conversion runner module
//!
//! Manages parallel conversion workers that process jobs from the queue.
//! Handles starting/stopping workers, job claiming, and event emission.

use crate::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use crate::conversion::progress::ConversionProgress;
use crate::conversion::queue::{ConversionQueue, SharedQueue};
use crate::error::{AppError, AppResult};
use crate::events::{ConversionComplete, ConversionError, ConversionProgress as EventProgress};
use crate::ffmpeg::detection::FFmpegInfo;
use crate::models::{ConversionJob, JobStatus};
use camino::Utf8PathBuf;
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, error, info, warn};

/// Maximum number of workers allowed
const MAX_WORKERS: usize = 16;

/// Default number of workers
const DEFAULT_WORKERS: usize = 2;

/// Conversion manager state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ManagerState {
    /// Manager is stopped
    Stopped,
    /// Manager is running
    Running,
    /// Manager is stopping (waiting for workers to finish)
    Stopping,
}

/// Worker thread handle
struct WorkerHandle {
    id: usize,
    handle: Option<thread::JoinHandle<()>>,
}

/// Manages conversion workers and job processing
pub struct ConversionManager {
    /// Queue instance
    pub queue: SharedQueue,
    /// App handle for emitting events
    app_handle: Arc<RwLock<Option<AppHandle>>>,
    /// Worker threads
    workers: Arc<Mutex<Vec<WorkerHandle>>>,
    /// Number of workers to run
    num_workers: AtomicUsize,
    /// Manager state
    state: Arc<RwLock<ManagerState>>,
    /// Stop flag for workers
    stop_flag: Arc<AtomicBool>,
    /// Condition variable for worker synchronization
    work_available: Arc<(Mutex<()>, Condvar)>,
}

impl ConversionManager {
    /// Create a new conversion manager
    pub fn new(persistence_path: Option<Utf8PathBuf>) -> AppResult<Self> {
        let queue = if let Some(path) = persistence_path {
            Arc::new(ConversionQueue::with_persistence(path)?)
        } else {
            Arc::new(ConversionQueue::new())
        };

        Ok(Self {
            queue,
            app_handle: Arc::new(RwLock::new(None)),
            workers: Arc::new(Mutex::new(Vec::new())),
            num_workers: AtomicUsize::new(DEFAULT_WORKERS),
            state: Arc::new(RwLock::new(ManagerState::Stopped)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            work_available: Arc::new((Mutex::new(()), Condvar::new())),
        })
    }

    /// Set the app handle for event emission
    pub fn set_app_handle(&self, handle: AppHandle) {
        let mut app_handle = self.app_handle.write();
        *app_handle = Some(handle);
    }

    /// Start the conversion queue with the specified number of workers
    pub fn start_queue(&self, max_concurrent: Option<usize>) -> AppResult<()> {
        let mut state = self.state.write();
        if *state == ManagerState::Running {
            drop(state);
            let (lock, cvar) = &*self.work_available;
            let _guard = lock.lock();
            cvar.notify_all();
            info!("Conversion queue already running; notified workers");
            return Ok(());
        }
        if *state != ManagerState::Stopped {
            return Err(AppError::StateError(
                "Conversion queue is already running or stopping".to_string(),
            ));
        }

        *state = ManagerState::Running;
        drop(state);

        // Set number of workers
        let num_workers = max_concurrent
            .unwrap_or(DEFAULT_WORKERS)
            .min(MAX_WORKERS)
            .max(1);
        self.num_workers.store(num_workers, Ordering::SeqCst);

        // Clear stop flag
        self.stop_flag.store(false, Ordering::SeqCst);

        // Start worker threads
        self.start_workers(num_workers)?;

        info!("Started conversion queue with {} workers", num_workers);
        Ok(())
    }

    /// Stop the conversion queue
    pub fn stop_queue(&self) -> AppResult<()> {
        let mut state = self.state.write();
        if *state != ManagerState::Running {
            return Ok(());
        }

        *state = ManagerState::Stopping;
        drop(state);

        // Signal workers to stop
        self.stop_flag.store(true, Ordering::SeqCst);

        // Wake all workers
        let (lock, cvar) = &*self.work_available;
        let _guard = lock.lock();
        cvar.notify_all();

        // Wait for workers to finish
        self.stop_workers();

        let mut state = self.state.write();
        *state = ManagerState::Stopped;

        info!("Stopped conversion queue");
        Ok(())
    }

    /// Enqueue a conversion job
    pub fn enqueue_job(&self, job: ConversionJob) -> AppResult<()> {
        self.queue.add_job(job)?;

        // Wake a worker to process the new job
        let (lock, cvar) = &*self.work_available;
        let _guard = lock.lock();
        cvar.notify_one();

        Ok(())
    }

    /// Get all conversion jobs
    pub fn list_jobs(&self) -> Vec<ConversionJob> {
        self.queue.get_all_jobs()
    }

    /// Get the current queue state
    pub fn get_queue_state(&self) -> (ManagerState, usize, usize, usize) {
        let state = *self.state.read();
        let pending = self.queue.len();
        let active = self.queue.active_job_count();
        let num_workers = self.num_workers.load(Ordering::SeqCst);
        (state, pending, active, num_workers)
    }

    /// Start worker threads
    fn start_workers(&self, count: usize) -> AppResult<()> {
        let mut workers = self.workers.lock();
        workers.clear();

        for id in 0..count {
            let worker = self.spawn_worker(id);
            workers.push(worker);
        }

        Ok(())
    }

    /// Stop all worker threads
    fn stop_workers(&self) {
        let mut workers = self.workers.lock();

        for worker in workers.iter_mut() {
            if let Some(handle) = worker.handle.take() {
                if let Err(e) = handle.join() {
                    error!("Worker {} panicked: {:?}", worker.id, e);
                }
            }
        }

        workers.clear();
    }

    /// Spawn a single worker thread
    fn spawn_worker(&self, id: usize) -> WorkerHandle {
        let queue = Arc::clone(&self.queue);
        let app_handle = Arc::clone(&self.app_handle);
        let stop_flag = Arc::clone(&self.stop_flag);
        let work_available = Arc::clone(&self.work_available);

        let handle = thread::spawn(move || {
            info!("Worker {} started", id);

            loop {
                // Check stop flag
                if stop_flag.load(Ordering::SeqCst) {
                    info!("Worker {} stopping", id);
                    break;
                }

                // Try to claim a job
                if let Some(job) = queue.claim_next_job() {
                    info!("Worker {} processing job {}", id, job.id);
                    Self::process_job(job, &queue, &app_handle);
                } else {
                    // No jobs available, wait for work
                    debug!("Worker {} waiting for work", id);
                    let (lock, cvar) = &*work_available;
                    let mut guard = lock.lock();

                    // Check stop flag again before waiting
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }

                    // Wait for signal or timeout
                    // parking_lot Condvar uses wait_for instead of wait_timeout
                    let _result = cvar.wait_for(&mut guard, Duration::from_secs(5));
                }
            }

            info!("Worker {} stopped", id);
        });

        WorkerHandle {
            id,
            handle: Some(handle),
        }
    }

    /// Process a single conversion job
    fn process_job(
        job: ConversionJob,
        queue: &SharedQueue,
        app_handle: &Arc<RwLock<Option<AppHandle>>>,
    ) {
        let job_id = job.id.clone();

        // Create progress callback
        let job_id_progress = job_id.clone();
        let queue_progress = Arc::clone(queue);
        let app_handle_progress = Arc::clone(app_handle);

        let progress_callback = move |progress: ConversionProgress| {
            // Update queue with full progress snapshot
            queue_progress.update_job_progress_detail(&job_id_progress, progress.clone());

            // Emit progress event with full payload
            if let Some(ref handle) = *app_handle_progress.read() {
                let event = EventProgress {
                    job_id: job_id_progress.clone(),
                    file_index: progress.file_index,
                    current_file: progress.current_file.clone(),
                    file_progress: progress.file_progress,
                    total_progress: progress.total_progress,
                    time_remaining_sec: progress.time_remaining_sec,
                    estimated_finish: Some(progress.estimated_finish),
                    speed: progress.speed,
                    stage: progress.stage,
                };
                if let Err(e) = handle.emit("conversion_progress", event) {
                    error!("Failed to emit conversion progress event: {}", e);
                }
            }
        };

        // Check for cancellation - queries the queue for job status
        let job_id_cancel = job_id.clone();
        let queue_cancel = Arc::clone(queue);
        let cancel_check = move || queue_cancel.is_job_cancelled(&job_id_cancel);

        // Get FFmpeg info
        let ffmpeg_info = match crate::ffmpeg::detection::detect_ffmpeg() {
            Ok(info) => info,
            Err(e) => {
                error!("FFmpeg not found: {}", e);
                if let Err(e) = queue.fail_job(&job_id, &e.to_string()) {
                    error!("Failed to update job status: {}", e);
                }
                return;
            }
        };

        // Create pipeline
        let pipeline = ConversionPipeline::new(ffmpeg_info, PipelineConfig::default());

        // Run the conversion pipeline
        match pipeline.run(&job, progress_callback, cancel_check) {
            Ok(output_path) => {
                info!("Job {} completed successfully", job_id);

                // Update job status
                if let Err(e) = queue.complete_job_with_output(&job_id, output_path.to_string()) {
                    error!("Failed to update job status: {}", e);
                }

                // Emit completion event
                if let Some(ref handle) = *app_handle.read() {
                    let event = ConversionComplete {
                        job_id: job_id.clone(),
                        output_path: output_path.to_string(),
                        duration_ms: 0, // We don't track duration in the runner yet
                    };
                    if let Err(e) = handle.emit("conversion_complete", event) {
                        error!("Failed to emit conversion complete event: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Job {} failed: {}", job_id, e);

                // Update job status
                if let Err(e) = queue.fail_job(&job_id, &e.to_string()) {
                    error!("Failed to update job status: {}", e);
                }

                // Emit error event
                if let Some(ref handle) = *app_handle.read() {
                    let event = ConversionError {
                        job_id: job_id.clone(),
                        error_message: e.to_string(),
                        recoverable: false,
                    };
                    if let Err(err) = handle.emit("conversion_error", event) {
                        error!("Failed to emit conversion error event: {}", err);
                    }
                }
            }
        }
    }
}

/// Thread-safe handle to a conversion manager
pub type SharedManager = Arc<ConversionManager>;

/// Create a shared conversion manager
pub fn create_shared_manager(persistence_path: Option<Utf8PathBuf>) -> AppResult<SharedManager> {
    Ok(Arc::new(ConversionManager::new(persistence_path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioBook, ConversionSettings};

    #[test]
    fn test_manager_creation() {
        let manager = ConversionManager::new(None).expect("Should create manager");
        let (state, _, _, _) = manager.get_queue_state();
        assert_eq!(state, ManagerState::Stopped);
    }

    #[test]
    fn test_manager_start_stop() {
        let manager = ConversionManager::new(None).expect("Should create manager");

        // Start queue
        manager.start_queue(Some(2)).expect("Should start queue");
        let (state, _, _, workers) = manager.get_queue_state();
        assert_eq!(state, ManagerState::Running);
        assert_eq!(workers, 2);

        // Starting again while running is idempotent
        manager
            .start_queue(Some(2))
            .expect("Starting running queue should succeed idempotently");
        let (state, _, _, _) = manager.get_queue_state();
        assert_eq!(state, ManagerState::Running);

        // Stop queue
        manager.stop_queue().expect("Should stop queue");
        let (state, _, _, _) = manager.get_queue_state();
        assert_eq!(state, ManagerState::Stopped);
    }

    #[test]
    fn test_manager_enqueue() {
        let manager = ConversionManager::new(None).expect("Should create manager");

        let audiobook = AudioBook::new("Test".to_string());
        let settings = ConversionSettings::default();
        let job = ConversionJob::new(audiobook, settings);

        manager.enqueue_job(job).expect("Should enqueue job");

        let jobs = manager.list_jobs();
        assert_eq!(jobs.len(), 1);
    }
}
