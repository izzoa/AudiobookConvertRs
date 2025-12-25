use tauri::{State, Window, AppHandle, Manager};
use crate::models::{ConversionSettings, ConversionJob, JobStatus};
use crate::state::AppState;
use crate::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use crate::conversion::runner::SharedManager;
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::events::{emit_progress, emit_completion, emit_error};
use std::thread;
use tracing::{info, error};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn enqueue_conversion(
    audiobook_id: String,
    settings: ConversionSettings,
    state: State<'_, AppState>,
    manager: State<'_, SharedManager>,
) -> Result<String, String> {
    // Get audiobook by ID - validate it exists
    let audiobook = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Basic validation
    if settings.output_path.as_str().is_empty() {
        return Err("Output path is required".to_string());
    }

    // Create Job
    let job = ConversionJob::new(audiobook, settings);
    let job_id = job.id.clone();

    // Enqueue the job
    manager.enqueue_job(job)
        .map_err(|e| format!("Failed to enqueue job: {}", e))?;

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
    manager.start_queue(max_concurrent)
        .map_err(|e| format!("Failed to start queue: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn stop_conversion_queue(
    manager: State<'_, SharedManager>,
) -> Result<(), String> {
    manager.stop_queue()
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
    // For now, just return OK as this isn't fully implemented
    // TODO: Add remove functionality to ConversionManager
    Ok(())
}

#[tauri::command]
pub async fn clear_completed_jobs(
    manager: State<'_, SharedManager>,
) -> Result<(), String> {
    // This would need to be implemented in the ConversionManager
    // For now, return OK
    Ok(())
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
    // Enqueue the job
    let job_id = enqueue_conversion(audiobook_id, settings, state, manager.clone()).await?;

    // Check if queue is already running
    let (state, _, _, _) = manager.get_queue_state();
    if state != crate::conversion::runner::ManagerState::Running {
        // Start the queue with default concurrency
        start_conversion_queue(Some(1), manager, app).await?;
    }

    Ok(job_id)
}

#[tauri::command]
pub async fn cancel_conversion(
    job_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    // Logic to cancel job
    // This requires communicating with the worker (Phase 5)
    // For now, we update status if it's in queue
    state.cancel_job(&job_id).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_conversion_status(
    job_id: String,
    state: State<'_, AppState>
) -> Result<ConversionJob, String> {
    state.get_job(&job_id).ok_or("Job not found".to_string())
}

/// DEPRECATED: Pause functionality is not implemented.
/// FFmpeg cannot be paused in a cross-platform way. This only updates the job status
/// in memory but does not actually pause the FFmpeg process.
/// These commands are kept for API compatibility but are not exposed in the UI.
#[tauri::command]
pub async fn pause_conversion(
    job_id: String,
    state: State<'_, AppState>
) -> Result<bool, String> {
    state.pause_job(&job_id).map_err(to_string_err)
}

/// DEPRECATED: Resume functionality is not implemented.
/// See pause_conversion documentation for details.
#[tauri::command]
pub async fn resume_conversion(
    job_id: String,
    state: State<'_, AppState>
) -> Result<bool, String> {
    state.resume_job(&job_id).map_err(to_string_err)
}
