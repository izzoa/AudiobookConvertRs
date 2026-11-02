//! FFmpeg detection and verification
//!
//! Finds a working FFmpeg — the path the user chose, a system install, or the
//! copy the app downloaded — and verifies compatibility.

use crate::config::get_ffmpeg_path;
use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// Minimum required FFmpeg version
pub const MIN_FFMPEG_VERSION: &str = "4.0";

/// Debug builds only: set to `downloaded-only` to skip system installs, so the
/// missing-FFmpeg path can be tried on a machine that has FFmpeg.
pub const SEARCH_OVERRIDE_ENV: &str = "AUDIOBOOKCONVERT_FFMPEG_SEARCH";

/// File name of the FFmpeg executable on this platform
pub const FFMPEG_EXE: &str = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };

/// Cached FFmpeg detection result
static FFMPEG_CACHE: Lazy<RwLock<Option<FFmpegInfo>>> = Lazy::new(|| RwLock::new(None));

/// Why the chosen path was passed over on the last search
static CHOSEN_PATH_ERROR: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

/// Where a resolved FFmpeg came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FFmpegSource {
    /// The path the user chose in Settings
    Chosen,
    /// A known install folder or PATH
    System,
    /// The copy the app downloaded
    Downloaded,
}

/// FFmpeg installation information
#[derive(Debug, Clone)]
pub struct FFmpegInfo {
    /// Path to the ffmpeg executable
    pub path: Utf8PathBuf,
    /// Version string (e.g., "5.1.2")
    pub version: String,
    /// Full version output
    pub version_output: String,
    /// Whether AAC encoding is supported
    pub supports_aac: bool,
    /// Whether libfdk_aac is available (better quality)
    pub supports_libfdk_aac: bool,
    /// Where this FFmpeg was found
    pub source: FFmpegSource,
}

impl FFmpegInfo {
    /// Get the preferred AAC encoder
    pub fn aac_encoder(&self) -> &'static str {
        if self.supports_libfdk_aac {
            "libfdk_aac"
        } else {
            "aac"
        }
    }
}

/// Detect FFmpeg: the chosen path, then known folders, then PATH, then the
/// downloaded copy. The first one that passes the check wins.
pub fn detect_ffmpeg() -> AppResult<FFmpegInfo> {
    // Check cache first
    if let Some(info) = FFMPEG_CACHE.read().as_ref() {
        debug!("Using cached FFmpeg info");
        return Ok(info.clone());
    }

    info!("Detecting FFmpeg installation...");

    let include_system = system_search_enabled(
        std::env::var(SEARCH_OVERRIDE_ENV).ok().as_deref(),
        cfg!(debug_assertions),
    );
    let candidates = search_candidates(get_ffmpeg_path(), include_system, downloaded_root().as_deref());
    let resolution = resolve(&candidates);
    *CHOSEN_PATH_ERROR.write() = resolution.chosen_error;

    match resolution.info {
        Some(info) => {
            info!("Found FFmpeg {} at {} ({:?})", info.version, info.path, info.source);
            cache_ffmpeg_info(info.clone());
            Ok(info)
        }
        None => Err(AppError::FFmpegNotFound),
    }
}

/// Why the chosen FFmpeg path was skipped on the last search, if it was
pub fn chosen_path_error() -> Option<String> {
    CHOSEN_PATH_ERROR.read().clone()
}

/// Folder holding downloaded FFmpeg builds, one subfolder per version
pub fn downloaded_root() -> Option<Utf8PathBuf> {
    let dir = dirs::data_local_dir()?.join("AudiobookConvertRs").join("ffmpeg");
    Utf8PathBuf::from_path_buf(dir).ok()
}

/// The installed download under `root`: its version and executable. Only the
/// newest is returned; folders still being written (`.tmp`, `.partial`) are skipped.
pub fn installed_download(root: &Utf8Path) -> Option<(String, Utf8PathBuf)> {
    let mut found: Vec<(String, Utf8PathBuf)> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with('.') || name.ends_with(".tmp") || name.ends_with(".partial") {
                return None;
            }
            let exe = Utf8PathBuf::from_path_buf(entry.path().join(FFMPEG_EXE)).ok()?;
            exe.is_file().then_some((name, exe))
        })
        .collect();
    found.sort_by_key(|(version, _)| std::cmp::Reverse(version_key(version)));
    found.into_iter().next()
}

/// Check one FFmpeg executable: it runs, is new enough and can encode AAC
pub fn verify_ffmpeg_at_path(path: &Utf8Path, source: FFmpegSource) -> AppResult<FFmpegInfo> {
    debug!("Checking FFmpeg at: {}", path);

    if !path.is_file() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let output = Command::new(path.as_str())
        .arg("-version")
        .output()
        .map_err(|e| AppError::ffmpeg_error(format!("could not run {}: {}", path, e)))?;

    if !output.status.success() {
        return Err(AppError::ffmpeg_error("FFmpeg -version failed"));
    }

    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    let version = parse_ffmpeg_version(&version_output)?;

    // Check version meets minimum
    if !version_meets_minimum(&version) {
        return Err(AppError::FFmpegVersionTooOld {
            version,
            minimum: MIN_FFMPEG_VERSION.to_string(),
        });
    }

    // Check for encoders
    let encoders = get_available_encoders(path)?;
    let supports_aac = encoders.iter().any(|e| e == "aac");
    let supports_libfdk_aac = encoders.iter().any(|e| e == "libfdk_aac");

    if !supports_aac {
        return Err(AppError::ffmpeg_error("FFmpeg does not support AAC encoding"));
    }

    Ok(FFmpegInfo {
        path: path.to_owned(),
        version,
        version_output,
        supports_aac,
        supports_libfdk_aac,
        source,
    })
}

/// One place FFmpeg might be
#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    path: Utf8PathBuf,
    source: FFmpegSource,
}

/// Outcome of trying every candidate
struct Resolution {
    info: Option<FFmpegInfo>,
    chosen_error: Option<String>,
}

/// Whether system installs are searched. Only debug builds honour the override.
fn system_search_enabled(override_value: Option<&str>, debug_build: bool) -> bool {
    !(debug_build && override_value == Some("downloaded-only"))
}

/// Every place to look, in order, without repeats
fn search_candidates(
    chosen: Option<Utf8PathBuf>,
    include_system: bool,
    downloaded_root: Option<&Utf8Path>,
) -> Vec<Candidate> {
    let mut list: Vec<Candidate> = Vec::new();
    let mut push = |path: Utf8PathBuf, source: FFmpegSource| {
        if !list.iter().any(|c| c.path == path) {
            list.push(Candidate { path, source });
        }
    };

    if let Some(path) = chosen {
        push(path, FFmpegSource::Chosen);
    }
    if include_system {
        for path in get_ffmpeg_search_paths().into_iter().chain(path_entries()) {
            push(path, FFmpegSource::System);
        }
    }
    if let Some((_, path)) = downloaded_root.and_then(installed_download) {
        push(path, FFmpegSource::Downloaded);
    }
    list
}

/// Try each candidate in order; remember why a chosen path failed
fn resolve(candidates: &[Candidate]) -> Resolution {
    let mut chosen_error = None;
    for candidate in candidates {
        match verify_ffmpeg_at_path(&candidate.path, candidate.source) {
            Ok(info) => return Resolution { info: Some(info), chosen_error },
            Err(e) => {
                if candidate.source == FFmpegSource::Chosen {
                    warn!("Chosen FFmpeg path not working: {}: {}", candidate.path, e);
                    chosen_error = Some(e.to_string());
                } else {
                    debug!("No usable FFmpeg at {}: {}", candidate.path, e);
                }
            }
        }
    }
    Resolution { info: None, chosen_error }
}

/// Get common FFmpeg installation paths by OS
fn get_ffmpeg_search_paths() -> Vec<Utf8PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        paths.push(Utf8PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe"));
        paths.push(Utf8PathBuf::from("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe"));
        paths.push(Utf8PathBuf::from("C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe"));

        // Check common Windows package managers
        if let Some(local_app_data) = dirs::data_local_dir() {
            if let Ok(path) = Utf8PathBuf::try_from(local_app_data) {
                paths.push(path.join("Programs/ffmpeg/bin/ffmpeg.exe"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(Utf8PathBuf::from("/opt/homebrew/bin/ffmpeg")); // Apple Silicon
        paths.push(Utf8PathBuf::from("/usr/local/bin/ffmpeg")); // Intel
        paths.push(Utf8PathBuf::from("/opt/local/bin/ffmpeg")); // MacPorts
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(Utf8PathBuf::from("/usr/bin/ffmpeg"));
        paths.push(Utf8PathBuf::from("/usr/local/bin/ffmpeg"));
        paths.push(Utf8PathBuf::from("/snap/bin/ffmpeg"));
        paths.push(Utf8PathBuf::from("/home/linuxbrew/.linuxbrew/bin/ffmpeg"));
    }

    paths
}

/// `ffmpeg` in each PATH folder, in PATH order. Absolute paths, so every later
/// run uses the same file the check approved.
fn path_entries() -> Vec<Utf8PathBuf> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    std::env::split_paths(&path)
        .filter_map(|dir| Utf8PathBuf::from_path_buf(dir.join(FFMPEG_EXE)).ok())
        .collect()
}

/// Dotted version as numbers, for ordering ("9.0.2" > "9.0")
fn version_key(version: &str) -> Vec<u32> {
    version.split('.').filter_map(|part| part.parse().ok()).collect()
}

/// Parse FFmpeg version from version output
fn parse_ffmpeg_version(output: &str) -> AppResult<String> {
    // Pattern: "ffmpeg version 5.1.2 ..." or "ffmpeg version N-xxxxx-..."
    let re = Regex::new(r"ffmpeg version (\d+\.\d+(?:\.\d+)?)")
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    if let Some(caps) = re.captures(output) {
        return Ok(caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default());
    }

    // Try alternative pattern for git builds
    let re_git = Regex::new(r"ffmpeg version N-(\d+)")
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    if let Some(caps) = re_git.captures(output) {
        // Git builds are always recent
        return Ok("99.0".to_string());
    }

    Err(AppError::FFmpegOutputParseError(
        "Could not parse FFmpeg version".to_string(),
    ))
}

/// Check if version meets minimum requirement
fn version_meets_minimum(version: &str) -> bool {
    let parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    let min_parts: Vec<u32> = MIN_FFMPEG_VERSION
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for i in 0..min_parts.len() {
        let v = parts.get(i).unwrap_or(&0);
        let m = min_parts.get(i).unwrap_or(&0);

        if v > m {
            return true;
        }
        if v < m {
            return false;
        }
    }

    true
}

/// Get list of available audio encoders
fn get_available_encoders(ffmpeg_path: &Utf8Path) -> AppResult<Vec<String>> {
    let output = Command::new(ffmpeg_path.as_str())
        .args(["-encoders", "-hide_banner"])
        .output()
        .map_err(|e| AppError::ffmpeg_error(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let encoders: Vec<String> = stdout
        .lines()
        .filter(|line| line.starts_with(" A")) // Audio encoders start with " A"
        .filter_map(|line| {
            line.split_whitespace()
                .nth(1)
                .map(|s| s.to_string())
        })
        .collect();

    Ok(encoders)
}

/// Cache FFmpeg info
fn cache_ffmpeg_info(info: FFmpegInfo) {
    let mut cache = FFMPEG_CACHE.write();
    *cache = Some(info);
}

/// Clear FFmpeg cache (useful for testing or when path changes)
pub fn clear_ffmpeg_cache() {
    let mut cache = FFMPEG_CACHE.write();
    *cache = None;
}

/// Check if FFmpeg is available (cached check)
pub fn is_ffmpeg_available() -> bool {
    detect_ffmpeg().is_ok()
}

/// Get installation instructions for the current OS
pub fn get_installation_instructions() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Install FFmpeg on Windows:\n\
            1. Download from https://ffmpeg.org/download.html#build-windows\n\
            2. Extract to C:\\ffmpeg\n\
            3. Add C:\\ffmpeg\\bin to your PATH\n\
            \n\
            Or use Chocolatey: choco install ffmpeg\n\
            Or use Scoop: scoop install ffmpeg";

    #[cfg(target_os = "macos")]
    return "Install FFmpeg on macOS:\n\
            Using Homebrew (recommended):\n\
            $ brew install ffmpeg\n\
            \n\
            Using MacPorts:\n\
            $ sudo port install ffmpeg";

    #[cfg(target_os = "linux")]
    return "Install FFmpeg on Linux:\n\
            Ubuntu/Debian: sudo apt install ffmpeg\n\
            Fedora: sudo dnf install ffmpeg\n\
            Arch: sudo pacman -S ffmpeg\n\
            \n\
            Or download from https://ffmpeg.org/download.html";

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "Download FFmpeg from https://ffmpeg.org/download.html";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ffmpeg_version() {
        assert_eq!(
            parse_ffmpeg_version("ffmpeg version 5.1.2 Copyright").unwrap(),
            "5.1.2"
        );
        assert_eq!(
            parse_ffmpeg_version("ffmpeg version 4.0 Copyright").unwrap(),
            "4.0"
        );
    }

    #[test]
    fn test_version_meets_minimum() {
        assert!(version_meets_minimum("5.1.2"));
        assert!(version_meets_minimum("4.0"));
        assert!(version_meets_minimum("4.0.0"));
        assert!(!version_meets_minimum("3.9"));
        assert!(!version_meets_minimum("3.0.0"));
    }

    #[test]
    fn override_needs_a_debug_build() {
        assert!(!system_search_enabled(Some("downloaded-only"), true));
        assert!(system_search_enabled(Some("downloaded-only"), false));
        assert!(system_search_enabled(None, true));
        assert!(system_search_enabled(Some("anything-else"), true));
    }

    #[test]
    fn candidates_run_chosen_then_system_then_downloaded() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        let installed = root.join("9.0.2");
        std::fs::create_dir(&installed).unwrap();
        std::fs::write(installed.join(FFMPEG_EXE), "").unwrap();

        let list = search_candidates(Some("/chosen/ffmpeg".into()), true, Some(root));
        assert_eq!(list.first().map(|c| c.source), Some(FFmpegSource::Chosen));
        assert_eq!(list.last().map(|c| c.source), Some(FFmpegSource::Downloaded));
        assert!(list[1..list.len() - 1].iter().all(|c| c.source == FFmpegSource::System));

        let without_system = search_candidates(None, false, Some(root));
        assert_eq!(without_system, vec![Candidate { path: installed.join(FFMPEG_EXE), source: FFmpegSource::Downloaded }]);
    }

    #[test]
    fn installed_download_skips_unfinished_folders() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        for name in ["8.1", "9.0.2", "9.1.tmp", "9.2.partial", "empty"] {
            std::fs::create_dir(root.join(name)).unwrap();
            if name != "empty" {
                std::fs::write(root.join(name).join(FFMPEG_EXE), "").unwrap();
            }
        }
        let (version, path) = installed_download(root).unwrap();
        assert_eq!(version, "9.0.2");
        assert_eq!(path, root.join("9.0.2").join(FFMPEG_EXE));
        assert!(installed_download(&root.join("missing")).is_none());
    }

    #[cfg(unix)]
    mod resolve {
        use super::super::*;
        use super::super::test_support::fake_ffmpeg;

        fn at(path: &Utf8Path, source: FFmpegSource) -> Candidate {
            Candidate { path: path.to_owned(), source }
        }

        #[test]
        fn chosen_path_beats_system() {
            let temp = tempfile::tempdir().unwrap();
            let dir = Utf8Path::from_path(temp.path()).unwrap();
            let chosen = fake_ffmpeg(&dir.join("chosen"), "9.0.2", "aac");
            let system = fake_ffmpeg(&dir.join("system"), "9.0.2", "aac");

            let found = resolve(&[at(&chosen, FFmpegSource::Chosen), at(&system, FFmpegSource::System)]);
            let info = found.info.unwrap();
            assert_eq!((info.path, info.source), (chosen, FFmpegSource::Chosen));
            assert!(found.chosen_error.is_none());
        }

        #[test]
        fn broken_chosen_path_falls_through_with_reason() {
            let temp = tempfile::tempdir().unwrap();
            let dir = Utf8Path::from_path(temp.path()).unwrap();
            let system = fake_ffmpeg(&dir.join("system"), "9.0.2", "aac");
            let too_old = fake_ffmpeg(&dir.join("old"), "3.4", "aac");

            let missing = resolve(&[at(&dir.join("gone/ffmpeg"), FFmpegSource::Chosen), at(&system, FFmpegSource::System)]);
            assert_eq!(missing.info.map(|i| i.source), Some(FFmpegSource::System));
            assert!(missing.chosen_error.unwrap().contains("not found"));

            let old = resolve(&[at(&too_old, FFmpegSource::Chosen), at(&system, FFmpegSource::System)]);
            assert_eq!(old.info.map(|i| i.source), Some(FFmpegSource::System));
            assert!(old.chosen_error.unwrap().contains("too old"));
        }

        #[test]
        fn downloaded_copy_only_when_nothing_else_works() {
            let temp = tempfile::tempdir().unwrap();
            let dir = Utf8Path::from_path(temp.path()).unwrap();
            let system = fake_ffmpeg(&dir.join("system"), "9.0.2", "aac");
            let no_aac = fake_ffmpeg(&dir.join("no-aac"), "9.0.2", "mp3");
            let downloaded = fake_ffmpeg(&dir.join("downloaded"), "9.0.2", "aac");

            let both = resolve(&[at(&system, FFmpegSource::System), at(&downloaded, FFmpegSource::Downloaded)]);
            assert_eq!(both.info.map(|i| i.source), Some(FFmpegSource::System));

            let fallback = resolve(&[at(&no_aac, FFmpegSource::System), at(&downloaded, FFmpegSource::Downloaded)]);
            assert_eq!(fallback.info.map(|i| i.path), Some(downloaded));

            assert!(resolve(&[at(&no_aac, FFmpegSource::System)]).info.is_none());
        }
    }
}

/// Fake `ffmpeg` executables for tests that must not depend on a real install
#[cfg(all(test, unix))]
pub(crate) mod test_support {
    use camino::{Utf8Path, Utf8PathBuf};
    use std::os::unix::fs::PermissionsExt;

    /// Writes `<dir>/ffmpeg`: a script answering `-version` with `version` and
    /// `-encoders` with one audio encoder, the two calls the check makes.
    pub fn fake_ffmpeg(dir: &Utf8Path, version: &str, encoder: &str) -> Utf8PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("ffmpeg");
        let script = format!(
            "#!/bin/sh\ncase \"$1\" in\n  -version) echo \"ffmpeg version {version} Copyright (c) 2000-2026\" ;;\n  -encoders) echo \" A....D {encoder}              Test encoder\" ;;\nesac\n"
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

