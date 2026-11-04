use crate::audio::player::{PlaybackStatus, SharedPlayer};
use camino::Utf8PathBuf;
use tauri::State;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn play_audio(
    path: String,
    start_ms: Option<u64>,
    player: State<'_, SharedPlayer>,
) -> Result<(), String> {
    let utf8_path = Utf8PathBuf::from(&path);
    player.play(&utf8_path, start_ms).map_err(to_string_err)
}

#[tauri::command]
pub async fn pause_audio(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.pause().map_err(to_string_err)
}

#[tauri::command]
pub async fn resume_audio(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.resume().map_err(to_string_err)
}

#[tauri::command]
pub async fn stop_audio(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.stop().map_err(to_string_err)
}

#[tauri::command]
pub async fn seek_audio(position_ms: u64, player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.seek(position_ms).map_err(to_string_err)
}

#[tauri::command]
pub async fn set_audio_volume(volume: f32, player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.set_volume(volume).map_err(to_string_err)
}

#[tauri::command]
pub async fn get_playback_status(
    player: State<'_, SharedPlayer>,
) -> Result<PlaybackStatus, String> {
    Ok(player.get_status())
}

/// The book's audio around `at_ms` (book time), `span_ms` either side, as a
/// waveform with its pauses, for checking where a chapter starts.
#[tauri::command]
pub async fn get_waveform(
    audiobook_id: String,
    at_ms: u64,
    span_ms: u64,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<crate::audio::waveform::Waveform, String> {
    let book = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg()
        .map_err(to_string_err)?
        .path;
    let from_ms = at_ms.saturating_sub(span_ms);
    let to_ms = (at_ms + span_ms).min(book.total_duration_ms);
    tauri::async_runtime::spawn_blocking(move || {
        crate::audio::waveform::waveform(&ffmpeg, &book.files, from_ms, to_ms)
    })
    .await
    .map_err(to_string_err)?
    .map_err(to_string_err)
}
