use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Window};

use crate::conversion::progress::ConversionStage;

#[derive(Clone, Serialize)]
pub struct ConversionProgress {
    pub job_id: String,
    pub file_index: usize,
    pub current_file: String,
    pub file_progress: f32,
    pub total_progress: f32,
    pub time_remaining_sec: u32,
    pub estimated_finish: Option<DateTime<Utc>>,
    pub speed: Option<f32>,
    pub stage: ConversionStage,
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

#[derive(Clone, Serialize)]
pub struct MetadataUpdated {
    pub audiobook_id: String,
}

/// Emit progress event to all windows via AppHandle for reliability
pub fn emit_progress(window: &Window, event: ConversionProgress) -> tauri::Result<()> {
    // Use app_handle().emit() to emit to all windows - more reliable from background threads
    window.app_handle().emit("conversion_progress", event)
}

pub fn emit_completion(window: &Window, event: ConversionComplete) -> tauri::Result<()> {
    window.app_handle().emit("conversion_complete", event)
}

pub fn emit_error(window: &Window, event: ConversionError) -> tauri::Result<()> {
    window.app_handle().emit("conversion_error", event)
}

pub fn emit_files_loaded(window: &Window, event: FilesLoaded) -> tauri::Result<()> {
    window.app_handle().emit("files_loaded", event)
}

pub fn emit_metadata_updated(window: &Window, event: MetadataUpdated) -> tauri::Result<()> {
    window.app_handle().emit("metadata_updated", event)
}
