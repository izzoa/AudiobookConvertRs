use tauri::{State, Window};
use crate::models::{ConversionSettings, ConversionJob, JobStatus};
use crate::state::AppState;
use crate::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::events::{emit_progress, emit_completion, emit_error};
use std::thread;
use tracing::{info, error};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn start_conversion(
    audiobook_id: String, 
    settings: ConversionSettings,
    state: State<'_, AppState>,
    window: Window
) -> Result<String, String> {
    // Get audiobook by ID - validate it exists
    let audiobook = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Basic validation
    if settings.output_path.as_str().is_empty() {
        return Err("Output path is required".to_string());
    }
    
    // Detect FFmpeg
    let ffmpeg_info = detect_ffmpeg().map_err(|e| format!("FFmpeg not found: {}", e))?;
    
    // Create Job
    let mut job = ConversionJob::new(audiobook, settings);
    let job_id = job.id.clone();
    job.start(); // Mark as running
    
    // Add to queue in state
    state.add_job(job.clone());
    state.set_active_job(Some(job_id.clone()));
    
    // Clone state for thread (AppState implements Clone via Arc)
    let state_clone: AppState = (*state).clone();
    
    // Spawn worker thread to process the job
    let job_id_clone = job_id.clone();
    let window_clone = window.clone();
    
    thread::spawn(move || {
        info!("Starting conversion worker for job {}", job_id_clone);
        
        // Clone window for panic handler
        let window_for_panic = window_clone.clone();
        let job_id_for_panic = job_id_clone.clone();
        let state_for_panic = state_clone.clone();
        
        // Wrap everything in catch_unwind to handle panics
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Create pipeline
            info!("Creating conversion pipeline");
            let pipeline = ConversionPipeline::new(ffmpeg_info, PipelineConfig::default());
            
            // Progress callback - emit events to frontend
            let window_progress = window_clone.clone();
            let job_id_progress = job_id_clone.clone();
            let state_progress = state_clone.clone();
            let progress_callback = move |progress: crate::conversion::progress::ConversionProgress| {
                info!("Emitting progress: {}% for job {}", progress.total_progress, job_id_progress);

                // Persist progress for UI polling fallback + status queries
                let job_id_for_update = job_id_progress.clone();
                let stage_message = progress.stage.to_string();
                let speed_str = progress.speed.map(|s| format!("{:.2}x", s));
                if let Err(e) = state_progress.update_job_with(&job_id_for_update, |j| {
                    j.progress_percent = progress.total_progress;
                    j.current_file_index = progress.file_index;
                    j.status_message = Some(stage_message.clone());
                    j.progress = Some(crate::models::ConversionProgress {
                        job_id: job_id_for_update.clone(),
                        file_index: progress.file_index,
                        current_file: progress.current_file.clone(),
                        file_progress: progress.file_progress,
                        total_progress: progress.total_progress,
                        time_remaining_sec: Some(progress.time_remaining_sec),
                        speed: speed_str.clone(),
                        status_message: stage_message.clone(),
                    });
                }) {
                    error!("Failed to persist progress to state: {}", e);
                }

                if let Err(e) = emit_progress(&window_progress, crate::events::ConversionProgress {
                    job_id: job_id_progress.clone(),
                    current_file: progress.current_file.clone(),
                    file_progress: progress.file_progress,
                    total_progress: progress.total_progress,
                    time_remaining_sec: progress.time_remaining_sec,
                }) {
                    error!("Failed to emit progress: {}", e);
                }
            };
            
        // Cancel check - consult shared state for cancellation flag
        let cancel_state = state_clone.clone();
        let cancel_job_id = job_id_clone.clone();
        let cancel_check = move || {
            if let Some(job) = cancel_state.get_job(&cancel_job_id) {
                job.status == JobStatus::Cancelled
            } else {
                false
            }
        };
            
            // Run the pipeline
            info!("Running conversion pipeline for {} files", job.audiobook.files.len());
            let start_time = std::time::Instant::now();
            match pipeline.run(&job, progress_callback, cancel_check) {
                Ok(output_path) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    info!("Conversion completed: {} ({}ms)", output_path, duration_ms);
                    
                    // Emit completion event
                    if let Err(e) = emit_completion(&window_clone, crate::events::ConversionComplete {
                        job_id: job_id_clone.clone(),
                        output_path: output_path.to_string(),
                        duration_ms,
                    }) {
                        error!("Failed to emit completion: {}", e);
                    }
                    
                    // Update job status in state
                    let _ = state_clone.update_job_with(&job_id_clone, |j| {
                        j.complete();
                        j.output_path = Some(output_path.to_string());
                    });
                    state_clone.set_active_job(None);
                }
                Err(e) => {
                    error!("Conversion failed: {}", e);
                    
                    // Emit error event
                    if let Err(err) = emit_error(&window_clone, crate::events::ConversionError {
                        job_id: job_id_clone.clone(),
                        error_message: e.to_string(),
                        recoverable: false,
                    }) {
                        error!("Failed to emit error: {}", err);
                    }
                    
                    // Update job status in state
                    let _ = state_clone.update_job_with(&job_id_clone, |j| {
                        j.fail(e.to_string());
                    });
                    state_clone.set_active_job(None);
                }
            }
        }));
        
        // Handle panic
        if let Err(panic_info) = result {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            
            error!("Conversion thread panicked: {}", panic_msg);
            
            // Emit error event
            if let Err(err) = emit_error(&window_for_panic, crate::events::ConversionError {
                job_id: job_id_for_panic.clone(),
                error_message: format!("Internal error (panic): {}", panic_msg),
                recoverable: false,
            }) {
                error!("Failed to emit panic error: {}", err);
            }
            
            // Update job status in state
            let _ = state_for_panic.update_job_with(&job_id_for_panic, |j| {
                j.fail(format!("Internal error: {}", panic_msg));
            });
            state_for_panic.set_active_job(None);
        }
    });
    
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

#[tauri::command]
pub async fn pause_conversion(
    job_id: String,
    state: State<'_, AppState>
) -> Result<bool, String> {
    state.pause_job(&job_id).map_err(to_string_err)
}

#[tauri::command]
pub async fn resume_conversion(
    job_id: String,
    state: State<'_, AppState>
) -> Result<bool, String> {
    state.resume_job(&job_id).map_err(to_string_err)
}
