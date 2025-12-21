use tauri::{State, Window};
use crate::models::{ConversionSettings, ConversionJob};
use crate::state::AppState;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn start_conversion(
    audiobook_id: String, 
    settings: ConversionSettings,
    state: State<'_, AppState>,
    _window: Window // Will be used for events later
) -> Result<String, String> {
    let current_opt = state.get_active_audiobook();
    
    let audiobook = match current_opt {
        Some(ab) if ab.id == audiobook_id => ab,
        _ => return Err("Audiobook not found or not loaded".to_string())
    };

    // basic validation
    if settings.output_path.as_str().is_empty() {
        return Err("Output path is required".to_string());
    }
    
    // Create Job
    let job = ConversionJob::new(audiobook, settings);
    let job_id = job.id.clone();
    
    // Add to queue in state
    state.add_job(job);
    
    // Trigger processing (Phase 5 will implement the actual worker trigger)
    // For now we just queue it.
    
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
