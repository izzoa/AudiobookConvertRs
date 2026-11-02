use crate::config::{self, UserSettings};
use crate::error::AppResult;
use crate::ffmpeg::detection::{self, FFmpegInfo, FFmpegSource};
use crate::ffmpeg::download::{self, DownloadError, DownloadOffer, FixedBuild};
use crate::state::AppState;
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

/// Serializable FFmpeg info for the frontend
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FFmpegStatus {
    /// Whether FFmpeg is available
    pub available: bool,
    /// Path to ffmpeg executable
    pub path: Option<String>,
    /// Version string
    pub version: Option<String>,
    /// Whether libfdk_aac encoder is available (better quality)
    pub supports_libfdk_aac: bool,
    /// Where the FFmpeg in use was found
    pub source: Option<FFmpegSource>,
    /// The path chosen in Settings, whether or not it works
    pub chosen_path: Option<String>,
    /// Why the chosen path was skipped, when it was
    pub chosen_path_error: Option<String>,
    /// Version of the downloaded copy, when one is installed
    pub downloaded_version: Option<String>,
    /// A build the app can download; only while FFmpeg is missing
    pub offer: Option<DownloadOffer>,
}

impl FFmpegStatus {
    /// Status from a detection result. The offer appears only when nothing was
    /// found and this platform has a fixed build.
    fn from_detection(
        detected: AppResult<FFmpegInfo>,
        chosen_path: Option<String>,
        chosen_path_error: Option<String>,
        downloaded_version: Option<String>,
        build: Option<&FixedBuild>,
    ) -> Self {
        match detected {
            Ok(info) => Self {
                available: true,
                path: Some(info.path.to_string()),
                version: Some(info.version),
                supports_libfdk_aac: info.supports_libfdk_aac,
                source: Some(info.source),
                chosen_path,
                chosen_path_error,
                downloaded_version,
                offer: None,
            },
            Err(_) => Self {
                available: false,
                path: None,
                version: None,
                supports_libfdk_aac: false,
                source: None,
                chosen_path,
                chosen_path_error,
                downloaded_version,
                offer: build.map(DownloadOffer::from),
            },
        }
    }
}

/// Detect FFmpeg and describe the result for the frontend
fn current_ffmpeg_status(state: &AppState) -> FFmpegStatus {
    let detected = detection::detect_ffmpeg();
    match &detected {
        Ok(info) => state.set_ffmpeg_status(true, Some(info.version.clone())),
        Err(_) => state.set_ffmpeg_status(false, None),
    }
    let downloaded_version = detection::downloaded_root()
        .and_then(|root| detection::installed_download(&root))
        .map(|(version, _)| version);
    FFmpegStatus::from_detection(
        detected,
        config::get_ffmpeg_path().map(|p| p.to_string()),
        detection::chosen_path_error(),
        downloaded_version,
        download::current_build(),
    )
}

#[tauri::command]
pub async fn get_settings(_state: State<'_, AppState>) -> Result<UserSettings, String> {
    Ok(config::get_settings())
}

#[tauri::command]
pub async fn update_settings(
    mut settings: UserSettings,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    // The FFmpeg path changes only through `set_ffmpeg_path`, which checks it
    // first; a Settings draft loaded before that must not undo the choice.
    settings.ffmpeg_path = config::get_ffmpeg_path();
    // config::update_settings handles validation and saving/updating global state
    config::update_settings(settings).map_err(to_string_err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_default_settings() -> Result<UserSettings, String> {
    Ok(UserSettings::default())
}

#[tauri::command]
pub async fn check_ffmpeg_available(state: State<'_, AppState>) -> Result<bool, String> {
    // Check cache in state first
    if state.is_ffmpeg_available() {
        return Ok(true);
    }

    // Check system
    match detection::detect_ffmpeg() {
        Ok(info) => {
            state.set_ffmpeg_status(true, Some(info.version));
            Ok(true)
        }
        Err(_) => {
            state.set_ffmpeg_status(false, None);
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn reset_settings(_state: State<'_, AppState>) -> Result<(), String> {
    let defaults = UserSettings::default();
    config::update_settings(defaults).map_err(to_string_err)?;
    // Defaults have no chosen FFmpeg path
    detection::clear_ffmpeg_cache();
    Ok(())
}

#[tauri::command]
pub async fn get_ffmpeg_version(state: State<'_, AppState>) -> Result<String, String> {
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
    refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<FFmpegStatus, String> {
    // "Check again" must search anew, not return the cached result
    if refresh.unwrap_or(false) {
        detection::clear_ffmpeg_cache();
    }
    Ok(current_ffmpeg_status(&state))
}

/// Check a chosen FFmpeg path and save it, or clear the choice with `None`.
/// A path that fails the check is not saved, and the error says why.
fn apply_chosen_path(
    path: Option<Utf8PathBuf>,
    save: impl FnOnce(Option<Utf8PathBuf>) -> AppResult<()>,
) -> AppResult<()> {
    if let Some(path) = &path {
        detection::verify_ffmpeg_at_path(path, FFmpegSource::Chosen)?;
    }
    save(path)?;
    detection::clear_ffmpeg_cache();
    Ok(())
}

/// Choose the FFmpeg executable to use, or clear the choice. Returns the new status.
#[tauri::command]
pub async fn set_ffmpeg_path(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<FFmpegStatus, String> {
    let path = path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .map(Utf8PathBuf::from);
    apply_chosen_path(path, config::set_ffmpeg_path).map_err(to_string_err)?;
    Ok(current_ffmpeg_status(&state))
}

/// Set while a download runs; only one runs at a time
static DOWNLOAD_RUNNING: AtomicBool = AtomicBool::new(false);
/// Set by `cancel_ffmpeg_download`, checked on every chunk
static DOWNLOAD_CANCEL: AtomicBool = AtomicBool::new(false);

/// Clears `DOWNLOAD_RUNNING` however the download ends
struct DownloadRunning;

impl Drop for DownloadRunning {
    fn drop(&mut self) {
        DOWNLOAD_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Payload of the `ffmpeg_download_progress` event
#[derive(Debug, Clone, Serialize)]
pub struct FFmpegDownloadProgress {
    pub received: u64,
    pub total: u64,
}

/// Download, verify and install this platform's fixed FFmpeg build.
/// Progress arrives as `ffmpeg_download_progress`; returns the new status.
#[tauri::command]
pub async fn download_ffmpeg(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<FFmpegStatus, String> {
    let build = download::current_build().ok_or_else(|| DownloadError::NoBuild.to_string())?;
    let root = detection::downloaded_root()
        .ok_or_else(|| "could not find the app's data folder".to_string())?;
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
            "ffmpeg_download_progress",
            FFmpegDownloadProgress { received, total },
        );
    };

    download::download_and_install(build, &root, &DOWNLOAD_CANCEL, progress)
        .await
        .map_err(to_string_err)?;
    Ok(current_ffmpeg_status(&state))
}

/// Stop a running download; it cleans up and returns "download cancelled"
#[tauri::command]
pub async fn cancel_ffmpeg_download() -> Result<(), String> {
    DOWNLOAD_CANCEL.store(true, Ordering::SeqCst);
    Ok(())
}

/// Delete the downloaded FFmpeg and search again. Returns the new status.
#[tauri::command]
pub async fn remove_downloaded_ffmpeg(state: State<'_, AppState>) -> Result<FFmpegStatus, String> {
    if DOWNLOAD_RUNNING.load(Ordering::SeqCst) {
        return Err(DownloadError::Busy.to_string());
    }
    if let Some(root) = detection::downloaded_root() {
        download::remove_downloaded(&root).map_err(to_string_err)?;
    }
    Ok(current_ffmpeg_status(&state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn a_file_that_cannot_run_is_not_saved() {
        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("ffmpeg")).unwrap();
        std::fs::write(&path, "not a program").unwrap();

        let mut saved = None;
        let result = apply_chosen_path(Some(path), |p| {
            saved = Some(p);
            Ok(())
        });
        assert!(result.is_err());
        assert!(saved.is_none(), "setting must stay unchanged");
    }

    #[cfg(unix)]
    #[test]
    fn a_working_ffmpeg_is_saved() {
        let temp = tempfile::tempdir().unwrap();
        let dir = camino::Utf8Path::from_path(temp.path()).unwrap();
        let fake = crate::ffmpeg::detection::test_support::fake_ffmpeg(dir, "9.0.2", "aac");

        let mut saved = None;
        apply_chosen_path(Some(fake.clone()), |p| {
            saved = Some(p);
            Ok(())
        })
        .unwrap();
        assert_eq!(saved, Some(Some(fake)));
    }

    #[test]
    fn none_clears_the_choice() {
        let mut saved = None;
        apply_chosen_path(None, |p| {
            saved = Some(p);
            Ok(())
        })
        .unwrap();
        assert_eq!(saved, Some(None));
    }

    fn info() -> FFmpegInfo {
        FFmpegInfo {
            path: "/opt/homebrew/bin/ffmpeg".into(),
            version: "9.0.2".into(),
            version_output: String::new(),
            supports_aac: true,
            supports_libfdk_aac: false,
            source: FFmpegSource::System,
        }
    }

    #[test]
    fn offer_only_while_missing_on_a_platform_with_a_build() {
        let build = download::build_for("aarch64-apple-darwin");

        let found = FFmpegStatus::from_detection(Ok(info()), None, None, None, build);
        assert!(found.available && found.offer.is_none());
        assert_eq!(found.source, Some(FFmpegSource::System));

        let missing =
            FFmpegStatus::from_detection(Err(AppError::FFmpegNotFound), None, None, None, build);
        let offer = missing.offer.expect("offer while missing");
        assert_eq!(
            (offer.version.as_str(), offer.size_bytes),
            ("9.0.2", 28_395_699)
        );

        let no_build =
            FFmpegStatus::from_detection(Err(AppError::FFmpegNotFound), None, None, None, None);
        assert!(no_build.offer.is_none());
    }

    #[test]
    fn status_round_trips_through_json() {
        let status = FFmpegStatus::from_detection(
            Err(AppError::FFmpegNotFound),
            Some("/gone/ffmpeg".into()),
            Some("File not found: /gone/ffmpeg".into()),
            Some("9.0.2".into()),
            download::build_for("x86_64-pc-windows-msvc"),
        );
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"chosen_path_error\":\"File not found"));
        assert_eq!(serde_json::from_str::<FFmpegStatus>(&json).unwrap(), status);

        let found = serde_json::to_value(FFmpegStatus::from_detection(
            Ok(info()),
            None,
            None,
            None,
            None,
        ))
        .unwrap();
        assert_eq!(found["source"], "system");
    }
}
