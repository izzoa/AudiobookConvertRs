//! FFmpeg command building utilities
//!
//! Provides a builder pattern for constructing FFmpeg commands
//! for audio conversion, concatenation, and metadata embedding.

use crate::error::{AppError, AppResult};
use crate::models::{AudioFormat, Chapter, ConversionSettings, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use std::fmt::Write;
use tracing::debug;

/// FFmpeg command builder
#[derive(Debug, Clone)]
pub struct FFmpegCommand {
    /// Path to FFmpeg executable
    ffmpeg_path: Utf8PathBuf,
    /// Input files
    inputs: Vec<FFmpegInput>,
    /// Output path
    output: Option<Utf8PathBuf>,
    /// Audio codec
    audio_codec: Option<String>,
    /// Audio bitrate in kbps
    bitrate_kbps: Option<u32>,
    /// Sample rate in Hz
    sample_rate_hz: Option<u32>,
    /// Number of audio channels
    channels: Option<u8>,
    /// Metadata to embed
    metadata: Vec<(String, String)>,
    /// Additional filters
    filters: Vec<String>,
    /// Additional output options
    output_options: Vec<String>,
    /// Overwrite output file
    overwrite: bool,
    /// Hide banner
    hide_banner: bool,
    /// Progress output (for parsing)
    progress_pipe: bool,
}

/// Input file specification
#[derive(Debug, Clone)]
pub struct FFmpegInput {
    pub path: Utf8PathBuf,
    pub options: Vec<String>,
}

impl Default for FFmpegCommand {
    fn default() -> Self {
        Self {
            ffmpeg_path: Utf8PathBuf::from("ffmpeg"),
            inputs: Vec::new(),
            output: None,
            audio_codec: None,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            metadata: Vec::new(),
            filters: Vec::new(),
            output_options: Vec::new(),
            overwrite: false,
            hide_banner: true,
            progress_pipe: true,
        }
    }
}

impl FFmpegCommand {
    /// Create a new FFmpeg command builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder with a specific FFmpeg path
    pub fn with_ffmpeg_path(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            ffmpeg_path: path.into(),
            ..Default::default()
        }
    }

    /// Add an input file
    pub fn input(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.inputs.push(FFmpegInput {
            path: path.into(),
            options: Vec::new(),
        });
        self
    }

    /// Add an input file with options
    pub fn input_with_options(
        mut self,
        path: impl Into<Utf8PathBuf>,
        options: Vec<String>,
    ) -> Self {
        self.inputs.push(FFmpegInput {
            path: path.into(),
            options,
        });
        self
    }

    /// Set the output file
    pub fn output(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Set the audio codec
    pub fn audio_codec(mut self, codec: impl Into<String>) -> Self {
        self.audio_codec = Some(codec.into());
        self
    }

    /// Set the audio bitrate in kbps
    pub fn bitrate(mut self, kbps: u32) -> Self {
        self.bitrate_kbps = Some(kbps);
        self
    }

    /// Set the sample rate in Hz
    pub fn sample_rate(mut self, hz: u32) -> Self {
        self.sample_rate_hz = Some(hz);
        self
    }

    /// Set the number of audio channels
    pub fn channels(mut self, num: u8) -> Self {
        self.channels = Some(num);
        self
    }

    /// Add a metadata field
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    /// Add all metadata from a Metadata struct
    pub fn with_metadata(mut self, metadata: &Metadata) -> Self {
        if let Some(ref title) = metadata.title {
            self.metadata.push(("title".into(), title.clone()));
        }
        if let Some(ref artist) = metadata.artist {
            self.metadata.push(("artist".into(), artist.clone()));
        }
        if let Some(ref album) = metadata.album {
            self.metadata.push(("album".into(), album.clone()));
        }
        if let Some(ref album_artist) = metadata.album_artist {
            self.metadata
                .push(("album_artist".into(), album_artist.clone()));
        }
        if let Some(ref genre) = metadata.genre {
            self.metadata.push(("genre".into(), genre.clone()));
        }
        if let Some(year) = metadata.year {
            self.metadata.push(("date".into(), year.to_string()));
        }
        if let Some(ref comments) = metadata.comments {
            self.metadata.push(("comment".into(), comments.clone()));
        }
        if let Some(ref composer) = metadata.composer {
            self.metadata.push(("composer".into(), composer.clone()));
        }
        if let Some(ref narrator) = metadata.narrator {
            self.metadata.push(("narrator".into(), narrator.clone()));
        }
        self
    }

    /// Add an audio filter
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Add an output option
    pub fn output_option(mut self, option: impl Into<String>) -> Self {
        self.output_options.push(option.into());
        self
    }

    /// Set whether to overwrite output files
    pub fn overwrite_output(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Set whether to hide the FFmpeg banner
    pub fn hide_banner(mut self, hide: bool) -> Self {
        self.hide_banner = hide;
        self
    }

    /// Build the command arguments
    pub fn build_args(&self) -> AppResult<Vec<String>> {
        let mut args = Vec::new();

        // Global options
        if self.overwrite {
            args.push("-y".to_string());
        }

        if self.hide_banner {
            args.push("-hide_banner".to_string());
        }

        // Progress output format (for parsing)
        if self.progress_pipe {
            args.push("-progress".to_string());
            args.push("pipe:1".to_string());
            args.push("-stats_period".to_string());
            args.push("0.5".to_string());
        }

        // Input files
        for input in &self.inputs {
            for opt in &input.options {
                args.push(opt.clone());
            }
            args.push("-i".to_string());
            args.push(input.path.to_string());
        }

        // Audio codec
        if let Some(ref codec) = self.audio_codec {
            args.push("-c:a".to_string());
            args.push(codec.clone());
        }

        // Bitrate
        if let Some(bitrate) = self.bitrate_kbps {
            args.push("-b:a".to_string());
            args.push(format!("{}k", bitrate));
        }

        // Sample rate
        if let Some(sr) = self.sample_rate_hz {
            args.push("-ar".to_string());
            args.push(sr.to_string());
        }

        // Channels
        if let Some(ch) = self.channels {
            args.push("-ac".to_string());
            args.push(ch.to_string());
        }

        // Filters
        if !self.filters.is_empty() {
            args.push("-af".to_string());
            args.push(self.filters.join(","));
        }

        // Metadata
        for (key, value) in &self.metadata {
            args.push("-metadata".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Additional output options
        for opt in &self.output_options {
            args.push(opt.clone());
        }

        // Output file
        if let Some(ref output) = self.output {
            args.push(output.to_string());
        } else {
            return Err(AppError::InvalidConversionSettings(
                "No output file specified".to_string(),
            ));
        }

        Ok(args)
    }

    /// Get the FFmpeg executable path
    pub fn executable(&self) -> &Utf8Path {
        &self.ffmpeg_path
    }
}

/// Build a command for converting a single audio file to M4B
pub fn build_convert_command(
    ffmpeg_path: &Utf8Path,
    input: &Utf8Path,
    output: &Utf8Path,
    settings: &ConversionSettings,
    encoder: &str,
) -> FFmpegCommand {
    let mut cmd = FFmpegCommand::with_ffmpeg_path(ffmpeg_path)
        .input(input)
        .output(output)
        .audio_codec(encoder)
        .bitrate(settings.bitrate_kbps)
        .overwrite_output(settings.overwrite_existing);

    if let Some(sr) = settings.sample_rate_hz {
        cmd = cmd.sample_rate(sr);
    }

    // M4B container format
    cmd = cmd.output_option("-f".to_string());
    cmd = cmd.output_option("ipod".to_string());

    // Copy video stream (for artwork)
    cmd = cmd.output_option("-c:v".to_string());
    cmd = cmd.output_option("copy".to_string());

    debug!("Built convert command for: {} -> {}", input, output);
    cmd
}

/// Build a command for concatenating multiple audio files
pub fn build_concat_command(
    ffmpeg_path: &Utf8Path,
    concat_file: &Utf8Path,
    output: &Utf8Path,
    settings: &ConversionSettings,
    encoder: &str,
) -> FFmpegCommand {
    let mut cmd = FFmpegCommand::with_ffmpeg_path(ffmpeg_path)
        .input_with_options(concat_file, vec!["-f".to_string(), "concat".to_string(), "-safe".to_string(), "0".to_string()])
        .output(output)
        .audio_codec(encoder)
        .bitrate(settings.bitrate_kbps)
        .overwrite_output(settings.overwrite_existing);

    if let Some(sr) = settings.sample_rate_hz {
        cmd = cmd.sample_rate(sr);
    }

    // M4B container format
    cmd = cmd.output_option("-f".to_string());
    cmd = cmd.output_option("ipod".to_string());

    debug!(
        "Built concat command: {} files -> {}",
        concat_file, output
    );
    cmd
}

/// Generate a concat file content for FFmpeg
///
/// The concat demuxer requires a text file with the format:
/// ```text
/// file '/path/to/file1.mp3'
/// file '/path/to/file2.mp3'
/// ```
pub fn generate_concat_file_content(files: &[Utf8PathBuf]) -> String {
    let mut content = String::new();
    for file in files {
        // Escape single quotes in paths
        let escaped = file.as_str().replace('\'', "'\\''");
        writeln!(content, "file '{}'", escaped).unwrap();
    }
    content
}

/// Build a command to add chapters to an M4B file
///
/// Note: This creates a metadata file and uses it to add chapters.
pub fn build_chapters_command(
    ffmpeg_path: &Utf8Path,
    input: &Utf8Path,
    chapters_file: &Utf8Path,
    output: &Utf8Path,
) -> FFmpegCommand {
    FFmpegCommand::with_ffmpeg_path(ffmpeg_path)
        .input(input)
        .input_with_options(chapters_file, vec!["-f".to_string(), "ffmetadata".to_string()])
        .output(output)
        .output_option("-map_metadata".to_string())
        .output_option("1".to_string())
        .output_option("-c".to_string())
        .output_option("copy".to_string())
        .overwrite_output(true)
}

/// Generate FFmpeg metadata file content with chapters
///
/// Format:
/// ```text
/// ;FFMETADATA1
/// [CHAPTER]
/// TIMEBASE=1/1000
/// START=0
/// END=60000
/// title=Chapter 1
/// ```
pub fn generate_ffmetadata_content(chapters: &[Chapter], metadata: Option<&Metadata>) -> String {
    let mut content = String::from(";FFMETADATA1\n");

    // Add global metadata
    if let Some(meta) = metadata {
        if let Some(ref title) = meta.title {
            writeln!(content, "title={}", escape_ffmetadata(title)).unwrap();
        }
        if let Some(ref artist) = meta.artist {
            writeln!(content, "artist={}", escape_ffmetadata(artist)).unwrap();
        }
        if let Some(ref album) = meta.album {
            writeln!(content, "album={}", escape_ffmetadata(album)).unwrap();
        }
        if let Some(year) = meta.year {
            writeln!(content, "date={}", year).unwrap();
        }
        if let Some(ref genre) = meta.genre {
            writeln!(content, "genre={}", escape_ffmetadata(genre)).unwrap();
        }
    }

    // Add chapters
    for chapter in chapters {
        writeln!(content, "\n[CHAPTER]").unwrap();
        writeln!(content, "TIMEBASE=1/1000").unwrap();
        writeln!(content, "START={}", chapter.start_time_ms).unwrap();
        writeln!(content, "END={}", chapter.end_time_ms).unwrap();
        writeln!(content, "title={}", escape_ffmetadata(&chapter.title)).unwrap();
    }

    content
}

/// Escape special characters for FFmpeg metadata format
fn escape_ffmetadata(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('=', "\\=")
        .replace(';', "\\;")
        .replace('#', "\\#")
        .replace('\n', "\\\n")
}

/// Build a command to embed artwork into an M4B file
pub fn build_artwork_command(
    ffmpeg_path: &Utf8Path,
    input: &Utf8Path,
    artwork: &Utf8Path,
    output: &Utf8Path,
) -> FFmpegCommand {
    FFmpegCommand::with_ffmpeg_path(ffmpeg_path)
        .input(input)
        .input(artwork)
        .output(output)
        .output_option("-map".to_string())
        .output_option("0:a".to_string())
        .output_option("-map".to_string())
        .output_option("1:v".to_string())
        .output_option("-c".to_string())
        .output_option("copy".to_string())
        .output_option("-disposition:v:0".to_string())
        .output_option("attached_pic".to_string())
        .overwrite_output(true)
}

/// Validate conversion settings
pub fn validate_settings(settings: &ConversionSettings) -> AppResult<()> {
    // Validate bitrate
    if settings.bitrate_kbps < 8 || settings.bitrate_kbps > 320 {
        return Err(AppError::InvalidConversionSettings(format!(
            "Bitrate must be between 8 and 320 kbps, got {}",
            settings.bitrate_kbps
        )));
    }

    // Validate sample rate if specified
    if let Some(sr) = settings.sample_rate_hz {
        let valid_rates = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 96000];
        if !valid_rates.contains(&sr) {
            return Err(AppError::InvalidConversionSettings(format!(
                "Invalid sample rate: {}. Valid rates: {:?}",
                sr, valid_rates
            )));
        }
    }

    // Validate output path
    if settings.output_path.as_str().is_empty() {
        return Err(AppError::InvalidConversionSettings(
            "Output path is required".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_command_build() {
        let cmd = FFmpegCommand::new()
            .input("/input/file.mp3")
            .output("/output/file.m4b")
            .audio_codec("aac")
            .bitrate(128);

        let args = cmd.build_args().unwrap();
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"-b:a".to_string()));
        assert!(args.contains(&"128k".to_string()));
    }

    #[test]
    fn test_metadata_in_command() {
        let mut metadata = Metadata::default();
        metadata.title = Some("Test Title".to_string());
        metadata.artist = Some("Test Artist".to_string());

        let cmd = FFmpegCommand::new()
            .input("/input.mp3")
            .output("/output.m4b")
            .with_metadata(&metadata);

        let args = cmd.build_args().unwrap();
        assert!(args.contains(&"-metadata".to_string()));
        assert!(args.contains(&"title=Test Title".to_string()));
        assert!(args.contains(&"artist=Test Artist".to_string()));
    }

    #[test]
    fn test_generate_concat_file() {
        let files = vec![
            Utf8PathBuf::from("/path/to/file1.mp3"),
            Utf8PathBuf::from("/path/to/file2.mp3"),
        ];

        let content = generate_concat_file_content(&files);
        assert!(content.contains("file '/path/to/file1.mp3'"));
        assert!(content.contains("file '/path/to/file2.mp3'"));
    }

    #[test]
    fn test_generate_ffmetadata() {
        let chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 60000),
            Chapter::new(1, "Chapter 2".to_string(), 60000, 120000),
        ];

        let content = generate_ffmetadata_content(&chapters, None);
        assert!(content.contains(";FFMETADATA1"));
        assert!(content.contains("[CHAPTER]"));
        assert!(content.contains("TIMEBASE=1/1000"));
        assert!(content.contains("START=0"));
        assert!(content.contains("END=60000"));
        assert!(content.contains("title=Chapter 1"));
    }

    #[test]
    fn test_escape_ffmetadata() {
        assert_eq!(escape_ffmetadata("test=value"), "test\\=value");
        assert_eq!(escape_ffmetadata("test;comment"), "test\\;comment");
        assert_eq!(escape_ffmetadata("test\\path"), "test\\\\path");
    }

    #[test]
    fn test_validate_settings() {
        let mut settings = ConversionSettings::default();
        settings.output_path = "/output/test.m4b".into();

        assert!(validate_settings(&settings).is_ok());

        settings.bitrate_kbps = 500;
        assert!(validate_settings(&settings).is_err());

        settings.bitrate_kbps = 128;
        settings.sample_rate_hz = Some(12345);
        assert!(validate_settings(&settings).is_err());
    }
}
