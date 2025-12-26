//! Tauri commands for audio quality analysis and normalization

use camino::Utf8PathBuf;
use tauri::{AppHandle, State};

use crate::audio::quality::{
    self, AudioLevels, NormalizationSettings, NormalizationSuggestion,
};
use crate::models::{estimate_file_size_bytes, FileSizeEstimate, QualityPreset, QualityPresetInfo};
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

/// Apply audio normalization to a file with progress events
#[tauri::command]
pub async fn apply_audio_normalization_with_progress(
    app: AppHandle,
    input: String,
    output: String,
    settings: NormalizationSettings,
    total_duration_ms: u64,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let input_path = Utf8PathBuf::from(&input);
    let output_path = Utf8PathBuf::from(&output);
    quality::apply_normalization_with_progress(&input_path, &output_path, &settings, total_duration_ms, &app)
        .map_err(|e| e.to_string())
}

/// Estimate output file size based on duration and bitrate
#[tauri::command]
pub async fn estimate_output_size(
    duration_ms: u64,
    bitrate_kbps: u32,
) -> Result<FileSizeEstimate, String> {
    let bytes = estimate_file_size_bytes(duration_ms, bitrate_kbps);
    Ok(FileSizeEstimate::new(bytes))
}

/// Get quality preset info for all presets
#[tauri::command]
pub async fn get_quality_preset_info() -> Result<Vec<QualityPresetInfo>, String> {
    let presets = vec![
        QualityPresetInfo {
            preset: QualityPreset::High,
            name: "High".to_string(),
            description: QualityPreset::High.description().to_string(),
            bitrate_kbps: QualityPreset::High.bitrate_kbps(),
            sample_rate_hz: QualityPreset::High.sample_rate_hz(),
            is_vbr: QualityPreset::High.is_vbr(),
        },
        QualityPresetInfo {
            preset: QualityPreset::Medium,
            name: "Medium".to_string(),
            description: QualityPreset::Medium.description().to_string(),
            bitrate_kbps: QualityPreset::Medium.bitrate_kbps(),
            sample_rate_hz: QualityPreset::Medium.sample_rate_hz(),
            is_vbr: QualityPreset::Medium.is_vbr(),
        },
        QualityPresetInfo {
            preset: QualityPreset::Low,
            name: "Low".to_string(),
            description: QualityPreset::Low.description().to_string(),
            bitrate_kbps: QualityPreset::Low.bitrate_kbps(),
            sample_rate_hz: QualityPreset::Low.sample_rate_hz(),
            is_vbr: QualityPreset::Low.is_vbr(),
        },
        QualityPresetInfo {
            preset: QualityPreset::Custom,
            name: "Custom".to_string(),
            description: QualityPreset::Custom.description().to_string(),
            bitrate_kbps: QualityPreset::Custom.bitrate_kbps(),
            sample_rate_hz: QualityPreset::Custom.sample_rate_hz(),
            is_vbr: QualityPreset::Custom.is_vbr(),
        },
    ];
    Ok(presets)
}
