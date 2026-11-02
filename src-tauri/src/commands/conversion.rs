use crate::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use crate::conversion::runner::SharedManager;
use crate::events::{emit_completion, emit_error, emit_progress};
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::models::{ConversionJob, ConversionSettings, JobStatus};
use crate::state::AppState;
use std::thread;
use tauri::{AppHandle, Manager, State, Window};
use tracing::{error, info};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn enqueue_conversion(
    audiobook_id: String,
    settings: ConversionSettings,
    state: State<'_, AppState>,
    manager: State<'_, SharedManager>,
    app: AppHandle,
) -> Result<String, String> {
    // Get audiobook by ID - validate it exists
    let audiobook = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Basic validation
    if settings.output_path.as_str().is_empty() {
        return Err("Output path is required".to_string());
    }

    let concurrent = (settings.concurrent_jobs as usize).max(1);

    // Create Job
    let job = ConversionJob::new(audiobook, settings);
    let job_id = job.id.clone();

    // Ensure app handle is set for event emission
    manager.set_app_handle(app);

    // Enqueue the job
    manager
        .enqueue_job(job)
        .map_err(|e| format!("Failed to enqueue job: {}", e))?;

    // Auto-start the queue if stopped so conversion begins immediately
    let (queue_state, _, _, _) = manager.get_queue_state();
    if queue_state == crate::conversion::runner::ManagerState::Stopped {
        manager
            .start_queue(Some(concurrent))
            .map_err(|e| format!("Failed to start queue: {}", e))?;
    }

    Ok(job_id)
}

#[tauri::command]
pub async fn start_conversion_queue(
    max_concurrent: Option<usize>,
    manager: State<'_, SharedManager>,
    app: AppHandle,
) -> Result<(), String> {
    // Set app handle for event emission
    manager.set_app_handle(app);

    // Start the queue
    manager
        .start_queue(max_concurrent)
        .map_err(|e| format!("Failed to start queue: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn stop_conversion_queue(manager: State<'_, SharedManager>) -> Result<(), String> {
    manager
        .stop_queue()
        .map_err(|e| format!("Failed to stop queue: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn list_conversion_jobs(
    manager: State<'_, SharedManager>,
) -> Result<Vec<ConversionJob>, String> {
    Ok(manager.list_jobs())
}

#[tauri::command]
pub async fn remove_conversion_job(
    job_id: String,
    manager: State<'_, SharedManager>,
) -> Result<(), String> {
    manager.queue.remove_job(&job_id).map_err(to_string_err)
}

#[tauri::command]
pub async fn clear_completed_jobs(manager: State<'_, SharedManager>) -> Result<(), String> {
    manager.queue.clear_completed().map_err(to_string_err)
}

// Legacy command for backward compatibility - enqueues and starts queue if not running
#[tauri::command]
pub async fn start_conversion(
    audiobook_id: String,
    settings: ConversionSettings,
    state: State<'_, AppState>,
    manager: State<'_, SharedManager>,
    app: AppHandle,
) -> Result<String, String> {
    enqueue_conversion(audiobook_id, settings, state, manager, app).await
}

#[tauri::command]
pub async fn cancel_conversion(
    job_id: String,
    manager: State<'_, SharedManager>,
) -> Result<(), String> {
    manager.queue.cancel_job(&job_id).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_conversion_status(
    job_id: String,
    manager: State<'_, SharedManager>,
    state: State<'_, AppState>,
) -> Result<ConversionJob, String> {
    manager
        .queue
        .get_job(&job_id)
        .or_else(|| state.get_job(&job_id))
        .ok_or_else(|| "Job not found".to_string())
}

/// DEPRECATED: Pause functionality is not implemented.
/// FFmpeg cannot be paused in a cross-platform way. This only updates the job status
/// in memory but does not actually pause the FFmpeg process.
/// These commands are kept for API compatibility but are not exposed in the UI.
#[tauri::command]
pub async fn pause_conversion(job_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.pause_job(&job_id).map_err(to_string_err)
}

/// DEPRECATED: Resume functionality is not implemented.
/// See pause_conversion documentation for details.
#[tauri::command]
pub async fn resume_conversion(job_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.resume_job(&job_id).map_err(to_string_err)
}

#[tauri::command]
pub async fn clear_pending_jobs(manager: State<'_, SharedManager>) -> Result<(), String> {
    manager.queue.clear_pending().map_err(to_string_err)
}

#[derive(serde::Serialize)]
pub struct QueueState {
    pub state: String,
    pub worker_count: usize,
    pub active_jobs: usize,
    pub pending_jobs: usize,
}

#[tauri::command]
pub async fn get_conversion_queue_state(
    manager: State<'_, SharedManager>,
) -> Result<QueueState, String> {
    let (state, pending, active, workers) = manager.get_queue_state();
    let state_str = match state {
        crate::conversion::runner::ManagerState::Stopped => "stopped",
        crate::conversion::runner::ManagerState::Running => "running",
        crate::conversion::runner::ManagerState::Stopping => "stopping",
    };
    Ok(QueueState {
        state: state_str.to_string(),
        worker_count: workers,
        active_jobs: active,
        pending_jobs: pending,
    })
}

#[tauri::command]
pub async fn retry_conversion(
    job_id: String,
    manager: State<'_, SharedManager>,
    app: AppHandle,
) -> Result<String, String> {
    let job = manager
        .queue
        .get_job(&job_id)
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if !job.status.is_terminal() {
        return Err("Can only retry completed, failed, or cancelled jobs".to_string());
    }

    let concurrent = (job.settings.concurrent_jobs as usize).max(1);
    let new_job = crate::models::ConversionJob::new(job.audiobook.clone(), job.settings.clone());
    let new_job_id = new_job.id.clone();

    manager.set_app_handle(app);
    manager
        .enqueue_job(new_job)
        .map_err(|e| format!("Failed to enqueue retry job: {}", e))?;

    let (queue_state, _, _, _) = manager.get_queue_state();
    if queue_state == crate::conversion::runner::ManagerState::Stopped {
        manager
            .start_queue(Some(concurrent))
            .map_err(|e| format!("Failed to start queue: {}", e))?;
    }

    Ok(new_job_id)
}
