//! Tauri commands for undo/redo functionality

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::State;

use crate::state::AppState;
use crate::undo::UndoManager;

/// Thread-safe wrapper for UndoManager
pub struct UndoState {
    pub(crate) inner: Arc<RwLock<UndoManager>>,
}

impl Default for UndoState {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(UndoManager::new())),
        }
    }
    
    /// Execute a command and add it to the undo stack
    pub fn execute(&self, audiobook: &mut crate::models::AudioBook, command: crate::undo::EditCommand) -> Result<(), crate::error::AppError> {
        self.inner.write().execute(audiobook, command)
    }
}

/// Undo the last action
#[tauri::command]
pub async fn undo(
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>,
) -> Result<Option<String>, String> {
    let audiobook_id = state
        .get_active_audiobook_id()
        .ok_or("No active audiobook")?;

    let mut audiobook = state
        .get_audiobook(&audiobook_id)
        .ok_or("Audiobook not found")?;

    let mut undo_manager = undo_state.inner.write();
    let description = undo_manager.undo(&mut audiobook).map_err(|e| e.to_string())?;

    state.update_audiobook(audiobook).map_err(|e| e.to_string())?;
    Ok(description)
}

/// Redo the last undone action
#[tauri::command]
pub async fn redo(
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>,
) -> Result<Option<String>, String> {
    let audiobook_id = state
        .get_active_audiobook_id()
        .ok_or("No active audiobook")?;

    let mut audiobook = state
        .get_audiobook(&audiobook_id)
        .ok_or("Audiobook not found")?;

    let mut undo_manager = undo_state.inner.write();
    let description = undo_manager.redo(&mut audiobook).map_err(|e| e.to_string())?;

    state.update_audiobook(audiobook).map_err(|e| e.to_string())?;
    Ok(description)
}

/// Check if undo is available
#[tauri::command]
pub async fn can_undo(undo_state: State<'_, UndoState>) -> Result<bool, String> {
    Ok(undo_state.inner.read().can_undo())
}

/// Check if redo is available
#[tauri::command]
pub async fn can_redo(undo_state: State<'_, UndoState>) -> Result<bool, String> {
    Ok(undo_state.inner.read().can_redo())
}

/// Get description of the next undo action
#[tauri::command]
pub async fn get_undo_description(undo_state: State<'_, UndoState>) -> Result<Option<String>, String> {
    Ok(undo_state.inner.read().peek_undo())
}

/// Get description of the next redo action
#[tauri::command]
pub async fn get_redo_description(undo_state: State<'_, UndoState>) -> Result<Option<String>, String> {
    Ok(undo_state.inner.read().peek_redo())
}

/// Response for undo state query
#[derive(Debug, Clone, serde::Serialize)]
pub struct UndoStateResponse {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_description: Option<String>,
    pub redo_description: Option<String>,
}

/// Get complete undo state
#[tauri::command]
pub async fn get_undo_state(undo_state: State<'_, UndoState>) -> Result<UndoStateResponse, String> {
    let manager = undo_state.inner.read();
    Ok(UndoStateResponse {
        can_undo: manager.can_undo(),
        can_redo: manager.can_redo(),
        undo_description: manager.peek_undo(),
        redo_description: manager.peek_redo(),
    })
}
