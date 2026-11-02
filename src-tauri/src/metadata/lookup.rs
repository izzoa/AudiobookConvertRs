//! Online metadata lookup: the shapes that every source (Audible, Open
//! Library) is mapped into, and the value normalization they share.
//!
//! The frontend only ever sees these types, so the review and apply path is
//! the same whichever catalog a result came from.

use crate::error::{AppError, AppResult};
use crate::metadata::artwork::{resize_artwork_if_needed, DEFAULT_MAX_ARTWORK_DIMENSION};
use crate::metadata::chapters::validate_chapters;
use crate::models::{normalize_asin, AudioBook, Chapter, Metadata};
use crate::undo::EditCommand;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Which catalog a result came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookupSource {
    Audible,
    OpenLibrary,
}

/// One search result: enough to recognise the book in a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupCandidate {
    pub source: LookupSource,
    /// ASIN for Audible; work key for Open Library.
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Option<String>,
    pub series_index: Option<String>,
    pub runtime_ms: Option<u64>,
    pub year: Option<u16>,
    pub cover_url: Option<String>,
}

/// Found chapters, already fitted against the open book.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChapterOffer {
    /// The book is the found edition; timings are used as listed.
    Exact { chapters: Vec<Chapter> },
    /// The found edition with its branding intro and outro cut off.
    Shifted { chapters: Vec<Chapter>, shift_ms: u64 },
    /// Another edition with the same chapter count: titles only.
    TitlesOnly { chapters: Vec<Chapter> },
    Unavailable { reason: String },
}

/// The full record for one chosen result, ready for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupRecord {
    pub source: LookupSource,
    pub id: String,
    /// Normalized values; a field the source does not provide is `None`.
    pub fields: Metadata,
    pub cover_url: Option<String>,
    pub runtime_ms: Option<u64>,
    pub chapters: ChapterOffer,
}

impl LookupSource {
    pub fn label(self) -> &'static str {
        match self {
            LookupSource::Audible => "Audible",
            LookupSource::OpenLibrary => "Open Library",
        }
    }
}

/// The edit that applies a reviewed lookup to `book`, as one undo step.
///
/// `selected` carries only the fields the user ticked; every other field,
/// the cover and the chapters keep their current values unless given here.
/// `cover` is base64 image data, already sized for embedding.
pub fn apply_edit(
    book: &AudioBook,
    source: LookupSource,
    selected: &Metadata,
    cover: Option<String>,
    chapters: Option<Vec<Chapter>>,
) -> AppResult<EditCommand> {
    let mut metadata = overlay_selected(&book.output_metadata, selected);
    if let Some(cover) = cover {
        metadata.artwork_data = Some(cover);
        metadata.artwork_path = None;
    }

    let mut commands = vec![EditCommand::UpdateMetadata {
        audiobook_id: book.id.clone(),
        old_metadata: book.output_metadata.clone(),
        new_metadata: metadata,
    }];
    if let Some(chapters) = chapters {
        validate_chapters(&chapters, book.total_duration_ms)?;
        commands.push(EditCommand::ReplaceChapters {
            audiobook_id: book.id.clone(),
            old_chapters: book.chapters.clone(),
            new_chapters: chapters,
        });
    }

    Ok(EditCommand::Batch {
        description: format!("Apply details from {}", source.label()),
        commands,
    })
}

/// `current` with every field `selected` sets replaced. The cover is not a
/// selectable field here; `apply_edit` sets it separately.
fn overlay_selected(current: &Metadata, selected: &Metadata) -> Metadata {
    let mut merged = current.clone();
    macro_rules! take_selected {
        ($($field:ident),* $(,)?) => {
            $(if selected.$field.is_some() {
                merged.$field = selected.$field.clone();
            })*
        };
    }
    take_selected!(
        title, artist, album, album_artist, narrator, composer, year, genre, track_number,
        total_tracks, disc_number, total_discs, comments, description, publisher, copyright,
        series, series_index, language,
    );
    if let Some(asin) = selected.asin.as_deref().and_then(normalize_asin) {
        merged.asin = Some(asin);
    }
    merged
}

/// Downloads a found cover and returns it as base64, sized like any other
/// cover the app embeds.
pub async fn download_cover(url: &str) -> AppResult<String> {
    let fail = |reason: String| AppError::MetadataLookupError { provider: "cover".to_string(), reason };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(fail(format!("not a web address: {}", url)));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;
    let response = client.get(url).send().await.map_err(|e| fail(format!("download failed: {}", e)))?;
    if !response.status().is_success() {
        return Err(fail(format!("download failed: HTTP {}", response.status())));
    }
    let bytes = response.bytes().await.map_err(|e| fail(format!("download failed: {}", e)))?;
    if bytes.is_empty() {
        return Err(fail("the image was empty".to_string()));
    }

    let sized = resize_artwork_if_needed(&bytes, DEFAULT_MAX_ARTWORK_DIMENSION)?;
    Ok(STANDARD.encode(sized))
}

/// "Scott Brick, Euan Morton", or nothing when no names are given.
pub fn join_names<S: AsRef<str>>(names: &[S]) -> Option<String> {
    let names: Vec<&str> = names.iter().map(|n| n.as_ref().trim()).filter(|n| !n.is_empty()).collect();
    (!names.is_empty()).then(|| names.join(", "))
}

static SERIES_NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.\d+|\d+(?:\.\d+)?").unwrap());

/// The book's place in its series. Catalogs send "Book 3" or
/// "2, Dramatized Adaptation"; the first number is the position, decimals
/// kept ("1.5"). Text with no number is kept as given.
pub fn series_position(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    Some(SERIES_NUMBER.find(raw).map_or(raw, |m| m.as_str()).to_string())
}

static BLOCK_END: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<\s*(?:/p|/div|/li|br\s*/?)\s*>").unwrap());
static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());
static NUMERIC_ENTITY: Lazy<Regex> = Lazy::new(|| Regex::new(r"&#(x[0-9a-fA-F]+|\d+);").unwrap());

/// A catalog summary as plain text: markup removed, one paragraph per block,
/// paragraphs separated by a blank line.
pub fn html_to_text(html: &str) -> Option<String> {
    let text = BLOCK_END.replace_all(html, "\n");
    let text = TAG.replace_all(&text, "");
    let text = NUMERIC_ENTITY.replace_all(&text, |caps: &regex::Captures| {
        let code = &caps[1];
        let value = match code.strip_prefix('x') {
            Some(hex) => u32::from_str_radix(hex, 16).ok(),
            None => code.parse().ok(),
        };
        value.and_then(char::from_u32).map(String::from).unwrap_or_default()
    });
    // &amp; last, so "&amp;lt;" stays the literal text "&lt;".
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");

    let paragraphs: Vec<String> = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect();
    (!paragraphs.is_empty()).then(|| paragraphs.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_joined_with_commas() {
        assert_eq!(join_names(&["Scott Brick", " Euan Morton "]).as_deref(), Some("Scott Brick, Euan Morton"));
        assert_eq!(join_names::<&str>(&[]), None);
        assert_eq!(join_names(&["", "  "]), None);
    }

    #[test]
    fn series_position_takes_the_first_number() {
        assert_eq!(series_position("2, Dramatized Adaptation").as_deref(), Some("2"));
        assert_eq!(series_position("1.5").as_deref(), Some("1.5"));
        assert_eq!(series_position("Book 3").as_deref(), Some("3"));
        assert_eq!(series_position(".5").as_deref(), Some(".5"));
        assert_eq!(series_position("Prequel").as_deref(), Some("Prequel"));
        assert_eq!(series_position("  "), None);
    }

    #[test]
    fn summary_html_becomes_plain_paragraphs() {
        assert_eq!(
            html_to_text("<p>First.</p><p>Second <i>part</i>.</p>").as_deref(),
            Some("First.\n\nSecond part.")
        );
        assert_eq!(
            html_to_text("Dune &amp; Dune&nbsp;Messiah<br/>Herbert&#39;s &#x2014; classic").as_deref(),
            Some("Dune & Dune Messiah\n\nHerbert's \u{2014} classic")
        );
        assert_eq!(html_to_text("<p> </p>"), None);
    }

    fn book_with_details() -> AudioBook {
        let mut book = AudioBook::new("Dune Messiah".to_string());
        book.total_duration_ms = 3_000_000;
        book.chapters = vec![
            Chapter::new(0, "Track 01".to_string(), 0, 1_500_000),
            Chapter::new(1, "Track 02".to_string(), 1_500_000, 3_000_000),
        ];
        book.output_metadata = Metadata {
            title: Some("Dune Messiah".to_string()),
            narrator: Some("Scott Brick".to_string()),
            series: Some("Dune".to_string()),
            series_index: Some("2".to_string()),
            track_number: Some(1),
            artwork_data: Some("b2xkLWNvdmVy".to_string()),
            ..Default::default()
        };
        book
    }

    #[test]
    fn applying_keeps_every_field_that_was_not_selected() {
        let mut book = book_with_details();
        // An Open Library result with only title and author ticked.
        let selected = Metadata {
            title: Some("Dune Messiah (Revised)".to_string()),
            artist: Some("Frank Herbert".to_string()),
            ..Default::default()
        };
        let edit = apply_edit(&book, LookupSource::OpenLibrary, &selected, None, None).unwrap();
        crate::undo::UndoManager::new().execute(&mut book, edit).unwrap();

        let m = &book.output_metadata;
        assert_eq!(m.title.as_deref(), Some("Dune Messiah (Revised)"));
        assert_eq!(m.artist.as_deref(), Some("Frank Herbert"));
        assert_eq!(m.narrator.as_deref(), Some("Scott Brick"));
        assert_eq!((m.series.as_deref(), m.series_index.as_deref()), (Some("Dune"), Some("2")));
        assert_eq!(m.track_number, Some(1));
        assert_eq!(m.artwork_data.as_deref(), Some("b2xkLWNvdmVy"));
        assert_eq!(book.chapters[0].title, "Track 01");
    }

    #[test]
    fn one_undo_restores_fields_cover_and_chapters_together() {
        let mut book = book_with_details();
        let before = book.clone();
        let selected = Metadata {
            narrator: Some("Scott Brick, Euan Morton".to_string()),
            series_index: Some("2.5".to_string()),
            asin: Some(" b002v8h660".to_string()),
            ..Default::default()
        };
        let chapters = vec![
            Chapter::new(0, "Opening Credits".to_string(), 0, 1_500_000),
            Chapter::new(1, "Book One".to_string(), 1_500_000, 3_000_000),
        ];
        let edit = apply_edit(&book, LookupSource::Audible, &selected, Some("bmV3LWNvdmVy".to_string()), Some(chapters)).unwrap();
        assert_eq!(edit.description(), "Apply details from Audible");

        let mut undo = crate::undo::UndoManager::new();
        undo.execute(&mut book, edit).unwrap();
        assert_eq!(book.output_metadata.narrator.as_deref(), Some("Scott Brick, Euan Morton"));
        assert_eq!(book.output_metadata.asin.as_deref(), Some("B002V8H660"));
        assert_eq!(book.output_metadata.artwork_data.as_deref(), Some("bmV3LWNvdmVy"));
        assert_eq!(book.chapters[0].title, "Opening Credits");

        undo.undo(&mut book).unwrap();
        assert!(!undo.can_undo());
        assert_eq!(book.output_metadata.narrator, before.output_metadata.narrator);
        assert_eq!(book.output_metadata.series_index, before.output_metadata.series_index);
        assert_eq!(book.output_metadata.asin, None);
        assert_eq!(book.output_metadata.artwork_data, before.output_metadata.artwork_data);
        assert_eq!(book.chapters[0].title, "Track 01");
    }

    #[test]
    fn chapters_that_do_not_fit_the_book_are_refused() {
        let book = book_with_details();
        let too_long = vec![Chapter::new(0, "All".to_string(), 0, 9_000_000)];
        assert!(apply_edit(&book, LookupSource::Audible, &Metadata::default(), None, Some(too_long)).is_err());
    }

    #[tokio::test]
    async fn a_cover_address_that_is_not_web_is_refused() {
        assert!(download_cover("file:///etc/passwd").await.is_err());
    }

    #[test]
    fn records_cross_the_ipc_boundary_unchanged() {
        let record = LookupRecord {
            source: LookupSource::Audible,
            id: "B002V8H660".to_string(),
            fields: Metadata { series_index: Some("2".to_string()), ..Default::default() },
            cover_url: None,
            runtime_ms: Some(32_269_161),
            chapters: ChapterOffer::Shifted {
                chapters: vec![Chapter::new(0, "Opening Credits".to_string(), 0, 37_850)],
                shift_ms: 3_970,
            },
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["source"], "audible");
        assert_eq!(json["chapters"]["kind"], "shifted");

        let back: LookupRecord = serde_json::from_value(json).unwrap();
        assert_eq!(back.fields.series_index.as_deref(), Some("2"));
        assert!(matches!(back.chapters, ChapterOffer::Shifted { shift_ms: 3_970, ref chapters } if chapters.len() == 1));
    }
}
