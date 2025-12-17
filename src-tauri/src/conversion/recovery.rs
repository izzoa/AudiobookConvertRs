//! Error recovery module
//!
//! Provides strategies for recovering from conversion failures,
//! including retry logic, skip functionality, and rollback.

use crate::conversion::temp::{cleanup_temp_directory, get_temp_directory_path};
use crate::error::{AppError, AppResult};
use crate::models::{ConversionJob, JobStatus};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use tracing::{debug, error, info, warn};

/// Maximum number of retries for a failed conversion
pub const DEFAULT_MAX_RETRIES: usize = 2;

/// Result of a recovery attempt
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Recovery succeeded, continue processing
    Continue,
    /// Recovery failed, skip this item
    Skip,
    /// Recovery failed, abort the job
    Abort(String),
}

/// Strategy for handling conversion failures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the failed operation
    Retry,
    /// Skip the problematic file
    Skip,
    /// Abort the entire job
    Abort,
}

/// Context for recovery decisions
#[derive(Debug)]
pub struct RecoveryContext {
    /// Job ID
    pub job_id: String,
    /// Current retry count
    pub retry_count: usize,
    /// Maximum retries allowed
    pub max_retries: usize,
    /// Index of current file being processed
    pub current_file_index: usize,
    /// Total number of files
    pub total_files: usize,
}

impl RecoveryContext {
    /// Create a new recovery context
    pub fn new(job_id: String, max_retries: usize) -> Self {
        Self {
            job_id,
            retry_count: 0,
            max_retries,
            current_file_index: 0,
            total_files: 1,
        }
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if retries are exhausted
    pub fn retries_exhausted(&self) -> bool {
        self.retry_count >= self.max_retries
    }
}

/// Determine the best recovery strategy for an error
pub fn determine_recovery_strategy(error: &AppError, ctx: &RecoveryContext) -> RecoveryStrategy {
    match error {
        // These errors are usually transient and worth retrying
        AppError::FFmpegTimeout { .. } => {
            if ctx.retry_count < ctx.max_retries {
                RecoveryStrategy::Retry
            } else {
                RecoveryStrategy::Abort
            }
        }

        // FFmpeg execution errors might be transient
        AppError::FFmpegExecutionError { .. } => {
            if ctx.retry_count < ctx.max_retries {
                RecoveryStrategy::Retry
            } else if ctx.total_files > 1 {
                RecoveryStrategy::Skip
            } else {
                RecoveryStrategy::Abort
            }
        }

        // File not found during conversion might be fixed by retry
        AppError::FileNotFound(_) => RecoveryStrategy::Skip,

        // These errors are not recoverable
        AppError::FFmpegNotFound => RecoveryStrategy::Abort,
        AppError::ConversionCancelled => RecoveryStrategy::Abort,
        AppError::ValidationError(_) => RecoveryStrategy::Abort,

        // Metadata errors - try to continue without metadata
        AppError::MetadataWriteError { .. } => {
            warn!("Metadata write failed, continuing without metadata");
            RecoveryStrategy::Skip
        }

        // Chapter errors - try to continue without chapters
        AppError::ChapterValidationError { .. } => {
            warn!("Chapter embedding failed, continuing without chapters");
            RecoveryStrategy::Skip
        }

        // For other errors, abort if retries exhausted
        _ => {
            if ctx.retry_count < ctx.max_retries {
                RecoveryStrategy::Retry
            } else {
                RecoveryStrategy::Abort
            }
        }
    }
}

/// Attempt to retry a failed conversion
///
/// Returns true if retry should be attempted, false otherwise.
pub fn retry_failed_conversion(
    job: &ConversionJob,
    error: &AppError,
    retry_count: &mut usize,
    max_retries: usize,
) -> bool {
    if *retry_count >= max_retries {
        error!(
            "Max retries ({}) exhausted for job {}",
            max_retries, job.id
        );
        return false;
    }

    let ctx = RecoveryContext {
        job_id: job.id.clone(),
        retry_count: *retry_count,
        max_retries,
        current_file_index: job.current_file_index,
        total_files: job.audiobook.files.len(),
    };

    match determine_recovery_strategy(error, &ctx) {
        RecoveryStrategy::Retry => {
            *retry_count += 1;
            info!(
                "Retrying job {} (attempt {}/{})",
                job.id, retry_count, max_retries
            );
            true
        }
        _ => false,
    }
}

/// Skip the current file and continue with the next
///
/// Updates the job state to skip the problematic file.
pub fn skip_current_file(job: &mut ConversionJob) -> AppResult<()> {
    if job.current_file_index + 1 >= job.audiobook.files.len() {
        return Err(AppError::ConversionError {
            reason: "Cannot skip - already on last file".to_string(),
        });
    }

    let skipped_file = &job.audiobook.files[job.current_file_index];
    warn!(
        "Skipping file {} in job {}",
        skipped_file.path.file_name().unwrap_or("unknown"),
        job.id
    );

    job.current_file_index += 1;
    Ok(())
}

/// Rollback a partial conversion
///
/// Cleans up any partial output and temporary files.
pub fn rollback_partial_conversion(job: &ConversionJob) -> AppResult<()> {
    info!("Rolling back partial conversion for job {}", job.id);

    // Clean up temp directory
    if let Ok(temp_path) = get_temp_directory_path(&job.id) {
        if temp_path.exists() {
            if let Err(e) = cleanup_temp_directory(&temp_path) {
                warn!("Failed to cleanup temp directory: {}", e);
            }
        }
    }

    // Delete partial output file if it exists
    if !job.settings.output_path.as_str().is_empty() && job.settings.output_path.exists() {
        debug!("Removing partial output: {}", job.settings.output_path);
        if let Err(e) = fs::remove_file(&job.settings.output_path) {
            warn!("Failed to remove partial output: {}", e);
        }
    }

    Ok(())
}

/// Get a user-friendly error message with recovery suggestions
pub fn get_error_message(error: &AppError) -> String {
    let base_message = error.to_string();

    if let Some(suggestion) = error.recovery_suggestion() {
        format!("{}\n\nSuggestion: {}", base_message, suggestion)
    } else {
        base_message
    }
}

/// Get detailed error information for logging
pub fn get_detailed_error(error: &AppError, job: &ConversionJob) -> String {
    let mut details = String::new();
    details.push_str(&format!("Job ID: {}\n", job.id));
    details.push_str(&format!("Status: {:?}\n", job.status));
    details.push_str(&format!(
        "Progress: {:.1}% (file {}/{})\n",
        job.progress_percent,
        job.current_file_index + 1,
        job.audiobook.files.len()
    ));
    details.push_str(&format!("Error: {}\n", error));

    if let Some(suggestion) = error.recovery_suggestion() {
        details.push_str(&format!("Recovery: {}\n", suggestion));
    }

    details
}

/// Clean up after a failed job
pub fn cleanup_failed_job(job: &ConversionJob) -> AppResult<()> {
    info!("Cleaning up failed job {}", job.id);

    // Rollback any partial work
    rollback_partial_conversion(job)?;

    Ok(())
}

/// Prepare for a retry attempt
///
/// Cleans up any partial state before retrying.
pub fn prepare_for_retry(job: &ConversionJob) -> AppResult<()> {
    debug!("Preparing for retry of job {}", job.id);

    // Clean temp directory but don't delete it
    if let Ok(temp_path) = get_temp_directory_path(&job.id) {
        if temp_path.exists() {
            // Delete contents but keep directory
            for entry in fs::read_dir(&temp_path).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(&path).ok();
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioBook, ConversionSettings};

    fn create_test_job() -> ConversionJob {
        let audiobook = AudioBook::new("Test".to_string());
        let settings = ConversionSettings::default();
        ConversionJob::new(audiobook, settings)
    }

    #[test]
    fn test_recovery_strategy_ffmpeg_not_found() {
        let error = AppError::FFmpegNotFound;
        let ctx = RecoveryContext::new("test".to_string(), 3);

        let strategy = determine_recovery_strategy(&error, &ctx);
        assert_eq!(strategy, RecoveryStrategy::Abort);
    }

    #[test]
    fn test_recovery_strategy_timeout_retry() {
        let error = AppError::FFmpegTimeout { seconds: 60 };
        let ctx = RecoveryContext::new("test".to_string(), 3);

        let strategy = determine_recovery_strategy(&error, &ctx);
        assert_eq!(strategy, RecoveryStrategy::Retry);
    }

    #[test]
    fn test_recovery_strategy_timeout_exhausted() {
        let error = AppError::FFmpegTimeout { seconds: 60 };
        let mut ctx = RecoveryContext::new("test".to_string(), 3);
        ctx.retry_count = 3;

        let strategy = determine_recovery_strategy(&error, &ctx);
        assert_eq!(strategy, RecoveryStrategy::Abort);
    }

    #[test]
    fn test_retry_failed_conversion() {
        let job = create_test_job();
        let error = AppError::FFmpegTimeout { seconds: 60 };
        let mut retry_count = 0;

        let should_retry = retry_failed_conversion(&job, &error, &mut retry_count, 3);
        assert!(should_retry);
        assert_eq!(retry_count, 1);
    }

    #[test]
    fn test_retry_exhausted() {
        let job = create_test_job();
        let error = AppError::FFmpegTimeout { seconds: 60 };
        let mut retry_count = 3;

        let should_retry = retry_failed_conversion(&job, &error, &mut retry_count, 3);
        assert!(!should_retry);
    }

    #[test]
    fn test_get_error_message() {
        let error = AppError::FFmpegNotFound;
        let message = get_error_message(&error);
        assert!(message.contains("FFmpeg"));
    }
}
