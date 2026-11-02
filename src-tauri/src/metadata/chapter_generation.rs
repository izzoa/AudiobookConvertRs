//! Smart chapter generation module
//!
//! Provides intelligent chapter generation strategies:
//! - Extract from source file metadata (M4B/MP3/FLAC cuesheet)
//! - Regex-based title cleanup

use crate::error::{AppError, AppResult};
use crate::metadata::chapters::fix_last_chapter_end_time;
use crate::metadata::format_specific::auto_extract_chapters;
use crate::models::{AudioFile, Chapter};
use regex::Regex;
use tracing::{debug, info, warn};

/// Extract chapters from source files metadata
///
/// Attempts to extract chapters from:
/// - M4B files with embedded chapters
/// - MP3 files with ID3 CHAP frames
/// - FLAC files with cue sheets
///
/// Returns chapters if found, otherwise returns empty vec
pub fn extract_chapters_from_sources(files: &[AudioFile]) -> AppResult<Vec<Chapter>> {
    debug!(
        "Attempting to extract chapters from {} source files",
        files.len()
    );

    let mut all_chapters = Vec::new();
    let mut current_offset_ms: u64 = 0;

    for (file_idx, file) in files.iter().enumerate() {
        // Try to extract chapters from this file
        match auto_extract_chapters(file) {
            Ok(mut chapters) => {
                if !chapters.is_empty() {
                    info!("Found {} chapters in {}", chapters.len(), file.filename());

                    // Adjust chapter times to account for previous files
                    for chapter in &mut chapters {
                        chapter.start_time_ms += current_offset_ms;
                        chapter.end_time_ms += current_offset_ms;
                    }

                    // Re-index chapters
                    for (idx, chapter) in chapters.iter_mut().enumerate() {
                        chapter.index = all_chapters.len() + idx;
                    }

                    all_chapters.extend(chapters);
                }
            }
            Err(e) => {
                debug!("No chapters extracted from {}: {}", file.filename(), e);
            }
        }

        current_offset_ms += file.duration_ms;
    }

    // Fix last chapter end time if needed
    if !all_chapters.is_empty() {
        let total_duration: u64 = files.iter().map(|f| f.duration_ms).sum();
        fix_last_chapter_end_time(&mut all_chapters, total_duration);
    }

    if all_chapters.is_empty() {
        info!("No chapters found in source files");
    } else {
        info!(
            "Extracted total of {} chapters from sources",
            all_chapters.len()
        );
    }

    Ok(all_chapters)
}

/// Rename chapters using regex pattern replacement
///
/// Applies the given regex pattern and replacement to all chapter titles.
/// This is useful for cleaning up automatically generated titles.
///
/// # Examples
/// - Pattern: `r"^\d+\s*-\s*"`, Replacement: "" -> Removes "01 - " prefix
/// - Pattern: `r"^Track\s+\d+\s*-\s*"`, Replacement: "" -> Removes "Track 03 - " prefix
/// - Pattern: `r"_"`, Replacement: " " -> Replaces underscores with spaces
pub fn rename_chapters_regex(
    chapters: &mut [Chapter],
    pattern: &str,
    replacement: &str,
) -> AppResult<()> {
    debug!(
        "Applying regex rename: pattern='{}', replacement='{}'",
        pattern, replacement
    );

    let re = Regex::new(pattern)
        .map_err(|e| AppError::ValidationError(format!("Invalid regex pattern: {}", e)))?;

    let mut renamed_count = 0;
    for chapter in chapters.iter_mut() {
        let new_title = re.replace_all(&chapter.title, replacement).to_string();
        if new_title != chapter.title {
            debug!("Renamed '{}' -> '{}'", chapter.title, new_title);
            chapter.title = new_title;
            renamed_count += 1;
        }
    }

    info!("Renamed {} chapter titles using regex", renamed_count);
    Ok(())
}

/// Common regex presets for chapter title cleanup
pub struct RegexPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub pattern: &'static str,
    pub replacement: &'static str,
}

/// Get list of common regex presets for chapter cleanup
pub fn get_regex_presets() -> Vec<RegexPreset> {
    vec![
        RegexPreset {
            name: "Remove numbered prefix",
            description: "Removes prefixes like '01 - ', '001. ', etc.",
            pattern: r"^\d+\s*[-.\)]\s*",
            replacement: "",
        },
        RegexPreset {
            name: "Remove 'Track' prefix",
            description: "Removes 'Track 01', 'Track 03', etc.",
            pattern: r"^Track\s+\d+\s*[-:.]?\s*",
            replacement: "",
        },
        RegexPreset {
            name: "Replace underscores",
            description: "Replaces underscores with spaces",
            pattern: r"_",
            replacement: " ",
        },
        RegexPreset {
            name: "Remove file extensions",
            description: "Removes common audio file extensions from titles",
            pattern: r"\.(mp3|flac|m4a|m4b|wav|ogg)$",
            replacement: "",
        },
        RegexPreset {
            name: "Clean multiple spaces",
            description: "Replaces multiple spaces with single space",
            pattern: r"\s{2,}",
            replacement: " ",
        },
        RegexPreset {
            name: "Remove 'Chapter' prefix",
            description: "Removes 'Chapter 1:', 'Chapter One -', etc.",
            pattern: r"^Chapter\s+\w+\s*[-:.]?\s*",
            replacement: "",
        },
    ]
}

/// Apply a preset to chapter titles
pub fn apply_preset(chapters: &mut [Chapter], preset_name: &str) -> AppResult<()> {
    let presets = get_regex_presets();
    let preset = presets
        .iter()
        .find(|p| p.name == preset_name)
        .ok_or_else(|| AppError::ValidationError(format!("Unknown preset: {}", preset_name)))?;

    rename_chapters_regex(chapters, preset.pattern, preset.replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chapters() -> Vec<Chapter> {
        vec![
            Chapter::new(0, "01 - Introduction".to_string(), 0, 60000),
            Chapter::new(1, "02 - Chapter One".to_string(), 60000, 120000),
            Chapter::new(2, "Track 03 - Chapter Two".to_string(), 120000, 180000),
        ]
    }

    #[test]
    fn test_rename_chapters_remove_prefix() {
        let mut chapters = create_test_chapters();

        rename_chapters_regex(&mut chapters, r"^\d+\s*-\s*", "").unwrap();

        assert_eq!(chapters[0].title, "Introduction");
        assert_eq!(chapters[1].title, "Chapter One");
        assert_eq!(chapters[2].title, "Track 03 - Chapter Two"); // Not affected
    }

    #[test]
    fn test_rename_chapters_remove_track() {
        let mut chapters = create_test_chapters();

        rename_chapters_regex(&mut chapters, r"^Track\s+\d+\s*-\s*", "").unwrap();

        assert_eq!(chapters[2].title, "Chapter Two");
        assert_eq!(chapters[0].title, "01 - Introduction"); // Not affected
    }

    #[test]
    fn test_rename_chapters_replace_underscores() {
        let mut chapters = vec![Chapter::new(0, "Chapter_One_Part_A".to_string(), 0, 60000)];

        rename_chapters_regex(&mut chapters, r"_", " ").unwrap();

        assert_eq!(chapters[0].title, "Chapter One Part A");
    }

    #[test]
    fn test_apply_preset() {
        let mut chapters = create_test_chapters();

        apply_preset(&mut chapters, "Remove numbered prefix").unwrap();

        assert_eq!(chapters[0].title, "Introduction");
        assert_eq!(chapters[1].title, "Chapter One");
    }

    #[test]
    fn test_invalid_regex() {
        let mut chapters = create_test_chapters();

        let result = rename_chapters_regex(&mut chapters, r"[invalid(", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_regex_presets() {
        let presets = get_regex_presets();
        assert!(presets.len() >= 5);
        assert!(presets.iter().any(|p| p.name == "Remove numbered prefix"));
    }
}
