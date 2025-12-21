use tauri::State;
use crate::config::{self, UserSettings};
use crate::state::AppState;
use crate::ffmpeg::detection;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
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
