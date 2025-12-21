//! Tauri commands for audio quality analysis and normalization

use camino::Utf8PathBuf;
use tauri::State;

use crate::audio::quality::{
    self, AudioLevels, NormalizationSettings, NormalizationSuggestion,
};
use crate::state::AppState;

/// Analyze audio levels of a file (loudness, peak, range)
#[tauri::command]
pub async fn analyze_audio_levels(
    file_path: String,
    _state: State<'_, AppState>,
) -> Result<AudioLevels, String> {
    let path = Utf8PathBuf::from(&file_path);
    quality::analyze_audio_levels(&path).map_err(|e| e.to_string())
}

/// Get normalization suggestions based on audio analysis
#[tauri::command]
pub async fn suggest_normalization(
    file_path: String,
    _state: State<'_, AppState>,
) -> Result<NormalizationSuggestion, String> {
    let path = Utf8PathBuf::from(&file_path);
    let levels = quality::analyze_audio_levels(&path).map_err(|e| e.to_string())?;
    Ok(quality::suggest_normalization(&levels))
}

/// Apply audio normalization to a file
#[tauri::command]
pub async fn apply_audio_normalization(
    input: String,
    output: String,
    settings: NormalizationSettings,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let input_path = Utf8PathBuf::from(&input);
    let output_path = Utf8PathBuf::from(&output);
    quality::apply_normalization(&input_path, &output_path, &settings).map_err(|e| e.to_string())
}
