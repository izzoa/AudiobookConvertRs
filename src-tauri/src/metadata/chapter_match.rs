//! Fits a catalog's chapter list to the open book.
//!
//! Catalog timings belong to one edition, so they are only offered when the
//! book's length says it is that edition. Otherwise the titles can still be
//! used when the chapter counts agree.

use crate::metadata::chapters::validate_chapters;
use crate::metadata::lookup::ChapterOffer;
use crate::models::Chapter;

/// How far the book's length may be from the edition's and still count as it.
pub const RUNTIME_TOLERANCE_MS: u64 = 2_000;

#[derive(Debug, Clone)]
pub struct FoundChapter {
    pub title: String,
    pub start_ms: u64,
}

/// A catalog's chapter list for one edition.
#[derive(Debug, Clone)]
pub struct FoundChapters {
    pub chapters: Vec<FoundChapter>,
    pub runtime_ms: u64,
    /// Branding before the first chapter ("This is Audible") and after the
    /// last; some converters cut both off.
    pub intro_ms: u64,
    pub outro_ms: u64,
}

pub fn offer_chapters(found: &FoundChapters, book_duration_ms: u64, current: &[Chapter]) -> ChapterOffer {
    if found.chapters.is_empty() {
        return ChapterOffer::Unavailable { reason: "This record has no chapter list.".to_string() };
    }
    let near = |a: u64, b: u64| a.abs_diff(b) <= RUNTIME_TOLERANCE_MS;

    if near(book_duration_ms, found.runtime_ms) {
        if let Some(chapters) = fit(found, 0, book_duration_ms) {
            return ChapterOffer::Exact { chapters };
        }
    }

    let branding_ms = found.intro_ms + found.outro_ms;
    if branding_ms > 0 && near(book_duration_ms, found.runtime_ms.saturating_sub(branding_ms)) {
        if let Some(chapters) = fit(found, found.intro_ms, book_duration_ms) {
            return ChapterOffer::Shifted { chapters, shift_ms: found.intro_ms };
        }
    }

    if !current.is_empty() && current.len() == found.chapters.len() {
        let chapters = current
            .iter()
            .zip(&found.chapters)
            .map(|(chapter, found)| Chapter {
                title: if found.title.trim().is_empty() { chapter.title.clone() } else { found.title.clone() },
                auto_detected: false,
                ..chapter.clone()
            })
            .collect();
        return ChapterOffer::TitlesOnly { chapters };
    }

    ChapterOffer::Unavailable {
        reason: format!(
            "The found edition runs {} with {} chapters; this book runs {} with {}.",
            hours_minutes(found.runtime_ms),
            found.chapters.len(),
            hours_minutes(book_duration_ms),
            current.len()
        ),
    }
}

/// Starts moved earlier by `shift_ms`, each chapter ending where the next one
/// begins and the last at the book's end. `None` when that is not a valid
/// chapter list for the book.
fn fit(found: &FoundChapters, shift_ms: u64, book_duration_ms: u64) -> Option<Vec<Chapter>> {
    let starts: Vec<u64> = found
        .chapters
        .iter()
        .enumerate()
        .map(|(i, c)| if i == 0 { 0 } else { c.start_ms.saturating_sub(shift_ms) })
        .collect();
    let chapters: Vec<Chapter> = found
        .chapters
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let end = starts.get(i + 1).copied().unwrap_or(book_duration_ms);
            Chapter::new(i, c.title.clone(), starts[i], end)
        })
        .collect();
    validate_chapters(&chapters, book_duration_ms).ok().map(|_| chapters)
}

/// "8h 57m", or "41m" under an hour.
fn hours_minutes(ms: u64) -> String {
    let minutes = ms / 60_000;
    match minutes / 60 {
        0 => format!("{}m", minutes),
        hours => format!("{}h {}m", hours, minutes % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first chapters of Dune Messiah (Audible B002V8H660) as Audnexus lists them.
    fn dune_messiah() -> FoundChapters {
        FoundChapters {
            chapters: vec![
                FoundChapter { title: "Opening Credits".to_string(), start_ms: 0 },
                FoundChapter { title: "Excerpts from the Death Cell Interview".to_string(), start_ms: 41_820 },
                FoundChapter { title: "Analysis of History".to_string(), start_ms: 602_001 },
            ],
            runtime_ms: 32_269_161,
            intro_ms: 3_970,
            outro_ms: 4_945,
        }
    }

    fn spans(chapters: &[Chapter]) -> Vec<(u64, u64)> {
        chapters.iter().map(|c| (c.start_time_ms, c.end_time_ms)).collect()
    }

    #[test]
    fn same_edition_uses_listed_timings() {
        let ChapterOffer::Exact { chapters } = offer_chapters(&dune_messiah(), 32_269_161, &[]) else {
            panic!("expected exact timings");
        };
        assert_eq!(spans(&chapters), vec![(0, 41_820), (41_820, 602_001), (602_001, 32_269_161)]);
        assert_eq!(chapters[1].title, "Excerpts from the Death Cell Interview");
    }

    #[test]
    fn a_book_slightly_shorter_than_the_edition_still_matches() {
        let ChapterOffer::Exact { chapters } = offer_chapters(&dune_messiah(), 32_268_000, &[]) else {
            panic!("expected exact timings");
        };
        assert_eq!(chapters.last().unwrap().end_time_ms, 32_268_000);
    }

    #[test]
    fn stripped_branding_shifts_every_start_by_the_intro() {
        let offer = offer_chapters(&dune_messiah(), 32_260_246, &[]);
        let ChapterOffer::Shifted { chapters, shift_ms } = offer else {
            panic!("expected shifted timings");
        };
        assert_eq!(shift_ms, 3_970);
        assert_eq!(spans(&chapters), vec![(0, 37_850), (37_850, 598_031), (598_031, 32_260_246)]);
    }

    #[test]
    fn same_count_from_another_edition_offers_titles_only() {
        let current: Vec<Chapter> = (0..3)
            .map(|i| Chapter::new(i, format!("Track {:02}", i + 1), i as u64 * 1_000_000, (i as u64 + 1) * 1_000_000))
            .collect();
        let ChapterOffer::TitlesOnly { chapters } = offer_chapters(&dune_messiah(), 33_120_000, &current) else {
            panic!("expected titles only");
        };
        assert_eq!(chapters[2].title, "Analysis of History");
        assert_eq!(spans(&chapters), spans(&current));
    }

    #[test]
    fn different_edition_and_count_says_why() {
        let mut found = dune_messiah();
        found.chapters = (0..28)
            .map(|i| FoundChapter { title: format!("Chapter {}", i + 1), start_ms: i * 1_152_000 })
            .collect();
        let current: Vec<Chapter> = (0..6).map(|i| Chapter::new(i, format!("Part {}", i + 1), i as u64 * 5_000_000, (i as u64 + 1) * 5_000_000)).collect();

        let ChapterOffer::Unavailable { reason } = offer_chapters(&found, 33_120_000, &current) else {
            panic!("expected unavailable");
        };
        assert_eq!(reason, "The found edition runs 8h 57m with 28 chapters; this book runs 9h 12m with 6.");
    }

    #[test]
    fn timings_that_do_not_fit_the_book_are_not_offered() {
        // Within tolerance, but the last chapter would start after the book ends.
        let mut found = dune_messiah();
        found.chapters.push(FoundChapter { title: "Closing Credits".to_string(), start_ms: 32_268_500 });
        assert!(matches!(
            offer_chapters(&found, 32_268_000, &[]),
            ChapterOffer::Unavailable { .. }
        ));
    }

    #[test]
    fn no_chapter_list_is_unavailable() {
        let mut found = dune_messiah();
        found.chapters.clear();
        assert!(matches!(offer_chapters(&found, 32_269_161, &[]), ChapterOffer::Unavailable { .. }));
    }
}
