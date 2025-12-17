//! Conversion pipeline module
//!
//! Implements the complete conversion workflow from input files to M4B output.

use crate::conversion::progress::{ConversionProgress, ConversionStage, ProgressTracker};
use crate::conversion::temp::TempDirectory;
use crate::error::{AppError, AppResult};
use crate::ffmpeg::command_builder::{
    build_chapters_command, build_concat_command, build_convert_command,
    generate_concat_file_content, FFmpegCommand,
};
use crate::ffmpeg::detection::FFmpegInfo;
use crate::ffmpeg::executor::{execute_ffmpeg_command, execute_with_progress, parse_ffmpeg_progress};
use crate::metadata::writing::{embed_artwork_in_m4b, generate_ffmpeg_chapters_metadata, write_metadata_to_m4b};
use crate::models::{AudioBook, Chapter, ConversionJob, ConversionSettings, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use tracing::{debug, error, info, warn};

/// Configuration for the conversion pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Whether to preserve intermediate files (for debugging)
    pub preserve_temp_files: bool,
    /// Maximum retries for FFmpeg commands
    pub max_retries: usize,
    /// Whether to verify output file
    pub verify_output: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            preserve_temp_files: false,
            max_retries: 2,
            verify_output: true,
        }
    }
}

/// The conversion pipeline orchestrates the complete conversion process
pub struct ConversionPipeline {
    /// FFmpeg information
    ffmpeg_info: FFmpegInfo,
    /// Pipeline configuration
    config: PipelineConfig,
}

impl ConversionPipeline {
    /// Create a new conversion pipeline
    pub fn new(ffmpeg_info: FFmpegInfo, config: PipelineConfig) -> Self {
        Self { ffmpeg_info, config }
    }

    /// Run the complete conversion pipeline
    ///
    /// # Arguments
    /// * `job` - The conversion job to process
    /// * `progress_callback` - Called with progress updates
    /// * `cancel_check` - Called to check if cancellation was requested
    ///
    /// # Returns
    /// Path to the output file on success
    pub fn run<F, C>(
        &self,
        job: &ConversionJob,
        mut progress_callback: F,
        cancel_check: C,
    ) -> AppResult<Utf8PathBuf>
    where
        F: FnMut(ConversionProgress),
        C: Fn() -> bool,
    {
        info!("Starting conversion pipeline for job {}", job.id);

        // Validate inputs
        validate_conversion_inputs(&job.audiobook, &job.settings)?;

        // Create temp directory
        let mut temp_dir = TempDirectory::new(&job.id)?;
        if self.config.preserve_temp_files {
            temp_dir.preserve();
        }

        // Initialize progress tracker
        let total_duration_ms: u64 = job.audiobook.files.iter().map(|f| f.duration_ms).sum();
        let mut tracker = ProgressTracker::new(
            job.id.clone(),
            total_duration_ms,
            job.audiobook.files.len(),
        );
        tracker.start();

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 1: Prepare encoding
        tracker.set_stage(ConversionStage::Preparing);
        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }

        // Determine output path
        let output_path = if job.settings.output_path.as_str().is_empty() {
            // Use default output path
            temp_dir.file_path(&format!("{}.m4b", job.audiobook.suggested_filename()))
        } else {
            job.settings.output_path.clone()
        };

        // Step 2: Execute FFmpeg encoding
        tracker.set_stage(ConversionStage::Encoding);
        let encoded_path = self.execute_encoding(
            job,
            &temp_dir,
            &mut tracker,
            &mut progress_callback,
            &cancel_check,
        )?;

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 3: Embed metadata
        tracker.set_stage(ConversionStage::Metadata);
        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }
        self.embed_metadata(&encoded_path, &job.audiobook)?;

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 4: Embed chapters
        if !job.audiobook.chapters.is_empty() {
            tracker.set_stage(ConversionStage::Chapters);
            if let Some(progress) = tracker.get_progress() {
                progress_callback(progress);
            }
            let chaptered_path = self.embed_chapters(
                &encoded_path,
                &job.audiobook.chapters,
                &temp_dir,
            )?;
            
            // If chapters were embedded to a new file, use that
            if chaptered_path != encoded_path {
                fs::remove_file(&encoded_path).ok();
            }
        }

        // Step 5: Verify output
        if self.config.verify_output {
            tracker.set_stage(ConversionStage::Verifying);
            if let Some(progress) = tracker.get_progress() {
                progress_callback(progress);
            }
            verify_output_file(&encoded_path)?;
        }

        // Step 6: Move to final location
        tracker.set_stage(ConversionStage::Finalizing);
        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }

        let final_path = if encoded_path != output_path {
            finalize_output(&encoded_path, &output_path)?
        } else {
            output_path
        };

        // Complete
        tracker.complete();
        progress_callback(tracker.get_progress_forced());

        info!("Conversion complete: {}", final_path);
        Ok(final_path)
    }

    /// Execute the FFmpeg encoding step
    fn execute_encoding<F, C>(
        &self,
        job: &ConversionJob,
        temp_dir: &TempDirectory,
        tracker: &mut ProgressTracker,
        progress_callback: &mut F,
        cancel_check: &C,
    ) -> AppResult<Utf8PathBuf>
    where
        F: FnMut(ConversionProgress),
        C: Fn() -> bool,
    {
        let files = &job.audiobook.files;
        let settings = &job.settings;
        let encoder = self.ffmpeg_info.aac_encoder();

        if files.is_empty() {
            return Err(AppError::ConversionError {
                reason: "No input files".to_string(),
            });
        }

        let output_path = temp_dir.file_path("output.m4b");

        if files.len() == 1 {
            // Single file conversion
            let input_path = &files[0].path;
            tracker.set_current_file(0, input_path.file_name().unwrap_or("input"), files[0].duration_ms);

            let command = build_convert_command(
                &self.ffmpeg_info.path,
                input_path,
                &output_path,
                settings,
                encoder,
            );

            self.execute_ffmpeg_with_progress(command, tracker, progress_callback, cancel_check)?;
        } else {
            // Multiple files - use concat demuxer
            let concat_file_path = temp_dir.file_path("concat.txt");
            
            // Convert AudioFile paths to Utf8PathBuf
            let file_paths: Vec<Utf8PathBuf> = files.iter().map(|f| f.path.clone()).collect();
            let concat_content = generate_concat_file_content(&file_paths);
            
            fs::write(&concat_file_path, concat_content).map_err(|e| AppError::FileWriteError {
                path: concat_file_path.clone().into(),
                reason: format!("Failed to write concat file: {}", e),
            })?;

            // Set tracker for total duration
            let total_duration: u64 = files.iter().map(|f| f.duration_ms).sum();
            tracker.set_current_file(0, "Concatenating files", total_duration);

            let command = build_concat_command(
                &self.ffmpeg_info.path,
                &concat_file_path,
                &output_path,
                settings,
                encoder,
            );

            self.execute_ffmpeg_with_progress(command, tracker, progress_callback, cancel_check)?;
        }

        Ok(output_path)
    }

    /// Execute FFmpeg with progress tracking
    fn execute_ffmpeg_with_progress<F, C>(
        &self,
        command: FFmpegCommand,
        tracker: &mut ProgressTracker,
        progress_callback: &mut F,
        cancel_check: &C,
    ) -> AppResult<()>
    where
        F: FnMut(ConversionProgress),
        C: Fn() -> bool,
    {
        // For now, use simple execution without live progress
        // Real-time progress would require async execution
        let output = execute_ffmpeg_command(&command)?;

        if !output.success() {
            return Err(AppError::FFmpegExecutionError {
                reason: format!("FFmpeg failed: {}", output.stderr),
            });
        }

        // Parse final progress from output
        for line in output.stderr.lines() {
            if let Some(progress) = parse_ffmpeg_progress(line) {
                tracker.update_position(progress.time_ms);
                if let Some(speed) = progress.speed {
                    tracker.update_speed(speed);
                }
            }
        }

        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }

        Ok(())
    }

    /// Embed metadata into the M4B file
    fn embed_metadata(&self, m4b_path: &Utf8Path, audiobook: &AudioBook) -> AppResult<()> {
        info!("Embedding metadata into {}", m4b_path);

        // Write text metadata
        write_metadata_to_m4b(m4b_path, &audiobook.output_metadata)?;

        // Embed artwork if available
        if let Some(ref artwork_path) = audiobook.output_metadata.artwork_path {
            if artwork_path.exists() {
                let artwork_data = fs::read(artwork_path).map_err(|e| AppError::FileReadError {
                    path: artwork_path.clone().into(),
                    reason: format!("Failed to read artwork: {}", e),
                })?;
                embed_artwork_in_m4b(m4b_path, &artwork_data)?;
            }
        }

        Ok(())
    }

    /// Embed chapters using FFmpeg metadata
    fn embed_chapters(
        &self,
        m4b_path: &Utf8Path,
        chapters: &[Chapter],
        temp_dir: &TempDirectory,
    ) -> AppResult<Utf8PathBuf> {
        if chapters.is_empty() {
            return Ok(m4b_path.to_owned());
        }

        info!("Embedding {} chapters into {}", chapters.len(), m4b_path);

        // Generate FFmpeg metadata file
        let metadata_content = generate_ffmpeg_chapters_metadata(chapters);
        let metadata_path = temp_dir.file_path("chapters.txt");
        
        fs::write(&metadata_path, &metadata_content).map_err(|e| AppError::FileWriteError {
            path: metadata_path.clone().into(),
            reason: format!("Failed to write chapters file: {}", e),
        })?;

        // Create chaptered output
        let output_path = temp_dir.file_path("output_chaptered.m4b");
        
        let command = build_chapters_command(
            &self.ffmpeg_info.path,
            m4b_path,
            &metadata_path,
            &output_path,
        );

        let output = execute_ffmpeg_command(&command)?;

        if !output.success() {
            warn!("Failed to embed chapters via FFmpeg: {}", output.stderr);
            // Return original path if chapter embedding fails
            return Ok(m4b_path.to_owned());
        }

        // Replace original with chaptered version
        fs::remove_file(m4b_path).ok();
        fs::rename(&output_path, m4b_path).map_err(|e| AppError::FileWriteError {
            path: m4b_path.to_path_buf().into(),
            reason: format!("Failed to rename chaptered file: {}", e),
        })?;

        Ok(m4b_path.to_owned())
    }
}

/// Validate conversion inputs before starting
pub fn validate_conversion_inputs(
    audiobook: &AudioBook,
    settings: &ConversionSettings,
) -> AppResult<()> {
    // Validate we have files
    if audiobook.files.is_empty() {
        return Err(AppError::ValidationError(
            "No audio files to convert".to_string(),
        ));
    }

    // Validate all input files exist
    for file in &audiobook.files {
        if !file.path.exists() {
            return Err(AppError::FileNotFound(file.path.clone().into()));
        }
    }

    // Validate settings
    settings.validate().map_err(|e| AppError::ValidationError(e))?;

    // Validate output directory is writable
    if !settings.output_path.as_str().is_empty() {
        if let Some(parent) = settings.output_path.parent() {
            if !parent.as_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| AppError::FileWriteError {
                    path: parent.to_path_buf().into(),
                    reason: format!("Cannot create output directory: {}", e),
                })?;
            }
        }
    }

    // Validate chapters don't exceed duration
    let total_duration: u64 = audiobook.files.iter().map(|f| f.duration_ms).sum();
    for chapter in &audiobook.chapters {
        if chapter.end_time_ms > total_duration {
            return Err(AppError::ChapterValidationError {
                reason: format!(
                    "Chapter '{}' end time ({} ms) exceeds total duration ({} ms)",
                    chapter.title, chapter.end_time_ms, total_duration
                ),
            });
        }
    }

    Ok(())
}

/// Prepare a temporary directory for the conversion
pub fn prepare_temp_directory(job_id: &str) -> AppResult<TempDirectory> {
    TempDirectory::new(job_id)
}

/// Verify the output M4B file is valid
pub fn verify_output_file(m4b_path: &Utf8Path) -> AppResult<()> {
    info!("Verifying output file: {}", m4b_path);

    // Check file exists
    if !m4b_path.exists() {
        return Err(AppError::ConversionError {
            reason: "Output file was not created".to_string(),
        });
    }

    // Check file has reasonable size
    let metadata = fs::metadata(m4b_path).map_err(|e| AppError::FileReadError {
        path: m4b_path.to_path_buf().into(),
        reason: format!("Cannot read output file: {}", e),
    })?;

    if metadata.len() == 0 {
        return Err(AppError::ConversionError {
            reason: "Output file is empty".to_string(),
        });
    }

    // Minimum size check (at least 1KB for any valid audio)
    if metadata.len() < 1024 {
        warn!("Output file is suspiciously small: {} bytes", metadata.len());
    }

    debug!("Output file verified: {} bytes", metadata.len());
    Ok(())
}

/// Move output file to final destination
fn finalize_output(temp_path: &Utf8Path, final_path: &Utf8Path) -> AppResult<Utf8PathBuf> {
    info!("Finalizing output: {} -> {}", temp_path, final_path);

    // Ensure parent directory exists
    if let Some(parent) = final_path.parent() {
        if !parent.as_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| AppError::FileWriteError {
                path: parent.to_path_buf().into(),
                reason: format!("Cannot create output directory: {}", e),
            })?;
        }
    }

    // Try to rename first (fast, same filesystem)
    match fs::rename(temp_path, final_path) {
        Ok(()) => {
            debug!("Renamed {} to {}", temp_path, final_path);
            return Ok(final_path.to_owned());
        }
        Err(e) => {
            debug!("Rename failed (cross-device?), will copy: {}", e);
        }
    }

    // Fall back to copy + delete
    fs::copy(temp_path, final_path).map_err(|e| AppError::FileWriteError {
        path: final_path.to_path_buf().into(),
        reason: format!("Failed to copy output file: {}", e),
    })?;

    fs::remove_file(temp_path).ok(); // Don't fail if we can't delete temp

    Ok(final_path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert!(!config.preserve_temp_files);
        assert_eq!(config.max_retries, 2);
        assert!(config.verify_output);
    }

    #[test]
    fn test_validate_empty_audiobook() {
        let audiobook = AudioBook::new("Test".to_string());
        let settings = ConversionSettings::default();

        let result = validate_conversion_inputs(&audiobook, &settings);
        assert!(result.is_err());
    }
}
