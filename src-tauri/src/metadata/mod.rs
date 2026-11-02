//! Metadata handling module
//!
//! This module handles:
//! - Metadata extraction from audio files
//! - Artwork detection and processing
//! - Chapter management
//! - Metadata writing to M4B files using mp4ameta
//! - CUE sheet parsing for FLAC files

pub mod extraction;
pub mod artwork;
pub mod chapters;
pub mod chapter_io;
pub mod chapter_generation;
pub mod format_specific;
pub mod writing;
pub mod editor;
pub mod open_library;
pub mod lookup;
pub mod audible;
pub mod chapter_match;
pub mod cue_parser;

// Re-export commonly used items
pub use extraction::{extract_metadata, extract_id3_tags, extract_vorbis_comments, extract_mp4_metadata, normalize_metadata};
pub use artwork::{extract_embedded_artwork, scan_directory_for_artwork, resize_artwork_if_needed, detect_image_format, ImageFormat};
pub use chapters::{create_chapters_from_files, create_chapters_auto_split, extract_chapters_from_m4b, validate_chapters, ChapterUpdate};
pub use chapter_io::{ChapterFormat, parse_chapters, render_chapters};
pub use chapter_generation::{extract_chapters_from_sources, rename_chapters_regex, apply_preset, get_regex_presets, RegexPreset};
pub use writing::{write_metadata_to_m4b, embed_artwork_in_m4b, write_chapters_to_m4b, write_all_to_m4b};
pub use editor::{update_audiobook_metadata, update_chapter, validate_metadata_fields};
pub use format_specific::{analyze_existing_m4b, extract_chapters_from_flac_cuesheet, extract_chapters_from_mp3, auto_extract_chapters, M4bAnalysis};
pub use cue_parser::{CueSheet, CueTrack, CueFile, CueDetectionResult, parse_cue_file, parse_cue_content, find_cue_file, detect_cue_for_files, generate_chapters_from_cue, generate_chapters_from_multifile_cue};

