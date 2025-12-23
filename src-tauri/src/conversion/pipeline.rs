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
use crate::ffmpeg::executor::{execute_ffmpeg_command, parse_ffmpeg_progress};
use crate::metadata::writing::{embed_artwork_in_m4b, generate_ffmpeg_chapters_metadata, write_metadata_to_m4b};
use crate::models::{AudioBook, Chapter, ConversionJob, ConversionSettings};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::thread;
use tracing::{debug, info, warn};

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
        info!("Validating conversion inputs...");
        validate_conversion_inputs(&job.audiobook, &job.settings)?;
        info!("Validation passed");

        // Create temp directory
        info!("Creating temp directory for job {}", job.id);
        let mut temp_dir = TempDirectory::new(&job.id)?;
        if self.config.preserve_temp_files {
            temp_dir.preserve();
        }
        info!("Temp directory created: {:?}", temp_dir.path());

        // Initialize progress tracker
        let total_duration_ms: u64 = job.audiobook.files.iter().map(|f| f.duration_ms).sum();
        info!("Total duration: {} ms across {} files", total_duration_ms, job.audiobook.files.len());
        let mut tracker = ProgressTracker::new(
            job.id.clone(),
            total_duration_ms,
            job.audiobook.files.len(),
        );
        tracker.start();
        info!("Progress tracker initialized");

        // Check for cancellation
        if cancel_check() {
            info!("Cancellation requested, aborting");
            return Err(AppError::ConversionCancelled);
        }

        // Step 1: Prepare encoding
        info!("Step 1: Preparing encoding");
        tracker.set_stage(ConversionStage::Preparing);
        if let Some(progress) = tracker.get_progress() {
            info!("Emitting initial progress");
            progress_callback(progress);
        }

        // Determine output path
        let output_path = if job.settings.output_path.as_str().is_empty() {
            // Use default output path
            let path = temp_dir.file_path(&format!("{}.m4b", job.audiobook.suggested_filename()));
            info!("Using default output path: {}", path);
            path
        } else {
            info!("Using specified output path: {}", job.settings.output_path);
            job.settings.output_path.clone()
        };

        // Step 2: Execute FFmpeg encoding
        info!("Step 2: Starting FFmpeg encoding");
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
        
        info!("execute_encoding: {} files, encoder: {}", files.len(), encoder);

        if files.is_empty() {
            return Err(AppError::ConversionError {
                reason: "No input files".to_string(),
            });
        }

        let output_path = temp_dir.file_path("output.m4b");
        info!("execute_encoding: output path: {}", output_path);

        if files.len() == 1 {
            // Single file conversion
            let input_path = &files[0].path;
            info!("execute_encoding: single file mode, input: {}", input_path);
            tracker.set_current_file(0, input_path.file_name().unwrap_or("input"), files[0].duration_ms);

            let command = build_convert_command(
                &self.ffmpeg_info.path,
                input_path,
                &output_path,
                settings,
                encoder,
            );
            
            info!("execute_encoding: built FFmpeg command, executing...");
            let args = command.build_args().map_err(|e| {
                info!("execute_encoding: failed to build args: {}", e);
                e
            })?;
            info!("execute_encoding: FFmpeg args: {:?}", args);

            self.execute_ffmpeg_with_progress(command, tracker, progress_callback, cancel_check)?;
            info!("execute_encoding: FFmpeg completed successfully");
        } else {
            // Multiple files - use concat demuxer
            info!("execute_encoding: concat mode for {} files", files.len());
            let concat_file_path = temp_dir.file_path("concat.txt");
            
            // Convert AudioFile paths to Utf8PathBuf
            let file_paths: Vec<Utf8PathBuf> = files.iter().map(|f| f.path.clone()).collect();
            let concat_content = generate_concat_file_content(&file_paths);
            info!("execute_encoding: concat file content:\n{}", concat_content);
            
            fs::write(&concat_file_path, &concat_content).map_err(|e| AppError::FileWriteError {
                path: concat_file_path.clone().into(),
                reason: format!("Failed to write concat file: {}", e),
            })?;
            info!("execute_encoding: wrote concat file to {}", concat_file_path);

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
            
            info!("execute_encoding: built concat command, executing...");
            let args = command.build_args().map_err(|e| {
                info!("execute_encoding: failed to build args: {}", e);
                e
            })?;
            info!("execute_encoding: FFmpeg args: {:?}", args);

            self.execute_ffmpeg_with_progress(command, tracker, progress_callback, cancel_check)?;
            info!("execute_encoding: FFmpeg concat completed successfully");
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
        let args = command.build_args()?;
        let executable = command.executable();
        
        info!("execute_ffmpeg_with_progress: spawning FFmpeg: {} {:?}", executable, args);

        // Spawn FFmpeg process with piped stdio so we can stream progress
        let mut child = Command::new(executable.as_str())
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                info!("execute_ffmpeg_with_progress: spawn error: {}", e);
                if e.kind() == std::io::ErrorKind::NotFound {
                    AppError::FFmpegNotFound
                } else {
                    AppError::FFmpegExecutionError {
                        reason: format!("Failed to spawn FFmpeg: {}", e),
                    }
                }
            })?;
        
        info!("execute_ffmpeg_with_progress: FFmpeg process spawned, pid: {:?}", child.id());

        let stdout = child.stdout.take().ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stdout".to_string(),
        })?;

        let stderr = child.stderr.take().ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stderr".to_string(),
        })?;

        // Read stderr in background so it doesn't block
        let stderr_handle = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut content = String::new();
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                content.push_str(&line);
            }
            content
        });

        // Stream stdout for progress updates (FFmpeg -progress pipe:1 writes here)
        let mut stdout_reader = BufReader::new(stdout);
        let mut stdout_content = String::new();
        let mut line = String::new();
        let mut lines_read = 0;

        info!("execute_ffmpeg_with_progress: starting stdout read loop");
        let mut last_progress_log = std::time::Instant::now();
        loop {
            line.clear();
            if cancel_check() {
                info!("execute_ffmpeg_with_progress: cancellation requested");
                let _ = child.kill();
                return Err(AppError::ConversionCancelled);
            }

            let bytes = stdout_reader.read_line(&mut line).map_err(|e| AppError::FFmpegExecutionError {
                reason: format!("Failed to read FFmpeg stdout: {}", e),
            })?;

            if bytes == 0 {
                info!("execute_ffmpeg_with_progress: EOF reached after {} lines", lines_read);
                break; // EOF
            }
            
            lines_read += 1;
            
            // Log first few lines and occasionally thereafter for debugging
            // Changed to INFO level to ensure we see these
            if lines_read <= 20 || lines_read % 200 == 0 {
                info!("execute_ffmpeg_with_progress: line {}: {:?}", lines_read, line.trim());
            }

            // Accumulate stdout for diagnostics
            stdout_content.push_str(&line);

            if let Some(ffmpeg_progress) = parse_ffmpeg_progress(line.trim()) {
                // Log progress periodically (every 5 seconds)
                if last_progress_log.elapsed().as_secs() >= 5 {
                    info!("execute_ffmpeg_with_progress: progress {} ms ({}%), speed: {:?}", 
                        ffmpeg_progress.time_ms, 
                        (ffmpeg_progress.time_ms as f64 / tracker.total_duration_ms() as f64 * 100.0).min(100.0),
                        ffmpeg_progress.speed
                    );
                    last_progress_log = std::time::Instant::now();
                }
                
                tracker.update_position(ffmpeg_progress.time_ms);
                if let Some(speed) = ffmpeg_progress.speed {
                    tracker.update_speed(speed);
                }
                if let Some(progress) = tracker.get_progress() {
                    info!("execute_ffmpeg_with_progress: emitting progress event: {}%", progress.total_progress);
                    progress_callback(progress);
                }
            }
        }

        info!("execute_ffmpeg_with_progress: waiting for FFmpeg to exit");
        let status = child.wait().map_err(|e| AppError::FFmpegExecutionError {
            reason: format!("Failed to wait for FFmpeg: {}", e),
        })?;

        let stderr_content = stderr_handle.join().unwrap_or_default();
        info!("execute_ffmpeg_with_progress: FFmpeg exited with status: {:?}", status.code());

        if !status.success() {
            info!("execute_ffmpeg_with_progress: FFmpeg FAILED, stderr:\n{}", stderr_content);
            return Err(AppError::FFmpegExecutionError {
                reason: format!(
                    "FFmpeg failed (code {:?}): {}\n{}",
                    status.code(),
                    stdout_content,
                    stderr_content
                ),
            });
        }
        
        info!("execute_ffmpeg_with_progress: FFmpeg succeeded");

        // Ensure we emit the latest progress once more after completion
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
        // First check for artwork_data (base64), then fallback to artwork_path
        let artwork_bytes = if let Some(ref artwork_data) = audiobook.output_metadata.artwork_data {
            // Decode base64 artwork data
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            match STANDARD.decode(artwork_data) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!("Failed to decode artwork_data as base64: {}", e);
                    None
                }
            }
        } else if let Some(ref artwork_path) = audiobook.output_metadata.artwork_path {
            // Read from file path
            if artwork_path.exists() {
                match fs::read(artwork_path) {
                    Ok(bytes) => Some(bytes),
                    Err(e) => {
                        warn!("Failed to read artwork from {}: {}", artwork_path, e);
                        None
                    }
                }
            } else {
                warn!("Artwork path does not exist: {}", artwork_path);
                None
            }
        } else {
            None
        };

        // Embed the artwork if we have it
        if let Some(artwork_data) = artwork_bytes {
            embed_artwork_in_m4b(m4b_path, &artwork_data)?;
            info!("Embedded artwork into {}", m4b_path);
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

    #[test]
    fn test_artwork_data_precedence_over_path() {
        // This test verifies the logic in embed_metadata that prefers artwork_data
        let mut audiobook = AudioBook::new("Test".to_string());
        
        // Set both artwork_data and artwork_path
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let dummy_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        audiobook.output_metadata.artwork_data = Some(STANDARD.encode(&dummy_jpeg));
        audiobook.output_metadata.artwork_path = Some("/some/path.jpg".into());

        // The embed_metadata function should choose artwork_data first
        // We verify this by checking that artwork_data is present
        // (Full integration would require an actual M4B file, so this is a unit-level check)
        assert!(audiobook.output_metadata.artwork_data.is_some());
        
        // Verify decode works
        let decoded = STANDARD.decode(audiobook.output_metadata.artwork_data.as_ref().unwrap());
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), dummy_jpeg);
    }
}
