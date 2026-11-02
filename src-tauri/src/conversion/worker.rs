//! Worker thread module
//!
//! Provides worker management for processing conversion jobs in parallel.

use crate::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use crate::conversion::progress::{ConversionProgress, ConversionStage};
use crate::conversion::queue::SharedQueue;
use crate::error::{AppError, AppResult};
use crate::ffmpeg::detection::FFmpegInfo;
use crate::models::{ConversionJob, JobStatus};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tracing::{debug, error, info, warn};

/// Message sent from worker to coordinator
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// Progress update
    Progress(ConversionProgress),
    /// Job completed successfully
    JobCompleted {
        job_id: String,
        output_path: String,
    },
    /// Job failed
    JobFailed {
        job_id: String,
        error: String,
    },
    /// Worker is idle and ready for more work
    Idle,
    /// Worker is shutting down
    Shutdown,
}

/// Command sent from coordinator to worker
#[derive(Debug, Clone)]
pub enum WorkerCommand {
    /// Process a job
    Process(ConversionJob),
    /// Cancel current job
    Cancel,
    /// Shutdown the worker
    Shutdown,
}

/// Configuration for the worker pool
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// FFmpeg information
    pub ffmpeg_info: FFmpegInfo,
    /// Pipeline configuration
    pub pipeline_config: PipelineConfig,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: 1, // Default to single worker for audiobook conversion
            ffmpeg_info: FFmpegInfo {
                path: "ffmpeg".into(),
                version: "unknown".to_string(),
                supports_libfdk_aac: false,
                version_output: String::new(),
                supports_aac: true,
                source: crate::ffmpeg::FFmpegSource::System,
            },
            pipeline_config: PipelineConfig::default(),
        }
    }
}

/// A worker that processes conversion jobs
pub struct ConversionWorker {
    /// Worker ID
    id: usize,
    /// Thread handle
    handle: Option<JoinHandle<()>>,
    /// Command sender
    command_tx: Sender<WorkerCommand>,
    /// Cancel flag
    cancel_flag: Arc<AtomicBool>,
}

impl ConversionWorker {
    /// Create and start a new worker
    pub fn new(
        id: usize,
        message_tx: Sender<WorkerMessage>,
        ffmpeg_info: FFmpegInfo,
        config: PipelineConfig,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag_clone = cancel_flag.clone();

        let handle = thread::spawn(move || {
            worker_loop(id, command_rx, message_tx, cancel_flag_clone, ffmpeg_info, config);
        });

        Self {
            id,
            handle: Some(handle),
            command_tx,
            cancel_flag,
        }
    }

    /// Send a command to the worker
    pub fn send(&self, command: WorkerCommand) -> AppResult<()> {
        self.command_tx.send(command).map_err(|e| {
            AppError::InternalError(format!("Failed to send command to worker: {}", e))
        })
    }

    /// Request cancellation of current job
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Reset cancel flag
    pub fn reset_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    /// Shutdown the worker
    pub fn shutdown(mut self) {
        let _ = self.send(WorkerCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Main worker loop
fn worker_loop(
    id: usize,
    command_rx: Receiver<WorkerCommand>,
    message_tx: Sender<WorkerMessage>,
    cancel_flag: Arc<AtomicBool>,
    ffmpeg_info: FFmpegInfo,
    config: PipelineConfig,
) {
    info!("Worker {} started", id);

    loop {
        // Signal idle
        if message_tx.send(WorkerMessage::Idle).is_err() {
            error!("Worker {} failed to send idle message", id);
            break;
        }

        // Wait for command
        match command_rx.recv() {
            Ok(WorkerCommand::Process(job)) => {
                debug!("Worker {} processing job {}", id, job.id);
                cancel_flag.store(false, Ordering::SeqCst);

                // Create progress callback
                let message_tx_clone = message_tx.clone();
                let progress_callback = move |progress: ConversionProgress| {
                    let _ = message_tx_clone.send(WorkerMessage::Progress(progress));
                };

                // Create cancel check function
                let cancel_flag_clone = cancel_flag.clone();
                let cancel_check = move || cancel_flag_clone.load(Ordering::SeqCst);

                // Run the conversion pipeline
                let pipeline = ConversionPipeline::new(ffmpeg_info.clone(), config.clone());
                
                match pipeline.run(&job, progress_callback, cancel_check) {
                    Ok(output_path) => {
                        info!("Worker {} completed job {}", id, job.id);
                        let _ = message_tx.send(WorkerMessage::JobCompleted {
                            job_id: job.id.clone(),
                            output_path: output_path.to_string(),
                        });
                    }
                    Err(e) => {
                        error!("Worker {} failed job {}: {}", id, job.id, e);
                        let _ = message_tx.send(WorkerMessage::JobFailed {
                            job_id: job.id.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
            Ok(WorkerCommand::Cancel) => {
                debug!("Worker {} received cancel command", id);
                cancel_flag.store(true, Ordering::SeqCst);
            }
            Ok(WorkerCommand::Shutdown) => {
                info!("Worker {} shutting down", id);
                let _ = message_tx.send(WorkerMessage::Shutdown);
                break;
            }
            Err(_) => {
                // Channel closed
                warn!("Worker {} command channel closed", id);
                break;
            }
        }
    }

    info!("Worker {} stopped", id);
}

/// Pool of workers for processing conversion jobs
pub struct WorkerPool {
    /// Workers
    workers: Vec<ConversionWorker>,
    /// Message receiver from workers
    message_rx: Receiver<WorkerMessage>,
    /// Shared job queue
    queue: SharedQueue,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Configuration
    config: WorkerPoolConfig,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(queue: SharedQueue, config: WorkerPoolConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel();
        let mut workers = Vec::with_capacity(config.num_workers);

        for i in 0..config.num_workers {
            let worker = ConversionWorker::new(
                i,
                message_tx.clone(),
                config.ffmpeg_info.clone(),
                config.pipeline_config.clone(),
            );
            workers.push(worker);
        }

        Self {
            workers,
            message_rx,
            queue,
            running: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    /// Start processing jobs from the queue
    ///
    /// This runs in the current thread and processes messages from workers.
    /// Returns when all workers are shutdown or the queue is empty and idle.
    pub fn run<F>(&self, mut progress_callback: F) -> AppResult<()>
    where
        F: FnMut(WorkerMessage),
    {
        self.running.store(true, Ordering::SeqCst);
        let mut idle_workers: Vec<usize> = Vec::new();

        while self.running.load(Ordering::SeqCst) {
            // Process worker messages
            match self.message_rx.recv() {
                Ok(message) => {
                    match &message {
                        WorkerMessage::Idle => {
                            // Worker is ready for a job
                            if let Some(job) = self.queue.get_next_job() {
                                // Find an idle worker to assign the job
                                if let Some(worker_idx) = idle_workers.pop() {
                                    if let Err(e) = self.workers[worker_idx]
                                        .send(WorkerCommand::Process(job))
                                    {
                                        error!("Failed to send job to worker: {}", e);
                                    }
                                }
                            } else if idle_workers.len() == self.workers.len() {
                                // All workers idle and no jobs - we're done
                                debug!("All workers idle, no pending jobs");
                                // Don't stop automatically, wait for more jobs
                            }
                        }
                        WorkerMessage::JobCompleted { job_id, output_path } => {
                            info!("Job {} completed: {}", job_id, output_path);
                            if let Err(e) = self.queue.complete_job(job_id) {
                                error!("Failed to mark job complete: {}", e);
                            }
                        }
                        WorkerMessage::JobFailed { job_id, error } => {
                            error!("Job {} failed: {}", job_id, error);
                            if let Err(e) = self.queue.fail_job(job_id, error) {
                                error!("Failed to mark job as failed: {}", e);
                            }
                        }
                        WorkerMessage::Shutdown => {
                            debug!("Worker shutdown received");
                        }
                        WorkerMessage::Progress(_) => {
                            // Progress is forwarded to callback
                        }
                    }

                    // Forward message to callback
                    progress_callback(message);
                }
                Err(_) => {
                    // All senders dropped
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process a single batch of jobs and return
    ///
    /// Useful for testing or one-shot conversion.
    pub fn process_batch<F>(&self, mut progress_callback: F) -> AppResult<()>
    where
        F: FnMut(WorkerMessage),
    {
        self.running.store(true, Ordering::SeqCst);
        let mut active_jobs = 0;
        let mut workers_ready: Vec<usize> = (0..self.workers.len()).collect();

        // Assign initial jobs
        while !workers_ready.is_empty() {
            if let Some(job) = self.queue.get_next_job() {
                let worker_idx = workers_ready.pop().unwrap();
                self.workers[worker_idx].send(WorkerCommand::Process(job))?;
                active_jobs += 1;
            } else {
                break;
            }
        }

        if active_jobs == 0 {
            debug!("No jobs to process");
            return Ok(());
        }

        // Process messages until all jobs complete
        while active_jobs > 0 {
            match self.message_rx.recv() {
                Ok(message) => {
                    match &message {
                        WorkerMessage::JobCompleted { job_id, output_path } => {
                            info!("Job {} completed: {}", job_id, output_path);
                            if let Err(e) = self.queue.complete_job(job_id) {
                                error!("Failed to mark job complete: {}", e);
                            }
                            active_jobs -= 1;

                            // Try to get next job
                            if let Some(job) = self.queue.get_next_job() {
                                // Find an idle worker (the one that just finished)
                                for (i, worker) in self.workers.iter().enumerate() {
                                    if worker.send(WorkerCommand::Process(job.clone())).is_ok() {
                                        active_jobs += 1;
                                        break;
                                    }
                                }
                            }
                        }
                        WorkerMessage::JobFailed { job_id, error } => {
                            error!("Job {} failed: {}", job_id, error);
                            if let Err(e) = self.queue.fail_job(job_id, error) {
                                error!("Failed to mark job as failed: {}", e);
                            }
                            active_jobs -= 1;
                        }
                        WorkerMessage::Idle => {
                            // Worker became idle
                        }
                        _ => {}
                    }

                    progress_callback(message);
                }
                Err(_) => break,
            }
        }

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Stop processing
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        for worker in &self.workers {
            worker.cancel();
        }
    }

    /// Shutdown all workers
    pub fn shutdown(self) {
        self.running.store(false, Ordering::SeqCst);
        for worker in self.workers {
            worker.shutdown();
        }
    }

    /// Cancel a specific job
    pub fn cancel_job(&self, job_id: &str) -> AppResult<()> {
        // Check if it's the active job in any worker
        if let Some(active) = self.queue.get_active_job() {
            if active.id == job_id {
                // Cancel all workers (we don't track which worker has which job)
                for worker in &self.workers {
                    worker.cancel();
                }
            }
        }
        self.queue.cancel_job(job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioBook, ConversionSettings};
    use crate::conversion::queue::create_shared_queue;

    #[test]
    fn test_worker_pool_config_default() {
        let config = WorkerPoolConfig::default();
        assert_eq!(config.num_workers, 1);
    }

    // Note: Full worker tests require FFmpeg to be installed
    // and would be integration tests rather than unit tests
}
