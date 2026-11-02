//! Metadata handling module
//!
//! This module handles:
//! - Metadata extraction from audio files
//! - Artwork detection and processing
//! - Chapter management
//! - Metadata writing to M4B files using mp4ameta
//! - CUE sheet parsing for FLAC files

pub mod artwork;
pub mod audible;
pub mod chapter_generation;
pub mod chapter_io;
pub mod chapter_match;
pub mod chapters;
pub mod cue_parser;
pub mod editor;
pub mod extraction;
pub mod format_specific;
pub mod lookup;
pub mod open_library;
pub mod writing;

// Re-export commonly used items
pub use artwork::{
    detect_image_format, extract_embedded_artwork, resize_artwork_if_needed,
    scan_directory_for_artwork, ImageFormat,
};
pub use chapter_generation::{
    apply_preset, extract_chapters_from_sources, get_regex_presets, rename_chapters_regex,
    RegexPreset,
};
pub use chapter_io::{parse_chapters, render_chapters, ChapterFormat};
pub use chapters::{
    create_chapters_auto_split, create_chapters_from_files, extract_chapters_from_m4b,
    validate_chapters, ChapterUpdate,
};
pub use cue_parser::{
    detect_cue_for_files, find_cue_file, generate_chapters_from_cue,
    generate_chapters_from_multifile_cue, parse_cue_content, parse_cue_file, CueDetectionResult,
    CueFile, CueSheet, CueTrack,
};
pub use editor::{update_audiobook_metadata, update_chapter, validate_metadata_fields};
pub use extraction::{
    extract_id3_tags, extract_metadata, extract_mp4_metadata, extract_vorbis_comments,
    normalize_metadata,
};
pub use format_specific::{
    analyze_existing_m4b, auto_extract_chapters, extract_chapters_from_flac_cuesheet,
    extract_chapters_from_mp3, M4bAnalysis,
};
pub use writing::{
    embed_artwork_in_m4b, write_all_to_m4b, write_chapters_to_m4b, write_metadata_to_m4b,
};
