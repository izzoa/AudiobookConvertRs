//! Metadata editing module
//!
//! Provides an API for in-memory editing of audiobook metadata
//! and chapters. Changes are tracked and can be validated before
//! being written to files.

use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, Chapter, Metadata};
use tracing::{debug, info};

// Re-export ChapterUpdate from chapters module for convenience
pub use crate::metadata::chapters::ChapterUpdate;

/// Update audiobook metadata
///
/// Replaces the audiobook's output_metadata with the new metadata.
/// Validates the new metadata before applying.
pub fn update_audiobook_metadata(audiobook: &mut AudioBook, metadata: Metadata) -> AppResult<()> {
    debug!("Updating metadata for audiobook: {}", audiobook.id);

    // Validate metadata
    validate_metadata_fields(&metadata)?;

    // Apply the new metadata
    audiobook.output_metadata = metadata;

    // Update the audiobook's title if provided
    if let Some(ref title) = audiobook.output_metadata.title {
        audiobook.title = title.clone();
    }

    // Update the author if provided
    if let Some(ref artist) = audiobook.output_metadata.artist {
        audiobook.author = Some(artist.clone());
    }

    // Mark as modified
    audiobook.mark_modified();

    info!("Updated metadata for audiobook: {}", audiobook.id);
    Ok(())
}

/// Update a single chapter in the audiobook
pub fn update_chapter(
    audiobook: &mut AudioBook,
    index: usize,
    update: ChapterUpdate,
) -> AppResult<()> {
    debug!("Updating chapter {} in audiobook: {}", index, audiobook.id);

    crate::metadata::chapters::update_chapter(&mut audiobook.chapters, index, update)?;

    audiobook.mark_modified();
    Ok(())
}

/// Add a new chapter to the audiobook
pub fn add_chapter(audiobook: &mut AudioBook, chapter: Chapter) -> AppResult<()> {
    debug!("Adding chapter to audiobook: {}", audiobook.id);

    crate::metadata::chapters::add_chapter(&mut audiobook.chapters, chapter)?;

    audiobook.mark_modified();
    Ok(())
}

/// Set audiobook artwork from raw bytes
///
/// Converts/resizes the image bytes to JPEG format and stores as base64.
/// This function runs CPU-intensive work via spawn_blocking.
pub fn set_artwork_bytes(audiobook: &mut AudioBook, bytes: &[u8]) -> AppResult<()> {
    debug!("Setting artwork from bytes for audiobook {}: {} bytes", audiobook.id, bytes.len());

    if bytes.is_empty() {
        return Err(AppError::ArtworkError("Artwork data is empty".to_string()));
    }

    // Convert/resize the artwork using the artwork module
    let processed_bytes = crate::metadata::artwork::resize_artwork_if_needed(
        bytes, 
        crate::metadata::artwork::DEFAULT_MAX_ARTWORK_DIMENSION
    )?;

    // Encode as base64
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let base64_data = STANDARD.encode(&processed_bytes);

    // Clear the path since we're setting from bytes, not a file
    audiobook.output_metadata.artwork_path = None;
    audiobook.output_metadata.artwork_data = Some(base64_data);
    audiobook.mark_modified();

    info!("Set artwork from bytes for audiobook {}", audiobook.id);
    Ok(())
}

/// Set audiobook artwork from a file path
pub fn set_artwork(audiobook: &mut AudioBook, path: impl Into<std::path::PathBuf>) -> AppResult<()> {
    let path_buf = path.into();
    let utf8_path = camino::Utf8PathBuf::from_path_buf(path_buf.clone())
        .map_err(|_| AppError::InvalidPath(path_buf.to_string_lossy().to_string()))?;

    debug!("Setting artwork for audiobook {}: {}", audiobook.id, utf8_path);

    // Read image data
    let image_data = std::fs::read(&utf8_path)
        .map_err(|e| AppError::file_read(&utf8_path, e.to_string()))?;
    
    // Convert/resize using set_artwork_bytes
    set_artwork_bytes(audiobook, &image_data)?;
    
    // Also store the path for reference
    audiobook.output_metadata.artwork_path = Some(utf8_path);

    Ok(())
}

/// Clear audiobook artwork
pub fn clear_artwork(audiobook: &mut AudioBook) -> AppResult<()> {
    debug!("Clearing artwork for audiobook {}", audiobook.id);

    audiobook.output_metadata.artwork_path = None;
    audiobook.output_metadata.artwork_data = None;
    audiobook.mark_modified();

    info!("Cleared artwork for audiobook {}", audiobook.id);
    Ok(())
}

/// Delete a chapter from the audiobook
pub fn delete_chapter(audiobook: &mut AudioBook, index: usize) -> AppResult<()> {
    debug!("Deleting chapter {} from audiobook: {}", index, audiobook.id);

    crate::metadata::chapters::delete_chapter(&mut audiobook.chapters, index)?;

    audiobook.mark_modified();
    Ok(())
}

/// Reorder chapters in the audiobook
pub fn reorder_chapters(
    audiobook: &mut AudioBook,
    old_index: usize,
    new_index: usize,
) -> AppResult<()> {
    debug!(
        "Reordering chapter {} to {} in audiobook: {}",
        old_index, new_index, audiobook.id
    );

    crate::metadata::chapters::reorder_chapters(&mut audiobook.chapters, old_index, new_index)?;

    audiobook.mark_modified();
    Ok(())
}

/// Validate all chapters in the audiobook
pub fn validate_audiobook_chapters(audiobook: &AudioBook) -> AppResult<()> {
    crate::metadata::chapters::validate_chapters(&audiobook.chapters, audiobook.total_duration_ms)
}

/// Validate metadata fields
///
/// Checks for:
/// - Field length limits
/// - Invalid characters
/// - Required fields (if any)
pub fn validate_metadata_fields(metadata: &Metadata) -> AppResult<()> {
    // Maximum field lengths (reasonable limits)
    const MAX_TITLE_LEN: usize = 500;
    const MAX_ARTIST_LEN: usize = 500;
    const MAX_ALBUM_LEN: usize = 500;
    const MAX_COMMENT_LEN: usize = 5000;
    const MAX_DESCRIPTION_LEN: usize = 10000;
    const MAX_SERIES_LEN: usize = 500;

    if let Some(ref title) = metadata.title {
        if title.len() > MAX_TITLE_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "title".to_string(),
                reason: format!("Title exceeds maximum length of {} characters", MAX_TITLE_LEN),
            });
        }
    }

    if let Some(ref artist) = metadata.artist {
        if artist.len() > MAX_ARTIST_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "artist".to_string(),
                reason: format!("Artist exceeds maximum length of {} characters", MAX_ARTIST_LEN),
            });
        }
    }

    if let Some(ref album) = metadata.album {
        if album.len() > MAX_ALBUM_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "album".to_string(),
                reason: format!("Album exceeds maximum length of {} characters", MAX_ALBUM_LEN),
            });
        }
    }

    if let Some(ref comments) = metadata.comments {
        if comments.len() > MAX_COMMENT_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "comments".to_string(),
                reason: format!(
                    "Comments exceeds maximum length of {} characters",
                    MAX_COMMENT_LEN
                ),
            });
        }
    }

    // Validate year if present
    if let Some(year) = metadata.year {
        if year < 1800 || year > 2100 {
            return Err(AppError::InvalidMetadataField {
                field: "year".to_string(),
                reason: format!("Year {} is not a valid year (1800-2100)", year),
            });
        }
    }

    if let Some(ref description) = metadata.description {
        if description.len() > MAX_DESCRIPTION_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "description".to_string(),
                reason: format!(
                    "Description exceeds maximum length of {} characters",
                    MAX_DESCRIPTION_LEN
                ),
            });
        }
    }

    if let Some(ref series) = metadata.series {
        if series.len() > MAX_SERIES_LEN {
            return Err(AppError::InvalidMetadataField {
                field: "series".to_string(),
                reason: format!("Series exceeds maximum length of {} characters", MAX_SERIES_LEN),
            });
        }
    }

    Ok(())
}

/// Create chapters from the audiobook's files
///
/// Convenience function that creates chapters based on file boundaries.
pub fn create_chapters_from_files(audiobook: &mut AudioBook) -> AppResult<()> {
    debug!("Creating chapters from files for audiobook: {}", audiobook.id);

    audiobook.chapters = crate::metadata::chapters::create_chapters_from_files(&audiobook.files);

    audiobook.mark_modified();
    info!("Created {} chapters from files", audiobook.chapters.len());
    Ok(())
}

/// Auto-split the audiobook into chapters
///
/// Creates `count` chapters of equal length.
pub fn auto_split_chapters(audiobook: &mut AudioBook, count: usize) -> AppResult<()> {
    debug!(
        "Auto-splitting audiobook into {} chapters: {}",
        count, audiobook.id
    );

    if count == 0 {
        return Err(AppError::InvalidChapter {
            reason: "Chapter count must be greater than 0".to_string(),
        });
    }

    audiobook.chapters =
        crate::metadata::chapters::create_chapters_auto_split(audiobook.total_duration_ms, count);

    audiobook.mark_modified();
    info!("Auto-split into {} chapters", audiobook.chapters.len());
    Ok(())
}

/// Merge metadata from source files into the audiobook's output metadata
///
/// Takes metadata from the first file and uses it as the base for output metadata.
pub fn merge_metadata_from_files(audiobook: &mut AudioBook) -> AppResult<()> {
    debug!("Merging metadata from files for audiobook: {}", audiobook.id);

    // Start with the first file's metadata
    if let Some(first_file) = audiobook.files.first() {
        audiobook.output_metadata = first_file.metadata.clone();
    }

    // Merge in metadata from other files (prefer existing values)
    for file in audiobook.files.iter().skip(1) {
        audiobook.output_metadata.merge_with(&file.metadata);
    }

    // Update title and author from merged metadata
    if let Some(ref title) = audiobook.output_metadata.title {
        audiobook.title = title.clone();
    }
    if let Some(ref artist) = audiobook.output_metadata.artist {
        audiobook.author = Some(artist.clone());
    }

    audiobook.mark_modified();
    info!("Merged metadata from {} files", audiobook.files.len());
    Ok(())
}

/// Check if the audiobook has unsaved changes
pub fn has_unsaved_changes(audiobook: &AudioBook) -> bool {
    audiobook.is_dirty
}

/// Mark the audiobook as saved (no pending changes)
pub fn mark_as_saved(audiobook: &mut AudioBook) {
    audiobook.mark_saved();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_audiobook() -> AudioBook {
        AudioBook::new("Test Audiobook".to_string())
    }

    #[test]
    fn test_update_audiobook_metadata() {
        let mut audiobook = create_test_audiobook();

        let new_metadata = Metadata {
            title: Some("New Title".to_string()),
            artist: Some("New Author".to_string()),
            ..Default::default()
        };

        update_audiobook_metadata(&mut audiobook, new_metadata).unwrap();

        assert_eq!(audiobook.title, "New Title");
        assert_eq!(audiobook.author, Some("New Author".to_string()));
        assert!(audiobook.is_dirty);
    }

    #[test]
    fn test_validate_metadata_fields_valid() {
        let metadata = Metadata {
            title: Some("Valid Title".to_string()),
            artist: Some("Valid Artist".to_string()),
            year: Some(2024),
            ..Default::default()
        };

        assert!(validate_metadata_fields(&metadata).is_ok());
    }

    #[test]
    fn test_validate_metadata_fields_title_too_long() {
        let metadata = Metadata {
            title: Some("x".repeat(600)),
            ..Default::default()
        };

        assert!(validate_metadata_fields(&metadata).is_err());
    }

    #[test]
    fn test_validate_metadata_fields_invalid_year() {
        let metadata = Metadata {
            year: Some(1500),
            ..Default::default()
        };

        assert!(validate_metadata_fields(&metadata).is_err());
    }

    #[test]
    fn test_auto_split_chapters_zero() {
        let mut audiobook = create_test_audiobook();
        audiobook.total_duration_ms = 3600000;

        let result = auto_split_chapters(&mut audiobook, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_unsaved_changes() {
        let mut audiobook = create_test_audiobook();
        assert!(!has_unsaved_changes(&audiobook));

        audiobook.mark_modified();
        assert!(has_unsaved_changes(&audiobook));

        mark_as_saved(&mut audiobook);
        assert!(!has_unsaved_changes(&audiobook));
    }

    #[test]
    fn test_set_artwork_bytes_valid() {
        let mut audiobook = create_test_audiobook();

        // Minimal valid JPEG (1x1 red pixel)
        let jpeg_bytes = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
            0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
            0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09,
            0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
            0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
            0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
            0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
            0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
            0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00,
            0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
        ];

        let result = set_artwork_bytes(&mut audiobook, &jpeg_bytes);
        assert!(result.is_ok());
        assert!(audiobook.output_metadata.artwork_data.is_some());
        assert!(audiobook.is_dirty);

        // Verify base64 is non-empty
        let artwork_data = audiobook.output_metadata.artwork_data.unwrap();
        assert!(!artwork_data.is_empty());
    }

    #[test]
    fn test_set_artwork_bytes_empty() {
        let mut audiobook = create_test_audiobook();
        let result = set_artwork_bytes(&mut audiobook, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_artwork_bytes_invalid_image() {
        let mut audiobook = create_test_audiobook();
        let invalid_bytes = vec![0x00, 0x01, 0x02, 0x03];
        let result = set_artwork_bytes(&mut audiobook, &invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_artwork() {
        let mut audiobook = create_test_audiobook();
        
        // Set some artwork first
        audiobook.output_metadata.artwork_data = Some("base64data".to_string());
        audiobook.output_metadata.artwork_path = Some("path.jpg".into());

        clear_artwork(&mut audiobook).unwrap();

        assert!(audiobook.output_metadata.artwork_data.is_none());
        assert!(audiobook.output_metadata.artwork_path.is_none());
        assert!(audiobook.is_dirty);
    }
}
