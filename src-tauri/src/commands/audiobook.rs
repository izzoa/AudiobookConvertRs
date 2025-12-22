use tauri::{State};
use std::path::PathBuf;
use camino::Utf8PathBuf;
use crate::models::{AudioBook};
use crate::state::AppState;
use crate::audio::batch::load_audiobook_from_files;

// Helper to convert Result to String for IPC
fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn create_audiobook_from_files(
    paths: Vec<String>,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let path_bufs: Vec<Utf8PathBuf> = paths.iter().map(Utf8PathBuf::from).collect();
    
    // Perform loading in blocking thread to avoid blocking async runtime
    let audiobook = tauri::async_runtime::spawn_blocking(move || {
        load_audiobook_from_files(&path_bufs)
    }).await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(to_string_err)?;

    // Update state
    state.add_audiobook(audiobook.clone());
    
    Ok(audiobook)
}

#[tauri::command]
pub async fn load_audiobook_file(
    path: String,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    // Placeholder for loading saved project JSON
    // For now we'll just return an error or basic implementation
    // Ideally this reads a JSON file and deserializes it
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("File not found".to_string());
    }

    let content = std::fs::read_to_string(&path_buf).map_err(to_string_err)?;
    let audiobook: AudioBook = serde_json::from_str(&content).map_err(to_string_err)?;

    state.add_audiobook(audiobook.clone());
    Ok(audiobook)
}

#[tauri::command]
pub async fn save_audiobook_project(
    audiobook: AudioBook,
    path: String
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&audiobook).map_err(to_string_err)?;
    std::fs::write(path, json).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_current_audiobook(
    state: State<'_, AppState>
) -> Result<(), String> {
    state.set_active_audiobook(None).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_active_audiobook(
    state: State<'_, AppState>
) -> Result<Option<AudioBook>, String> {
    Ok(state.get_active_audiobook())
}
