//! Metadata extraction module
//!
//! Provides unified metadata extraction across all audio formats using Symphonia.
//! This module wraps Symphonia's metadata capabilities and provides a consistent
//! interface for extracting ID3, Vorbis, and MP4 metadata.

use crate::error::{AppError, AppResult};
use crate::models::Metadata;
use camino::Utf8Path;
use std::fs::File;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
use symphonia::core::probe::Hint;
use tracing::{debug, warn};

/// Extract metadata from any supported audio file
///
/// Uses Symphonia to automatically detect the format and extract metadata.
/// Returns a normalized `Metadata` struct with all available fields populated.
pub fn extract_metadata(path: &Utf8Path) -> AppResult<Metadata> {
    debug!("Extracting metadata from: {}", path);

    // Verify file exists
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    // Open file for reading
    let file = File::open(path.as_std_path())
        .map_err(|e| AppError::metadata_read(path, e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint using the file extension
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext);
    }

    // Probe the file to get format
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AppError::metadata_read(path, e.to_string()))?;

    let mut format_reader = probed.format;
    let mut metadata = Metadata::default();

    // Extract metadata from the format reader
    if let Some(meta_rev) = format_reader.metadata().current() {
        extract_from_revision(&mut metadata, meta_rev);
    }

    // Also check for metadata in the probed result
    if let Some(probe_meta) = probed.metadata.get() {
        if let Some(meta_rev) = probe_meta.current() {
            extract_from_revision(&mut metadata, meta_rev);
        }
    }

    // If no title from metadata, use filename
    if metadata.title.is_none() {
        metadata.title = path.file_stem().map(|s| s.to_string());
    }

    // Normalize the metadata
    normalize_metadata(&mut metadata);

    debug!("Extracted metadata: title={:?}, artist={:?}", metadata.title, metadata.artist);
    Ok(metadata)
}

/// Extract ID3 tags from MP3 files
///
/// This is a convenience function that specifically extracts ID3v1/ID3v2 tags.
/// For MP3 files, this is equivalent to `extract_metadata`.
pub fn extract_id3_tags(path: &Utf8Path) -> AppResult<Metadata> {
    // Verify it's an MP3 file
    if let Some(ext) = path.extension() {
        if ext.to_lowercase() != "mp3" {
            warn!("extract_id3_tags called on non-MP3 file: {}", path);
        }
    }
    extract_metadata(path)
}

/// Extract Vorbis comments from FLAC/OGG files
///
/// This is a convenience function for FLAC and OGG files which use Vorbis comments.
pub fn extract_vorbis_comments(path: &Utf8Path) -> AppResult<Metadata> {
    // Verify it's a FLAC or OGG file
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_lowercase();
        if ext_lower != "flac" && ext_lower != "ogg" && ext_lower != "oga" {
            warn!("extract_vorbis_comments called on non-FLAC/OGG file: {}", path);
        }
    }
    extract_metadata(path)
}

/// Extract MP4 metadata from M4A/M4B files
///
/// This is a convenience function for MP4 container files.
/// Also uses mp4ameta for additional M4B-specific metadata.
pub fn extract_mp4_metadata(path: &Utf8Path) -> AppResult<Metadata> {
    // Verify it's an M4A/M4B file
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_lowercase();
        if ext_lower != "m4a" && ext_lower != "m4b" {
            warn!("extract_mp4_metadata called on non-M4A/M4B file: {}", path);
        }
    }

    // First extract with Symphonia
    let mut metadata = extract_metadata(path)?;

    // Try to get additional metadata with mp4ameta
    if let Ok(tag) = mp4ameta::Tag::read_from_path(path.as_std_path()) {
        // mp4ameta can provide additional fields that Symphonia might miss
        if metadata.title.is_none() {
            metadata.title = tag.title().map(|s| s.to_string());
        }
        if metadata.artist.is_none() {
            metadata.artist = tag.artist().map(|s| s.to_string());
        }
        if metadata.album.is_none() {
            metadata.album = tag.album().map(|s| s.to_string());
        }
        if metadata.album_artist.is_none() {
            metadata.album_artist = tag.album_artist().map(|s| s.to_string());
        }
        if metadata.year.is_none() {
            if let Some(year_str) = tag.year() {
                if let Ok(y) = year_str.parse::<u16>() {
                    metadata.year = Some(y);
                }
            }
        }
        if metadata.genre.is_none() {
            metadata.genre = tag.genre().map(|s| s.to_string());
        }
        if metadata.comments.is_none() {
            metadata.comments = tag.comment().map(|s| s.to_string());
        }
        if metadata.composer.is_none() {
            metadata.composer = tag.composer().map(|s| s.to_string());
        }
        if metadata.copyright.is_none() {
            metadata.copyright = tag.copyright().map(|s| s.to_string());
        }
    }

    Ok(metadata)
}

/// Extract metadata fields from a Symphonia metadata revision
fn extract_from_revision(metadata: &mut Metadata, meta_rev: &symphonia::core::meta::MetadataRevision) {
    for tag in meta_rev.tags() {
        let value_str = value_to_string(&tag.value);
        
        match tag.std_key {
            Some(StandardTagKey::TrackTitle) => {
                if metadata.title.is_none() {
                    metadata.title = Some(value_str);
                }
            }
            Some(StandardTagKey::Artist) => {
                if metadata.artist.is_none() {
                    metadata.artist = Some(value_str);
                }
            }
            Some(StandardTagKey::Album) => {
                if metadata.album.is_none() {
                    metadata.album = Some(value_str);
                }
            }
            Some(StandardTagKey::AlbumArtist) => {
                if metadata.album_artist.is_none() {
                    metadata.album_artist = Some(value_str);
                }
            }
            Some(StandardTagKey::Composer) => {
                if metadata.composer.is_none() {
                    metadata.composer = Some(value_str);
                }
            }
            Some(StandardTagKey::Genre) => {
                if metadata.genre.is_none() {
                    metadata.genre = Some(value_str);
                }
            }
            Some(StandardTagKey::Date) | Some(StandardTagKey::OriginalDate) => {
                if metadata.year.is_none() {
                    // Try to parse year from date string
                    if let Ok(year) = value_str.parse::<u16>() {
                        metadata.year = Some(year);
                    } else if value_str.len() >= 4 {
                        // Try first 4 characters as year (e.g., "2024-01-15")
                        if let Ok(year) = value_str[..4].parse::<u16>() {
                            metadata.year = Some(year);
                        }
                    }
                }
            }
            Some(StandardTagKey::Comment) => {
                if metadata.comments.is_none() {
                    metadata.comments = Some(value_str);
                }
            }
            Some(StandardTagKey::Description) => {
                if metadata.description.is_none() {
                    metadata.description = Some(value_str);
                }
            }
            Some(StandardTagKey::TrackNumber) => {
                if metadata.track_number.is_none() {
                    if let Ok(num) = value_str.parse::<u32>() {
                        metadata.track_number = Some(num);
                    }
                }
            }
            Some(StandardTagKey::TrackTotal) => {
                if metadata.total_tracks.is_none() {
                    if let Ok(num) = value_str.parse::<u32>() {
                        metadata.total_tracks = Some(num);
                    }
                }
            }
            Some(StandardTagKey::DiscNumber) => {
                if metadata.disc_number.is_none() {
                    if let Ok(num) = value_str.parse::<u32>() {
                        metadata.disc_number = Some(num);
                    }
                }
            }
            Some(StandardTagKey::DiscTotal) => {
                if metadata.total_discs.is_none() {
                    if let Ok(num) = value_str.parse::<u32>() {
                        metadata.total_discs = Some(num);
                    }
                }
            }
            Some(StandardTagKey::Label) => {
                if metadata.publisher.is_none() {
                    metadata.publisher = Some(value_str);
                }
            }
            Some(StandardTagKey::IdentCatalogNumber) | Some(StandardTagKey::IdentIsrc) => {
                // These could be useful for publisher/copyright
            }
            _ => {}
        }
    }

    // Extract artwork
    for visual in meta_rev.visuals() {
        if !visual.data.is_empty() && metadata.artwork_data.is_none() {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            metadata.artwork_data = Some(STANDARD.encode(&visual.data));
            break; // Just take the first artwork
        }
    }
}

/// Convert Symphonia Value to String
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Binary(_) => String::new(), // Don't convert binary to string
        Value::Boolean(b) => b.to_string(),
        Value::Flag => String::new(),
        Value::Float(f) => f.to_string(),
        Value::SignedInt(i) => i.to_string(),
        Value::UnsignedInt(u) => u.to_string(),
    }
}

/// Normalize metadata fields
///
/// - Trims whitespace from all string fields
/// - Handles encoding issues
/// - Removes problematic characters
pub fn normalize_metadata(metadata: &mut Metadata) {
    fn normalize_field(field: &mut Option<String>) {
        if let Some(ref mut val) = field {
            // Trim whitespace
            *val = val.trim().to_string();
            
            // Remove null characters and other control characters
            *val = val.chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                .collect();
            
            // If empty after trimming, set to None
            if val.is_empty() {
                *field = None;
            }
        }
    }

    normalize_field(&mut metadata.title);
    normalize_field(&mut metadata.artist);
    normalize_field(&mut metadata.album);
    normalize_field(&mut metadata.album_artist);
    normalize_field(&mut metadata.narrator);
    normalize_field(&mut metadata.composer);
    normalize_field(&mut metadata.genre);
    normalize_field(&mut metadata.comments);
    normalize_field(&mut metadata.description);
    normalize_field(&mut metadata.publisher);
    normalize_field(&mut metadata.copyright);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_metadata_trims_whitespace() {
        let mut metadata = Metadata {
            title: Some("  Test Title  ".to_string()),
            artist: Some("\n\tArtist Name\t".to_string()),
            ..Default::default()
        };

        normalize_metadata(&mut metadata);

        assert_eq!(metadata.title, Some("Test Title".to_string()));
        assert_eq!(metadata.artist, Some("Artist Name".to_string()));
    }

    #[test]
    fn test_normalize_metadata_removes_empty() {
        let mut metadata = Metadata {
            title: Some("   ".to_string()),
            artist: Some("".to_string()),
            album: Some("Valid Album".to_string()),
            ..Default::default()
        };

        normalize_metadata(&mut metadata);

        assert_eq!(metadata.title, None);
        assert_eq!(metadata.artist, None);
        assert_eq!(metadata.album, Some("Valid Album".to_string()));
    }

    #[test]
    fn test_normalize_metadata_removes_control_chars() {
        let mut metadata = Metadata {
            title: Some("Test\x00Title".to_string()),
            ..Default::default()
        };

        normalize_metadata(&mut metadata);

        assert_eq!(metadata.title, Some("TestTitle".to_string()));
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&Value::String("test".to_string())), "test");
        assert_eq!(value_to_string(&Value::SignedInt(42)), "42");
        assert_eq!(value_to_string(&Value::UnsignedInt(100)), "100");
        assert_eq!(value_to_string(&Value::Float(3.14)), "3.14");
        assert_eq!(value_to_string(&Value::Boolean(true)), "true");
    }

    #[test]
    fn test_extract_metadata_file_not_found() {
        let path = Utf8Path::new("/nonexistent/file.mp3");
        let result = extract_metadata(path);
        assert!(result.is_err());
    }
}
