//! Batch conversion enhancements
//!
//! Provides functionality for:
//! - Loading and converting multiple audiobooks
//! - Conversion presets (save/load settings)
//! - Dry-run mode for previewing conversions

use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, ConversionSettings};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{debug, info};

/// Conversion preset - saved conversion settings with a name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionPreset {
    /// Unique preset name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// The conversion settings
    pub settings: PresetSettings,
    /// When the preset was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the preset was last modified
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Preset settings (subset of ConversionSettings without paths)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSettings {
    /// Audio bitrate in kbps
    pub bitrate_kbps: u32,
    /// Sample rate (None = keep original)
    pub sample_rate: Option<u32>,
    /// Number of concurrent conversion jobs
    pub concurrent_jobs: u8,
    /// Whether to overwrite existing output files
    pub overwrite_existing: bool,
    /// Whether to embed chapters
    pub embed_chapters: bool,
    /// Whether to embed artwork
    pub embed_artwork: bool,
    /// Whether to copy metadata from source
    pub copy_metadata: bool,
    /// Apply normalization
    pub normalize: bool,
    /// Target loudness for normalization (LUFS)
    pub target_lufs: Option<f32>,
}

impl Default for PresetSettings {
    fn default() -> Self {
        Self {
            bitrate_kbps: 64,
            sample_rate: None,
            concurrent_jobs: 1,
            overwrite_existing: false,
            embed_chapters: true,
            embed_artwork: true,
            copy_metadata: true,
            normalize: false,
            target_lufs: None,
        }
    }
}

impl ConversionPreset {
    /// Create a new preset with the given name and settings
    pub fn new(name: impl Into<String>, settings: PresetSettings) -> Self {
        let now = chrono::Utc::now();
        Self {
            name: name.into(),
            description: None,
            settings,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a preset from ConversionSettings
    pub fn from_conversion_settings(name: impl Into<String>, settings: &ConversionSettings) -> Self {
        Self::new(name, PresetSettings {
            bitrate_kbps: settings.bitrate_kbps,
            sample_rate: settings.sample_rate_hz,
            concurrent_jobs: settings.concurrent_jobs,
            overwrite_existing: settings.overwrite_existing,
            embed_chapters: settings.embed_chapters,
            embed_artwork: settings.embed_artwork,
            copy_metadata: settings.copy_metadata,
            normalize: false,
            target_lufs: None,
        })
    }

    /// Apply this preset to create ConversionSettings
    pub fn apply(&self, output_path: Utf8PathBuf) -> ConversionSettings {
        ConversionSettings {
            bitrate_kbps: self.settings.bitrate_kbps,
            sample_rate_hz: self.settings.sample_rate,
            output_format: crate::models::AudioFormat::M4b,
            output_path,
            concurrent_jobs: self.settings.concurrent_jobs,
            overwrite_existing: self.settings.overwrite_existing,
            embed_chapters: self.settings.embed_chapters,
            embed_artwork: self.settings.embed_artwork,
            copy_metadata: self.settings.copy_metadata,
        }
    }
}

/// Preset manager for saving and loading presets
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetManager {
    presets: Vec<ConversionPreset>,
}

impl PresetManager {
    /// Create a new preset manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Load presets from a file
    pub fn load(path: &Utf8Path) -> AppResult<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path.as_std_path())
            .map_err(|e| AppError::FileReadError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        serde_json::from_str(&content)
            .map_err(|e| AppError::ConfigError(e.to_string()))
    }

    /// Save presets to a file
    pub fn save(&self, path: &Utf8Path) -> AppResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent.as_std_path())
                .map_err(|e| AppError::FileWriteError {
                    path: path.as_std_path().to_path_buf(),
                    reason: e.to_string(),
                })?;
        }

        fs::write(path.as_std_path(), content)
            .map_err(|e| AppError::FileWriteError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        info!("Saved {} presets to {}", self.presets.len(), path);
        Ok(())
    }

    /// Add or update a preset
    pub fn save_preset(&mut self, preset: ConversionPreset) {
        if let Some(existing) = self.presets.iter_mut().find(|p| p.name == preset.name) {
            *existing = preset;
        } else {
            self.presets.push(preset);
        }
    }

    /// Get a preset by name
    pub fn get_preset(&self, name: &str) -> Option<&ConversionPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// Delete a preset by name
    pub fn delete_preset(&mut self, name: &str) -> bool {
        let len_before = self.presets.len();
        self.presets.retain(|p| p.name != name);
        self.presets.len() != len_before
    }

    /// List all presets
    pub fn list_presets(&self) -> &[ConversionPreset] {
        &self.presets
    }

    /// Get preset names
    pub fn preset_names(&self) -> Vec<String> {
        self.presets.iter().map(|p| p.name.clone()).collect()
    }
}

/// Dry-run analysis result for a single audiobook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunResult {
    /// The audiobook title
    pub title: String,
    /// Total input duration in milliseconds
    pub input_duration_ms: u64,
    /// Number of input files
    pub input_file_count: usize,
    /// Total input size in bytes
    pub input_size_bytes: u64,
    /// Estimated output size in bytes
    pub estimated_output_size_bytes: u64,
    /// Estimated conversion time in seconds
    pub estimated_time_seconds: u32,
    /// Output path
    pub output_path: String,
    /// Whether output file already exists
    pub output_exists: bool,
    /// Number of chapters that will be embedded
    pub chapter_count: usize,
    /// Whether artwork will be embedded
    pub has_artwork: bool,
    /// Any warnings or issues detected
    pub warnings: Vec<String>,
}

/// Perform a dry-run analysis of a conversion
///
/// Analyzes the audiobook and settings to estimate output size,
/// conversion time, and detect potential issues without actually converting.
pub fn dry_run_analysis(audiobook: &AudioBook, settings: &ConversionSettings) -> DryRunResult {
    debug!("Performing dry-run analysis for: {}", audiobook.title);

    let input_duration_ms = audiobook.total_duration_ms;
    let input_file_count = audiobook.files.len();
    let input_size_bytes: u64 = audiobook.files.iter().map(|f| f.file_size_bytes).sum();

    // Estimate output size based on bitrate
    // Formula: (bitrate_kbps * 1000 / 8) * (duration_ms / 1000) = bytes
    let duration_secs = input_duration_ms as f64 / 1000.0;
    let estimated_output_size_bytes = (settings.bitrate_kbps as f64 * 1000.0 / 8.0 * duration_secs) as u64;

    // Estimate conversion time (rough: assume 10x realtime for most systems)
    let estimated_time_seconds = (duration_secs / 10.0) as u32;

    let output_path = settings.output_path.to_string();
    let output_exists = settings.output_path.exists();

    let chapter_count = audiobook.chapters.len();
    let has_artwork = audiobook.output_metadata.artwork_data.is_some() 
        || audiobook.output_metadata.artwork_path.is_some();

    let mut warnings = Vec::new();

    // Check for potential issues
    if output_exists && !settings.overwrite_existing {
        warnings.push(format!("Output file already exists: {}", output_path));
    }

    if input_file_count == 0 {
        warnings.push("No input files".to_string());
    }

    if chapter_count == 0 && settings.embed_chapters {
        warnings.push("No chapters to embed".to_string());
    }

    if !has_artwork && settings.embed_artwork {
        warnings.push("No artwork available to embed".to_string());
    }

    // Note: Disk space checking would require additional dependencies (fs2)
    // For now, we just estimate needed space
    let _needed_space = estimated_output_size_bytes * 2;

    DryRunResult {
        title: audiobook.title.clone(),
        input_duration_ms,
        input_file_count,
        input_size_bytes,
        estimated_output_size_bytes,
        estimated_time_seconds,
        output_path,
        output_exists,
        chapter_count,
        has_artwork,
        warnings,
    }
}

/// Perform dry-run analysis for multiple audiobooks
pub fn batch_dry_run(
    audiobooks: &[AudioBook],
    settings: &ConversionSettings,
    output_dir: &Utf8Path,
) -> Vec<DryRunResult> {
    audiobooks.iter().map(|ab| {
        let output_path: Utf8PathBuf = output_dir.join(format!("{}.m4b", ab.title));
        let mut settings = settings.clone();
        settings.output_path = output_path;
        dry_run_analysis(ab, &settings)
    }).collect()
}

/// Summary of batch dry-run analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDryRunSummary {
    /// Total number of audiobooks
    pub total_audiobooks: usize,
    /// Total input duration across all audiobooks
    pub total_input_duration_ms: u64,
    /// Total input size in bytes
    pub total_input_size_bytes: u64,
    /// Total estimated output size in bytes
    pub total_estimated_output_bytes: u64,
    /// Total estimated conversion time in seconds
    pub total_estimated_time_seconds: u32,
    /// Number of audiobooks with warnings
    pub audiobooks_with_warnings: usize,
    /// All warnings collected
    pub all_warnings: Vec<String>,
}

/// Summarize batch dry-run results
pub fn summarize_batch_dry_run(results: &[DryRunResult]) -> BatchDryRunSummary {
    let total_audiobooks = results.len();
    let total_input_duration_ms: u64 = results.iter().map(|r| r.input_duration_ms).sum();
    let total_input_size_bytes: u64 = results.iter().map(|r| r.input_size_bytes).sum();
    let total_estimated_output_bytes: u64 = results.iter().map(|r| r.estimated_output_size_bytes).sum();
    let total_estimated_time_seconds: u32 = results.iter().map(|r| r.estimated_time_seconds).sum();
    let audiobooks_with_warnings = results.iter().filter(|r| !r.warnings.is_empty()).count();
    let all_warnings: Vec<String> = results.iter()
        .flat_map(|r| r.warnings.iter().map(|w| format!("{}: {}", r.title, w)))
        .collect();

    BatchDryRunSummary {
        total_audiobooks,
        total_input_duration_ms,
        total_input_size_bytes,
        total_estimated_output_bytes,
        total_estimated_time_seconds,
        audiobooks_with_warnings,
        all_warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_creation() {
        let preset = ConversionPreset::new("High Quality", PresetSettings {
            bitrate_kbps: 192,
            ..Default::default()
        });

        assert_eq!(preset.name, "High Quality");
        assert_eq!(preset.settings.bitrate_kbps, 192);
    }

    #[test]
    fn test_preset_manager() {
        let mut manager = PresetManager::new();
        
        let preset = ConversionPreset::new("Test", PresetSettings::default());
        manager.save_preset(preset);

        assert_eq!(manager.list_presets().len(), 1);
        assert!(manager.get_preset("Test").is_some());
        assert!(manager.delete_preset("Test"));
        assert_eq!(manager.list_presets().len(), 0);
    }

    #[test]
    fn test_preset_apply() {
        let preset = ConversionPreset::new("Test", PresetSettings {
            bitrate_kbps: 128,
            embed_chapters: false,
            ..Default::default()
        });

        let settings = preset.apply("/output/test.m4b".into());
        assert_eq!(settings.bitrate_kbps, 128);
        assert!(!settings.embed_chapters);
    }
}
