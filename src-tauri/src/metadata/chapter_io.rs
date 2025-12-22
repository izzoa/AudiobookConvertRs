//! Chapter import/export module
//!
//! Provides parsing and rendering for various chapter file formats:
//! - Audacity label track format (.txt)
//! - FFmpeg metadata format (.ffmeta)
//! - CSV format (.csv)

use crate::error::{AppError, AppResult};
use crate::models::{Chapter, parse_time_string, format_duration_ms};
use tracing::{debug, info, warn};

/// Supported chapter file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChapterFormat {
    /// Audacity label track: start_sec<TAB>end_sec<TAB>title
    Audacity,
    /// FFmpeg metadata: ;FFMETADATA1 with [CHAPTER] blocks
    FFmetadata,
    /// CSV: title,start,end
    Csv,
}

impl ChapterFormat {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "txt" => Some(ChapterFormat::Audacity), // Default txt to Audacity
            "ffmeta" | "ffmetadata" => Some(ChapterFormat::FFmetadata),
            "csv" => Some(ChapterFormat::Csv),
            _ => None,
        }
    }

    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ChapterFormat::Audacity => "txt",
            ChapterFormat::FFmetadata => "ffmeta",
            ChapterFormat::Csv => "csv",
        }
    }
}

impl std::fmt::Display for ChapterFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChapterFormat::Audacity => write!(f, "Audacity Label"),
            ChapterFormat::FFmetadata => write!(f, "FFmpeg Metadata"),
            ChapterFormat::Csv => write!(f, "CSV"),
        }
    }
}

/// Parse chapters from a string in the specified format
pub fn parse_chapters(content: &str, format: ChapterFormat) -> AppResult<Vec<Chapter>> {
    debug!("Parsing chapters in {} format", format);

    let chapters = match format {
        ChapterFormat::Audacity => parse_audacity_labels(content)?,
        ChapterFormat::FFmetadata => parse_ffmetadata(content)?,
        ChapterFormat::Csv => parse_csv(content)?,
    };

    info!("Parsed {} chapters from {} format", chapters.len(), format);
    Ok(chapters)
}

/// Render chapters to a string in the specified format
pub fn render_chapters(chapters: &[Chapter], format: ChapterFormat) -> String {
    debug!("Rendering {} chapters to {} format", chapters.len(), format);

    match format {
        ChapterFormat::Audacity => render_audacity_labels(chapters),
        ChapterFormat::FFmetadata => render_ffmetadata(chapters),
        ChapterFormat::Csv => render_csv(chapters),
    }
}

/// Parse Audacity label track format
/// Format: start_sec<TAB>end_sec<TAB>title
fn parse_audacity_labels(content: &str) -> AppResult<Vec<Chapter>> {
    let mut chapters = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            warn!("Skipping invalid line {}: not enough fields", line_num + 1);
            continue;
        }

        let start_sec: f64 = parts[0].trim().parse()
            .map_err(|e| AppError::ValidationError(format!("Invalid start time on line {}: {}", line_num + 1, e)))?;
        let end_sec: f64 = parts[1].trim().parse()
            .map_err(|e| AppError::ValidationError(format!("Invalid end time on line {}: {}", line_num + 1, e)))?;
        let title = parts[2..].join("\t").trim().to_string();

        if title.is_empty() {
            warn!("Skipping line {} with empty title", line_num + 1);
            continue;
        }

        let start_ms = (start_sec * 1000.0) as u64;
        let end_ms = (end_sec * 1000.0) as u64;

        if end_ms <= start_ms {
            return Err(AppError::ValidationError(format!(
                "Invalid chapter on line {}: end time must be after start time",
                line_num + 1
            )));
        }

        chapters.push(Chapter {
            index: chapters.len(),
            title,
            start_time_ms: start_ms,
            end_time_ms: end_ms,
            auto_detected: false,
        });
    }

    Ok(chapters)
}

/// Parse FFmpeg metadata format
/// Format: ;FFMETADATA1 followed by [CHAPTER] blocks
fn parse_ffmetadata(content: &str) -> AppResult<Vec<Chapter>> {
    let mut chapters = Vec::new();
    let mut current_chapter: Option<(u64, u64, String)> = None;

    for line in content.lines() {
        let line = line.trim();

        if line == "[CHAPTER]" {
            // Save previous chapter if exists
            if let Some((start, end, title)) = current_chapter.take() {
                chapters.push(Chapter {
                    index: chapters.len(),
                    title,
                    start_time_ms: start,
                    end_time_ms: end,
                    auto_detected: false,
                });
            }
            current_chapter = Some((0, 0, String::new()));
        } else if line.starts_with("START=") {
            if let Some((ref mut start, _, _)) = current_chapter {
                *start = line[6..].trim().parse()
                    .map_err(|e| AppError::ValidationError(format!("Invalid START value: {}", e)))?;
            }
        } else if line.starts_with("END=") {
            if let Some((_, ref mut end, _)) = current_chapter {
                *end = line[4..].trim().parse()
                    .map_err(|e| AppError::ValidationError(format!("Invalid END value: {}", e)))?;
            }
        } else if line.starts_with("title=") {
            if let Some((_, _, ref mut title)) = current_chapter {
                *title = unescape_ffmetadata(&line[6..]);
            }
        }
    }

    // Save last chapter
    if let Some((start, end, title)) = current_chapter {
        if !title.is_empty() && end > start {
            chapters.push(Chapter {
                index: chapters.len(),
                title,
                start_time_ms: start,
                end_time_ms: end,
                auto_detected: false,
            });
        }
    }

    Ok(chapters)
}

/// Parse CSV format
/// Format: title,start,end (with or without header)
fn parse_csv(content: &str) -> AppResult<Vec<Chapter>> {
    let mut chapters = Vec::new();
    let mut lines = content.lines();

    // Skip header if present (check if first line contains "title" or "start")
    if let Some(first_line) = lines.next() {
        let lower = first_line.to_lowercase();
        if !lower.contains("title") && !lower.contains("start") {
            // Not a header, parse it
            if let Some(chapter) = parse_csv_line(first_line, 0)? {
                chapters.push(chapter);
            }
        }
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(chapter) = parse_csv_line(line, chapters.len())? {
            chapters.push(chapter);
        }
    }

    Ok(chapters)
}

/// Parse a single CSV line
fn parse_csv_line(line: &str, index: usize) -> AppResult<Option<Chapter>> {
    // Simple CSV parsing (handles basic quoted fields)
    let parts: Vec<String> = parse_csv_fields(line);

    if parts.len() < 3 {
        warn!("Skipping CSV line with insufficient fields: {}", line);
        return Ok(None);
    }

    let title = parts[0].trim().to_string();
    let start_str = parts[1].trim();
    let end_str = parts[2].trim();

    if title.is_empty() {
        return Ok(None);
    }

    // Try to parse times - accept both HH:MM:SS and seconds
    let start_ms = parse_time_flexible(start_str)?;
    let end_ms = parse_time_flexible(end_str)?;

    if end_ms <= start_ms {
        return Err(AppError::ValidationError(format!(
            "Invalid chapter '{}': end time must be after start time",
            title
        )));
    }

    Ok(Some(Chapter {
        index,
        title,
        start_time_ms: start_ms,
        end_time_ms: end_ms,
        auto_detected: false,
    }))
}

/// Parse time flexibly - accept HH:MM:SS(.ms) or seconds
fn parse_time_flexible(s: &str) -> AppResult<u64> {
    // Try parsing as time string first (HH:MM:SS)
    if s.contains(':') {
        if let Some(ms) = parse_time_string(s) {
            return Ok(ms);
        }
    }

    // Try parsing as seconds (float)
    s.parse::<f64>()
        .map(|secs| (secs * 1000.0) as u64)
        .map_err(|e| AppError::ValidationError(format!("Invalid time value '{}': {}", s, e)))
}

/// Simple CSV field parser (handles quoted fields with commas)
fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    if !current_field.is_empty() {
        fields.push(current_field.trim().to_string());
    }

    fields
}

/// Unescape FFmetadata special characters
fn unescape_ffmetadata(s: &str) -> String {
    s.replace("\\=", "=")
        .replace("\\;", ";")
        .replace("\\#", "#")
        .replace("\\\n", "\n")
        .replace("\\\\", "\\")
}

/// Escape FFmetadata special characters
fn escape_ffmetadata(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('=', "\\=")
        .replace(';', "\\;")
        .replace('#', "\\#")
        .replace('\n', "\\\n")
}

/// Render chapters as Audacity label track format
fn render_audacity_labels(chapters: &[Chapter]) -> String {
    let mut output = String::new();

    for chapter in chapters {
        let start_sec = chapter.start_time_ms as f64 / 1000.0;
        let end_sec = chapter.end_time_ms as f64 / 1000.0;
        output.push_str(&format!("{:.6}\t{:.6}\t{}\n", start_sec, end_sec, chapter.title));
    }

    output
}

/// Render chapters as FFmpeg metadata format
fn render_ffmetadata(chapters: &[Chapter]) -> String {
    let mut output = String::from(";FFMETADATA1\n");

    for chapter in chapters {
        output.push_str("\n[CHAPTER]\n");
        output.push_str("TIMEBASE=1/1000\n");
        output.push_str(&format!("START={}\n", chapter.start_time_ms));
        output.push_str(&format!("END={}\n", chapter.end_time_ms));
        output.push_str(&format!("title={}\n", escape_ffmetadata(&chapter.title)));
    }

    output
}

/// Render chapters as CSV format
fn render_csv(chapters: &[Chapter]) -> String {
    let mut output = String::from("title,start,end\n");

    for chapter in chapters {
        let start = format_duration_ms(chapter.start_time_ms);
        let end = format_duration_ms(chapter.end_time_ms);
        
        // Escape title if it contains commas or quotes
        let title = if chapter.title.contains(',') || chapter.title.contains('"') {
            format!("\"{}\"", chapter.title.replace('"', "\"\""))
        } else {
            chapter.title.clone()
        };

        output.push_str(&format!("{},{},{}\n", title, start, end));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_audacity_labels() {
        let content = "0.000000\t60.000000\tChapter 1\n120.500000\t180.750000\tChapter 2\n";
        let chapters = parse_audacity_labels(content).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 60000);
        assert_eq!(chapters[1].title, "Chapter 2");
        assert_eq!(chapters[1].start_time_ms, 120500);
        assert_eq!(chapters[1].end_time_ms, 180750);
    }

    #[test]
    fn test_parse_ffmetadata() {
        let content = ";FFMETADATA1\n\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=60000\ntitle=Introduction\n\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=60000\nEND=120000\ntitle=Chapter One\n";
        let chapters = parse_ffmetadata(content).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Introduction");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 60000);
        assert_eq!(chapters[1].title, "Chapter One");
    }

    #[test]
    fn test_parse_csv() {
        let content = "title,start,end\nChapter 1,00:00,01:00\nChapter 2,01:00,02:00\n";
        let chapters = parse_csv(content).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[0].start_time_ms, 0);
        assert_eq!(chapters[0].end_time_ms, 60000);
    }

    #[test]
    fn test_parse_csv_seconds() {
        let content = "Chapter 1,0.0,60.5\nChapter 2,60.5,120.0\n";
        let chapters = parse_csv(content).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].end_time_ms, 60500);
        assert_eq!(chapters[1].start_time_ms, 60500);
    }

    #[test]
    fn test_render_audacity() {
        let chapters = vec![
            Chapter::new(0, "Test".to_string(), 0, 60000),
            Chapter::new(1, "Test 2".to_string(), 60000, 120000),
        ];

        let output = render_audacity_labels(&chapters);
        assert!(output.contains("0.000000\t60.000000\tTest"));
        assert!(output.contains("60.000000\t120.000000\tTest 2"));
    }

    #[test]
    fn test_render_csv() {
        let chapters = vec![
            Chapter::new(0, "Test, with comma".to_string(), 0, 60000),
        ];

        let output = render_csv(&chapters);
        assert!(output.contains("\"Test, with comma\""));
    }

    #[test]
    fn test_escape_unescape_ffmetadata() {
        let original = "Test=value;comment#note\nnewline";
        let escaped = escape_ffmetadata(original);
        let unescaped = unescape_ffmetadata(&escaped);
        assert_eq!(original, unescaped);
    }
}

