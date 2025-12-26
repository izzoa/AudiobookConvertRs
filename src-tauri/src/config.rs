//! Configuration management for AudiobookConvertRs
//!
//! Handles user settings persistence including:
//! - Default conversion settings (bitrate, output directory)
//! - FFmpeg path configuration
//! - UI preferences
//! - Recent files list

use crate::error::{AppError, AppResult};
use crate::models::{EncodingMode, QualityPreset};
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Global configuration instance
static CONFIG: Lazy<RwLock<UserSettings>> = Lazy::new(|| {
    RwLock::new(UserSettings::load().unwrap_or_default())
});

/// User-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    /// Default output audio bitrate in kbps
    pub default_bitrate_kbps: u32,

    /// Default output directory (None = same as input)
    pub default_output_directory: Option<Utf8PathBuf>,

    /// Whether to auto-detect and use artwork from audio files
    pub auto_detect_artwork: bool,

    /// Whether to scan directory for cover images
    pub scan_for_cover_images: bool,

    /// Number of concurrent conversion jobs
    pub concurrent_jobs: u8,

    /// Custom FFmpeg path (None = auto-detect)
    pub ffmpeg_path: Option<Utf8PathBuf>,

    /// UI theme preference
    pub theme: Theme,

    /// Window dimensions
    pub window_width: u32,
    pub window_height: u32,

    /// Whether to show advanced options
    pub show_advanced_options: bool,

    /// Whether to confirm before overwriting files
    pub confirm_overwrite: bool,

    /// Whether to play notification sound on completion
    pub notification_sound: bool,

    /// Recent file paths (most recent first)
    pub recent_files: Vec<Utf8PathBuf>,

    /// Maximum number of recent files to remember
    pub max_recent_files: usize,

    /// Default sample rate for output (None = match source)
    pub default_sample_rate: Option<u32>,

    /// Default quality preset
    pub default_quality_preset: QualityPreset,

    /// Default encoding mode (VBR or CBR)
    pub default_encoding_mode: EncodingMode,

    /// Whether to embed chapters by default
    pub embed_chapters_by_default: bool,

    /// Whether to copy metadata by default
    pub copy_metadata_by_default: bool,

    /// Whether to embed artwork by default
    pub embed_artwork_by_default: bool,

    /// Whether to overwrite existing files by default
    pub overwrite_existing_by_default: bool,

    /// Log level for debugging
    pub log_level: LogLevel,

    /// Normalization target LUFS
    pub normalization_target_lufs: f32,

    /// Normalization target true peak in dBTP
    pub normalization_target_true_peak: f32,

    /// Normalization target LRA in LU
    pub normalization_target_lra: f32,

    /// Whether to use two-pass normalization
    pub normalization_two_pass: bool,
}

/// UI theme options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// Log level options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            default_bitrate_kbps: 128,
            default_output_directory: None,
            auto_detect_artwork: true,
            scan_for_cover_images: true,
            concurrent_jobs: 1,
            ffmpeg_path: None,
            theme: Theme::System,
            window_width: 1200,
            window_height: 800,
            show_advanced_options: false,
            confirm_overwrite: true,
            notification_sound: true,
            recent_files: Vec::new(),
            max_recent_files: 10,
            default_sample_rate: None,
            default_quality_preset: QualityPreset::Medium,
            default_encoding_mode: EncodingMode::Vbr,
            embed_chapters_by_default: true,
            copy_metadata_by_default: true,
            embed_artwork_by_default: true,
            overwrite_existing_by_default: false,
            log_level: LogLevel::Info,
            normalization_target_lufs: -16.0,
            normalization_target_true_peak: -1.5,
            normalization_target_lra: 11.0,
            normalization_two_pass: true,
        }
    }
}

impl UserSettings {
    /// Get the configuration directory path
    pub fn config_dir() -> AppResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AppError::ConfigError("Could not determine config directory".into()))?
            .join("AudiobookConvertRs");

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| {
                AppError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        Ok(config_dir)
    }

    /// Get the configuration file path
    pub fn config_file_path() -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    /// Load settings from disk
    pub fn load() -> AppResult<Self> {
        let path = Self::config_file_path()?;

        if !path.exists() {
            debug!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path).map_err(|e| {
            AppError::ConfigLoadError(format!("Failed to read config file: {}", e))
        })?;

        let settings: Self = serde_json::from_str(&contents).map_err(|e| {
            warn!("Failed to parse config file, using defaults: {}", e);
            AppError::ConfigLoadError(format!("Failed to parse config file: {}", e))
        })?;

        info!("Loaded configuration from {:?}", path);
        Ok(settings)
    }

    /// Save settings to disk
    pub fn save(&self) -> AppResult<()> {
        let path = Self::config_file_path()?;

        let contents = serde_json::to_string_pretty(self).map_err(|e| {
            AppError::ConfigSaveError(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(&path, contents).map_err(|e| {
            AppError::ConfigSaveError(format!("Failed to write config file: {}", e))
        })?;

        debug!("Saved configuration to {:?}", path);
        Ok(())
    }

    /// Add a file to the recent files list
    pub fn add_recent_file(&mut self, path: Utf8PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);

        // Add to front
        self.recent_files.insert(0, path);

        // Trim to max size
        self.recent_files.truncate(self.max_recent_files);
    }

    /// Clear recent files list
    pub fn clear_recent_files(&mut self) {
        self.recent_files.clear();
    }

    /// Validate settings
    pub fn validate(&self) -> AppResult<()> {
        if self.default_bitrate_kbps < 8 || self.default_bitrate_kbps > 320 {
            return Err(AppError::ConfigError(
                "Default bitrate must be between 8 and 320 kbps".into(),
            ));
        }

        if self.concurrent_jobs < 1 || self.concurrent_jobs > 16 {
            return Err(AppError::ConfigError(
                "Concurrent jobs must be between 1 and 16".into(),
            ));
        }

        if let Some(ref sr) = self.default_sample_rate {
            let valid_rates = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 96000];
            if !valid_rates.contains(sr) {
                return Err(AppError::ConfigError(format!(
                    "Invalid sample rate: {}. Valid rates: {:?}",
                    sr, valid_rates
                )));
            }
        }

        Ok(())
    }
}

// ============================================
// Global Config Access
// ============================================

/// Get a clone of the current settings
pub fn get_settings() -> UserSettings {
    CONFIG.read().clone()
}

/// Update settings globally
pub fn update_settings(settings: UserSettings) -> AppResult<()> {
    settings.validate()?;

    let mut config = CONFIG.write();
    *config = settings;
    config.save()?;

    info!("Settings updated and saved");
    Ok(())
}

/// Update a single setting using a closure
pub fn update_setting<F>(updater: F) -> AppResult<()>
where
    F: FnOnce(&mut UserSettings),
{
    let mut config = CONFIG.write();
    updater(&mut config);
    config.validate()?;
    config.save()?;
    Ok(())
}

/// Reset settings to defaults
pub fn reset_settings() -> AppResult<()> {
    let mut config = CONFIG.write();
    *config = UserSettings::default();
    config.save()?;

    info!("Settings reset to defaults");
    Ok(())
}

/// Add a file to recent files
pub fn add_recent_file(path: Utf8PathBuf) -> AppResult<()> {
    update_setting(|s| s.add_recent_file(path))
}

/// Get the FFmpeg path (custom or auto-detected)
pub fn get_ffmpeg_path() -> Option<Utf8PathBuf> {
    CONFIG.read().ffmpeg_path.clone()
}

/// Set a custom FFmpeg path
pub fn set_ffmpeg_path(path: Option<Utf8PathBuf>) -> AppResult<()> {
    update_setting(|s| s.ffmpeg_path = path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = UserSettings::default();
        assert_eq!(settings.default_bitrate_kbps, 128);
        assert_eq!(settings.concurrent_jobs, 1);
        assert!(settings.auto_detect_artwork);
    }

    #[test]
    fn test_settings_validation() {
        let mut settings = UserSettings::default();
        assert!(settings.validate().is_ok());

        settings.default_bitrate_kbps = 500;
        assert!(settings.validate().is_err());

        settings.default_bitrate_kbps = 128;
        settings.concurrent_jobs = 100;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_recent_files() {
        let mut settings = UserSettings::default();
        settings.max_recent_files = 3;

        settings.add_recent_file("/file1.mp3".into());
        settings.add_recent_file("/file2.mp3".into());
        settings.add_recent_file("/file3.mp3".into());
        settings.add_recent_file("/file4.mp3".into());

        assert_eq!(settings.recent_files.len(), 3);
        assert_eq!(settings.recent_files[0].as_str(), "/file4.mp3");

        // Adding existing file moves it to front
        settings.add_recent_file("/file2.mp3".into());
        assert_eq!(settings.recent_files[0].as_str(), "/file2.mp3");
        assert_eq!(settings.recent_files.len(), 3);
    }

    #[test]
    fn test_serialization() {
        let settings = UserSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: UserSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(settings.default_bitrate_kbps, deserialized.default_bitrate_kbps);
    }
}

