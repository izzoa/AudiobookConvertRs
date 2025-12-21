//! Metadata handling module
//!
//! This module handles:
//! - Metadata extraction from audio files
//! - Artwork detection and processing
//! - Chapter management
//! - Metadata writing to M4B files using mp4ameta

pub mod extraction;
pub mod artwork;
pub mod chapters;
pub mod format_specific;
pub mod writing;
pub mod editor;

// Re-export commonly used items
pub use extraction::{extract_metadata, extract_id3_tags, extract_vorbis_comments, extract_mp4_metadata, normalize_metadata};
pub use artwork::{extract_embedded_artwork, scan_directory_for_artwork, resize_artwork_if_needed, detect_image_format, ImageFormat};
pub use chapters::{create_chapters_from_files, create_chapters_auto_split, extract_chapters_from_m4b, validate_chapters, ChapterUpdate};
pub use writing::{write_metadata_to_m4b, embed_artwork_in_m4b, write_chapters_to_m4b, write_all_to_m4b};
pub use editor::{update_audiobook_metadata, update_chapter, validate_metadata_fields};
pub use format_specific::{analyze_existing_m4b, extract_chapters_from_flac_cuesheet, extract_chapters_from_mp3, auto_extract_chapters, M4bAnalysis};

