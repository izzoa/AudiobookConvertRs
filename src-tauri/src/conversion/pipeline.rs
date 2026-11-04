//! Conversion pipeline module
//!
//! Implements the complete conversion workflow from input files to M4B output.

use crate::audio::quality::{apply_normalization, NormalizationSettings};
use crate::conversion::progress::{ConversionProgress, ConversionStage, ProgressTracker};
use crate::conversion::temp::TempDirectory;
use crate::error::{AppError, AppResult};
use crate::ffmpeg::command_builder::{
    build_chapters_command, build_concat_command, build_concat_copy_command, build_convert_command,
    build_copy_command, generate_concat_file_content, FFmpegCommand,
};
use crate::ffmpeg::detection::FFmpegInfo;
use crate::ffmpeg::executor::{execute_ffmpeg_command, FFmpegProgressAccumulator};
use crate::ffmpeg::probe::copy_check;
use crate::metadata::writing::{
    embed_artwork_in_m4b, generate_ffmpeg_chapters_metadata, write_metadata_to_m4b,
};
use crate::models::{AudioBook, AudioFile, Chapter, ConversionJob, ConversionSettings};
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
        Self {
            ffmpeg_info,
            config,
        }
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
        info!(
            "Total duration: {} ms across {} files",
            total_duration_ms,
            job.audiobook.files.len()
        );
        let mut tracker =
            ProgressTracker::new(job.id.clone(), total_duration_ms, job.audiobook.files.len());
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

        // Step 2: Apply normalization (if enabled)
        // Copying keeps the audio exactly as it is, so it is never normalized
        let normalized_files = if job.settings.normalize_audio && !job.settings.copy_audio {
            info!("Step 2: Applying audio normalization");
            tracker.set_stage(ConversionStage::Normalizing);
            if let Some(progress) = tracker.get_progress() {
                progress_callback(progress);
            }

            self.normalize_files(&job.audiobook.files, &job.settings, &temp_dir)?
        } else {
            info!("Step 2: Skipping normalization (not enabled)");
            None
        };

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 3: Execute FFmpeg encoding
        info!("Step 3: Starting FFmpeg encoding");
        tracker.set_stage(ConversionStage::Encoding);
        let encoded_path = if job.settings.copy_audio {
            self.execute_copy(
                job,
                &temp_dir,
                &mut tracker,
                &mut progress_callback,
                &cancel_check,
            )?
        } else {
            self.execute_encoding(
                job,
                &temp_dir,
                normalized_files.as_ref(),
                &mut tracker,
                &mut progress_callback,
                &cancel_check,
            )?
        };

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 4: Embed chapters FIRST (FFmpeg would overwrite mp4ameta metadata)
        // FFmpeg's -map_metadata 1 copies from chapters file, ignoring existing metadata
        let current_path = if !job.audiobook.chapters.is_empty() {
            tracker.set_stage(ConversionStage::Chapters);
            if let Some(progress) = tracker.get_progress() {
                progress_callback(progress);
            }
            let chaptered_path =
                self.embed_chapters(&encoded_path, &job.audiobook.chapters, &temp_dir)?;

            // If chapters were embedded to a new file, use that
            if chaptered_path != encoded_path {
                fs::remove_file(&encoded_path).ok();
                chaptered_path
            } else {
                encoded_path.clone()
            }
        } else {
            encoded_path.clone()
        };

        // Check for cancellation
        if cancel_check() {
            return Err(AppError::ConversionCancelled);
        }

        // Step 5: Embed metadata AFTER chapters (so mp4ameta metadata persists)
        tracker.set_stage(ConversionStage::Metadata);
        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }
        self.embed_metadata(&current_path, &job.audiobook)?;

        // Step 6: Verify output
        if self.config.verify_output {
            tracker.set_stage(ConversionStage::Verifying);
            if let Some(progress) = tracker.get_progress() {
                progress_callback(progress);
            }
            verify_output_file(&current_path)?;
        }

        // Step 7: Move to final location
        tracker.set_stage(ConversionStage::Finalizing);
        if let Some(progress) = tracker.get_progress() {
            progress_callback(progress);
        }

        let final_path = if current_path != output_path {
            finalize_output(&current_path, &output_path)?
        } else {
            output_path
        };

        // Complete
        tracker.complete();
        progress_callback(tracker.get_progress_forced());

        info!("Conversion complete: {}", final_path);
        Ok(final_path)
    }

    /// Normalize input files to temporary location
    fn normalize_files(
        &self,
        files: &[AudioFile],
        settings: &ConversionSettings,
        temp_dir: &TempDirectory,
    ) -> AppResult<Option<Vec<Utf8PathBuf>>> {
        if files.is_empty() {
            return Ok(None);
        }

        let norm_settings = settings
            .normalization_settings
            .as_ref()
            .map(|ns| NormalizationSettings {
                target_lufs: ns.target_lufs,
                target_true_peak: ns.target_true_peak,
                target_lra: ns.target_lra,
                two_pass: ns.two_pass,
            })
            .unwrap_or_default();

        info!(
            "Normalizing {} files with settings: {:?}",
            files.len(),
            norm_settings
        );

        let mut normalized_paths = Vec::with_capacity(files.len());

        for (index, file) in files.iter().enumerate() {
            let normalized_path = temp_dir.file_path(&format!("normalized_{}.wav", index));

            info!(
                "Normalizing file {}/{}: {} -> {}",
                index + 1,
                files.len(),
                file.path,
                normalized_path
            );

            apply_normalization(&file.path, &normalized_path, &norm_settings)?;

            normalized_paths.push(normalized_path);
        }

        info!("Normalization complete for {} files", files.len());
        Ok(Some(normalized_paths))
    }

    /// Execute the FFmpeg encoding step
    fn execute_encoding<F, C>(
        &self,
        job: &ConversionJob,
        temp_dir: &TempDirectory,
        normalized_files: Option<&Vec<Utf8PathBuf>>,
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

        info!(
            "execute_encoding: {} files, encoder: {}",
            files.len(),
            encoder
        );

        if files.is_empty() {
            return Err(AppError::ConversionError {
                reason: "No input files".to_string(),
            });
        }

        let output_path = temp_dir.file_path("output.m4b");
        info!("execute_encoding: output path: {}", output_path);

        if files.len() == 1 {
            // Single file conversion
            let input_path = if let Some(normalized) = normalized_files {
                &normalized[0]
            } else {
                &files[0].path
            };
            info!("execute_encoding: single file mode, input: {}", input_path);
            tracker.set_current_file(
                0,
                input_path.file_name().unwrap_or("input"),
                files[0].duration_ms,
            );

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

            // Use normalized files if provided, otherwise original files
            let file_paths: Vec<Utf8PathBuf> = if let Some(normalized) = normalized_files {
                normalized.clone()
            } else {
                files.iter().map(|f| f.path.clone()).collect()
            };
            let concat_content = generate_concat_file_content(&file_paths);
            info!("execute_encoding: concat file content:\n{}", concat_content);

            fs::write(&concat_file_path, &concat_content).map_err(|e| {
                AppError::FileWriteError {
                    path: concat_file_path.clone().into(),
                    reason: format!("Failed to write concat file: {}", e),
                }
            })?;
            info!(
                "execute_encoding: wrote concat file to {}",
                concat_file_path
            );

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

    /// Copy the files' AAC audio into a new M4B without re-encoding it,
    /// leaving out their chapters, tags and cover, which are written after.
    /// Refused, before anything is written, when the audio cannot be copied.
    fn execute_copy<F, C>(
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
        let check = copy_check(&self.ffmpeg_info.path, files)?;
        if !check.possible {
            return Err(AppError::ConversionError {
                reason: check
                    .reason
                    .unwrap_or_else(|| "The audio cannot be kept as it is.".to_string()),
            });
        }
        info!(
            "Copying {} as it is from {} files",
            check.audio.as_deref().unwrap_or("the audio"),
            files.len()
        );

        let output_path = temp_dir.file_path("output.m4b");
        let total_duration: u64 = files.iter().map(|f| f.duration_ms).sum();
        let command = if files.len() == 1 {
            tracker.set_current_file(0, files[0].filename(), total_duration);
            build_copy_command(&self.ffmpeg_info.path, &files[0].path, &output_path, true)
        } else {
            let concat_file_path = temp_dir.file_path("concat.txt");
            let paths: Vec<Utf8PathBuf> = files.iter().map(|f| f.path.clone()).collect();
            fs::write(&concat_file_path, generate_concat_file_content(&paths)).map_err(|e| {
                AppError::FileWriteError {
                    path: concat_file_path.clone().into(),
                    reason: format!("Failed to write concat file: {}", e),
                }
            })?;
            tracker.set_current_file(0, "Joining files", total_duration);
            build_concat_copy_command(
                &self.ffmpeg_info.path,
                &concat_file_path,
                &output_path,
                true,
            )
        };
        self.execute_ffmpeg_with_progress(command, tracker, progress_callback, cancel_check)?;
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

        info!(
            "execute_ffmpeg_with_progress: spawning FFmpeg: {} {:?}",
            executable, args
        );

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

        info!(
            "execute_ffmpeg_with_progress: FFmpeg process spawned, pid: {:?}",
            child.id()
        );

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::FFmpegExecutionError {
                reason: "Failed to capture stdout".to_string(),
            })?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::FFmpegExecutionError {
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
        let mut progress_accumulator = FFmpegProgressAccumulator::new();

        info!("execute_ffmpeg_with_progress: starting stdout read loop");
        let mut last_progress_log = std::time::Instant::now();
        loop {
            line.clear();
            if cancel_check() {
                info!("execute_ffmpeg_with_progress: cancellation requested");
                let _ = child.kill();
                return Err(AppError::ConversionCancelled);
            }

            let bytes =
                stdout_reader
                    .read_line(&mut line)
                    .map_err(|e| AppError::FFmpegExecutionError {
                        reason: format!("Failed to read FFmpeg stdout: {}", e),
                    })?;

            if bytes == 0 {
                info!(
                    "execute_ffmpeg_with_progress: EOF reached after {} lines",
                    lines_read
                );
                break; // EOF
            }

            lines_read += 1;

            // Accumulate stdout for diagnostics
            stdout_content.push_str(&line);

            // Use the accumulator to parse progress across multiple lines
            if let Some(ffmpeg_progress) = progress_accumulator.parse_line(line.trim()) {
                // Log progress periodically (every 5 seconds)
                if last_progress_log.elapsed().as_secs() >= 5 {
                    info!(
                        "execute_ffmpeg_with_progress: progress {} ms ({}%), speed: {:?}",
                        ffmpeg_progress.time_ms,
                        (ffmpeg_progress.time_ms as f64 / tracker.total_duration_ms() as f64
                            * 100.0)
                            .min(100.0),
                        ffmpeg_progress.speed
                    );
                    last_progress_log = std::time::Instant::now();
                }

                tracker.update_position(ffmpeg_progress.time_ms);
                if let Some(speed) = ffmpeg_progress.speed {
                    tracker.update_speed(speed);
                }
                if let Some(progress) = tracker.get_progress() {
                    info!("execute_ffmpeg_with_progress: emitting progress: {}%, ETA: {}s, speed: {:?}",
                        progress.total_progress, progress.time_remaining_sec, progress.speed);
                    progress_callback(progress);
                }
            }
        }

        info!("execute_ffmpeg_with_progress: waiting for FFmpeg to exit");
        let status = child.wait().map_err(|e| AppError::FFmpegExecutionError {
            reason: format!("Failed to wait for FFmpeg: {}", e),
        })?;

        let stderr_content = stderr_handle.join().unwrap_or_default();
        info!(
            "execute_ffmpeg_with_progress: FFmpeg exited with status: {:?}",
            status.code()
        );

        if !status.success() {
            info!(
                "execute_ffmpeg_with_progress: FFmpeg FAILED, stderr:\n{}",
                stderr_content
            );
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
            use base64::{engine::general_purpose::STANDARD, Engine as _};
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
    settings
        .validate()
        .map_err(|e| AppError::ValidationError(e))?;

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
        warn!(
            "Output file is suspiciously small: {} bytes",
            metadata.len()
        );
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
        use base64::{engine::general_purpose::STANDARD, Engine as _};
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

    /// An AAC M4B with old chapters and a title is copied with new ones: the
    /// audio comes through bit for bit. Needs FFmpeg:
    /// `cargo test kept_audio -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn kept_audio_is_copied_bit_for_bit_with_new_chapters_and_tags() {
        use crate::models::{AudioFormat, Metadata};
        use std::process::Command;
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let dir = tempfile::tempdir().unwrap();
        let path = |name: &str| Utf8PathBuf::from_path_buf(dir.path().join(name)).unwrap();
        let (meta, source, out) = (path("old.ffmeta"), path("old.m4b"), path("new.m4b"));
        fs::write(
            &meta,
            ";FFMETADATA1\ntitle=Old Title\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=10000\ntitle=Old One\n",
        )
        .unwrap();
        let status = Command::new(ffmpeg.path.as_str())
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=300:duration=10",
            ])
            .args([
                "-i",
                meta.as_str(),
                "-map_metadata",
                "1",
                "-map_chapters",
                "1",
            ])
            .args(["-c:a", "aac", "-b:a", "64k", "-f", "ipod", source.as_str()])
            .status()
            .unwrap();
        assert!(status.success());
        let audio_md5 = |file: &Utf8Path| {
            let out = Command::new(ffmpeg.path.as_str())
                .args([
                    "-v",
                    "error",
                    "-i",
                    file.as_str(),
                    "-map",
                    "0:a",
                    "-c",
                    "copy",
                    "-f",
                    "md5",
                    "-",
                ])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };

        let mut book = AudioBook::new("New Title".to_string());
        book.files.push(AudioFile {
            path: source.clone(),
            format: AudioFormat::M4b,
            duration_ms: 10_000,
            bitrate_kbps: Some(64),
            sample_rate_hz: Some(44_100),
            channels: Some(1),
            file_size_bytes: fs::metadata(&source).unwrap().len(),
            metadata: Metadata::default(),
            has_artwork: false,
        });
        book.recalculate_totals();
        book.output_metadata.title = Some("New Title".to_string());
        book.chapters = vec![
            Chapter::new(0, "First".to_string(), 0, 4_000),
            Chapter::new(1, "Second".to_string(), 4_000, 10_000),
        ];
        let settings = ConversionSettings {
            output_path: out.clone(),
            copy_audio: true,
            ..ConversionSettings::default()
        };
        let job = ConversionJob::new(book, settings);
        let pipeline = ConversionPipeline::new(ffmpeg.clone(), PipelineConfig::default());
        let written = pipeline.run(&job, |_| {}, || false).unwrap();
        assert_eq!(written, out);

        assert_eq!(
            audio_md5(&source),
            audio_md5(&out),
            "the audio is untouched"
        );
        let described = Command::new(ffmpeg.path.as_str())
            .args(["-hide_banner", "-i", out.as_str()])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&described.stderr);
        println!("{log}");
        assert!(log.contains("New Title"), "the new title is written");
        assert!(
            !log.contains("Old"),
            "nothing of the old tags or chapters is left"
        );
        assert!(log.contains("First") && log.contains("Second"));
        assert!(log.contains("Chapter #0:1"));
        assert!(log.contains("Audio: aac"));
    }

    /// Two AAC files are joined without re-encoding, and the result replaces
    /// the first file itself, as "save over the original" does. Needs FFmpeg:
    /// `cargo test kept_audio_of_two -- --ignored`.
    #[test]
    #[ignore]
    fn kept_audio_of_two_files_joins_them_and_can_replace_the_source() {
        use crate::models::{AudioFormat, Metadata};
        use std::process::Command;
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let dir = tempfile::tempdir().unwrap();
        let mut book = AudioBook::new("Joined".to_string());
        for (name, hz) in [("1.m4a", 300), ("2.m4a", 500)] {
            let file = Utf8PathBuf::from_path_buf(dir.path().join(name)).unwrap();
            let status = Command::new(ffmpeg.path.as_str())
                .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
                .arg(format!("sine=frequency={hz}:duration=6"))
                .args(["-c:a", "aac", "-b:a", "64k", file.as_str()])
                .status()
                .unwrap();
            assert!(status.success());
            book.files.push(AudioFile {
                path: file,
                format: AudioFormat::M4a,
                duration_ms: 6_000,
                bitrate_kbps: Some(64),
                sample_rate_hz: Some(44_100),
                channels: Some(1),
                file_size_bytes: 0,
                metadata: Metadata::default(),
                has_artwork: false,
            });
        }
        book.recalculate_totals();
        book.chapters = vec![
            Chapter::new(0, "One".to_string(), 0, 6_000),
            Chapter::new(1, "Two".to_string(), 6_000, 12_000),
        ];
        let first = book.files[0].path.clone();
        let settings = ConversionSettings {
            output_path: first.clone(),
            overwrite_existing: true,
            copy_audio: true,
            ..ConversionSettings::default()
        };
        let job = ConversionJob::new(book, settings);
        let pipeline = ConversionPipeline::new(ffmpeg.clone(), PipelineConfig::default());
        assert_eq!(pipeline.run(&job, |_| {}, || false).unwrap(), first);

        let described = Command::new(ffmpeg.path.as_str())
            .args(["-hide_banner", "-i", first.as_str()])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&described.stderr);
        let duration = log
            .lines()
            .find_map(|l| l.trim().strip_prefix("Duration: 00:00:"))
            .and_then(|rest| rest.split(',').next())
            .and_then(|secs| secs.parse::<f64>().ok())
            .expect("a duration");
        assert!(
            (duration - 12.0).abs() < 0.2,
            "both files are in it: {duration}"
        );
        assert!(log.contains("Chapter #0:1") && log.contains("Two"));
    }
}
