//! Speech model: its status, getting it on request, and removing it.

use crate::ffmpeg::download::DownloadError;
use crate::speech::model::{self, SpeechModelStatus, MODEL};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

/// Set while a download runs; only one runs at a time
static DOWNLOAD_RUNNING: AtomicBool = AtomicBool::new(false);
/// Set by `cancel_speech_model_download`, checked on every chunk
static DOWNLOAD_CANCEL: AtomicBool = AtomicBool::new(false);

/// Clears `DOWNLOAD_RUNNING` however the download ends
struct DownloadRunning;

impl Drop for DownloadRunning {
    fn drop(&mut self) {
        DOWNLOAD_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Payload of the `speech_model_download_progress` event
#[derive(Debug, Clone, Serialize)]
pub struct SpeechModelDownloadProgress {
    pub received: u64,
    pub total: u64,
}

#[tauri::command]
pub async fn get_speech_model_status() -> Result<SpeechModelStatus, String> {
    Ok(model::status())
}

/// Download, verify and install the speech model. Progress arrives as
/// `speech_model_download_progress`; returns the new status.
#[tauri::command]
pub async fn download_speech_model(app: AppHandle) -> Result<SpeechModelStatus, String> {
    let root =
        model::model_root().ok_or_else(|| "could not find the app's data folder".to_string())?;
    if DOWNLOAD_RUNNING.swap(true, Ordering::SeqCst) {
        return Err(DownloadError::Busy.to_string());
    }
    let _running = DownloadRunning;
    DOWNLOAD_CANCEL.store(false, Ordering::SeqCst);

    let mut last_sent: Option<Instant> = None;
    let progress = |received: u64, total: u64| {
        // About ten updates a second; the final one always goes out
        if received < total && last_sent.is_some_and(|t| t.elapsed() < Duration::from_millis(100)) {
            return;
        }
        last_sent = Some(Instant::now());
        let _ = app.emit(
            "speech_model_download_progress",
            SpeechModelDownloadProgress { received, total },
        );
    };

    model::download(&MODEL, &root, &DOWNLOAD_CANCEL, progress)
        .await
        .map_err(to_string_err)?;
    Ok(model::status())
}

/// Stop a running download; it cleans up and returns "download cancelled"
#[tauri::command]
pub async fn cancel_speech_model_download() -> Result<(), String> {
    DOWNLOAD_CANCEL.store(true, Ordering::SeqCst);
    Ok(())
}

/// Delete the speech model. Returns the new status.
#[tauri::command]
pub async fn remove_speech_model() -> Result<SpeechModelStatus, String> {
    if DOWNLOAD_RUNNING.load(Ordering::SeqCst) {
        return Err(DownloadError::Busy.to_string());
    }
    if let Some(root) = model::model_root() {
        model::remove(&root).map_err(to_string_err)?;
    }
    Ok(model::status())
}
