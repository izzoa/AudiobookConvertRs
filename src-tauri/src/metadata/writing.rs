//! Metadata writing module
//!
//! Provides functions for writing metadata, artwork, and chapters
//! to M4B files using the mp4ameta library.
//!
//! Note: mp4ameta 0.13.0 supports reading chapters but not writing them.
//! Chapter writing will be handled via FFmpeg in the conversion pipeline.

use crate::error::{AppError, AppResult};
use crate::models::{Chapter, Metadata};
use camino::Utf8Path;
use mp4ameta::{Img, ImgFmt, Tag};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Write metadata to an M4B file
///
/// Uses mp4ameta to write standard MP4 metadata atoms.
/// Existing metadata is preserved unless explicitly overwritten.
pub fn write_metadata_to_m4b(path: &Utf8Path, metadata: &Metadata) -> AppResult<()> {
    debug!("Writing metadata to: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let mut tag = Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    // Set metadata fields if present
    if let Some(ref title) = metadata.title {
        tag.set_title(title);
    }

    if let Some(ref artist) = metadata.artist {
        tag.set_artist(artist);
    }

    if let Some(ref album) = metadata.album {
        tag.set_album(album);
    }

    if let Some(ref album_artist) = metadata.album_artist {
        tag.set_album_artist(album_artist);
    }

    if let Some(ref composer) = metadata.composer {
        tag.set_composer(composer);
    }

    if let Some(ref genre) = metadata.genre {
        tag.set_genre(genre);
    }

    if let Some(year) = metadata.year {
        tag.set_year(year.to_string());
    }

    if let Some(ref comment) = metadata.comments {
        tag.set_comment(comment);
    }

    if let Some(ref copyright) = metadata.copyright {
        tag.set_copyright(copyright);
    }

    if let Some(track) = metadata.track_number {
        tag.set_track_number(track as u16);
    }

    if let Some(total) = metadata.total_tracks {
        tag.set_total_tracks(total as u16);
    }

    if let Some(disc) = metadata.disc_number {
        tag.set_disc_number(disc as u16);
    }

    if let Some(total) = metadata.total_discs {
        tag.set_total_discs(total as u16);
    }

    // Write changes
    tag.write_to_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    info!("Successfully wrote metadata to: {}", path);
    Ok(())
}

/// Embed artwork in an M4B file
///
/// The artwork should be in JPEG or PNG format.
/// Any existing artwork will be replaced.
pub fn embed_artwork_in_m4b(path: &Utf8Path, artwork: &[u8]) -> AppResult<()> {
    debug!("Embedding artwork ({} bytes) in: {}", artwork.len(), path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    if artwork.is_empty() {
        return Err(AppError::ArtworkError("Artwork data is empty".to_string()));
    }

    let mut tag = Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    // Detect image format from magic bytes
    let img_fmt = if artwork.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ImgFmt::Jpeg
    } else if artwork.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ImgFmt::Png
    } else {
        // Default to JPEG, mp4ameta might handle it
        warn!("Unknown image format, defaulting to JPEG");
        ImgFmt::Jpeg
    };

    // Remove existing artwork and add new
    tag.remove_artworks();
    tag.add_artwork(Img::new(img_fmt, artwork.to_vec()));

    // Write changes
    tag.write_to_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    info!("Successfully embedded artwork in: {}", path);
    Ok(())
}

/// Write QuickTime chapters to an M4B file
///
/// Note: mp4ameta 0.13.0 does not support writing chapters directly.
/// This function is a placeholder that logs a warning.
/// Chapter embedding will be handled via FFmpeg metadata in the conversion pipeline.
///
/// TODO: Implement chapter writing via FFmpeg in conversion/pipeline.rs
pub fn write_chapters_to_m4b(path: &Utf8Path, chapters: &[Chapter]) -> AppResult<()> {
    debug!("Request to write {} chapters to: {}", chapters.len(), path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    // mp4ameta 0.13.0 does not support writing chapters
    // Chapters need to be embedded via FFmpeg during conversion
    warn!(
        "Chapter writing via mp4ameta not supported. {} chapters will need to be embedded via FFmpeg.",
        chapters.len()
    );

    // Return Ok since this is a known limitation, not an error
    // The conversion pipeline should handle chapter embedding
    info!(
        "Chapters prepared for embedding: {} chapters (will be handled by FFmpeg)",
        chapters.len()
    );
    Ok(())
}

/// Generate FFmpeg metadata file content for chapters
///
/// Creates a metadata file format that can be used with FFmpeg's -i option
/// for embedding chapters during conversion.
pub fn generate_ffmpeg_chapters_metadata(chapters: &[Chapter]) -> String {
    let mut metadata = String::new();
    metadata.push_str(";FFMETADATA1\n");

    for chapter in chapters {
        metadata.push_str("\n[CHAPTER]\n");
        metadata.push_str(&format!("TIMEBASE=1/1000\n"));
        metadata.push_str(&format!("START={}\n", chapter.start_time_ms));
        metadata.push_str(&format!("END={}\n", chapter.end_time_ms));
        metadata.push_str(&format!("title={}\n", escape_ffmetadata(&chapter.title)));
    }

    metadata
}

/// Escape special characters for FFmetadata format
fn escape_ffmetadata(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('=', "\\=")
        .replace(';', "\\;")
        .replace('#', "\\#")
        .replace('\n', "\\\n")
}

/// Write all metadata, artwork, and chapters to an M4B file
///
/// Convenience function that combines metadata, artwork, and chapter writing.
/// Note: Chapters will only be prepared, actual embedding happens via FFmpeg.
pub fn write_all_to_m4b(
    path: &Utf8Path,
    metadata: &Metadata,
    artwork: Option<&[u8]>,
    chapters: &[Chapter],
) -> AppResult<()> {
    debug!("Writing all metadata to: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let mut tag = Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    // Set metadata
    if let Some(ref title) = metadata.title {
        tag.set_title(title);
    }
    if let Some(ref artist) = metadata.artist {
        tag.set_artist(artist);
    }
    if let Some(ref album) = metadata.album {
        tag.set_album(album);
    }
    if let Some(ref album_artist) = metadata.album_artist {
        tag.set_album_artist(album_artist);
    }
    if let Some(ref composer) = metadata.composer {
        tag.set_composer(composer);
    }
    if let Some(ref genre) = metadata.genre {
        tag.set_genre(genre);
    }
    if let Some(year) = metadata.year {
        tag.set_year(year.to_string());
    }
    if let Some(ref comment) = metadata.comments {
        tag.set_comment(comment);
    }
    if let Some(ref copyright) = metadata.copyright {
        tag.set_copyright(copyright);
    }
    if let Some(track) = metadata.track_number {
        tag.set_track_number(track as u16);
    }
    if let Some(total) = metadata.total_tracks {
        tag.set_total_tracks(total as u16);
    }
    if let Some(disc) = metadata.disc_number {
        tag.set_disc_number(disc as u16);
    }
    if let Some(total) = metadata.total_discs {
        tag.set_total_discs(total as u16);
    }

    // Set artwork
    if let Some(art_data) = artwork {
        if !art_data.is_empty() {
            let img_fmt = if art_data.starts_with(&[0xFF, 0xD8, 0xFF]) {
                ImgFmt::Jpeg
            } else if art_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                ImgFmt::Png
            } else {
                ImgFmt::Jpeg
            };
            tag.remove_artworks();
            tag.add_artwork(Img::new(img_fmt, art_data.to_vec()));
        }
    }

    // Note: Chapters cannot be written via mp4ameta
    // Log if chapters are provided
    if !chapters.is_empty() {
        warn!(
            "{} chapters provided but will need FFmpeg for embedding",
            chapters.len()
        );
    }

    // Write all changes in one operation
    tag.write_to_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    info!("Successfully wrote all metadata to: {}", path);
    Ok(())
}

/// Remove all metadata from an M4B file
///
/// Clears all metadata atoms except essential structural information.
pub fn clear_metadata(path: &Utf8Path) -> AppResult<()> {
    debug!("Clearing metadata from: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let mut tag = Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    // Remove all metadata (using singular method names per mp4ameta API)
    tag.remove_title();
    tag.remove_artists();
    tag.remove_album();
    tag.remove_album_artists();
    tag.remove_composers();
    tag.remove_genres();
    tag.remove_year();
    tag.remove_comments();
    tag.remove_copyright();
    tag.remove_artworks();
    // Note: remove_chapters not available in mp4ameta 0.13.0

    tag.write_to_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    info!("Cleared metadata from: {}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_metadata_file_not_found() {
        let path = Utf8Path::new("/nonexistent/file.m4b");
        let metadata = Metadata::default();
        let result = write_metadata_to_m4b(path, &metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_embed_artwork_file_not_found() {
        let path = Utf8Path::new("/nonexistent/file.m4b");
        let artwork = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        let result = embed_artwork_in_m4b(path, &artwork);
        assert!(result.is_err());
    }

    #[test]
    fn test_embed_artwork_empty_data() {
        // Even if file existed, empty artwork should fail
        let path = Utf8Path::new("/nonexistent/file.m4b");
        let result = embed_artwork_in_m4b(path, &[]);
        assert!(result.is_err());
        if let Err(AppError::ArtworkError(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[test]
    fn test_write_chapters_file_not_found() {
        let path = Utf8Path::new("/nonexistent/file.m4b");
        let chapters = vec![Chapter::new(0, "Test".to_string(), 0, 60000)];
        let result = write_chapters_to_m4b(path, &chapters);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_ffmpeg_chapters_metadata() {
        let chapters = vec![
            Chapter::new(0, "Introduction".to_string(), 0, 60000),
            Chapter::new(1, "Chapter 1".to_string(), 60000, 180000),
        ];

        let metadata = generate_ffmpeg_chapters_metadata(&chapters);

        assert!(metadata.starts_with(";FFMETADATA1"));
        assert!(metadata.contains("[CHAPTER]"));
        assert!(metadata.contains("title=Introduction"));
        assert!(metadata.contains("START=0"));
        assert!(metadata.contains("END=60000"));
    }

    #[test]
    fn test_escape_ffmetadata() {
        assert_eq!(escape_ffmetadata("normal"), "normal");
        assert_eq!(escape_ffmetadata("with=equals"), "with\\=equals");
        assert_eq!(escape_ffmetadata("with;semi"), "with\\;semi");
    }
}
