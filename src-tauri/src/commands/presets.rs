//! Tauri commands for conversion presets and dry-run analysis

use camino::Utf8PathBuf;
use tauri::State;

use crate::conversion::batch::{
    ConversionPreset, DryRunResult, PresetManager, dry_run_analysis,
};
use crate::models::ConversionSettings;
use crate::state::AppState;

/// Get the presets file path in user config directory
fn get_presets_path() -> Utf8PathBuf {
    dirs::config_dir()
        .map(|p| Utf8PathBuf::from_path_buf(p).unwrap_or_else(|_| Utf8PathBuf::from(".")))
        .unwrap_or_else(|| Utf8PathBuf::from("."))
        .join("audiobookconverter")
        .join("presets.json")
}

/// List all saved presets
#[tauri::command]
pub async fn list_presets(_state: State<'_, AppState>) -> Result<Vec<ConversionPreset>, String> {
    let path = get_presets_path();
    let manager = PresetManager::load(&path).map_err(|e| e.to_string())?;
    Ok(manager.list_presets().to_vec())
}

/// Get a preset by name
#[tauri::command]
pub async fn get_preset(
    name: String,
    _state: State<'_, AppState>,
) -> Result<ConversionPreset, String> {
    let path = get_presets_path();
    let manager = PresetManager::load(&path).map_err(|e| e.to_string())?;
    manager
        .get_preset(&name)
        .cloned()
        .ok_or_else(|| format!("Preset '{}' not found", name))
}

/// Save a preset (creates new or updates existing)
#[tauri::command]
pub async fn save_preset(
    preset: ConversionPreset,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let path = get_presets_path();
    let mut manager = PresetManager::load(&path).map_err(|e| e.to_string())?;
    manager.save_preset(preset);
    manager.save(&path).map_err(|e| e.to_string())
}

/// Delete a preset by name
#[tauri::command]
pub async fn delete_preset(
    name: String,
    _state: State<'_, AppState>,
) -> Result<bool, String> {
    let path = get_presets_path();
    let mut manager = PresetManager::load(&path).map_err(|e| e.to_string())?;
    let deleted = manager.delete_preset(&name);
    manager.save(&path).map_err(|e| e.to_string())?;
    Ok(deleted)
}

/// Perform a dry-run analysis of conversion settings
#[tauri::command]
pub async fn dry_run_conversion(
    settings: ConversionSettings,
    state: State<'_, AppState>,
) -> Result<DryRunResult, String> {
    let audiobook = state
        .get_active_audiobook()
        .ok_or("No active audiobook loaded")?;

    Ok(dry_run_analysis(&audiobook, &settings))
}
