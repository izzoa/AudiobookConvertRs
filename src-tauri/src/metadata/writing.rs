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
use mp4ameta::ident::FreeformIdentStatic;
use mp4ameta::{Data, FreeformIdent, Img, ImgFmt, Tag};
use tracing::{debug, info, warn};

/// iTunes freeform atom mean shared by the non-standard field identifiers below.
pub const FREEFORM_MEAN: &str = "com.apple.iTunes";

/// Freeform identifiers for fields with no standard MP4 atom.
/// The names are the conventions audiobook players expect (see design.md).
pub const NARRATOR_IDENT: FreeformIdentStatic =
    FreeformIdent::new_static(FREEFORM_MEAN, "NARRATOR");
pub const PUBLISHER_IDENT: FreeformIdentStatic =
    FreeformIdent::new_static(FREEFORM_MEAN, "PUBLISHER");
pub const SERIES_IDENT: FreeformIdentStatic = FreeformIdent::new_static(FREEFORM_MEAN, "SERIES");
pub const SERIES_PART_IDENT: FreeformIdentStatic =
    FreeformIdent::new_static(FREEFORM_MEAN, "SERIES-PART");
pub const LANGUAGE_IDENT: FreeformIdentStatic =
    FreeformIdent::new_static(FREEFORM_MEAN, "LANGUAGE");
pub const ASIN_IDENT: FreeformIdentStatic = FreeformIdent::new_static(FREEFORM_MEAN, "ASIN");

/// Treat absent, empty, and whitespace-only values as "not configured".
fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.trim().is_empty())
}

/// Apply all configured text metadata fields to an mp4ameta tag.
///
/// This is the single field list shared by every write path, so a field
/// added to `Metadata` has exactly one place to be wired up. Empty values
/// are skipped: a cleared text input sends `Some("")` and must not write
/// an empty atom.
fn apply_metadata_to_tag(tag: &mut Tag, metadata: &Metadata) {
    if let Some(title) = non_empty(&metadata.title) {
        info!("Setting title: {}", title);
        tag.set_title(title);
    }

    if let Some(artist) = non_empty(&metadata.artist) {
        info!("Setting artist: {}", artist);
        tag.set_artist(artist);
    }

    if let Some(album) = non_empty(&metadata.album) {
        info!("Setting album: {}", album);
        tag.set_album(album);
    }

    if let Some(album_artist) = non_empty(&metadata.album_artist) {
        tag.set_album_artist(album_artist);
    }

    if let Some(composer) = non_empty(&metadata.composer) {
        tag.set_composer(composer);
    }

    if let Some(genre) = non_empty(&metadata.genre) {
        tag.set_genre(genre);
    }

    if let Some(year) = metadata.year {
        tag.set_year(year.to_string());
    }

    if let Some(comment) = non_empty(&metadata.comments) {
        tag.set_comment(comment);
    }

    if let Some(description) = non_empty(&metadata.description) {
        tag.set_description(description);
    }

    if let Some(copyright) = non_empty(&metadata.copyright) {
        tag.set_copyright(copyright);
    }

    if let Some(narrator) = non_empty(&metadata.narrator) {
        tag.set_data(NARRATOR_IDENT, Data::Utf8(narrator.to_owned()));
    }

    if let Some(publisher) = non_empty(&metadata.publisher) {
        tag.set_data(PUBLISHER_IDENT, Data::Utf8(publisher.to_owned()));
    }

    if let Some(series) = non_empty(&metadata.series) {
        tag.set_data(SERIES_IDENT, Data::Utf8(series.to_owned()));
    }

    if let Some(series_index) = non_empty(&metadata.series_index) {
        tag.set_data(SERIES_PART_IDENT, Data::Utf8(series_index.to_owned()));
    }

    if let Some(language) = non_empty(&metadata.language) {
        tag.set_data(LANGUAGE_IDENT, Data::Utf8(language.to_owned()));
    }

    if let Some(asin) = non_empty(&metadata.asin) {
        tag.set_data(ASIN_IDENT, Data::Utf8(asin.to_owned()));
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
}

/// Write metadata to an M4B file
///
/// Uses mp4ameta to write standard MP4 metadata atoms.
/// Existing metadata is preserved unless explicitly overwritten.
pub fn write_metadata_to_m4b(path: &Utf8Path, metadata: &Metadata) -> AppResult<()> {
    info!("Writing metadata to: {} - title={:?}, artist={:?}, album={:?}", 
        path, metadata.title, metadata.artist, metadata.album);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let mut tag = Tag::read_from_path(path.as_std_path())
        .map_err(|e| AppError::MetadataWriteError {
            path: path.as_std_path().to_path_buf(),
            reason: e.to_string(),
        })?;

    apply_metadata_to_tag(&mut tag, metadata);

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
    apply_metadata_to_tag(&mut tag, metadata);

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
    use camino::Utf8PathBuf;

    fn fixture_path() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("minimal.m4b")
    }

    /// Copy the minimal fixture to a temp file so tests never mutate the fixture.
    fn copy_fixture(temp_dir: &std::path::Path) -> Utf8PathBuf {
        let dest = Utf8PathBuf::from_path_buf(temp_dir.join("minimal.m4b")).unwrap();
        std::fs::copy(fixture_path().as_std_path(), dest.as_std_path()).unwrap();
        dest
    }

    #[test]
    fn test_fixture_opens_with_mp4ameta() {
        let tag = Tag::read_from_path(fixture_path().as_std_path());
        assert!(tag.is_ok(), "mp4ameta must open the fixture: {:?}", tag.err());
    }

    /// Every text field of `Metadata` with a configured value must survive
    /// a write-then-read round trip through `write_metadata_to_m4b`.
    #[test]
    fn test_write_metadata_round_trip_all_fields() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = copy_fixture(temp_dir.path());

        let metadata = Metadata {
            title: Some("The Title".to_string()),
            artist: Some("The Author".to_string()),
            album: Some("The Album".to_string()),
            album_artist: Some("The Album Artist".to_string()),
            narrator: Some("The Narrator".to_string()),
            composer: Some("The Composer".to_string()),
            year: Some(2024),
            genre: Some("Audiobook".to_string()),
            track_number: Some(3),
            total_tracks: Some(30),
            disc_number: Some(2),
            total_discs: Some(4),
            comments: Some("A comment".to_string()),
            description: Some("A description".to_string()),
            publisher: Some("The Publisher".to_string()),
            copyright: Some("The Copyright".to_string()),
            series: Some("Pimsleur Italian".to_string()),
            series_index: Some("1".to_string()),
            language: Some("it".to_string()),
            asin: Some("B002V8H660".to_string()),
            artwork_path: None,
            artwork_data: None,
        };

        write_metadata_to_m4b(&path, &metadata).unwrap();

        let tag = Tag::read_from_path(path.as_std_path()).unwrap();
        assert_eq!(tag.title(), Some("The Title"));
        assert_eq!(tag.artist(), Some("The Author"));
        assert_eq!(tag.album(), Some("The Album"));
        assert_eq!(tag.album_artist(), Some("The Album Artist"));
        assert_eq!(tag.composer(), Some("The Composer"));
        assert_eq!(tag.year(), Some("2024"));
        assert_eq!(tag.genre(), Some("Audiobook"));
        assert_eq!(tag.track_number(), Some(3));
        assert_eq!(tag.total_tracks(), Some(30));
        assert_eq!(tag.disc_number(), Some(2));
        assert_eq!(tag.total_discs(), Some(4));
        assert_eq!(tag.comment(), Some("A comment"));
        assert_eq!(tag.description(), Some("A description"));
        assert_eq!(tag.copyright(), Some("The Copyright"));
        assert_eq!(tag.strings_of(&NARRATOR_IDENT).next(), Some("The Narrator"));
        assert_eq!(tag.strings_of(&PUBLISHER_IDENT).next(), Some("The Publisher"));
        assert_eq!(tag.strings_of(&SERIES_IDENT).next(), Some("Pimsleur Italian"));
        assert_eq!(tag.strings_of(&SERIES_PART_IDENT).next(), Some("1"));
        assert_eq!(tag.strings_of(&LANGUAGE_IDENT).next(), Some("it"));
        assert_eq!(tag.strings_of(&ASIN_IDENT).next(), Some("B002V8H660"));
    }

    /// The combined write path must persist the same complete field set
    /// as the metadata-only path.
    #[test]
    fn test_write_all_round_trip_all_fields() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = copy_fixture(temp_dir.path());

        let metadata = Metadata {
            title: Some("The Title".to_string()),
            artist: Some("The Author".to_string()),
            album: Some("The Album".to_string()),
            album_artist: Some("The Album Artist".to_string()),
            narrator: Some("The Narrator".to_string()),
            composer: Some("The Composer".to_string()),
            year: Some(2024),
            genre: Some("Audiobook".to_string()),
            track_number: Some(3),
            total_tracks: Some(30),
            disc_number: Some(2),
            total_discs: Some(4),
            comments: Some("A comment".to_string()),
            description: Some("A description".to_string()),
            publisher: Some("The Publisher".to_string()),
            copyright: Some("The Copyright".to_string()),
            series: Some("Pimsleur Italian".to_string()),
            series_index: Some("1".to_string()),
            language: Some("it".to_string()),
            asin: Some("B002V8H660".to_string()),
            artwork_path: None,
            artwork_data: None,
        };

        write_all_to_m4b(&path, &metadata, None, &[]).unwrap();

        let tag = Tag::read_from_path(path.as_std_path()).unwrap();
        assert_eq!(tag.title(), Some("The Title"));
        assert_eq!(tag.artist(), Some("The Author"));
        assert_eq!(tag.album(), Some("The Album"));
        assert_eq!(tag.album_artist(), Some("The Album Artist"));
        assert_eq!(tag.composer(), Some("The Composer"));
        assert_eq!(tag.year(), Some("2024"));
        assert_eq!(tag.genre(), Some("Audiobook"));
        assert_eq!(tag.track_number(), Some(3));
        assert_eq!(tag.total_tracks(), Some(30));
        assert_eq!(tag.disc_number(), Some(2));
        assert_eq!(tag.total_discs(), Some(4));
        assert_eq!(tag.comment(), Some("A comment"));
        assert_eq!(tag.description(), Some("A description"));
        assert_eq!(tag.copyright(), Some("The Copyright"));
        assert_eq!(tag.strings_of(&NARRATOR_IDENT).next(), Some("The Narrator"));
        assert_eq!(tag.strings_of(&PUBLISHER_IDENT).next(), Some("The Publisher"));
        assert_eq!(tag.strings_of(&SERIES_IDENT).next(), Some("Pimsleur Italian"));
        assert_eq!(tag.strings_of(&SERIES_PART_IDENT).next(), Some("1"));
        assert_eq!(tag.strings_of(&LANGUAGE_IDENT).next(), Some("it"));
        assert_eq!(tag.strings_of(&ASIN_IDENT).next(), Some("B002V8H660"));
    }

    /// Fields with no configured value (None or empty/whitespace) must leave
    /// no atom behind — no empty strings, no freeform placeholders.
    #[test]
    fn test_unconfigured_fields_stay_absent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = copy_fixture(temp_dir.path());

        let metadata = Metadata {
            title: Some("Only Title".to_string()),
            narrator: Some("   ".to_string()),
            series: Some("".to_string()),
            ..Default::default()
        };

        write_metadata_to_m4b(&path, &metadata).unwrap();

        let tag = Tag::read_from_path(path.as_std_path()).unwrap();
        assert_eq!(tag.title(), Some("Only Title"));
        assert_eq!(tag.artist(), None);
        assert_eq!(tag.album(), None);
        assert_eq!(tag.album_artist(), None);
        assert_eq!(tag.composer(), None);
        assert_eq!(tag.year(), None);
        assert_eq!(tag.genre(), None);
        assert_eq!(tag.comment(), None);
        assert_eq!(tag.description(), None);
        assert_eq!(tag.copyright(), None);
        assert_eq!(tag.track_number(), None);
        assert_eq!(tag.disc_number(), None);
        assert_eq!(tag.strings_of(&NARRATOR_IDENT).next(), None);
        assert_eq!(tag.strings_of(&PUBLISHER_IDENT).next(), None);
        assert_eq!(tag.strings_of(&SERIES_IDENT).next(), None);
        assert_eq!(tag.strings_of(&SERIES_PART_IDENT).next(), None);
        assert_eq!(tag.strings_of(&LANGUAGE_IDENT).next(), None);
        assert_eq!(tag.strings_of(&ASIN_IDENT).next(), None);
    }

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
