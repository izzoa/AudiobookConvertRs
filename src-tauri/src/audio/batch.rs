//! Batch file processing module
//!
//! Provides functions to load multiple audio files and create audiobooks
//! with proper validation, metadata merging, and chapter creation.

use crate::audio::loader::load_audio_file;
use crate::audio::timing::create_chapters_from_files;
use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, AudioFile, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use std::cmp::Ordering;
use tracing::{debug, info, warn};

/// Result of loading multiple files
pub struct BatchLoadResult {
    /// Successfully loaded files
    pub files: Vec<AudioFile>,
    /// Errors encountered during loading
    pub errors: Vec<(Utf8PathBuf, AppError)>,
}

impl BatchLoadResult {
    /// Check if all files loaded successfully
    pub fn all_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get count of successful loads
    pub fn success_count(&self) -> usize {
        self.files.len()
    }

    /// Get count of failed loads
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// Load multiple audio files in parallel
///
/// Returns both successful loads and errors, allowing partial success.
pub fn load_files_batch(paths: &[Utf8PathBuf]) -> BatchLoadResult {
    info!("Loading {} audio files in batch", paths.len());

    let results: Vec<(Utf8PathBuf, Result<AudioFile, AppError>)> = paths
        .par_iter()
        .map(|path| {
            let result = load_audio_file(path);
            (path.clone(), result)
        })
        .collect();

    let mut files = Vec::new();
    let mut errors = Vec::new();

    for (path, result) in results {
        match result {
            Ok(file) => files.push(file),
            Err(e) => {
                warn!("Failed to load {}: {}", path, e);
                errors.push((path, e));
            }
        }
    }

    // Sort files by path for consistent ordering
    files.sort_by(|a, b| natural_sort(&a.path, &b.path));

    info!(
        "Batch load complete: {} successful, {} failed",
        files.len(),
        errors.len()
    );

    BatchLoadResult { files, errors }
}

/// Create an audiobook from a list of file paths
///
/// This function:
/// 1. Loads all audio files using Symphonia
/// 2. Validates all files are readable
/// 3. Calculates total duration
/// 4. Merges metadata from files
/// 5. Creates initial chapters from file boundaries
pub fn load_audiobook_from_files(paths: &[Utf8PathBuf]) -> AppResult<AudioBook> {
    if paths.is_empty() {
        return Err(AppError::NoFilesProvided);
    }

    info!("Creating audiobook from {} files", paths.len());

    // Load all files
    let batch_result = load_files_batch(paths);

    if batch_result.files.is_empty() {
        // All files failed
        if let Some((_, first_error)) = batch_result.errors.into_iter().next() {
            return Err(first_error);
        }
        return Err(AppError::NoFilesProvided);
    }

    // Log any partial failures
    for (path, error) in &batch_result.errors {
        warn!("Skipping file {}: {}", path, error);
    }

    let files = batch_result.files;

    // Merge metadata from all files (use first file's metadata as base)
    let merged_metadata = merge_metadata_from_files(&files);

    // Create chapters from file boundaries
    let chapters = create_chapters_from_files(&files);

    // Calculate totals
    let total_duration_ms: u64 = files.iter().map(|f| f.duration_ms).sum();
    let total_size_bytes: u64 = files.iter().map(|f| f.file_size_bytes).sum();

    // Determine title from metadata or first file
    let title = merged_metadata
        .title
        .clone()
        .or_else(|| merged_metadata.album.clone())
        .unwrap_or_else(|| {
            files
                .first()
                .map(|f| f.stem().to_string())
                .unwrap_or_else(|| "Untitled Audiobook".to_string())
        });

    // Create audiobook
    let mut audiobook = AudioBook::new(title);
    audiobook.author = merged_metadata.artist.clone();
    audiobook.files = files;
    audiobook.chapters = chapters;
    audiobook.output_metadata = merged_metadata;
    audiobook.total_duration_ms = total_duration_ms;
    audiobook.total_size_bytes = total_size_bytes;

    info!(
        "Created audiobook: {} ({} files, {})",
        audiobook.title,
        audiobook.files.len(),
        audiobook.formatted_duration()
    );

    Ok(audiobook)
}

/// Merge metadata from multiple files
///
/// Strategy:
/// - Use non-empty values from the first file as the base
/// - For album/series info, prefer consistency across files
fn merge_metadata_from_files(files: &[AudioFile]) -> Metadata {
    if files.is_empty() {
        return Metadata::default();
    }

    let mut merged = files[0].metadata.clone();

    // Try to fill in missing fields from other files
    for file in files.iter().skip(1) {
        if merged.title.is_none() {
            merged.title = file.metadata.title.clone();
        }
        if merged.artist.is_none() {
            merged.artist = file.metadata.artist.clone();
        }
        if merged.album.is_none() {
            merged.album = file.metadata.album.clone();
        }
        if merged.album_artist.is_none() {
            merged.album_artist = file.metadata.album_artist.clone();
        }
        if merged.year.is_none() {
            merged.year = file.metadata.year;
        }
        if merged.genre.is_none() {
            merged.genre = file.metadata.genre.clone();
        }
        if merged.narrator.is_none() {
            merged.narrator = file.metadata.narrator.clone();
        }
        if merged.publisher.is_none() {
            merged.publisher = file.metadata.publisher.clone();
        }
        if merged.artwork_data.is_none() && file.metadata.artwork_data.is_some() {
            merged.artwork_data = file.metadata.artwork_data.clone();
        }
        if merged.artwork_path.is_none() && file.metadata.artwork_path.is_some() {
            merged.artwork_path = file.metadata.artwork_path.clone();
        }
    }

    merged
}

/// Natural sort comparison for file paths
///
/// Handles numeric sequences properly (e.g., "Chapter 2" before "Chapter 10")
fn natural_sort(a: &Utf8Path, b: &Utf8Path) -> Ordering {
    let a_name = a.file_name().unwrap_or(a.as_str());
    let b_name = b.file_name().unwrap_or(b.as_str());

    natural_cmp(a_name, b_name)
}

/// Natural string comparison
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Compare numeric sequences
                    let a_num = extract_number(&mut a_chars);
                    let b_num = extract_number(&mut b_chars);
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Compare characters (case-insensitive)
                    let ac_lower = ac.to_ascii_lowercase();
                    let bc_lower = bc.to_ascii_lowercase();
                    match ac_lower.cmp(&bc_lower) {
                        Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                            continue;
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Extract a number from an iterator of characters
fn extract_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> u64 {
    let mut num = 0u64;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num = num * 10 + c.to_digit(10).unwrap() as u64;
            chars.next();
        } else {
            break;
        }
    }
    num
}

/// Validate that all files can be combined
///
/// Checks:
/// - All files exist
/// - All files are valid audio
/// - Sufficient disk space for output (optional)
pub fn validate_files_for_conversion(
    files: &[AudioFile],
    output_path: &Utf8Path,
) -> AppResult<()> {
    if files.is_empty() {
        return Err(AppError::NoFilesProvided);
    }

    // Check all source files still exist
    for file in files {
        if !file.path.exists() {
            return Err(AppError::FileNotFound(file.path.as_std_path().to_path_buf()));
        }
    }

    // Check output directory exists and is writable
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            return Err(AppError::file_write(
                output_path,
                "Output directory does not exist",
            ));
        }
    }

    // Check output doesn't already exist (unless overwrite is enabled)
    if output_path.exists() {
        return Err(AppError::OutputFileExists(
            output_path.as_std_path().to_path_buf(),
        ));
    }

    debug!("All {} files validated for conversion", files.len());
    Ok(())
}

/// Estimate output file size based on input files and settings
pub fn estimate_output_size(files: &[AudioFile], target_bitrate_kbps: u32) -> u64 {
    let total_duration_ms: u64 = files.iter().map(|f| f.duration_ms).sum();
    let duration_sec = total_duration_ms as f64 / 1000.0;

    // Output size ≈ (bitrate_kbps * duration_sec) / 8 * 1024 (convert to bytes)
    // Add ~5% overhead for container and metadata
    let base_size = (target_bitrate_kbps as f64 * duration_sec / 8.0 * 1024.0) as u64;
    (base_size as f64 * 1.05) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_cmp() {
        assert_eq!(natural_cmp("file1", "file2"), Ordering::Less);
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("file10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("abc", "abc"), Ordering::Equal);
        assert_eq!(natural_cmp("Chapter 1", "Chapter 2"), Ordering::Less);
        assert_eq!(natural_cmp("Chapter 2", "Chapter 10"), Ordering::Less);
    }

    #[test]
    fn test_extract_number() {
        let mut chars = "123abc".chars().peekable();
        assert_eq!(extract_number(&mut chars), 123);
        assert_eq!(chars.next(), Some('a'));
    }

    #[test]
    fn test_estimate_output_size() {
        use crate::models::{AudioFormat, Metadata};

        let files = vec![AudioFile {
            path: "/test/file.mp3".into(),
            format: AudioFormat::Mp3,
            duration_ms: 3600000, // 1 hour
            bitrate_kbps: Some(128),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            file_size_bytes: 57_600_000,
            metadata: Metadata::default(),
            has_artwork: false,
        }];

        // 128 kbps * 3600 sec / 8 = 57,600,000 bytes
        let estimated = estimate_output_size(&files, 128);
        assert!(estimated > 55_000_000 && estimated < 65_000_000);
    }

    #[test]
    fn test_batch_load_result() {
        let result = BatchLoadResult {
            files: vec![],
            errors: vec![],
        };
        assert!(result.all_successful());
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.error_count(), 0);
    }
}
