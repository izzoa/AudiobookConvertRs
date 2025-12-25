use tauri::State;
use camino::Utf8PathBuf;
use crate::audio::player::{SharedPlayer, PlaybackStatus};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn play_audio(
    path: String,
    start_ms: Option<u64>,
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    let utf8_path = Utf8PathBuf::from(&path);
    player.play(&utf8_path, start_ms).map_err(to_string_err)
}

#[tauri::command]
pub async fn pause_audio(
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    player.pause().map_err(to_string_err)
}

#[tauri::command]
pub async fn resume_audio(
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    player.resume().map_err(to_string_err)
}

#[tauri::command]
pub async fn stop_audio(
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    player.stop().map_err(to_string_err)
}

#[tauri::command]
pub async fn seek_audio(
    position_ms: u64,
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    player.seek(position_ms).map_err(to_string_err)
}

#[tauri::command]
pub async fn set_audio_volume(
    volume: f32,
    player: State<'_, SharedPlayer>
) -> Result<(), String> {
    player.set_volume(volume).map_err(to_string_err)
}

#[tauri::command]
pub async fn get_playback_status(
    player: State<'_, SharedPlayer>
) -> Result<PlaybackStatus, String> {
    Ok(player.get_status())
}
