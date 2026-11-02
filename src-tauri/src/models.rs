//! Core data structures for AudiobookConvertRs
//!
//! Defines all the main types used throughout the application:
//! - AudioFormat: Supported audio file formats
//! - Metadata: Audio file metadata (title, artist, etc.)
//! - Chapter: Audiobook chapter information
//! - AudioFile: Individual audio file representation
//! - AudioBook: Collection of audio files forming an audiobook
//! - ConversionSettings: Settings for the conversion process
//! - ConversionJob: A queued or running conversion task
//! - ConversionProgress: Progress information for IPC events

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub use crate::conversion::progress::ConversionProgress;

/// Quality presets for audio encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    High,
    #[default]
    Medium,
    Low,
    Custom,
}

impl QualityPreset {
    pub fn bitrate_kbps(&self) -> u32 {
        match self {
            QualityPreset::High => 256,
            QualityPreset::Medium => 128,
            QualityPreset::Low => 64,
            QualityPreset::Custom => 128,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        match self {
            QualityPreset::High => 44100,
            QualityPreset::Medium => 44100,
            QualityPreset::Low => 22050,
            QualityPreset::Custom => 44100,
        }
    }

    pub fn is_vbr(&self) -> bool {
        match self {
            QualityPreset::High => true,
            QualityPreset::Medium => true,
            QualityPreset::Low => false,
            QualityPreset::Custom => true,
        }
    }

    pub fn vbr_quality(&self) -> u8 {
        match self {
            QualityPreset::High => 2,
            QualityPreset::Medium => 4,
            QualityPreset::Low => 6,
            QualityPreset::Custom => 4,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            QualityPreset::High => "Best quality, larger files",
            QualityPreset::Medium => "Good balance of quality and size",
            QualityPreset::Low => "Smaller files, reduced quality",
            QualityPreset::Custom => "Manual configuration",
        }
    }
}

impl std::fmt::Display for QualityPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityPreset::High => write!(f, "High"),
            QualityPreset::Medium => write!(f, "Medium"),
            QualityPreset::Low => write!(f, "Low"),
            QualityPreset::Custom => write!(f, "Custom"),
        }
    }
}

/// Encoding mode: VBR (variable) or CBR (constant)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EncodingMode {
    #[default]
    Vbr,
    Cbr,
}

impl EncodingMode {
    pub fn description(&self) -> &'static str {
        match self {
            EncodingMode::Vbr => "Variable bitrate - better quality at same size",
            EncodingMode::Cbr => "Constant bitrate - consistent file size",
        }
    }
}

impl std::fmt::Display for EncodingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingMode::Vbr => write!(f, "VBR"),
            EncodingMode::Cbr => write!(f, "CBR"),
        }
    }
}

/// Estimated file size result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSizeEstimate {
    pub bytes: u64,
    pub formatted: String,
}

impl FileSizeEstimate {
    pub fn new(bytes: u64) -> Self {
        Self {
            bytes,
            formatted: format_bytes(bytes),
        }
    }
}

/// Estimate output file size based on duration and bitrate
pub fn estimate_file_size_bytes(duration_ms: u64, bitrate_kbps: u32) -> u64 {
    (duration_ms * bitrate_kbps as u64) / 8
}

/// Quality preset info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPresetInfo {
    pub preset: QualityPreset,
    pub name: String,
    pub description: String,
    pub bitrate_kbps: u32,
    pub sample_rate_hz: u32,
    pub is_vbr: bool,
}

/// Supported audio formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    Flac,
    M4a,
    M4b,
    Wav,
    Ogg,
    Wma,
    Aac,
}

impl AudioFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::M4a => "m4a",
            AudioFormat::M4b => "m4b",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Wma => "wma",
            AudioFormat::Aac => "aac",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::M4a | AudioFormat::M4b => "audio/mp4",
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Wma => "audio/x-ms-wma",
            AudioFormat::Aac => "audio/aac",
        }
    }

    /// Check if this format is supported for input
    pub fn is_input_supported(&self) -> bool {
        // All formats are supported for input via Symphonia or FFmpeg
        true
    }

    /// Check if this format is supported for output
    pub fn is_output_supported(&self) -> bool {
        // Currently only M4B output is supported
        matches!(self, AudioFormat::M4b)
    }

    /// Try to detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "mp3" => Some(AudioFormat::Mp3),
            "flac" => Some(AudioFormat::Flac),
            "m4a" => Some(AudioFormat::M4a),
            "m4b" => Some(AudioFormat::M4b),
            "wav" => Some(AudioFormat::Wav),
            "ogg" | "oga" => Some(AudioFormat::Ogg),
            "wma" => Some(AudioFormat::Wma),
            "aac" => Some(AudioFormat::Aac),
            _ => None,
        }
    }
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension().to_uppercase())
    }
}

/// Audio file metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub narrator: Option<String>,
    pub composer: Option<String>,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    pub comments: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub copyright: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<u32>,
    pub language: Option<String>,
    pub artwork_path: Option<Utf8PathBuf>,
    /// Embedded artwork as base64 encoded data (for IPC transfer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_data: Option<String>,
}

impl Metadata {
    /// Create new empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all metadata fields are empty
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.year.is_none()
            && self.genre.is_none()
    }

    /// Merge with another metadata, preferring non-empty values from self
    pub fn merge_with(&mut self, other: &Metadata) {
        if self.title.is_none() {
            self.title = other.title.clone();
        }
        if self.artist.is_none() {
            self.artist = other.artist.clone();
        }
        if self.album.is_none() {
            self.album = other.album.clone();
        }
        if self.album_artist.is_none() {
            self.album_artist = other.album_artist.clone();
        }
        if self.narrator.is_none() {
            self.narrator = other.narrator.clone();
        }
        if self.composer.is_none() {
            self.composer = other.composer.clone();
        }
        if self.year.is_none() {
            self.year = other.year;
        }
        if self.genre.is_none() {
            self.genre = other.genre.clone();
        }
        if self.comments.is_none() {
            self.comments = other.comments.clone();
        }
        if self.description.is_none() {
            self.description = other.description.clone();
        }
        if self.publisher.is_none() {
            self.publisher = other.publisher.clone();
        }
        if self.copyright.is_none() {
            self.copyright = other.copyright.clone();
        }
        if self.series.is_none() {
            self.series = other.series.clone();
        }
        if self.series_index.is_none() {
            self.series_index = other.series_index;
        }
        if self.language.is_none() {
            self.language = other.language.clone();
        }
        if self.artwork_path.is_none() {
            self.artwork_path = other.artwork_path.clone();
        }
    }

    /// Sanitize metadata fields for filesystem safety
    pub fn sanitize(&mut self) {
        fn sanitize_string(s: &mut Option<String>) {
            if let Some(ref mut val) = s {
                *val = val.trim().to_string();
                // Remove characters that are problematic for filenames
                *val = val
                    .chars()
                    .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
                    .collect();
            }
        }

        sanitize_string(&mut self.title);
        sanitize_string(&mut self.artist);
        sanitize_string(&mut self.album);
        sanitize_string(&mut self.narrator);
        sanitize_string(&mut self.genre);
        sanitize_string(&mut self.series);
        sanitize_string(&mut self.language);
    }
}

/// Audiobook chapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    /// Zero-based index
    pub index: usize,
    /// Chapter title
    pub title: String,
    /// Start time in milliseconds
    pub start_time_ms: u64,
    /// End time in milliseconds
    pub end_time_ms: u64,
    /// Whether this chapter was automatically detected
    pub auto_detected: bool,
}

impl Chapter {
    /// Create a new chapter
    pub fn new(index: usize, title: String, start_time_ms: u64, end_time_ms: u64) -> Self {
        Self {
            index,
            title,
            start_time_ms,
            end_time_ms,
            auto_detected: false,
        }
    }

    /// Get chapter duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        self.end_time_ms.saturating_sub(self.start_time_ms)
    }

    /// Check if chapter times are valid
    pub fn is_valid(&self) -> bool {
        self.start_time_ms < self.end_time_ms && !self.title.is_empty()
    }

    /// Format start time as HH:MM:SS
    pub fn formatted_start_time(&self) -> String {
        format_duration_ms(self.start_time_ms)
    }

    /// Format end time as HH:MM:SS
    pub fn formatted_end_time(&self) -> String {
        format_duration_ms(self.end_time_ms)
    }

    /// Format duration as HH:MM:SS
    pub fn formatted_duration(&self) -> String {
        format_duration_ms(self.duration_ms())
    }
}

/// Individual audio file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFile {
    /// Full path to the file
    pub path: Utf8PathBuf,
    /// Detected audio format
    pub format: AudioFormat,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Bitrate in kbps (if available)
    pub bitrate_kbps: Option<u32>,
    /// Sample rate in Hz (if available)
    pub sample_rate_hz: Option<u32>,
    /// Number of audio channels
    pub channels: Option<u8>,
    /// File size in bytes
    pub file_size_bytes: u64,
    /// Extracted metadata
    pub metadata: Metadata,
    /// Whether artwork was found in this file
    pub has_artwork: bool,
}

impl AudioFile {
    /// Get the filename without extension
    pub fn stem(&self) -> &str {
        self.path
            .file_stem()
            .unwrap_or(self.path.as_str())
    }

    /// Get the filename with extension
    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .unwrap_or(self.path.as_str())
    }

    /// Format duration as HH:MM:SS
    pub fn formatted_duration(&self) -> String {
        format_duration_ms(self.duration_ms)
    }

    /// Format file size as human-readable string
    pub fn formatted_size(&self) -> String {
        format_bytes(self.file_size_bytes)
    }
}

/// Audiobook containing multiple audio files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBook {
    /// Unique identifier
    pub id: String,
    /// Title of the audiobook
    pub title: String,
    /// Author of the book
    pub author: Option<String>,
    /// Source audio files
    pub files: Vec<AudioFile>,
    /// Chapter information
    pub chapters: Vec<Chapter>,
    /// Output metadata (can be edited by user)
    pub output_metadata: Metadata,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Total size of all input files
    pub total_size_bytes: u64,
    /// When this audiobook was created
    pub created_at: DateTime<Utc>,
    /// When this audiobook was last modified
    pub modified_at: DateTime<Utc>,
    /// Whether there are unsaved changes
    #[serde(default)]
    pub is_dirty: bool,
}

impl AudioBook {
    /// Create a new audiobook with a unique ID
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            author: None,
            files: Vec::new(),
            chapters: Vec::new(),
            output_metadata: Metadata::default(),
            total_duration_ms: 0,
            total_size_bytes: 0,
            created_at: now,
            modified_at: now,
            is_dirty: false,
        }
    }

    /// Mark the audiobook as modified
    pub fn mark_modified(&mut self) {
        self.modified_at = Utc::now();
        self.is_dirty = true;
    }

    /// Mark the audiobook as saved (not dirty)
    pub fn mark_saved(&mut self) {
        self.is_dirty = false;
    }

    /// Get the total formatted duration
    pub fn formatted_duration(&self) -> String {
        format_duration_ms(self.total_duration_ms)
    }

    /// Get the total formatted size
    pub fn formatted_size(&self) -> String {
        format_bytes(self.total_size_bytes)
    }

    /// Recalculate totals from files
    pub fn recalculate_totals(&mut self) {
        self.total_duration_ms = self.files.iter().map(|f| f.duration_ms).sum();
        self.total_size_bytes = self.files.iter().map(|f| f.file_size_bytes).sum();
    }

    /// Get suggested output filename based on metadata
    pub fn suggested_filename(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref author) = self.author {
            if !author.is_empty() {
                parts.push(author.clone());
            }
        }

        if !self.title.is_empty() {
            parts.push(self.title.clone());
        }

        if parts.is_empty() {
            "audiobook".to_string()
        } else {
            format!("{}.m4b", parts.join(" - "))
        }
    }
}

/// Conversion defaults (no output path - used for global/project defaults)
/// This represents the default values that can be overridden at project level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConversionDefaults {
    /// Output audio bitrate in kbps
    pub bitrate_kbps: u32,
    /// Output sample rate in Hz (None = use source)
    pub sample_rate_hz: Option<u32>,
    /// Encoding mode (VBR or CBR)
    pub encoding_mode: EncodingMode,
    /// Whether to overwrite existing output file
    pub overwrite_existing: bool,
    /// Whether to embed chapters
    pub embed_chapters: bool,
    /// Whether to embed artwork
    pub embed_artwork: bool,
    /// Whether to copy metadata from source
    pub copy_metadata: bool,
}

impl Default for ConversionDefaults {
    fn default() -> Self {
        Self {
            bitrate_kbps: 128,
            sample_rate_hz: None,
            encoding_mode: EncodingMode::Vbr,
            overwrite_existing: false,
            embed_chapters: true,
            embed_artwork: true,
            copy_metadata: true,
        }
    }
}

impl ConversionDefaults {
    /// Create ConversionDefaults from UserSettings
    pub fn from_user_settings(settings: &crate::config::UserSettings) -> Self {
        Self {
            bitrate_kbps: settings.default_bitrate_kbps,
            sample_rate_hz: settings.default_sample_rate,
            encoding_mode: settings.default_encoding_mode,
            overwrite_existing: settings.overwrite_existing_by_default,
            embed_chapters: settings.embed_chapters_by_default,
            embed_artwork: settings.embed_artwork_by_default,
            copy_metadata: settings.copy_metadata_by_default,
        }
    }

    /// Convert to full ConversionSettings by adding output path
    pub fn to_settings(&self, output_path: camino::Utf8PathBuf, concurrent_jobs: u8) -> ConversionSettings {
        ConversionSettings {
            bitrate_kbps: self.bitrate_kbps,
            sample_rate_hz: self.sample_rate_hz,
            encoding_mode: self.encoding_mode,
            output_format: AudioFormat::M4b,
            output_path,
            concurrent_jobs,
            overwrite_existing: self.overwrite_existing,
            embed_chapters: self.embed_chapters,
            embed_artwork: self.embed_artwork,
            copy_metadata: self.copy_metadata,
            normalize_audio: false,
            normalization_settings: None,
        }
    }
}

/// Normalization settings for conversion pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// Target integrated loudness in LUFS
    pub target_lufs: f32,
    /// Target true peak in dBTP
    pub target_true_peak: f32,
    /// Target loudness range in LU
    pub target_lra: f32,
    /// Whether to use two-pass normalization for better accuracy
    pub two_pass: bool,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            target_lufs: -16.0,
            target_true_peak: -1.5,
            target_lra: 11.0,
            two_pass: true,
        }
    }
}

/// Settings for audio conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionSettings {
    /// Output audio bitrate in kbps
    pub bitrate_kbps: u32,
    /// Output sample rate in Hz (None = use source)
    pub sample_rate_hz: Option<u32>,
    /// Encoding mode (VBR or CBR)
    #[serde(default)]
    pub encoding_mode: EncodingMode,
    /// Output format (currently always M4B)
    pub output_format: AudioFormat,
    /// Full path to output file
    pub output_path: Utf8PathBuf,
    /// Number of concurrent encoding jobs
    pub concurrent_jobs: u8,
    /// Whether to overwrite existing output file
    pub overwrite_existing: bool,
    /// Whether to embed chapters
    pub embed_chapters: bool,
    /// Whether to embed artwork
    pub embed_artwork: bool,
    /// Whether to copy metadata from source
    pub copy_metadata: bool,
    /// Whether to normalize audio before encoding
    #[serde(default)]
    pub normalize_audio: bool,
    /// Normalization settings (used when normalize_audio is true)
    #[serde(default)]
    pub normalization_settings: Option<NormalizationConfig>,
}

impl Default for ConversionSettings {
    fn default() -> Self {
        Self {
            bitrate_kbps: 128,
            sample_rate_hz: None,
            encoding_mode: EncodingMode::Vbr,
            output_format: AudioFormat::M4b,
            output_path: Utf8PathBuf::new(),
            concurrent_jobs: 1,
            overwrite_existing: false,
            embed_chapters: true,
            embed_artwork: true,
            copy_metadata: true,
            normalize_audio: false,
            normalization_settings: None,
        }
    }
}

impl ConversionSettings {
    /// Validate conversion settings
    pub fn validate(&self) -> Result<(), String> {
        if self.bitrate_kbps < 8 || self.bitrate_kbps > 320 {
            return Err(format!(
                "Bitrate must be between 8 and 320 kbps, got {}",
                self.bitrate_kbps
            ));
        }

        if let Some(sr) = self.sample_rate_hz {
            let valid_rates = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 96000];
            if !valid_rates.contains(&sr) {
                return Err(format!(
                    "Invalid sample rate: {}. Valid rates: {:?}",
                    sr, valid_rates
                ));
            }
        }

        if self.output_path.as_str().is_empty() {
            return Err("Output path is required".to_string());
        }

        Ok(())
    }
}

/// Status of a conversion job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Complete,
    Error,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Complete | JobStatus::Error | JobStatus::Cancelled)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, JobStatus::Running | JobStatus::Paused)
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "Queued"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Paused => write!(f, "Paused"),
            JobStatus::Complete => write!(f, "Complete"),
            JobStatus::Error => write!(f, "Error"),
            JobStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A conversion job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionJob {
    /// Unique job identifier
    pub id: String,
    /// The audiobook being converted
    pub audiobook: AudioBook,
    /// Conversion settings
    pub settings: ConversionSettings,
    /// Current status
    pub status: JobStatus,
    /// Progress percentage (0.0 - 100.0)
    pub progress_percent: f32,
    /// Index of file currently being processed
    pub current_file_index: usize,
    /// Total number of files
    pub total_files: usize,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// Detailed progress information (optional; primarily for UI polling fallback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ConversionProgress>,
    /// Output path when completed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    /// Human-readable status message (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job started running
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl ConversionJob {
    /// Create a new conversion job
    pub fn new(audiobook: AudioBook, settings: ConversionSettings) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            total_files: audiobook.files.len(),
            audiobook,
            settings,
            status: JobStatus::Queued,
            progress_percent: 0.0,
            current_file_index: 0,
            error_message: None,
            progress: None,
            output_path: None,
            status_message: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Mark job as started
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark job as complete
    pub fn complete(&mut self) {
        self.status = JobStatus::Complete;
        self.progress_percent = 100.0;
        self.status_message = Some("complete".to_string());
        self.completed_at = Some(Utc::now());
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Error;
        self.error_message = Some(error.into());
        self.status_message = Some("error".to_string());
        self.completed_at = Some(Utc::now());
    }

    /// Mark job as cancelled
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.status_message = Some("cancelled".to_string());
        self.completed_at = Some(Utc::now());
    }

    /// Pause the job (only if running)
    pub fn pause(&mut self) -> bool {
        if self.status == JobStatus::Running {
            self.status = JobStatus::Paused;
            true
        } else {
            false
        }
    }

    /// Resume the job (only if paused)
    pub fn resume(&mut self) -> bool {
        if self.status == JobStatus::Paused {
            self.status = JobStatus::Running;
            true
        } else {
            false
        }
    }

    /// Get elapsed time since job started
    pub fn elapsed(&self) -> Option<chrono::Duration> {
        self.started_at.map(|started| {
            let end = self.completed_at.unwrap_or_else(Utc::now);
            end - started
        })
    }
}

// ============================================
// Helper Functions
// ============================================

/// Format milliseconds as HH:MM:SS
pub fn format_duration_ms(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Parse time string (HH:MM:SS or MM:SS) to milliseconds
pub fn parse_time_string(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.trim().split(':').collect();

    match parts.len() {
        2 => {
            // MM:SS
            let minutes: u64 = parts[0].parse().ok()?;
            let seconds: u64 = parts[1].parse().ok()?;
            Some((minutes * 60 + seconds) * 1000)
        }
        3 => {
            // HH:MM:SS
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;
            let seconds: u64 = parts[2].parse().ok()?;
            Some((hours * 3600 + minutes * 60 + seconds) * 1000)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::M4b.extension(), "m4b");
    }

    #[test]
    fn test_audio_format_from_extension() {
        assert_eq!(AudioFormat::from_extension("mp3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("MP3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("xyz"), None);
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(0), "00:00");
        assert_eq!(format_duration_ms(61000), "01:01");
        assert_eq!(format_duration_ms(3661000), "01:01:01");
        assert_eq!(format_duration_ms(7200000), "02:00:00");
    }

    #[test]
    fn test_parse_time_string() {
        assert_eq!(parse_time_string("01:30"), Some(90000));
        assert_eq!(parse_time_string("01:00:00"), Some(3600000));
        assert_eq!(parse_time_string("invalid"), None);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_chapter_duration() {
        let chapter = Chapter::new(0, "Test".to_string(), 1000, 5000);
        assert_eq!(chapter.duration_ms(), 4000);
    }

    #[test]
    fn test_metadata_merge() {
        let mut m1 = Metadata::new();
        m1.title = Some("Title 1".to_string());

        let mut m2 = Metadata::new();
        m2.title = Some("Title 2".to_string());
        m2.artist = Some("Artist".to_string());

        m1.merge_with(&m2);
        assert_eq!(m1.title, Some("Title 1".to_string())); // Keep existing
        assert_eq!(m1.artist, Some("Artist".to_string())); // Take from other
    }

    #[test]
    fn test_conversion_settings_validation() {
        let mut settings = ConversionSettings::default();
        settings.output_path = "/test/output.m4b".into();
        assert!(settings.validate().is_ok());

        settings.bitrate_kbps = 500;
        assert!(settings.validate().is_err());

        settings.bitrate_kbps = 128;
        settings.sample_rate_hz = Some(12345);
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_job_status() {
        assert!(JobStatus::Complete.is_terminal());
        assert!(JobStatus::Error.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Running.is_active());
    }

    #[test]
    fn test_conversion_defaults_default() {
        let defaults = ConversionDefaults::default();
        assert_eq!(defaults.bitrate_kbps, 128);
        assert_eq!(defaults.sample_rate_hz, None);
        assert!(!defaults.overwrite_existing);
        assert!(defaults.embed_chapters);
        assert!(defaults.embed_artwork);
        assert!(defaults.copy_metadata);
    }

    #[test]
    fn test_conversion_defaults_to_settings() {
        let defaults = ConversionDefaults {
            bitrate_kbps: 96,
            sample_rate_hz: Some(44100),
            encoding_mode: EncodingMode::Cbr,
            overwrite_existing: true,
            embed_chapters: false,
            embed_artwork: true,
            copy_metadata: false,
        };

        let settings = defaults.to_settings("/output/test.m4b".into(), 2);

        assert_eq!(settings.bitrate_kbps, 96);
        assert_eq!(settings.sample_rate_hz, Some(44100));
        assert_eq!(settings.encoding_mode, EncodingMode::Cbr);
        assert!(settings.overwrite_existing);
        assert!(!settings.embed_chapters);
        assert!(settings.embed_artwork);
        assert!(!settings.copy_metadata);
        assert_eq!(settings.output_path.as_str(), "/output/test.m4b");
        assert_eq!(settings.concurrent_jobs, 2);
        assert_eq!(settings.output_format, AudioFormat::M4b);
    }

    #[test]
    fn test_conversion_defaults_serde_roundtrip() {
        let defaults = ConversionDefaults {
            bitrate_kbps: 192,
            sample_rate_hz: Some(48000),
            encoding_mode: EncodingMode::Vbr,
            overwrite_existing: false,
            embed_chapters: true,
            embed_artwork: false,
            copy_metadata: true,
        };

        let json = serde_json::to_string(&defaults).expect("Serialize failed");
        let parsed: ConversionDefaults = serde_json::from_str(&json).expect("Deserialize failed");

        assert_eq!(parsed.bitrate_kbps, defaults.bitrate_kbps);
        assert_eq!(parsed.sample_rate_hz, defaults.sample_rate_hz);
        assert_eq!(parsed.encoding_mode, defaults.encoding_mode);
        assert_eq!(parsed.overwrite_existing, defaults.overwrite_existing);
        assert_eq!(parsed.embed_chapters, defaults.embed_chapters);
        assert_eq!(parsed.embed_artwork, defaults.embed_artwork);
        assert_eq!(parsed.copy_metadata, defaults.copy_metadata);
    }

    #[test]
    fn test_quality_preset_bitrates() {
        assert_eq!(QualityPreset::High.bitrate_kbps(), 256);
        assert_eq!(QualityPreset::Medium.bitrate_kbps(), 128);
        assert_eq!(QualityPreset::Low.bitrate_kbps(), 64);
        assert_eq!(QualityPreset::Custom.bitrate_kbps(), 128);
    }

    #[test]
    fn test_quality_preset_sample_rates() {
        assert_eq!(QualityPreset::High.sample_rate_hz(), 44100);
        assert_eq!(QualityPreset::Medium.sample_rate_hz(), 44100);
        assert_eq!(QualityPreset::Low.sample_rate_hz(), 22050);
        assert_eq!(QualityPreset::Custom.sample_rate_hz(), 44100);
    }

    #[test]
    fn test_quality_preset_vbr() {
        assert!(QualityPreset::High.is_vbr());
        assert!(QualityPreset::Medium.is_vbr());
        assert!(!QualityPreset::Low.is_vbr());
        assert!(QualityPreset::Custom.is_vbr());
    }

    #[test]
    fn test_encoding_mode_serialization() {
        let vbr = EncodingMode::Vbr;
        let cbr = EncodingMode::Cbr;

        let vbr_json = serde_json::to_string(&vbr).unwrap();
        let cbr_json = serde_json::to_string(&cbr).unwrap();

        assert_eq!(vbr_json, "\"vbr\"");
        assert_eq!(cbr_json, "\"cbr\"");

        let parsed_vbr: EncodingMode = serde_json::from_str(&vbr_json).unwrap();
        let parsed_cbr: EncodingMode = serde_json::from_str(&cbr_json).unwrap();

        assert_eq!(parsed_vbr, EncodingMode::Vbr);
        assert_eq!(parsed_cbr, EncodingMode::Cbr);
    }

    #[test]
    fn test_file_size_estimation() {
        let duration_ms: u64 = 3600000;
        let bitrate_kbps: u32 = 128;
        let estimated = estimate_file_size_bytes(duration_ms, bitrate_kbps);
        assert_eq!(estimated, 57600000);
    }

    #[test]
    fn test_file_size_estimate_formatting() {
        let estimate = FileSizeEstimate::new(57600000);
        assert_eq!(estimate.bytes, 57600000);
        assert_eq!(estimate.formatted, "54.93 MB");
    }
}

