//! FFmpeg detection and verification
//!
//! Detects FFmpeg installation on the system and verifies compatibility.

use crate::config::get_ffmpeg_path;
use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use regex::Regex;
use std::process::Command;
use tracing::{debug, info, warn};

/// Minimum required FFmpeg version
pub const MIN_FFMPEG_VERSION: &str = "4.0";

/// Cached FFmpeg detection result
static FFMPEG_CACHE: Lazy<RwLock<Option<FFmpegInfo>>> = Lazy::new(|| RwLock::new(None));

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

/// Detect FFmpeg on the system
pub fn detect_ffmpeg() -> AppResult<FFmpegInfo> {
    // Check cache first
    if let Some(info) = FFMPEG_CACHE.read().as_ref() {
        debug!("Using cached FFmpeg info");
        return Ok(info.clone());
    }

    info!("Detecting FFmpeg installation...");

    // Try custom path first
    if let Some(custom_path) = get_ffmpeg_path() {
        if let Ok(info) = verify_ffmpeg_at_path(&custom_path) {
            cache_ffmpeg_info(info.clone());
            return Ok(info);
        }
        warn!("Custom FFmpeg path not working: {}", custom_path);
    }

    // Try common paths
    let paths_to_try = get_ffmpeg_search_paths();

    for path in paths_to_try {
        if let Ok(info) = verify_ffmpeg_at_path(&path) {
            cache_ffmpeg_info(info.clone());
            return Ok(info);
        }
    }

    // Try PATH
    if let Ok(info) = verify_ffmpeg_in_path() {
        cache_ffmpeg_info(info.clone());
        return Ok(info);
    }

    Err(AppError::FFmpegNotFound)
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

/// Verify FFmpeg at a specific path
fn verify_ffmpeg_at_path(path: &Utf8Path) -> AppResult<FFmpegInfo> {
    debug!("Checking FFmpeg at: {}", path);

    if !path.exists() {
        return Err(AppError::FFmpegNotFound);
    }

    let output = Command::new(path.as_str())
        .arg("-version")
        .output()
        .map_err(|e| AppError::ffmpeg_error(format!("Failed to execute FFmpeg: {}", e)))?;

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

    let info = FFmpegInfo {
        path: path.to_owned(),
        version,
        version_output,
        supports_aac,
        supports_libfdk_aac,
    };

    info!("Found FFmpeg {} at {}", info.version, info.path);
    Ok(info)
}

/// Verify FFmpeg in system PATH
fn verify_ffmpeg_in_path() -> AppResult<FFmpegInfo> {
    debug!("Checking FFmpeg in PATH...");

    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|_| AppError::FFmpegNotFound)?;

    if !output.status.success() {
        return Err(AppError::FFmpegNotFound);
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

    // Determine actual path using `which` or `where`
    let path = find_ffmpeg_in_path()?;

    // Check for encoders
    let encoders = get_available_encoders(&path)?;
    let supports_aac = encoders.iter().any(|e| e == "aac");
    let supports_libfdk_aac = encoders.iter().any(|e| e == "libfdk_aac");

    if !supports_aac {
        return Err(AppError::ffmpeg_error("FFmpeg does not support AAC encoding"));
    }

    let info = FFmpegInfo {
        path,
        version,
        version_output,
        supports_aac,
        supports_libfdk_aac,
    };

    info!("Found FFmpeg {} in PATH", info.version);
    Ok(info)
}

/// Find ffmpeg path using which/where command
fn find_ffmpeg_in_path() -> AppResult<Utf8PathBuf> {
    #[cfg(windows)]
    let cmd = "where";
    #[cfg(not(windows))]
    let cmd = "which";

    let output = Command::new(cmd)
        .arg("ffmpeg")
        .output()
        .map_err(|_| AppError::FFmpegNotFound)?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        if !path_str.is_empty() {
            return Ok(Utf8PathBuf::from(path_str));
        }
    }

    // Fallback
    Ok(Utf8PathBuf::from("ffmpeg"))
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
}

