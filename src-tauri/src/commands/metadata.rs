use tauri::State;
use crate::models::{Metadata, Chapter, AudioBook};
use crate::metadata::editor::{self, ChapterUpdate};
use crate::state::AppState;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn update_audiobook_metadata(
    audiobook_id: String,
    metadata: Metadata,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    
    // Update the in-memory copy
    editor::update_audiobook_metadata(&mut current, metadata)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn add_chapter(
    audiobook_id: String,
    chapter: Chapter,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
        
    editor::add_chapter(&mut current, chapter)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn update_chapter(
    audiobook_id: String,
    index: usize,
    update: ChapterUpdate,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
        
    editor::update_chapter(&mut current, index, update)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn delete_chapter(
    audiobook_id: String,
    index: usize,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
        
    editor::delete_chapter(&mut current, index)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn reorder_chapters(
    audiobook_id: String,
    old_index: usize,
    new_index: usize,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
        
    editor::reorder_chapters(&mut current, old_index, new_index)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn auto_create_chapters(
    audiobook_id: String,
    count: usize,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    
    editor::auto_split_chapters(&mut current, count)
        .map_err(to_string_err)?;

    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn upload_artwork(
    audiobook_id: String,
    image_path: String,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    let path = std::path::PathBuf::from(image_path);
    if !path.exists() {
        return Err("Image file not found".to_string());
    }
    
    editor::set_artwork(&mut current, path)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn set_artwork_base64(
    audiobook_id: String,
    artwork_base64: String,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Decode base64
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let bytes = STANDARD.decode(&artwork_base64)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    // Process artwork (includes CPU-intensive image processing)
    // Note: For production, consider wrapping the heavy image processing
    // in spawn_blocking, but for now this is acceptable
    editor::set_artwork_bytes(&mut current, &bytes)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn clear_artwork(
    audiobook_id: String,
    state: State<'_, AppState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    editor::clear_artwork(&mut current)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}
