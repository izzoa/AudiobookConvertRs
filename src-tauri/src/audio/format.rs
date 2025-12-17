//! Audio format detection and validation
//!
//! Provides utilities for detecting and validating audio file formats
//! using Symphonia's probing capabilities.

use crate::error::{AppError, AppResult};
use crate::models::AudioFormat;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs::File;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::debug;

/// Supported input audio formats
pub const SUPPORTED_INPUT_FORMATS: &[AudioFormat] = &[
    AudioFormat::Mp3,
    AudioFormat::Flac,
    AudioFormat::M4a,
    AudioFormat::M4b,
    AudioFormat::Wav,
    AudioFormat::Ogg,
    AudioFormat::Aac,
];

/// Supported input file extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "m4b", "wav", "ogg", "oga", "aac",
];

/// Check if a file format is supported for input
pub fn is_supported_format(path: &Utf8Path) -> bool {
    path.extension()
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Validate an audio file
///
/// Checks:
/// - File exists
/// - File is readable
/// - File has valid audio format
/// - File contains audio stream
pub fn validate_audio_file(path: &Utf8Path) -> AppResult<()> {
    debug!("Validating audio file: {}", path);

    // Check file exists
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    // Check file is readable
    let file = File::open(path.as_std_path())
        .map_err(|e| AppError::PermissionDenied(path.as_std_path().to_path_buf()))?;

    // Check file is not empty
    let metadata = file.metadata()
        .map_err(|e| AppError::file_read(path, e.to_string()))?;

    if metadata.len() == 0 {
        return Err(AppError::CorruptedAudioFile {
            path: path.as_std_path().to_path_buf(),
            reason: "File is empty".to_string(),
        });
    }

    // Probe to verify it's a valid audio file
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|_| AppError::CorruptedAudioFile {
            path: path.as_std_path().to_path_buf(),
            reason: "Could not parse audio format".to_string(),
        })?;

    // Verify there's at least one audio track
    let has_audio = probed
        .format
        .tracks()
        .iter()
        .any(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL);

    if !has_audio {
        return Err(AppError::NoAudioStream(path.as_std_path().to_path_buf()));
    }

    debug!("Audio file validated successfully: {}", path);
    Ok(())
}

/// Detect audio format using Symphonia probing
pub fn detect_format(path: &Utf8Path) -> AppResult<AudioFormat> {
    // First try extension-based detection (fast path)
    if let Some(ext) = path.extension() {
        if let Some(format) = AudioFormat::from_extension(ext) {
            // Verify with Symphonia
            if validate_audio_file(path).is_ok() {
                return Ok(format);
            }
        }
    }

    // Fallback to probing
    let file = File::open(path.as_std_path())
        .map_err(|e| AppError::file_read(path, e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), mss, &format_opts, &metadata_opts)
        .map_err(|_| AppError::FormatDetectionFailed(path.as_std_path().to_path_buf()))?;

    // Try to determine format from codec
    let track = probed
        .format
        .tracks()
        .first()
        .ok_or_else(|| AppError::NoAudioStream(path.as_std_path().to_path_buf()))?;

    // Map codec to format
    let codec = track.codec_params.codec;
    let format = match codec.to_string().as_str() {
        s if s.contains("mp3") || s.contains("mpeg") => AudioFormat::Mp3,
        s if s.contains("flac") => AudioFormat::Flac,
        s if s.contains("aac") => AudioFormat::Aac,
        s if s.contains("vorbis") => AudioFormat::Ogg,
        s if s.contains("pcm") || s.contains("wav") => AudioFormat::Wav,
        _ => {
            // Try extension as last resort
            path.extension()
                .and_then(AudioFormat::from_extension)
                .ok_or_else(|| AppError::FormatDetectionFailed(path.as_std_path().to_path_buf()))?
        }
    };

    Ok(format)
}

/// Get file type filter string for file dialogs
pub fn get_audio_file_filter() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        ("Audio Files", &["mp3", "flac", "m4a", "m4b", "wav", "ogg", "aac"]),
        ("MP3 Files", &["mp3"]),
        ("FLAC Files", &["flac"]),
        ("M4A/M4B Files", &["m4a", "m4b"]),
        ("WAV Files", &["wav"]),
        ("OGG Files", &["ogg", "oga"]),
        ("All Files", &["*"]),
    ]
}

/// Validate multiple files and return results
pub fn validate_files(paths: &[&Utf8Path]) -> Vec<(Utf8PathBuf, AppResult<()>)> {
    paths
        .iter()
        .map(|p| ((*p).to_owned(), validate_audio_file(p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format() {
        assert!(is_supported_format(Utf8Path::new("/test/file.mp3")));
        assert!(is_supported_format(Utf8Path::new("/test/file.FLAC")));
        assert!(is_supported_format(Utf8Path::new("/test/file.m4b")));
        assert!(!is_supported_format(Utf8Path::new("/test/file.txt")));
        assert!(!is_supported_format(Utf8Path::new("/test/file")));
    }

    #[test]
    fn test_supported_formats_list() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp3"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"flac"));
        assert!(!SUPPORTED_EXTENSIONS.contains(&"txt"));
    }
}

