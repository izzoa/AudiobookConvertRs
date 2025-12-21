//! Format-specific metadata and chapter handling
//!
//! Provides specialized handling for:
//! - Existing M4B files (chapter preservation, metadata overwriting)
//! - FLAC cue sheets (optional)
//! - MP3 ID3 CHAP frames (optional)

use crate::error::{AppError, AppResult};
use crate::models::{AudioFormat, AudioFile, Chapter, Metadata};
use crate::metadata::chapters::extract_chapters_from_m4b;
use crate::metadata::extraction::extract_metadata;
use camino::Utf8Path;
use regex::Regex;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};

/// Result of analyzing an existing M4B file
#[derive(Debug, Clone)]
pub struct M4bAnalysis {
    /// Original metadata from the file
    pub metadata: Metadata,
    /// Existing chapters in the file
    pub chapters: Vec<Chapter>,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Whether the file has embedded artwork
    pub has_artwork: bool,
}

/// Analyze an existing M4B file for re-encoding or metadata update
///
/// Returns the existing metadata, chapters, and artwork status
/// so they can be preserved or overwritten as needed.
pub fn analyze_existing_m4b(path: &Utf8Path) -> AppResult<M4bAnalysis> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    info!("Analyzing existing M4B: {}", path);

    let metadata = extract_metadata(path)?;
    let chapters = extract_chapters_from_m4b(path).unwrap_or_else(|e| {
        warn!("Failed to extract chapters from M4B: {}", e);
        Vec::new()
    });

    let duration_ms = get_m4b_duration(path)?;
    let has_artwork = metadata.artwork_data.is_some();

    Ok(M4bAnalysis {
        metadata,
        chapters,
        duration_ms,
        has_artwork,
    })
}

/// Get the duration of an M4B file using FFprobe
fn get_m4b_duration(path: &Utf8Path) -> AppResult<u64> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: format!("ffprobe failed: {}", e),
            
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration_secs: f64 = stdout.trim().parse().unwrap_or(0.0);
    
    Ok((duration_secs * 1000.0) as u64)
}

/// Detect if an audio file is an M4B and analyze it
pub fn detect_and_analyze_m4b(file: &AudioFile) -> Option<M4bAnalysis> {
    if file.format == AudioFormat::M4b {
        analyze_existing_m4b(&file.path).ok()
    } else {
        None
    }
}

/// Parse a FLAC cue sheet and convert to chapters
///
/// FLAC files can embed cue sheets that contain track/chapter information.
/// This extracts the cue sheet using FFmpeg and parses it into chapters.
pub fn extract_chapters_from_flac_cuesheet(path: &Utf8Path) -> AppResult<Vec<Chapter>> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    debug!("Extracting cue sheet from FLAC: {}", path);

    // Use FFmpeg to extract cue sheet metadata
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format_tags=cuesheet",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: format!("ffprobe failed: {}", e),
            
        })?;

    let cuesheet = String::from_utf8_lossy(&output.stdout);
    
    if cuesheet.trim().is_empty() {
        debug!("No cue sheet found in FLAC file");
        return Ok(Vec::new());
    }

    parse_cuesheet(&cuesheet)
}

/// Parse a cue sheet string into chapters
fn parse_cuesheet(cuesheet: &str) -> AppResult<Vec<Chapter>> {
    let mut chapters = Vec::new();
    let mut current_title = String::new();
    let mut current_index = 0;

    let track_re = Regex::new(r"(?i)TRACK\s+(\d+)").unwrap();
    let title_re = Regex::new(r#"(?i)TITLE\s+"([^"]+)""#).unwrap();
    let index_re = Regex::new(r"(?i)INDEX\s+01\s+(\d+):(\d+):(\d+)").unwrap();

    let mut pending_start_ms: Option<u64> = None;

    for line in cuesheet.lines() {
        let line = line.trim();

        if let Some(caps) = track_re.captures(line) {
            // Finalize previous chapter if we have one pending
            if let Some(start_ms) = pending_start_ms.take() {
                if !current_title.is_empty() || current_index > 0 {
                    // We'll set end time when we find the next track
                    chapters.push(Chapter {
                        index: current_index,
                        title: if current_title.is_empty() {
                            format!("Track {}", current_index + 1)
                        } else {
                            std::mem::take(&mut current_title)
                        },
                        start_time_ms: start_ms,
                        end_time_ms: 0, // Will be updated
                        auto_detected: true,
                    });
                }
            }
            current_index = caps.get(1)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(1) - 1;
        }

        if let Some(caps) = title_re.captures(line) {
            current_title = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        }

        if let Some(caps) = index_re.captures(line) {
            let minutes: u64 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let seconds: u64 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let frames: u64 = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            
            // CD frames are 1/75th of a second
            let ms = (minutes * 60 * 1000) + (seconds * 1000) + (frames * 1000 / 75);
            
            // Update previous chapter's end time
            if let Some(last) = chapters.last_mut() {
                if last.end_time_ms == 0 {
                    last.end_time_ms = ms;
                }
            }
            
            pending_start_ms = Some(ms);
        }
    }

    // Handle last chapter
    if let Some(start_ms) = pending_start_ms {
        chapters.push(Chapter {
            index: current_index,
            title: if current_title.is_empty() {
                format!("Track {}", current_index + 1)
            } else {
                current_title
            },
            start_time_ms: start_ms,
            end_time_ms: 0, // Will need to be set to total duration
            auto_detected: true,
        });
    }

    // Reindex chapters
    for (i, chapter) in chapters.iter_mut().enumerate() {
        chapter.index = i;
    }

    info!("Parsed {} chapters from cue sheet", chapters.len());
    Ok(chapters)
}

/// Extract ID3 CHAP frames from MP3 files
///
/// MP3 files can contain chapter information in ID3v2 CHAP and CTOC frames.
/// This uses FFprobe to extract the chapter data.
pub fn extract_chapters_from_mp3(path: &Utf8Path) -> AppResult<Vec<Chapter>> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    debug!("Extracting ID3 chapters from MP3: {}", path);

    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_chapters",
            "-of", "json",
            path.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: format!("ffprobe failed: {}", e),
            
        })?;

    let json = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_chapters(&json)
}

/// Parse FFprobe JSON chapter output
fn parse_ffprobe_chapters(json: &str) -> AppResult<Vec<Chapter>> {
    // Simple regex-based parsing for chapter data
    // Format: "start_time": "0.000000", "end_time": "300.000000", "tags": {"title": "Chapter 1"}
    
    let mut chapters = Vec::new();
    let start_re = Regex::new(r#""start_time"\s*:\s*"([\d.]+)""#).unwrap();
    let end_re = Regex::new(r#""end_time"\s*:\s*"([\d.]+)""#).unwrap();
    let title_re = Regex::new(r#""title"\s*:\s*"([^"]+)""#).unwrap();

    // Split by chapter blocks
    let chapter_blocks: Vec<&str> = json.split("\"id\"").skip(1).collect();

    for (i, block) in chapter_blocks.iter().enumerate() {
        let start_secs: f64 = start_re
            .captures(block)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0.0);

        let end_secs: f64 = end_re
            .captures(block)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0.0);

        let title = title_re
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| format!("Chapter {}", i + 1));

        if end_secs > start_secs {
            chapters.push(Chapter {
                index: i,
                title,
                start_time_ms: (start_secs * 1000.0) as u64,
                end_time_ms: (end_secs * 1000.0) as u64,
                auto_detected: true,
            });
        }
    }

    info!("Extracted {} chapters from MP3", chapters.len());
    Ok(chapters)
}

/// Auto-detect and extract chapters from any supported format
pub fn auto_extract_chapters(file: &AudioFile) -> AppResult<Vec<Chapter>> {
    match file.format {
        AudioFormat::M4b | AudioFormat::M4a => {
            extract_chapters_from_m4b(&file.path)
        }
        AudioFormat::Flac => {
            extract_chapters_from_flac_cuesheet(&file.path)
        }
        AudioFormat::Mp3 => {
            extract_chapters_from_mp3(&file.path)
        }
        _ => {
            debug!("No chapter extraction for format: {}", file.format);
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cuesheet() {
        let cuesheet = r#"
TRACK 01 AUDIO
  TITLE "Introduction"
  INDEX 01 00:00:00
TRACK 02 AUDIO
  TITLE "Chapter One"
  INDEX 01 05:30:00
TRACK 03 AUDIO
  TITLE "Chapter Two"
  INDEX 01 12:45:37
"#;

        let chapters = parse_cuesheet(cuesheet).unwrap();
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Introduction");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[1].title, "Chapter One");
        assert_eq!(chapters[1].start_time_ms, 5 * 60 * 1000 + 30 * 1000); // 5:30
        assert_eq!(chapters[2].title, "Chapter Two");
    }

    #[test]
    fn test_parse_ffprobe_chapters() {
        let json = r#"{
            "chapters": [
                {
                    "id": 0,
                    "start_time": "0.000000",
                    "end_time": "300.000000",
                    "tags": {
                        "title": "Chapter 1"
                    }
                },
                {
                    "id": 1,
                    "start_time": "300.000000",
                    "end_time": "600.000000",
                    "tags": {
                        "title": "Chapter 2"
                    }
                }
            ]
        }"#;

        let chapters = parse_ffprobe_chapters(json).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 300000);
        assert_eq!(chapters[1].title, "Chapter 2");
    }
}
