//! Audio timing and duration utilities
//!
//! Functions for calculating, formatting, and parsing audio durations.

use crate::error::{AppError, AppResult};
use crate::models::{format_duration_ms, parse_time_string, AudioFile, Chapter};

/// Calculate total duration of multiple audio files in milliseconds
pub fn calculate_total_duration(files: &[AudioFile]) -> u64 {
    files.iter().map(|f| f.duration_ms).sum()
}

/// Format milliseconds as HH:MM:SS.mmm (with milliseconds)
pub fn format_duration_ms_precise(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let remaining_ms = ms % 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            hours, minutes, seconds, remaining_ms
        )
    } else {
        format!("{:02}:{:02}.{:03}", minutes, seconds, remaining_ms)
    }
}

/// Create chapters by auto-splitting duration into equal parts
pub fn auto_split_chapters(total_duration_ms: u64, chapter_count: usize) -> Vec<Chapter> {
    if chapter_count == 0 {
        return Vec::new();
    }

    let chapter_duration = total_duration_ms / chapter_count as u64;

    (0..chapter_count)
        .map(|i| {
            let start = i as u64 * chapter_duration;
            let end = if i == chapter_count - 1 {
                total_duration_ms // Last chapter goes to the end
            } else {
                (i as u64 + 1) * chapter_duration
            };

            Chapter {
                index: i,
                title: format!("Chapter {}", i + 1),
                start_time_ms: start,
                end_time_ms: end,
                auto_detected: true,
                placement: None,
            }
        })
        .collect()
}

/// One chapter per file. Lives in `metadata::chapters` so there is a single
/// naming rule; it is re-exported here because the audio module used to own it.
pub use crate::metadata::chapters::create_chapters_from_files;

/// Validate chapters against total duration
pub fn validate_chapters(chapters: &[Chapter], total_duration_ms: u64) -> AppResult<()> {
    if chapters.is_empty() {
        return Ok(());
    }

    // Check for valid times
    for chapter in chapters {
        if chapter.start_time_ms >= chapter.end_time_ms {
            return Err(AppError::InvalidChapter {
                reason: format!(
                    "Chapter '{}' has invalid times: start ({}) >= end ({})",
                    chapter.title,
                    format_duration_ms(chapter.start_time_ms),
                    format_duration_ms(chapter.end_time_ms)
                ),
            });
        }

        if chapter.end_time_ms > total_duration_ms {
            return Err(AppError::ChapterExceedsDuration {
                end_ms: chapter.end_time_ms,
                total_ms: total_duration_ms,
            });
        }
    }

    // Check for overlaps
    for i in 0..chapters.len() - 1 {
        let current = &chapters[i];
        let next = &chapters[i + 1];

        if current.end_time_ms > next.start_time_ms {
            return Err(AppError::ChapterOverlap {
                index1: current.index,
                index2: next.index,
            });
        }
    }

    Ok(())
}

/// Estimate time remaining for conversion
pub fn estimate_time_remaining(elapsed_ms: u64, progress_percent: f32) -> Option<u64> {
    if progress_percent <= 0.0 || progress_percent >= 100.0 {
        return None;
    }

    let total_estimated = (elapsed_ms as f64 * 100.0 / progress_percent as f64) as u64;
    Some(total_estimated.saturating_sub(elapsed_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_ms_precise() {
        assert_eq!(format_duration_ms_precise(0), "00:00.000");
        assert_eq!(format_duration_ms_precise(1234), "00:01.234");
        assert_eq!(format_duration_ms_precise(61234), "01:01.234");
        assert_eq!(format_duration_ms_precise(3661234), "01:01:01.234");
    }

    #[test]
    fn test_auto_split_chapters() {
        let chapters = auto_split_chapters(60000, 3); // 60 seconds, 3 chapters

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 20000);
        assert_eq!(chapters[1].start_time_ms, 20000);
        assert_eq!(chapters[1].end_time_ms, 40000);
        assert_eq!(chapters[2].start_time_ms, 40000);
        assert_eq!(chapters[2].end_time_ms, 60000); // Last chapter goes to end
    }

    #[test]
    fn test_validate_chapters() {
        let valid_chapters = vec![
            Chapter::new(0, "Ch1".to_string(), 0, 10000),
            Chapter::new(1, "Ch2".to_string(), 10000, 20000),
        ];
        assert!(validate_chapters(&valid_chapters, 20000).is_ok());

        // Overlapping chapters
        let overlapping = vec![
            Chapter::new(0, "Ch1".to_string(), 0, 15000),
            Chapter::new(1, "Ch2".to_string(), 10000, 20000),
        ];
        assert!(validate_chapters(&overlapping, 20000).is_err());

        // Chapter exceeds duration
        let exceeds = vec![Chapter::new(0, "Ch1".to_string(), 0, 25000)];
        assert!(validate_chapters(&exceeds, 20000).is_err());
    }

    #[test]
    fn test_estimate_time_remaining() {
        // 50% done after 30 seconds = ~30 seconds remaining
        let remaining = estimate_time_remaining(30000, 50.0);
        assert_eq!(remaining, Some(30000));

        // Edge cases
        assert_eq!(estimate_time_remaining(1000, 0.0), None);
        assert_eq!(estimate_time_remaining(1000, 100.0), None);
    }
}
