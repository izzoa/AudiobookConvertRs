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

/// Settings for audio conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionSettings {
    /// Output audio bitrate in kbps
    pub bitrate_kbps: u32,
    /// Output sample rate in Hz (None = use source)
    pub sample_rate_hz: Option<u32>,
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
}

impl Default for ConversionSettings {
    fn default() -> Self {
        Self {
            bitrate_kbps: 128,
            sample_rate_hz: None,
            output_format: AudioFormat::M4b,
            output_path: Utf8PathBuf::new(),
            concurrent_jobs: 1,
            overwrite_existing: false,
            embed_chapters: true,
            embed_artwork: true,
            copy_metadata: true,
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
        self.completed_at = Some(Utc::now());
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Error;
        self.error_message = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Mark job as cancelled
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
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

/// Progress information sent to frontend during conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionProgress {
    /// Job identifier
    pub job_id: String,
    /// Index of current file being processed
    pub file_index: usize,
    /// Name of current file
    pub current_file: String,
    /// Progress of current file (0.0 - 100.0)
    pub file_progress: f32,
    /// Overall progress (0.0 - 100.0)
    pub total_progress: f32,
    /// Estimated time remaining in seconds
    pub time_remaining_sec: Option<u32>,
    /// Current processing speed (e.g., "2.5x")
    pub speed: Option<String>,
    /// Current status message
    pub status_message: String,
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
}

