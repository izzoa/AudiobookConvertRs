//! Format-specific metadata and chapter handling
//!
//! Provides specialized handling for:
//! - Existing M4B files (chapter preservation, metadata overwriting)
//! - FLAC cue sheets (optional)
//! - MP3 ID3 CHAP frames (optional)

use crate::audio::loader::load_audio_file;
use crate::error::{AppError, AppResult};
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::metadata::chapters::extract_chapters_from_m4b;
use crate::metadata::extraction::extract_metadata;
use crate::models::{AudioFile, AudioFormat, Chapter, Metadata};
use camino::Utf8Path;
use regex::Regex;
use std::fs::File;
use std::process::{Command, Stdio};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
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

    let duration_ms = load_audio_file(path)?.duration_ms;
    let has_artwork = metadata.artwork_data.is_some();

    Ok(M4bAnalysis {
        metadata,
        chapters,
        duration_ms,
        has_artwork,
    })
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
/// Rippers embed the cue sheet as a `CUESHEET` Vorbis comment, which Symphonia
/// reads like any other tag; no FFmpeg tool is needed.
pub fn extract_chapters_from_flac_cuesheet(path: &Utf8Path) -> AppResult<Vec<Chapter>> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    debug!("Extracting cue sheet from FLAC: {}", path);

    match read_tag(path, "CUESHEET")? {
        Some(cuesheet) if !cuesheet.trim().is_empty() => parse_cuesheet(&cuesheet),
        _ => {
            debug!("No cue sheet found in FLAC file");
            Ok(Vec::new())
        }
    }
}

/// The first tag named `name` (any case), from the reader or the probe result
fn read_tag(path: &Utf8Path, name: &str) -> AppResult<Option<String>> {
    let file =
        File::open(path.as_std_path()).map_err(|e| AppError::metadata_read(path, e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AppError::metadata_read(path, e.to_string()))?;

    let find = |rev: &symphonia::core::meta::MetadataRevision| {
        rev.tags()
            .iter()
            .find(|tag| tag.key.eq_ignore_ascii_case(name))
            .map(|tag| tag.value.to_string())
    };
    let mut reader = probed.format;
    if let Some(value) = reader.metadata().current().and_then(find) {
        return Ok(Some(value));
    }
    Ok(probed
        .metadata
        .get()
        .as_ref()
        .and_then(|m| m.current())
        .and_then(find))
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
            current_index = caps
                .get(1)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(1)
                - 1;
        }

        if let Some(caps) = title_re.captures(line) {
            current_title = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
        }

        if let Some(caps) = index_re.captures(line) {
            let minutes: u64 = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let seconds: u64 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let frames: u64 = caps
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);

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
/// The resolved FFmpeg prints them as ffmetadata; it is never looked up by
/// name, because an app opened from Finder may have no Homebrew folder on its PATH.
pub fn extract_chapters_from_mp3(path: &Utf8Path) -> AppResult<Vec<Chapter>> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    debug!("Extracting ID3 chapters from MP3: {}", path);

    let ffmpeg = detect_ffmpeg()?;
    let output = Command::new(ffmpeg.path.as_str())
        .args(["-v", "error", "-i", path.as_str(), "-f", "ffmetadata", "-"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| AppError::ffmpeg_error(format!("could not read chapters: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::ffmpeg_error(format!(
            "could not read chapters: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let chapters = parse_ffmetadata_chapters(&String::from_utf8_lossy(&output.stdout));
    info!("Extracted {} chapters from MP3", chapters.len());
    Ok(chapters)
}

/// Chapters from FFmpeg's ffmetadata text. Each `[CHAPTER]` gives START and END
/// in its own TIMEBASE, nanoseconds when TIMEBASE is missing.
fn parse_ffmetadata_chapters(text: &str) -> Vec<Chapter> {
    #[derive(Default)]
    struct Section {
        timebase: Option<(u64, u64)>,
        start: Option<i64>,
        end: Option<i64>,
        title: Option<String>,
    }

    let mut sections = Vec::new();
    let mut current: Option<Section> = None;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            sections.extend(current.take());
            if line.eq_ignore_ascii_case("[CHAPTER]") {
                current = Some(Section::default());
            }
            continue;
        }
        let (Some(section), Some((key, value))) = (current.as_mut(), line.split_once('=')) else {
            continue;
        };
        match key.trim().to_ascii_uppercase().as_str() {
            "TIMEBASE" => {
                section.timebase = value
                    .split_once('/')
                    .and_then(|(n, d)| Some((n.trim().parse().ok()?, d.trim().parse().ok()?)))
                    .filter(|&(_, d): &(u64, u64)| d > 0);
            }
            "START" => section.start = value.trim().parse().ok(),
            "END" => section.end = value.trim().parse().ok(),
            "TITLE" => section.title = Some(unescape_ffmetadata(value)),
            _ => {}
        }
    }
    sections.extend(current);

    sections
        .into_iter()
        .filter_map(|section| {
            let (num, den) = section.timebase.unwrap_or((1, 1_000_000_000));
            let to_ms = |t: i64| (t.max(0) as u128 * num as u128 * 1000 / den as u128) as u64;
            let (start, end) = (to_ms(section.start?), to_ms(section.end?));
            (end > start).then_some((start, end, section.title))
        })
        .enumerate()
        .map(|(i, (start_time_ms, end_time_ms, title))| Chapter {
            index: i,
            title: title
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| format!("Chapter {}", i + 1)),
            start_time_ms,
            end_time_ms,
            auto_detected: true,
        })
        .collect()
}

/// ffmetadata escapes `=`, `;`, `#`, `\` and newlines with a backslash
fn unescape_ffmetadata(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.extend(chars.next());
        } else {
            out.push(c);
        }
    }
    out
}

/// Auto-detect and extract chapters from any supported format
pub fn auto_extract_chapters(file: &AudioFile) -> AppResult<Vec<Chapter>> {
    match file.format {
        AudioFormat::M4b | AudioFormat::M4a => extract_chapters_from_m4b(&file.path),
        AudioFormat::Flac => extract_chapters_from_flac_cuesheet(&file.path),
        AudioFormat::Mp3 => extract_chapters_from_mp3(&file.path),
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
    fn ffmetadata_chapters_use_each_timebase() {
        let text = ";FFMETADATA1\ntitle=Book\n\
[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=300000\ntitle=Opening \\= Prologue\n\
[CHAPTER]\nTIMEBASE=1/44100\nSTART=13230000\nEND=26460000\ntitle=Part Two\n\
[CHAPTER]\nSTART=600000000000\nEND=660000000000\n\
[CHAPTER]\nTIMEBASE=1/1000\nSTART=5\nEND=5\ntitle=Empty\n\
[STREAM]\ntitle=not a chapter\n";

        let chapters = parse_ffmetadata_chapters(text);
        let summary: Vec<_> = chapters
            .iter()
            .map(|c| (c.index, c.title.as_str(), c.start_time_ms, c.end_time_ms))
            .collect();
        assert_eq!(
            summary,
            [
                (0, "Opening = Prologue", 0, 300_000),
                (1, "Part Two", 300_000, 600_000),
                (2, "Chapter 3", 600_000, 660_000),
            ]
        );
    }
}
