//! CUE sheet parsing module
//!
//! Parses CUE sheet files and converts track information to chapters.
//! Supports single-file and multi-file CUE sheets.
//!
//! CUE format reference: https://wiki.hydrogenaud.io/index.php?title=Cue_sheet

use crate::error::{AppError, AppResult};
use crate::models::Chapter;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct CueTrack {
    pub number: u32,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start_time_ms: u64,
    pub file_index: usize,
}

#[derive(Debug, Clone)]
pub struct CueFile {
    pub path: String,
    pub file_type: String,
}

#[derive(Debug, Clone)]
pub struct CueSheet {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub files: Vec<CueFile>,
    pub tracks: Vec<CueTrack>,
}

impl Default for CueSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl CueSheet {
    pub fn new() -> Self {
        Self {
            title: None,
            performer: None,
            files: Vec::new(),
            tracks: Vec::new(),
        }
    }

    pub fn is_single_file(&self) -> bool {
        self.files.len() == 1
    }

    pub fn is_multi_file(&self) -> bool {
        self.files.len() > 1
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CueDetectionResult {
    pub cue_path: String,
    pub audio_files: Vec<String>,
    pub track_count: usize,
    pub is_multi_file: bool,
    pub album_title: Option<String>,
    pub album_artist: Option<String>,
}

pub fn parse_cue_timestamp(timestamp: &str) -> Option<u64> {
    let parts: Vec<&str> = timestamp.trim().split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let minutes: u64 = parts[0].parse().ok()?;
    let seconds: u64 = parts[1].parse().ok()?;
    let frames: u64 = parts[2].parse().ok()?;

    Some(minutes * 60 * 1000 + seconds * 1000 + (frames * 1000 / 75))
}

fn parse_quoted_string(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_cue_file(path: &Utf8Path) -> AppResult<CueSheet> {
    debug!("Parsing CUE file: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let content = fs::read_to_string(path.as_std_path())
        .map_err(|e| AppError::file_read(path, e.to_string()))?;

    parse_cue_content(&content)
}

pub fn parse_cue_content(content: &str) -> AppResult<CueSheet> {
    let mut cue = CueSheet::new();
    let mut current_file_index: Option<usize> = None;
    let mut current_track: Option<CueTrack> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("REM") {
            continue;
        }

        let upper_line = line.to_uppercase();

        if upper_line.starts_with("TITLE ") {
            let value = parse_quoted_string(&line[6..]);
            if current_track.is_some() {
                if let Some(ref mut track) = current_track {
                    track.title = Some(value);
                }
            } else {
                cue.title = Some(value);
            }
        } else if upper_line.starts_with("PERFORMER ") {
            let value = parse_quoted_string(&line[10..]);
            if current_track.is_some() {
                if let Some(ref mut track) = current_track {
                    track.performer = Some(value);
                }
            } else {
                cue.performer = Some(value);
            }
        } else if upper_line.starts_with("FILE ") {
            if let Some(track) = current_track.take() {
                cue.tracks.push(track);
            }

            let rest = &line[5..];
            if let Some((file_path, file_type)) = parse_file_line(rest) {
                cue.files.push(CueFile {
                    path: file_path,
                    file_type,
                });
                current_file_index = Some(cue.files.len() - 1);
            }
        } else if upper_line.starts_with("TRACK ") {
            if let Some(track) = current_track.take() {
                cue.tracks.push(track);
            }

            let parts: Vec<&str> = line[6..].split_whitespace().collect();
            if let Some(num_str) = parts.first() {
                if let Ok(num) = num_str.parse::<u32>() {
                    current_track = Some(CueTrack {
                        number: num,
                        title: None,
                        performer: None,
                        start_time_ms: 0,
                        file_index: current_file_index.unwrap_or(0),
                    });
                }
            }
        } else if upper_line.starts_with("INDEX 01 ") || upper_line.starts_with("INDEX 1 ") {
            let timestamp_start = if upper_line.starts_with("INDEX 01 ") {
                9
            } else {
                8
            };
            let timestamp = &line[timestamp_start..];
            if let Some(ms) = parse_cue_timestamp(timestamp) {
                if let Some(ref mut track) = current_track {
                    track.start_time_ms = ms;
                }
            }
        }
    }

    if let Some(track) = current_track {
        cue.tracks.push(track);
    }

    cue.tracks.sort_by_key(|t| (t.file_index, t.start_time_ms));

    info!(
        "Parsed CUE sheet: {} tracks across {} file(s)",
        cue.tracks.len(),
        cue.files.len()
    );

    Ok(cue)
}

fn parse_file_line(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim();

    if rest.starts_with('"') {
        if let Some(end_quote) = rest[1..].find('"') {
            let file_path = rest[1..=end_quote].to_string();
            let remaining = rest[end_quote + 2..].trim();
            let file_type = remaining
                .split_whitespace()
                .next()
                .unwrap_or("WAVE")
                .to_string();
            return Some((file_path, file_type));
        }
    } else {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 2 {
            let file_path = parts[0].to_string();
            let file_type = parts[1].to_string();
            return Some((file_path, file_type));
        } else if !parts.is_empty() {
            return Some((parts[0].to_string(), "WAVE".to_string()));
        }
    }

    None
}

pub fn generate_chapters_from_cue(cue: &CueSheet, total_duration_ms: u64) -> Vec<Chapter> {
    let mut chapters = Vec::new();

    for (idx, track) in cue.tracks.iter().enumerate() {
        let title = track
            .title
            .clone()
            .unwrap_or_else(|| format!("Track {}", track.number));

        let end_time_ms = if idx + 1 < cue.tracks.len() {
            let next_track = &cue.tracks[idx + 1];
            if next_track.file_index == track.file_index {
                next_track.start_time_ms
            } else {
                total_duration_ms
            }
        } else {
            total_duration_ms
        };

        chapters.push(Chapter {
            index: idx,
            title,
            start_time_ms: track.start_time_ms,
            end_time_ms,
            auto_detected: true,
        });
    }

    info!("Generated {} chapters from CUE sheet", chapters.len());
    chapters
}

pub fn generate_chapters_from_multifile_cue(
    cue: &CueSheet,
    file_durations: &[u64],
) -> Vec<Chapter> {
    if cue.files.len() != file_durations.len() {
        warn!(
            "CUE file count ({}) doesn't match duration count ({})",
            cue.files.len(),
            file_durations.len()
        );
    }

    let mut cumulative_offsets: Vec<u64> = Vec::with_capacity(cue.files.len());
    let mut offset: u64 = 0;
    for &duration in file_durations {
        cumulative_offsets.push(offset);
        offset += duration;
    }
    let total_duration = offset;

    let mut chapters = Vec::new();

    for (idx, track) in cue.tracks.iter().enumerate() {
        let title = track
            .title
            .clone()
            .unwrap_or_else(|| format!("Track {}", track.number));

        let file_offset = cumulative_offsets
            .get(track.file_index)
            .copied()
            .unwrap_or(0);
        let absolute_start = file_offset + track.start_time_ms;

        let end_time_ms = if idx + 1 < cue.tracks.len() {
            let next_track = &cue.tracks[idx + 1];
            let next_file_offset = cumulative_offsets
                .get(next_track.file_index)
                .copied()
                .unwrap_or(0);
            next_file_offset + next_track.start_time_ms
        } else {
            total_duration
        };

        chapters.push(Chapter {
            index: idx,
            title,
            start_time_ms: absolute_start,
            end_time_ms,
            auto_detected: true,
        });
    }

    info!(
        "Generated {} chapters from multi-file CUE sheet",
        chapters.len()
    );
    chapters
}

pub fn find_cue_file(audio_path: &Utf8Path) -> Option<Utf8PathBuf> {
    let parent = audio_path.parent()?;
    let stem = audio_path.file_stem()?;

    let cue_path = parent.join(format!("{}.cue", stem));
    if cue_path.exists() {
        debug!("Found CUE file with matching name: {}", cue_path);
        return Some(cue_path);
    }

    if let Ok(entries) = std::fs::read_dir(parent.as_std_path()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("cue") {
                if let Ok(utf8_path) = Utf8PathBuf::try_from(path.clone()) {
                    if let Ok(cue) = parse_cue_file(&utf8_path) {
                        let audio_filename = audio_path.file_name().unwrap_or("");
                        for file in &cue.files {
                            if file.path == audio_filename || file.path.ends_with(audio_filename) {
                                debug!("Found CUE file referencing audio: {}", utf8_path);
                                return Some(utf8_path);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn detect_cue_for_files(audio_paths: &[Utf8PathBuf]) -> Option<CueDetectionResult> {
    if audio_paths.is_empty() {
        return None;
    }

    if let Some(first_path) = audio_paths.first() {
        if let Some(cue_path) = find_cue_file(first_path) {
            if let Ok(cue) = parse_cue_file(&cue_path) {
                let audio_files: Vec<String> = cue.files.iter().map(|f| f.path.clone()).collect();

                return Some(CueDetectionResult {
                    cue_path: cue_path.to_string(),
                    audio_files,
                    track_count: cue.tracks.len(),
                    is_multi_file: cue.is_multi_file(),
                    album_title: cue.title,
                    album_artist: cue.performer,
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cue_timestamp() {
        assert_eq!(parse_cue_timestamp("00:00:00"), Some(0));
        assert_eq!(parse_cue_timestamp("01:00:00"), Some(60000));
        assert_eq!(parse_cue_timestamp("00:01:00"), Some(1000));
        assert_eq!(parse_cue_timestamp("00:00:75"), Some(1000));
        assert_eq!(parse_cue_timestamp("05:32:15"), Some(332200));
        assert_eq!(parse_cue_timestamp("invalid"), None);
    }

    #[test]
    fn test_parse_quoted_string() {
        assert_eq!(parse_quoted_string("\"Hello World\""), "Hello World");
        assert_eq!(parse_quoted_string("Hello"), "Hello");
        assert_eq!(parse_quoted_string("  \"Trimmed\"  "), "Trimmed");
    }

    #[test]
    fn test_parse_simple_cue() {
        let content = r#"
TITLE "Test Album"
PERFORMER "Test Artist"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "Chapter One"
    PERFORMER "Author Name"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Chapter Two"
    INDEX 01 05:32:15
"#;

        let cue = parse_cue_content(content).unwrap();
        assert_eq!(cue.title, Some("Test Album".to_string()));
        assert_eq!(cue.performer, Some("Test Artist".to_string()));
        assert_eq!(cue.files.len(), 1);
        assert_eq!(cue.files[0].path, "album.flac");
        assert_eq!(cue.tracks.len(), 2);

        assert_eq!(cue.tracks[0].number, 1);
        assert_eq!(cue.tracks[0].title, Some("Chapter One".to_string()));
        assert_eq!(cue.tracks[0].start_time_ms, 0);

        assert_eq!(cue.tracks[1].number, 2);
        assert_eq!(cue.tracks[1].title, Some("Chapter Two".to_string()));
        assert_eq!(cue.tracks[1].start_time_ms, 332200);
    }

    #[test]
    fn test_parse_multifile_cue() {
        let content = r#"
TITLE "Multi-CD Album"
FILE "cd1.flac" WAVE
  TRACK 01 AUDIO
    TITLE "CD1 Track 1"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "CD1 Track 2"
    INDEX 01 05:00:00
FILE "cd2.flac" WAVE
  TRACK 03 AUDIO
    TITLE "CD2 Track 1"
    INDEX 01 00:00:00
"#;

        let cue = parse_cue_content(content).unwrap();
        assert_eq!(cue.files.len(), 2);
        assert_eq!(cue.tracks.len(), 3);

        assert_eq!(cue.tracks[0].file_index, 0);
        assert_eq!(cue.tracks[1].file_index, 0);
        assert_eq!(cue.tracks[2].file_index, 1);
    }

    #[test]
    fn test_generate_chapters_single_file() {
        let content = r#"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "Chapter One"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Chapter Two"
    INDEX 01 05:00:00
  TRACK 03 AUDIO
    TITLE "Chapter Three"
    INDEX 01 10:00:00
"#;

        let cue = parse_cue_content(content).unwrap();
        let chapters = generate_chapters_from_cue(&cue, 900000);

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Chapter One");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 300000);

        assert_eq!(chapters[1].title, "Chapter Two");
        assert_eq!(chapters[1].start_time_ms, 300000);
        assert_eq!(chapters[1].end_time_ms, 600000);

        assert_eq!(chapters[2].title, "Chapter Three");
        assert_eq!(chapters[2].start_time_ms, 600000);
        assert_eq!(chapters[2].end_time_ms, 900000);
    }

    #[test]
    fn test_generate_chapters_multifile() {
        let content = r#"
FILE "cd1.flac" WAVE
  TRACK 01 AUDIO
    TITLE "CD1 Track 1"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "CD1 Track 2"
    INDEX 01 05:00:00
FILE "cd2.flac" WAVE
  TRACK 03 AUDIO
    TITLE "CD2 Track 1"
    INDEX 01 00:00:00
"#;

        let cue = parse_cue_content(content).unwrap();
        let file_durations = vec![600000, 600000];
        let chapters = generate_chapters_from_multifile_cue(&cue, &file_durations);

        assert_eq!(chapters.len(), 3);

        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 300000);

        assert_eq!(chapters[1].start_time_ms, 300000);
        assert_eq!(chapters[1].end_time_ms, 600000);

        assert_eq!(chapters[2].start_time_ms, 600000);
        assert_eq!(chapters[2].end_time_ms, 1200000);
    }

    #[test]
    fn test_unicode_titles() {
        let content = r#"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "第一章 Introduction"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Глава 2 - Введение"
    INDEX 01 05:00:00
  TRACK 03 AUDIO
    TITLE "日本語タイトル"
    INDEX 01 10:00:00
"#;

        let cue = parse_cue_content(content).unwrap();
        assert_eq!(cue.tracks.len(), 3);
        assert_eq!(cue.tracks[0].title, Some("第一章 Introduction".to_string()));
        assert_eq!(cue.tracks[1].title, Some("Глава 2 - Введение".to_string()));
        assert_eq!(cue.tracks[2].title, Some("日本語タイトル".to_string()));
    }

    #[test]
    fn test_missing_titles() {
        let content = r#"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 05:00:00
"#;

        let cue = parse_cue_content(content).unwrap();
        let chapters = generate_chapters_from_cue(&cue, 600000);

        assert_eq!(chapters[0].title, "Track 1");
        assert_eq!(chapters[1].title, "Track 2");
    }
}
