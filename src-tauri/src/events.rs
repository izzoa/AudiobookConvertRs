use serde::Serialize;
use tauri::{Emitter, Window};

#[derive(Clone, Serialize)]
pub struct ConversionProgress {
    pub job_id: String,
    pub current_file: String,
    pub file_progress: f32,
    pub total_progress: f32,
    pub time_remaining_sec: u32,
}

#[derive(Clone, Serialize)]
pub struct ConversionComplete {
    pub job_id: String,
    pub output_path: String,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct ConversionError {
    pub job_id: String,
    pub error_message: String,
    pub recoverable: bool,
}

#[derive(Clone, Serialize)]
pub struct FilesLoaded {
    pub file_count: usize,
    pub total_duration_ms: u64,
}

pub fn emit_progress(window: &Window, event: ConversionProgress) -> tauri::Result<()> {
    window.emit("conversion_progress", event)
}

pub fn emit_completion(window: &Window, event: ConversionComplete) -> tauri::Result<()> {
    window.emit("conversion_complete", event)
}

pub fn emit_error(window: &Window, event: ConversionError) -> tauri::Result<()> {
    window.emit("conversion_error", event)
}

pub fn emit_files_loaded(window: &Window, event: FilesLoaded) -> tauri::Result<()> {
    window.emit("files_loaded", event)
}
