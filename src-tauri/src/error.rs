//! Error handling module for AudiobookConvertRs
//!
//! Defines custom error types for all operations in the application,
//! providing user-friendly messages and proper error conversion.

use std::path::PathBuf;
use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    // ============================================
    // File I/O Errors
    // ============================================
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unable to read file: {path} - {reason}")]
    FileReadError { path: PathBuf, reason: String },

    #[error("Unable to write file: {path} - {reason}")]
    FileWriteError { path: PathBuf, reason: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("Insufficient disk space: need {needed} bytes, have {available} bytes")]
    InsufficientDiskSpace { needed: u64, available: u64 },

    // ============================================
    // Audio Format Errors
    // ============================================
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Unable to detect audio format for: {0}")]
    FormatDetectionFailed(PathBuf),

    #[error("Corrupted audio file: {path} - {reason}")]
    CorruptedAudioFile { path: PathBuf, reason: String },

    #[error("No audio stream found in: {0}")]
    NoAudioStream(PathBuf),

    #[error("Invalid audio duration: {0}")]
    InvalidDuration(String),

    // ============================================
    // Metadata Errors
    // ============================================
    #[error("Failed to read metadata from: {path} - {reason}")]
    MetadataReadError { path: PathBuf, reason: String },

    #[error("Failed to write metadata to: {path} - {reason}")]
    MetadataWriteError { path: PathBuf, reason: String },

    #[error("Invalid metadata field: {field} - {reason}")]
    InvalidMetadataField { field: String, reason: String },

    #[error("Artwork processing failed: {0}")]
    ArtworkError(String),

    // ============================================
    // Chapter Errors
    // ============================================
    #[error("Invalid chapter: {reason}")]
    InvalidChapter { reason: String },

    #[error("Chapter times overlap: chapter {index1} and chapter {index2}")]
    ChapterOverlap { index1: usize, index2: usize },

    #[error("Chapter end time ({end_ms}ms) exceeds total duration ({total_ms}ms)")]
    ChapterExceedsDuration { end_ms: u64, total_ms: u64 },

    // ============================================
    // FFmpeg Errors
    // ============================================
    #[error("FFmpeg not found. Please install FFmpeg and ensure it's in your PATH.")]
    FFmpegNotFound,

    #[error("FFmpeg version {version} is too old. Minimum required: {minimum}")]
    FFmpegVersionTooOld { version: String, minimum: String },

    #[error("FFmpeg execution failed: {reason}")]
    FFmpegExecutionError { reason: String },

    #[error("FFmpeg command timed out after {seconds} seconds")]
    FFmpegTimeout { seconds: u64 },

    #[error("FFmpeg output parsing failed: {0}")]
    FFmpegOutputParseError(String),

    // ============================================
    // Conversion Errors
    // ============================================
    // ============================================
    // Conversion Errors
    // ============================================
    #[error("Conversion failed: {reason}")]
    ConversionFailed { reason: String },

    #[error("Conversion error: {reason}")]
    ConversionError { reason: String },

    #[error("Conversion cancelled by user")]
    ConversionCancelled,

    #[error("Output file already exists: {0}")]
    OutputFileExists(PathBuf),

    #[error("Invalid conversion settings: {0}")]
    InvalidConversionSettings(String),

    // ============================================
    // Validation Errors
    // ============================================
    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Chapter validation failed: {reason}")]
    ChapterValidationError { reason: String },

    #[error("No files provided for conversion")]
    NoFilesProvided,

    #[error("Maximum file count exceeded: {count} (max: {max})")]
    TooManyFiles { count: usize, max: usize },

    // ============================================
    // State/IPC Errors
    // ============================================
    #[error("Invalid audiobook ID: {0}")]
    InvalidAudiobookId(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("State access error: {0}")]
    StateError(String),

    // ============================================
    // Configuration Errors
    // ============================================
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to load configuration: {0}")]
    ConfigLoadError(String),

    #[error("Failed to save configuration: {0}")]
    ConfigSaveError(String),

    // ============================================
    // Generic Errors
    // ============================================
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

// ============================================
// Error Conversions
// ============================================

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

impl AppError {
    /// Returns true if this error is recoverable (user can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AppError::FileNotFound(_)
                | AppError::PermissionDenied(_)
                | AppError::InsufficientDiskSpace { .. }
                | AppError::FFmpegNotFound
                | AppError::OutputFileExists(_)
                | AppError::ConversionCancelled
        )
    }

    /// Returns a user-friendly recovery suggestion
    pub fn recovery_suggestion(&self) -> Option<&'static str> {
        match self {
            AppError::FileNotFound(_) => Some("Please check if the file exists and try again."),
            AppError::PermissionDenied(_) => {
                Some("Please check file permissions and try again.")
            }
            AppError::InsufficientDiskSpace { .. } => {
                Some("Free up disk space and try again.")
            }
            AppError::FFmpegNotFound => {
                Some("Install FFmpeg from https://ffmpeg.org/download.html")
            }
            AppError::OutputFileExists(_) => {
                Some("Choose a different output path or delete the existing file.")
            }
            AppError::UnsupportedFormat(_) => {
                Some("Supported formats: MP3, FLAC, M4A, M4B, WAV, OGG")
            }
            _ => None,
        }
    }

    /// Create a file read error with context
    pub fn file_read(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        AppError::FileReadError {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a file write error with context
    pub fn file_write(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        AppError::FileWriteError {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a metadata read error with context
    pub fn metadata_read(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        AppError::MetadataReadError {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create an FFmpeg execution error with context
    pub fn ffmpeg_error(reason: impl Into<String>) -> Self {
        AppError::FFmpegExecutionError {
            reason: reason.into(),
        }
    }

    /// Create a conversion error with context
    pub fn conversion_failed(reason: impl Into<String>) -> Self {
        AppError::ConversionFailed {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::FileNotFound(PathBuf::from("/test/file.mp3"));
        assert_eq!(err.to_string(), "File not found: /test/file.mp3");
    }

    #[test]
    fn test_error_is_recoverable() {
        assert!(AppError::FileNotFound(PathBuf::from("test")).is_recoverable());
        assert!(AppError::FFmpegNotFound.is_recoverable());
        assert!(!AppError::InternalError("test".into()).is_recoverable());
    }

    #[test]
    fn test_error_recovery_suggestion() {
        assert!(AppError::FFmpegNotFound.recovery_suggestion().is_some());
        assert!(AppError::InternalError("test".into())
            .recovery_suggestion()
            .is_none());
    }

    #[test]
    fn test_error_to_string_conversion() {
        let err = AppError::UnsupportedFormat("xyz".into());
        let s: String = err.into();
        assert!(s.contains("Unsupported audio format"));
    }
}

