use tauri::State;
use crate::models::{Metadata, Chapter, AudioBook};
use crate::metadata::editor::{self, ChapterUpdate};
use crate::metadata::chapter_io::{ChapterFormat, parse_chapters, render_chapters};
use crate::metadata::chapters::validate_chapters;
use crate::metadata::chapter_generation::{extract_chapters_from_sources, rename_chapters_regex, get_regex_presets};
use crate::state::AppState;
use crate::commands::undo::UndoState;
use crate::undo::EditCommand;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn update_audiobook_metadata(
    audiobook_id: String,
    metadata: Metadata,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    
    let old_metadata = current.output_metadata.clone();
    
    // Create edit command
    let command = EditCommand::UpdateMetadata {
        audiobook_id: audiobook_id.clone(),
        old_metadata,
        new_metadata: metadata,
    };
    
    // Execute via undo manager
    undo_state.execute(&mut current, command)
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

#[tauri::command]
pub async fn import_chapters(
    audiobook_id: String,
    format: String,
    content: String,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Parse format
    let chapter_format = match format.to_lowercase().as_str() {
        "audacity" => ChapterFormat::Audacity,
        "ffmetadata" => ChapterFormat::FFmetadata,
        "csv" => ChapterFormat::Csv,
        _ => return Err(format!("Unknown format: {}", format)),
    };

    // Parse chapters from content
    let chapters = parse_chapters(&content, chapter_format)
        .map_err(to_string_err)?;

    // Validate chapters against total duration
    validate_chapters(&chapters, current.total_duration_ms)
        .map_err(to_string_err)?;

    let old_chapters = current.chapters.clone();
    
    // Create edit command for undo
    let command = EditCommand::ReplaceChapters {
        audiobook_id: audiobook_id.clone(),
        old_chapters,
        new_chapters: chapters,
    };
    
    // Execute via undo manager
    undo_state.execute(&mut current, command)
        .map_err(to_string_err)?;

    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn export_chapters(
    audiobook_id: String,
    format: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Parse format
    let chapter_format = match format.to_lowercase().as_str() {
        "audacity" => ChapterFormat::Audacity,
        "ffmetadata" => ChapterFormat::FFmetadata,
        "csv" => ChapterFormat::Csv,
        _ => return Err(format!("Unknown format: {}", format)),
    };

    // Render chapters to string
    let content = render_chapters(&current.chapters, chapter_format);
    Ok(content)
}

#[tauri::command]
pub async fn set_chapters(
    audiobook_id: String,
    chapters: Vec<Chapter>,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Validate chapters
    validate_chapters(&chapters, current.total_duration_ms)
        .map_err(to_string_err)?;

    let old_chapters = current.chapters.clone();
    
    // Create edit command
    let command = EditCommand::ReplaceChapters {
        audiobook_id: audiobook_id.clone(),
        old_chapters,
        new_chapters: chapters,
    };
    
    // Execute via undo manager
    undo_state.execute(&mut current, command)
        .map_err(to_string_err)?;

    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn generate_chapters_from_source(
    audiobook_id: String,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Extract chapters from source files
    let chapters = extract_chapters_from_sources(&current.files)
        .map_err(to_string_err)?;

    if chapters.is_empty() {
        return Err("No chapters found in source files".to_string());
    }

    let old_chapters = current.chapters.clone();
    
    // Create edit command
    let command = EditCommand::ReplaceChapters {
        audiobook_id: audiobook_id.clone(),
        old_chapters,
        new_chapters: chapters,
    };
    
    // Execute via undo manager
    undo_state.execute(&mut current, command)
        .map_err(to_string_err)?;

    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[tauri::command]
pub async fn rename_chapters_with_regex(
    audiobook_id: String,
    pattern: String,
    replacement: String,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>
) -> Result<AudioBook, String> {
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    let old_chapters = current.chapters.clone();
    
    // Apply regex rename to a clone
    let mut new_chapters = current.chapters.clone();
    rename_chapters_regex(&mut new_chapters, &pattern, &replacement)
        .map_err(to_string_err)?;

    // Create edit command
    let command = EditCommand::ReplaceChapters {
        audiobook_id: audiobook_id.clone(),
        old_chapters,
        new_chapters,
    };
    
    // Execute via undo manager
    undo_state.execute(&mut current, command)
        .map_err(to_string_err)?;

    state.update_audiobook(current.clone()).map_err(to_string_err)?;
    Ok(current)
}

#[derive(serde::Serialize)]
pub struct RegexPresetInfo {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub replacement: String,
}

#[tauri::command]
pub async fn get_chapter_regex_presets() -> Result<Vec<RegexPresetInfo>, String> {
    let presets = get_regex_presets();
    let info: Vec<RegexPresetInfo> = presets.into_iter().map(|p| RegexPresetInfo {
        name: p.name.to_string(),
        description: p.description.to_string(),
        pattern: p.pattern.to_string(),
        replacement: p.replacement.to_string(),
    }).collect();
    Ok(info)
}
