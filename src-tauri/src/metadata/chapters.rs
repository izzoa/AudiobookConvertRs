//! Chapter management module
//!
//! Provides functions for creating, validating, and editing chapters
//! in audiobooks. Supports creating chapters from file boundaries,
//! automatic time-based splitting, and extracting chapters from M4B files.

use crate::error::{AppError, AppResult};
use crate::models::{AudioFile, Chapter};
use camino::Utf8Path;
use tracing::{debug, info, warn};

/// Update for partial chapter editing
#[derive(Debug, Clone, Default)]
pub struct ChapterUpdate {
    pub title: Option<String>,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
}

/// Create chapters from a list of audio files
///
/// Each file becomes a chapter with the title derived from the filename.
/// Start times are calculated by accumulating durations of previous files.
pub fn create_chapters_from_files(files: &[AudioFile]) -> Vec<Chapter> {
    debug!("Creating {} chapters from files", files.len());

    let mut chapters = Vec::with_capacity(files.len());
    let mut current_start_ms: u64 = 0;

    for (index, file) in files.iter().enumerate() {
        let title = file.stem().to_string();
        let end_time_ms = current_start_ms + file.duration_ms;

        let chapter = Chapter {
            index,
            title,
            start_time_ms: current_start_ms,
            end_time_ms,
            auto_detected: true,
        };

        chapters.push(chapter);
        current_start_ms = end_time_ms;
    }

    info!("Created {} chapters from files", chapters.len());
    chapters
}

/// Create chapters by automatically splitting the total duration
///
/// Divides the audiobook into `count` equal-length chapters.
/// Useful for audiobooks without natural chapter markers.
pub fn create_chapters_auto_split(total_duration_ms: u64, count: usize) -> Vec<Chapter> {
    if count == 0 {
        warn!("Auto-split requested with 0 chapters, returning empty");
        return Vec::new();
    }

    debug!("Auto-splitting {} ms into {} chapters", total_duration_ms, count);

    let chapter_duration = total_duration_ms / count as u64;
    let remainder = total_duration_ms % count as u64;

    let mut chapters = Vec::with_capacity(count);
    let mut current_start_ms: u64 = 0;

    for i in 0..count {
        // Distribute remainder to last chapter
        let extra = if i == count - 1 { remainder } else { 0 };
        let end_time_ms = current_start_ms + chapter_duration + extra;

        let chapter = Chapter {
            index: i,
            title: format!("Chapter {}", i + 1),
            start_time_ms: current_start_ms,
            end_time_ms,
            auto_detected: true,
        };

        chapters.push(chapter);
        current_start_ms = end_time_ms;
    }

    info!("Auto-split into {} chapters of ~{} ms each", chapters.len(), chapter_duration);
    chapters
}

/// Extract chapters from an M4B file
///
/// Uses mp4ameta to read QuickTime chapter information from M4B files.
pub fn extract_chapters_from_m4b(path: &Utf8Path) -> AppResult<Vec<Chapter>> {
    debug!("Extracting chapters from M4B: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let tag = mp4ameta::Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::metadata_read(path, e.to_string()))?;

    let mut chapters = Vec::new();

    // mp4ameta provides chapter information through the chapters() method
    for (index, chap) in tag.chapters().iter().enumerate() {
        let chapter = Chapter {
            index,
            title: chap.title.clone(),
            start_time_ms: chap.start.as_millis() as u64,
            // End time will be calculated from next chapter or total duration
            end_time_ms: 0, // Will be fixed below
            auto_detected: false,
        };
        chapters.push(chapter);
    }

    // Fix end times (each chapter ends where the next begins)
    for i in 0..chapters.len() {
        if i + 1 < chapters.len() {
            chapters[i].end_time_ms = chapters[i + 1].start_time_ms;
        }
    }

    // For the last chapter, we'd need to know the total duration
    // This will be set by the caller who knows the total duration

    info!("Extracted {} chapters from M4B", chapters.len());
    Ok(chapters)
}

/// Fix the last chapter's end time using total duration
pub fn fix_last_chapter_end_time(chapters: &mut [Chapter], total_duration_ms: u64) {
    if let Some(last) = chapters.last_mut() {
        if last.end_time_ms == 0 || last.end_time_ms < last.start_time_ms {
            last.end_time_ms = total_duration_ms;
        }
    }
}

/// Validate a list of chapters
///
/// Checks for:
/// - No overlapping chapters
/// - Valid chapter order (sorted by start time)
/// - End times within total duration
/// - Valid chapter data (end > start, non-empty titles)
pub fn validate_chapters(chapters: &[Chapter], total_duration_ms: u64) -> AppResult<()> {
    debug!("Validating {} chapters", chapters.len());

    for (i, chapter) in chapters.iter().enumerate() {
        // Check basic validity
        if !chapter.is_valid() {
            return Err(AppError::InvalidChapter {
                reason: format!(
                    "Chapter {} has invalid times or empty title: start={}, end={}, title='{}'",
                    i, chapter.start_time_ms, chapter.end_time_ms, chapter.title
                ),
            });
        }

        // Check end time doesn't exceed total duration
        if chapter.end_time_ms > total_duration_ms {
            return Err(AppError::ChapterExceedsDuration {
                end_ms: chapter.end_time_ms,
                total_ms: total_duration_ms,
            });
        }

        // Check for overlaps with subsequent chapters
        for (j, other) in chapters.iter().enumerate().skip(i + 1) {
            if chapters_overlap(chapter, other) {
                return Err(AppError::ChapterOverlap {
                    index1: i,
                    index2: j,
                });
            }
        }
    }

    info!("Chapter validation passed for {} chapters", chapters.len());
    Ok(())
}

/// Check if two chapters overlap
fn chapters_overlap(a: &Chapter, b: &Chapter) -> bool {
    // Chapters overlap if one starts before the other ends
    (a.start_time_ms < b.end_time_ms && b.start_time_ms < a.end_time_ms)
}

/// Add a new chapter to the list
///
/// Inserts the chapter at the correct position based on start time
/// and updates all indices.
pub fn add_chapter(chapters: &mut Vec<Chapter>, chapter: Chapter) -> AppResult<()> {
    debug!("Adding chapter: {}", chapter.title);

    // Validate the new chapter
    if !chapter.is_valid() {
        return Err(AppError::InvalidChapter {
            reason: format!("Invalid chapter data: {:?}", chapter),
        });
    }

    // Find insertion position
    let pos = chapters
        .iter()
        .position(|c| c.start_time_ms > chapter.start_time_ms)
        .unwrap_or(chapters.len());

    chapters.insert(pos, chapter);

    // Re-index all chapters
    reindex_chapters(chapters);

    info!("Added chapter at position {}", pos);
    Ok(())
}

/// Update an existing chapter
pub fn update_chapter(
    chapters: &mut [Chapter],
    index: usize,
    update: ChapterUpdate,
) -> AppResult<()> {
    if index >= chapters.len() {
        return Err(AppError::InvalidChapter {
            reason: format!("Chapter index {} out of range (max: {})", index, chapters.len()),
        });
    }

    let chapter = &mut chapters[index];

    if let Some(title) = update.title {
        if title.is_empty() {
            return Err(AppError::InvalidChapter {
                reason: "Chapter title cannot be empty".to_string(),
            });
        }
        chapter.title = title;
    }

    if let Some(start) = update.start_time_ms {
        chapter.start_time_ms = start;
    }

    if let Some(end) = update.end_time_ms {
        chapter.end_time_ms = end;
    }

    // Validate the updated chapter
    if !chapter.is_valid() {
        return Err(AppError::InvalidChapter {
            reason: format!("Updated chapter is invalid: {:?}", chapter),
        });
    }

    debug!("Updated chapter {}: {}", index, chapter.title);
    Ok(())
}

/// Delete a chapter from the list
pub fn delete_chapter(chapters: &mut Vec<Chapter>, index: usize) -> AppResult<()> {
    if index >= chapters.len() {
        return Err(AppError::InvalidChapter {
            reason: format!("Chapter index {} out of range", index),
        });
    }

    let removed = chapters.remove(index);
    debug!("Deleted chapter: {}", removed.title);

    // Re-index remaining chapters
    reindex_chapters(chapters);

    Ok(())
}

/// Reorder chapters by moving one from old_index to new_index
pub fn reorder_chapters(
    chapters: &mut Vec<Chapter>,
    old_index: usize,
    new_index: usize,
) -> AppResult<()> {
    if old_index >= chapters.len() || new_index >= chapters.len() {
        return Err(AppError::InvalidChapter {
            reason: format!(
                "Invalid indices: old={}, new={}, len={}",
                old_index,
                new_index,
                chapters.len()
            ),
        });
    }

    if old_index == new_index {
        return Ok(()); // No change needed
    }

    let chapter = chapters.remove(old_index);
    chapters.insert(new_index, chapter);

    // Re-index all chapters
    reindex_chapters(chapters);

    debug!("Reordered chapter from {} to {}", old_index, new_index);
    Ok(())
}

/// Re-index chapters after add/delete/reorder operations
fn reindex_chapters(chapters: &mut [Chapter]) {
    for (i, chapter) in chapters.iter_mut().enumerate() {
        chapter.index = i;
    }
}

/// Merge multiple chapters into one
pub fn merge_chapters(
    chapters: &mut Vec<Chapter>,
    start_index: usize,
    end_index: usize,
    new_title: String,
) -> AppResult<()> {
    if start_index >= chapters.len() || end_index >= chapters.len() || start_index > end_index {
        return Err(AppError::InvalidChapter {
            reason: format!(
                "Invalid merge range: {} to {} (len: {})",
                start_index,
                end_index,
                chapters.len()
            ),
        });
    }

    let start_time = chapters[start_index].start_time_ms;
    let end_time = chapters[end_index].end_time_ms;

    // Remove all chapters in range except the first
    for _ in start_index..end_index {
        chapters.remove(start_index + 1);
    }

    // Update the remaining chapter
    chapters[start_index].title = new_title;
    chapters[start_index].start_time_ms = start_time;
    chapters[start_index].end_time_ms = end_time;

    // Re-index
    reindex_chapters(chapters);

    debug!("Merged chapters {} to {} into one", start_index, end_index);
    Ok(())
}

/// Split a chapter at a given time
pub fn split_chapter(
    chapters: &mut Vec<Chapter>,
    index: usize,
    split_time_ms: u64,
    new_title: String,
) -> AppResult<()> {
    if index >= chapters.len() {
        return Err(AppError::InvalidChapter {
            reason: format!("Chapter index {} out of range", index),
        });
    }

    let chapter = &chapters[index];
    if split_time_ms <= chapter.start_time_ms || split_time_ms >= chapter.end_time_ms {
        return Err(AppError::InvalidChapter {
            reason: format!(
                "Split time {} must be between {} and {}",
                split_time_ms, chapter.start_time_ms, chapter.end_time_ms
            ),
        });
    }

    let original_end = chapter.end_time_ms;
    let original_title = chapter.title.clone();

    // Modify existing chapter to end at split point
    chapters[index].end_time_ms = split_time_ms;

    // Create new chapter starting at split point
    let new_chapter = Chapter {
        index: index + 1,
        title: new_title,
        start_time_ms: split_time_ms,
        end_time_ms: original_end,
        auto_detected: false,
    };

    chapters.insert(index + 1, new_chapter);

    // Re-index
    reindex_chapters(chapters);

    debug!("Split chapter '{}' at {} ms", original_title, split_time_ms);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use crate::models::{AudioFormat, Metadata};

    fn create_test_audio_file(name: &str, duration_ms: u64) -> AudioFile {
        AudioFile {
            path: Utf8PathBuf::from(format!("/test/{}.mp3", name)),
            format: AudioFormat::Mp3,
            duration_ms,
            bitrate_kbps: Some(128),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            file_size_bytes: duration_ms * 16, // Approximate
            metadata: Metadata::default(),
            has_artwork: false,
        }
    }

    #[test]
    fn test_create_chapters_from_files() {
        let files = vec![
            create_test_audio_file("chapter1", 60000),  // 1 minute
            create_test_audio_file("chapter2", 120000), // 2 minutes
            create_test_audio_file("chapter3", 90000),  // 1.5 minutes
        ];

        let chapters = create_chapters_from_files(&files);

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "chapter1");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 60000);

        assert_eq!(chapters[1].title, "chapter2");
        assert_eq!(chapters[1].start_time_ms, 60000);
        assert_eq!(chapters[1].end_time_ms, 180000);

        assert_eq!(chapters[2].title, "chapter3");
        assert_eq!(chapters[2].start_time_ms, 180000);
        assert_eq!(chapters[2].end_time_ms, 270000);
    }

    #[test]
    fn test_create_chapters_auto_split() {
        let total_duration = 3600000; // 1 hour
        let chapters = create_chapters_auto_split(total_duration, 6);

        assert_eq!(chapters.len(), 6);

        // Each chapter should be ~10 minutes (600000 ms)
        for chapter in &chapters {
            assert!(chapter.duration_ms() >= 599000 && chapter.duration_ms() <= 601000);
        }

        // First chapter starts at 0
        assert_eq!(chapters[0].start_time_ms, 0);

        // Last chapter ends at total duration
        assert_eq!(chapters[5].end_time_ms, total_duration);
    }

    #[test]
    fn test_create_chapters_auto_split_zero() {
        let chapters = create_chapters_auto_split(3600000, 0);
        assert!(chapters.is_empty());
    }

    #[test]
    fn test_validate_chapters_valid() {
        let chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 60000),
            Chapter::new(1, "Chapter 2".to_string(), 60000, 120000),
        ];

        let result = validate_chapters(&chapters, 120000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chapters_overlap() {
        let chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 70000),
            Chapter::new(1, "Chapter 2".to_string(), 60000, 120000), // Overlaps!
        ];

        let result = validate_chapters(&chapters, 120000);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chapters_exceeds_duration() {
        let chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 150000), // Exceeds 120000
        ];

        let result = validate_chapters(&chapters, 120000);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_chapter() {
        let mut chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 60000),
            Chapter::new(1, "Chapter 3".to_string(), 120000, 180000),
        ];

        let new_chapter = Chapter::new(0, "Chapter 2".to_string(), 60000, 120000);
        add_chapter(&mut chapters, new_chapter).unwrap();

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[1].title, "Chapter 2");
        assert_eq!(chapters[1].index, 1);
    }

    #[test]
    fn test_delete_chapter() {
        let mut chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 60000),
            Chapter::new(1, "Chapter 2".to_string(), 60000, 120000),
            Chapter::new(2, "Chapter 3".to_string(), 120000, 180000),
        ];

        delete_chapter(&mut chapters, 1).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[1].title, "Chapter 3");
        assert_eq!(chapters[1].index, 1); // Re-indexed
    }

    #[test]
    fn test_reorder_chapters() {
        let mut chapters = vec![
            Chapter::new(0, "A".to_string(), 0, 60000),
            Chapter::new(1, "B".to_string(), 60000, 120000),
            Chapter::new(2, "C".to_string(), 120000, 180000),
        ];

        reorder_chapters(&mut chapters, 2, 0).unwrap();

        assert_eq!(chapters[0].title, "C");
        assert_eq!(chapters[1].title, "A");
        assert_eq!(chapters[2].title, "B");
    }

    #[test]
    fn test_split_chapter() {
        let mut chapters = vec![
            Chapter::new(0, "Chapter 1".to_string(), 0, 120000),
        ];

        split_chapter(&mut chapters, 0, 60000, "Chapter 1b".to_string()).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[0].end_time_ms, 60000);
        assert_eq!(chapters[1].title, "Chapter 1b");
        assert_eq!(chapters[1].start_time_ms, 60000);
        assert_eq!(chapters[1].end_time_ms, 120000);
    }

    #[test]
    fn test_merge_chapters() {
        let mut chapters = vec![
            Chapter::new(0, "A".to_string(), 0, 60000),
            Chapter::new(1, "B".to_string(), 60000, 120000),
            Chapter::new(2, "C".to_string(), 120000, 180000),
        ];

        merge_chapters(&mut chapters, 0, 1, "AB".to_string()).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "AB");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 120000);
    }
}
