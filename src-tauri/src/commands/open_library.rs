use tauri::State;
use crate::metadata::open_library::{self, OpenLibrarySearchResult, OpenLibrarySelection};
use crate::metadata::editor;
use crate::models::AudioBook;
use crate::state::AppState;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn open_library_search(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<OpenLibrarySearchResult>, String> {
    open_library::search(&query, limit)
        .await
        .map_err(to_string_err)
}

#[tauri::command]
pub async fn open_library_apply(
    audiobook_id: String,
    selection: OpenLibrarySelection,
    download_cover: bool,
    state: State<'_, AppState>,
) -> Result<AudioBook, String> {
    // Get the audiobook
    let mut current = state.get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    // Apply the selection
    let (metadata, cover_bytes) = open_library::apply_selection(&selection, download_cover)
        .await
        .map_err(to_string_err)?;

    // Update the audiobook metadata
    editor::update_audiobook_metadata(&mut current, metadata)
        .map_err(to_string_err)?;

    // If we have cover bytes, set them
    if let Some(bytes) = cover_bytes {
        editor::set_artwork_bytes(&mut current, &bytes)
            .map_err(to_string_err)?;
    }

    // Save the updated audiobook
    state.update_audiobook(current.clone()).map_err(to_string_err)?;

    Ok(current)
}

