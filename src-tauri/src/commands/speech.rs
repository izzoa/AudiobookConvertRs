//! Speech model: its status, getting it on request, and removing it.

use crate::commands::undo::UndoState;
use crate::error::AppError;
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::ffmpeg::download::DownloadError;
use crate::models::AudioBook;
use crate::speech::detect::{Detection, Stage};
use crate::speech::model::{self, SpeechModelStatus, MODEL};
use crate::speech::{whisper_language, Transcriber};
use crate::state::AppState;
use crate::undo::EditCommand;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

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

/// Set while chapters are being detected from speech; one runs at a time
static DETECT_RUNNING: AtomicBool = AtomicBool::new(false);
/// Set by `cancel_spoken_chapters`
static DETECT_CANCEL: AtomicBool = AtomicBool::new(false);

struct DetectRunning;

impl Drop for DetectRunning {
    fn drop(&mut self) {
        DETECT_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Payload of the `spoken_chapters_progress` event
#[derive(Debug, Clone, Serialize)]
pub struct SpokenChaptersProgress {
    pub audiobook_id: String,
    pub stage: Stage,
    pub done: usize,
    pub total: usize,
}

/// Replace the book's chapters with ones made where a narrator is heard
/// announcing a heading. Needs the speech model. Progress arrives as
/// `spoken_chapters_progress`; a cancelled run returns "cancelled".
#[tauri::command]
pub async fn detect_spoken_chapters(
    audiobook_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>,
) -> Result<AudioBook, String> {
    let book = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    if book.files.is_empty() {
        return Err("the book has no files to listen to".to_string());
    }
    let model_path =
        model::installed_model().ok_or_else(|| "the speech model is not installed".to_string())?;
    let ffmpeg = detect_ffmpeg().map_err(to_string_err)?.path;
    if DETECT_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("chapters are already being detected".to_string());
    }
    let _running = DetectRunning;
    DETECT_CANCEL.store(false, Ordering::SeqCst);

    let language = book
        .output_metadata
        .language
        .as_deref()
        .and_then(whisper_language);
    let files = book.files.clone();
    let id = audiobook_id.clone();
    let chapters = tauri::async_runtime::spawn_blocking(move || {
        let transcriber = Transcriber::load(&model_path)?;
        let detection = Detection {
            transcriber: &transcriber,
            ffmpeg: &ffmpeg,
            files: &files,
            language,
            cancel: &DETECT_CANCEL,
        };
        let mut last_sent: Option<Instant> = None;
        detection.run(|stage, done, total| {
            if done > 0
                && done < total
                && last_sent.is_some_and(|t| t.elapsed() < Duration::from_millis(200))
            {
                return;
            }
            last_sent = Some(Instant::now());
            let _ = app.emit(
                "spoken_chapters_progress",
                SpokenChaptersProgress {
                    audiobook_id: id.clone(),
                    stage,
                    done,
                    total,
                },
            );
        })
    })
    .await
    .map_err(to_string_err)?
    .map_err(|e| match e {
        AppError::Speech(message) => message,
        other => other.to_string(),
    })?;

    // The book may have changed while listening; apply to the current one
    let mut current = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    let command = EditCommand::ReplaceChapters {
        audiobook_id: audiobook_id.clone(),
        old_chapters: current.chapters.clone(),
        new_chapters: chapters,
    };
    undo_state
        .execute(&mut current, command)
        .map_err(to_string_err)?;
    state
        .update_audiobook(current.clone())
        .map_err(to_string_err)?;
    Ok(current)
}

/// Stop a running detection; it returns "cancelled"
#[tauri::command]
pub async fn cancel_spoken_chapters() -> Result<(), String> {
    DETECT_CANCEL.store(true, Ordering::SeqCst);
    Ok(())
}
