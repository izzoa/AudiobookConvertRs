use tauri::State;
use crate::models::{Metadata, Chapter};
use crate::metadata::editor::{self, ChapterUpdate};
use crate::state::AppState;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn update_audiobook_metadata(
    _audiobook_id: String, // Ideally verify ID matches current
    metadata: Metadata,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
    
    // We update the in-memory copy
    editor::update_audiobook_metadata(&mut current, metadata)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn add_chapter(
    _audiobook_id: String,
    chapter: Chapter,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
        
    editor::add_chapter(&mut current, chapter)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn update_chapter(
    _audiobook_id: String,
    index: usize,
    update: ChapterUpdate,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
        
    editor::update_chapter(&mut current, index, update)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_chapter(
    _audiobook_id: String,
    index: usize,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
        
    editor::delete_chapter(&mut current, index)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_chapters(
    _audiobook_id: String,
    old_index: usize,
    new_index: usize,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
        
    editor::reorder_chapters(&mut current, old_index, new_index)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn auto_create_chapters(
    _audiobook_id: String,
    count: usize,
    state: State<'_, AppState>
) -> Result<(), String> {
   let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;
    
    editor::auto_split_chapters(&mut current, count)
        .map_err(to_string_err)?;

    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn upload_artwork(
    _audiobook_id: String,
    image_path: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut current = state.get_active_audiobook()
        .ok_or("No audiobook loaded")?;

    let path = std::path::PathBuf::from(image_path);
    if !path.exists() {
        return Err("Image file not found".to_string());
    }
    
    editor::set_artwork(&mut current, path)
        .map_err(to_string_err)?;
        
    state.update_audiobook(current).map_err(to_string_err)?;
    Ok(())
}
