use serde::Serialize;
use tauri::State;
use crate::config::{self, UserSettings};
use crate::state::AppState;
use crate::ffmpeg::detection;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

/// Serializable FFmpeg info for the frontend
#[derive(Debug, Clone, Serialize)]
pub struct FFmpegStatus {
    /// Whether FFmpeg is available
    pub available: bool,
    /// Path to ffmpeg executable
    pub path: Option<String>,
    /// Version string
    pub version: Option<String>,
    /// Whether libfdk_aac encoder is available (better quality)
    pub supports_libfdk_aac: bool,
}

#[tauri::command]
pub async fn get_settings(
    _state: State<'_, AppState>
) -> Result<UserSettings, String> {
    Ok(config::get_settings())
}

#[tauri::command]
pub async fn update_settings(
    settings: UserSettings,
    _state: State<'_, AppState>
) -> Result<(), String> {
    // config::update_settings handles validation and saving/updating global state
    config::update_settings(settings).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_default_settings() -> Result<UserSettings, String> {
    Ok(UserSettings::default())
}

#[tauri::command]
pub async fn check_ffmpeg_available(
    state: State<'_, AppState>
) -> Result<bool, String> {
    // Check cache in state first
    if state.is_ffmpeg_available() {
        return Ok(true);
    }
    
    // Check system
    match detection::detect_ffmpeg() {
        Ok(info) => {
            state.set_ffmpeg_status(true, Some(info.version));
            Ok(true)
        },
        Err(_) => {
            state.set_ffmpeg_status(false, None);
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn reset_settings(
    _state: State<'_, AppState>
) -> Result<(), String> {
    let defaults = UserSettings::default();
    config::update_settings(defaults).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_ffmpeg_version(
    state: State<'_, AppState>
) -> Result<String, String> {
    // Check state first
    if let Some(v) = state.get_ffmpeg_version() {
        return Ok(v);
    }

    // Try detecting
    detection::detect_ffmpeg()
        .map(|info| {
            // Cache it
            state.set_ffmpeg_status(true, Some(info.version.clone()));
            info.version
        })
        .map_err(to_string_err)
}

#[tauri::command]
pub async fn get_ffmpeg_info(
    state: State<'_, AppState>
) -> Result<FFmpegStatus, String> {
    match detection::detect_ffmpeg() {
        Ok(info) => {
            state.set_ffmpeg_status(true, Some(info.version.clone()));
            Ok(FFmpegStatus {
                available: true,
                path: Some(info.path.to_string()),
                version: Some(info.version),
                supports_libfdk_aac: info.supports_libfdk_aac,
            })
        }
        Err(_) => {
            state.set_ffmpeg_status(false, None);
            Ok(FFmpegStatus {
                available: false,
                path: None,
                version: None,
                supports_libfdk_aac: false,
            })
        }
    }
}
