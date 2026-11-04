//! Fits a catalog's chapter list to the open book.
//!
//! Catalog timings belong to one edition, so they are used as listed only
//! when the book's length says it is that edition. The titles can still be
//! used when the chapter counts agree, and when the book is nearly as long as
//! the edition the found chapters are placed on the book's own track starts,
//! pauses or estimates.

use crate::metadata::chapters::validate_chapters;
use crate::metadata::lookup::{ChapterOffer, FittedPlacement, PlacementCounts, PlacementKind};
use crate::models::{AudioFile, Chapter, ChapterPlacement};

/// How far the book's length may be from the edition's and still count as it.
pub const RUNTIME_TOLERANCE_MS: u64 = 2_000;

/// How far, in percent of the edition's length without branding, the book may
/// be from it for the found chapters to be fitted.
pub const FIT_LENGTH_TOLERANCE_PERCENT: u64 = 2;

/// How far a fitted chapter may move from its estimate to a track start or a pause.
pub const SNAP_WINDOW_MS: u64 = 30_000;

/// Looks for a pause near an estimated start (book time) and returns where the
/// chapter should start in it, or `None` when there is none.
pub type PauseFinder<'a> = dyn FnMut(u64) -> Option<u64> + 'a;

/// Listens near an estimated start (book time) for a chapter's spoken title
/// and returns where the chapter should start, or `None` when it is not heard.
pub type HeardFinder<'a> = dyn FnMut(&str, u64) -> Option<u64> + 'a;

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

/// The chapters row for a found edition. `track_starts` are where the book's
/// files begin on its timeline (the first is 0). For fitted chapters with no
/// track start nearby, `find_heard` is asked first, then `find_pause`.
pub fn offer_chapters(
    found: &FoundChapters,
    book_duration_ms: u64,
    current: &[Chapter],
    track_starts: &[u64],
    find_heard: &mut HeardFinder,
    find_pause: &mut PauseFinder,
) -> ChapterOffer {
    if found.chapters.is_empty() {
        return ChapterOffer::Unavailable {
            reason: "This record has no chapter list.".to_string(),
        };
    }
    let near = |a: u64, b: u64| a.abs_diff(b) <= RUNTIME_TOLERANCE_MS;

    if near(book_duration_ms, found.runtime_ms) {
        if let Some(chapters) = fit(found, 0, book_duration_ms) {
            return ChapterOffer::Exact { chapters };
        }
    }

    let branding_ms = found.intro_ms + found.outro_ms;
    if branding_ms > 0
        && near(
            book_duration_ms,
            found.runtime_ms.saturating_sub(branding_ms),
        )
    {
        if let Some(chapters) = fit(found, found.intro_ms, book_duration_ms) {
            return ChapterOffer::Shifted {
                chapters,
                shift_ms: found.intro_ms,
            };
        }
    }

    if !current.is_empty() && current.len() == found.chapters.len() {
        let chapters = current
            .iter()
            .zip(&found.chapters)
            .map(|(chapter, found)| Chapter {
                title: if found.title.trim().is_empty() {
                    chapter.title.clone()
                } else {
                    found.title.clone()
                },
                auto_detected: false,
                ..chapter.clone()
            })
            .collect();
        return ChapterOffer::TitlesOnly { chapters };
    }

    if let Some(offer) = fit_to_book(
        found,
        book_duration_ms,
        track_starts,
        find_heard,
        find_pause,
    ) {
        return offer;
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
        .map(|(i, c)| {
            if i == 0 {
                0
            } else {
                c.start_ms.saturating_sub(shift_ms)
            }
        })
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
    validate_chapters(&chapters, book_duration_ms)
        .ok()
        .map(|_| chapters)
}

/// The found chapters placed on a book of nearly the same length, or `None`
/// when the lengths are too far apart or the result is not a valid list.
///
/// Each start is estimated by scaling the found start (measured from the end
/// of the intro) to the book, moved by how far the last chapter placed on a
/// track start or at a pause landed from its own scaled start. A chapter then
/// goes to the nearest track start within [`SNAP_WINDOW_MS`], else to where its
/// title is heard, else to a pause, else stays at the estimate. Starts only ever move forward.
fn fit_to_book(
    found: &FoundChapters,
    book_duration_ms: u64,
    track_starts: &[u64],
    find_heard: &mut HeardFinder,
    find_pause: &mut PauseFinder,
) -> Option<ChapterOffer> {
    let reference_ms = found
        .runtime_ms
        .saturating_sub(found.intro_ms + found.outro_ms);
    if reference_ms == 0
        || book_duration_ms.abs_diff(reference_ms) * 100
            > reference_ms * FIT_LENGTH_TOLERANCE_PERCENT
    {
        return None;
    }
    let scale = book_duration_ms as f64 / reference_ms as f64;

    let mut starts = Vec::with_capacity(found.chapters.len());
    let mut placements = Vec::with_capacity(found.chapters.len());
    let mut counts = PlacementCounts::default();
    let mut offset_ms: i64 = 0;
    let mut previous: Option<u64> = None;

    for (index, chapter) in found.chapters.iter().enumerate() {
        let (start, kind, moved_ms) = match previous {
            // The book's first track starts the first chapter.
            None => (0, PlacementKind::Track, 0),
            Some(previous) => {
                let scaled =
                    (chapter.start_ms.saturating_sub(found.intro_ms) as f64 * scale).round() as i64;
                let estimate = scaled + offset_ms;
                if estimate <= previous as i64 || estimate >= book_duration_ms as i64 {
                    return None;
                }
                let estimate = estimate as u64;
                let usable = |t: u64| t > previous && t < book_duration_ms;
                let track = track_starts
                    .iter()
                    .copied()
                    .filter(|&t| usable(t) && t.abs_diff(estimate) <= SNAP_WINDOW_MS)
                    .min_by_key(|&t| t.abs_diff(estimate));
                let (start, kind) = match track {
                    Some(t) => (t, PlacementKind::Track),
                    None => match find_heard(&chapter.title, estimate).filter(|&h| usable(h)) {
                        Some(h) => (h, PlacementKind::Heard),
                        None => match find_pause(estimate).filter(|&p| usable(p)) {
                            Some(p) => (p, PlacementKind::Pause),
                            None => (estimate, PlacementKind::Estimate),
                        },
                    },
                };
                if kind != PlacementKind::Estimate {
                    offset_ms = start as i64 - scaled;
                }
                (start, kind, start as i64 - estimate as i64)
            }
        };
        match kind {
            PlacementKind::Track => counts.track += 1,
            PlacementKind::Heard => counts.heard += 1,
            PlacementKind::Pause => counts.pause += 1,
            PlacementKind::Estimate => counts.estimate += 1,
        }
        placements.push(FittedPlacement {
            index,
            kind,
            moved_ms,
        });
        starts.push(start);
        previous = Some(start);
    }

    let chapters: Vec<Chapter> = found
        .chapters
        .iter()
        .zip(&placements)
        .enumerate()
        .map(|(i, (c, placement))| {
            let end = starts.get(i + 1).copied().unwrap_or(book_duration_ms);
            let mut chapter = Chapter::new(i, c.title.clone(), starts[i], end);
            chapter.placement = match placement.kind {
                PlacementKind::Track | PlacementKind::Heard => None,
                PlacementKind::Pause => Some(ChapterPlacement::Pause),
                PlacementKind::Estimate => Some(ChapterPlacement::Estimate),
            };
            chapter
        })
        .collect();
    validate_chapters(&chapters, book_duration_ms).ok()?;
    Some(ChapterOffer::Fitted {
        chapters,
        placements,
        counts,
    })
}

/// Where each of the book's files begins on its timeline.
pub fn track_starts(files: &[AudioFile]) -> Vec<u64> {
    files
        .iter()
        .scan(0u64, |start, file| {
            let this = *start;
            *start += file.duration_ms;
            Some(this)
        })
        .collect()
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

    /// A book of one file, with no pauses to be found.
    fn offer(found: &FoundChapters, book_duration_ms: u64, current: &[Chapter]) -> ChapterOffer {
        offer_chapters(
            found,
            book_duration_ms,
            current,
            &[0],
            &mut |_, _| None,
            &mut |_| None,
        )
    }

    /// `n` evenly spaced chapters over `runtime_ms`, with no branding.
    fn even_edition(n: u64, runtime_ms: u64) -> FoundChapters {
        FoundChapters {
            chapters: (0..n)
                .map(|i| FoundChapter {
                    title: format!("Chapter {}", i + 1),
                    start_ms: i * runtime_ms / n,
                })
                .collect(),
            runtime_ms,
            intro_ms: 0,
            outro_ms: 0,
        }
    }

    fn starts_of(chapters: &[Chapter]) -> Vec<u64> {
        chapters.iter().map(|c| c.start_time_ms).collect()
    }

    fn kinds(placements: &[FittedPlacement]) -> Vec<PlacementKind> {
        placements.iter().map(|p| p.kind).collect()
    }

    #[test]
    fn many_tracks_from_another_edition_are_fitted_to_track_starts() {
        // 48 chapters over 8h 57m. The book runs 8h 59m in 120 tracks, and
        // every chapter begins a track up to 12 s off its scaled position.
        let found = even_edition(48, 32_220_000);
        let book_ms = 32_340_000;
        let scale = book_ms as f64 / 32_220_000.0;
        let starts: Vec<u64> = found
            .chapters
            .iter()
            .enumerate()
            .map(|(i, c)| match i {
                0 => 0,
                _ => (c.start_ms as f64 * scale) as u64 + (i as u64 % 5) * 3_000,
            })
            .collect();
        let mut tracks = Vec::new();
        for (i, &start) in starts.iter().enumerate() {
            let end = starts.get(i + 1).copied().unwrap_or(book_ms);
            let parts = if i % 2 == 0 { 2 } else { 3 };
            tracks.extend((0..parts).map(|p| start + (end - start) * p / parts));
        }
        assert_eq!(tracks.len(), 120);

        let ChapterOffer::Fitted {
            chapters, counts, ..
        } = offer_chapters(&found, book_ms, &[], &tracks, &mut |_, _| None, &mut |_| {
            None
        })
        else {
            panic!("expected fitted chapters");
        };
        assert_eq!(
            counts,
            PlacementCounts {
                track: 48,
                heard: 0,
                pause: 0,
                estimate: 0
            }
        );
        assert_eq!(starts_of(&chapters), starts);
        assert_eq!(chapters[47].title, "Chapter 48");
        assert_eq!(chapters[47].end_time_ms, book_ms);
        assert!(chapters.iter().all(|c| c.placement.is_none()));
    }

    #[test]
    fn a_single_file_falls_back_to_estimates_without_pauses() {
        let found = even_edition(48, 32_220_000);
        let current = vec![Chapter::new(0, "Dune".to_string(), 0, 32_280_000)];
        let ChapterOffer::Fitted {
            chapters,
            placements,
            counts,
        } = offer_chapters(
            &found,
            32_280_000,
            &current,
            &[0],
            &mut |_, _| None,
            &mut |_| None,
        )
        else {
            panic!("expected fitted chapters");
        };
        assert_eq!(
            counts,
            PlacementCounts {
                track: 1,
                heard: 0,
                pause: 0,
                estimate: 47
            }
        );
        assert_eq!(chapters[1].title, "Chapter 2");
        assert_eq!(chapters[1].placement, Some(ChapterPlacement::Estimate));
        assert_eq!(placements[1].moved_ms, 0);
        assert_eq!(chapters[47].end_time_ms, 32_280_000);
    }

    #[test]
    fn a_placed_chapter_moves_the_estimates_after_it() {
        // A 400 s edition with chapters every 100 s, and a 404 s book: the
        // scaled starts are 101 s, 202 s and 303 s. Chapter 1 begins a track
        // 8 s late, nothing is near chapter 2, and chapter 3 has a pause.
        let found = FoundChapters {
            chapters: (0..4)
                .map(|i| FoundChapter {
                    title: format!("C{}", i),
                    start_ms: i * 100_000,
                })
                .collect(),
            runtime_ms: 400_000,
            intro_ms: 0,
            outro_ms: 0,
        };
        let mut asked = Vec::new();
        let mut finder = |estimate: u64| {
            asked.push(estimate);
            (estimate > 300_000).then_some(estimate + 3_000)
        };
        let ChapterOffer::Fitted {
            chapters,
            placements,
            ..
        } = offer_chapters(
            &found,
            404_000,
            &[],
            &[0, 109_000],
            &mut |_, _| None,
            &mut finder,
        )
        else {
            panic!("expected fitted chapters");
        };
        // Chapter 2 is estimated 8 s late too, and keeps that offset for chapter 3
        assert_eq!(asked, [210_000, 311_000]);
        assert_eq!(starts_of(&chapters), [0, 109_000, 210_000, 314_000]);
        assert_eq!(
            kinds(&placements),
            [
                PlacementKind::Track,
                PlacementKind::Track,
                PlacementKind::Estimate,
                PlacementKind::Pause
            ]
        );
        let moved: Vec<i64> = placements.iter().map(|p| p.moved_ms).collect();
        assert_eq!(moved, [0, 8_000, 0, 3_000]);
        assert_eq!(chapters[3].placement, Some(ChapterPlacement::Pause));
    }

    #[test]
    fn a_heard_title_beats_a_pause_and_moves_later_estimates() {
        // Chapters every 100 s of a 400 s edition, one 404 s file: the scaled
        // starts are 101 s, 202 s and 303 s. Only C1 is heard, 6 s late.
        let found = FoundChapters {
            chapters: (0..4)
                .map(|i| FoundChapter {
                    title: format!("C{}", i),
                    start_ms: i * 100_000,
                })
                .collect(),
            runtime_ms: 400_000,
            intro_ms: 0,
            outro_ms: 0,
        };
        let mut paused_at = Vec::new();
        let offer = {
            let mut heard =
                |title: &str, estimate: u64| (title == "C1").then_some(estimate + 6_000);
            let mut pause = |estimate: u64| {
                paused_at.push(estimate);
                Some(estimate + 1_000)
            };
            offer_chapters(&found, 404_000, &[], &[0], &mut heard, &mut pause)
        };
        let ChapterOffer::Fitted {
            chapters,
            placements,
            counts,
        } = offer
        else {
            panic!("expected fitted chapters");
        };
        // C2 is estimated 6 s late after C1, and C3 7 s late after C2's pause
        assert_eq!(starts_of(&chapters), [0, 107_000, 209_000, 311_000]);
        assert_eq!(
            kinds(&placements),
            [
                PlacementKind::Track,
                PlacementKind::Heard,
                PlacementKind::Pause,
                PlacementKind::Pause
            ]
        );
        assert_eq!(
            paused_at,
            [208_000, 310_000],
            "no pause search for a heard chapter"
        );
        assert_eq!((counts.heard, counts.pause), (1, 2));
        assert_eq!(chapters[1].placement, None, "a heard chapter is not marked");
        assert_eq!(chapters[2].placement, Some(ChapterPlacement::Pause));
    }

    #[test]
    fn a_chapter_never_lands_on_or_before_the_previous_start() {
        // Chapter 1 is estimated 10.1 s in; the only track start in reach is
        // the book's start, and the pause offered is there too.
        let mut found = even_edition(2, 400_000);
        found.chapters[1].start_ms = 10_000;
        let ChapterOffer::Fitted {
            chapters,
            placements,
            ..
        } = offer_chapters(
            &found,
            404_000,
            &[],
            &[0, 50_000],
            &mut |_, _| None,
            &mut |_| Some(0),
        )
        else {
            panic!("expected fitted chapters");
        };
        assert_eq!(starts_of(&chapters), [0, 10_100]);
        assert_eq!(placements[1].kind, PlacementKind::Estimate);
    }

    #[test]
    fn titles_only_still_wins_when_the_counts_match() {
        let current: Vec<Chapter> = (0..3)
            .map(|i| {
                Chapter::new(
                    i,
                    format!("Track {}", i + 1),
                    i as u64 * 10_800_000,
                    (i as u64 + 1) * 10_800_000,
                )
            })
            .collect();
        // 32,400,000 ms is within 2% of the edition, so a fit would also apply
        assert!(matches!(
            offer_chapters(
                &dune_messiah(),
                32_400_000,
                &current,
                &[0],
                &mut |_, _| None,
                &mut |_| None
            ),
            ChapterOffer::TitlesOnly { .. }
        ));
    }

    #[test]
    fn track_starts_follow_file_durations() {
        let file = |duration_ms| AudioFile {
            path: camino::Utf8PathBuf::from("/test/a.mp3"),
            format: crate::models::AudioFormat::Mp3,
            duration_ms,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            file_size_bytes: 0,
            metadata: crate::models::Metadata::default(),
            has_artwork: false,
        };
        assert_eq!(
            track_starts(&[file(1_000), file(2_500), file(400)]),
            [0, 1_000, 3_500]
        );
    }

    /// The first chapters of Dune Messiah (Audible B002V8H660) as Audnexus lists them.
    fn dune_messiah() -> FoundChapters {
        FoundChapters {
            chapters: vec![
                FoundChapter {
                    title: "Opening Credits".to_string(),
                    start_ms: 0,
                },
                FoundChapter {
                    title: "Excerpts from the Death Cell Interview".to_string(),
                    start_ms: 41_820,
                },
                FoundChapter {
                    title: "Analysis of History".to_string(),
                    start_ms: 602_001,
                },
            ],
            runtime_ms: 32_269_161,
            intro_ms: 3_970,
            outro_ms: 4_945,
        }
    }

    fn spans(chapters: &[Chapter]) -> Vec<(u64, u64)> {
        chapters
            .iter()
            .map(|c| (c.start_time_ms, c.end_time_ms))
            .collect()
    }

    #[test]
    fn same_edition_uses_listed_timings() {
        let ChapterOffer::Exact { chapters } = offer(&dune_messiah(), 32_269_161, &[]) else {
            panic!("expected exact timings");
        };
        assert_eq!(
            spans(&chapters),
            vec![(0, 41_820), (41_820, 602_001), (602_001, 32_269_161)]
        );
        assert_eq!(chapters[1].title, "Excerpts from the Death Cell Interview");
    }

    #[test]
    fn a_book_slightly_shorter_than_the_edition_still_matches() {
        let ChapterOffer::Exact { chapters } = offer(&dune_messiah(), 32_268_000, &[]) else {
            panic!("expected exact timings");
        };
        assert_eq!(chapters.last().unwrap().end_time_ms, 32_268_000);
    }

    #[test]
    fn stripped_branding_shifts_every_start_by_the_intro() {
        let offer = offer(&dune_messiah(), 32_260_246, &[]);
        let ChapterOffer::Shifted { chapters, shift_ms } = offer else {
            panic!("expected shifted timings");
        };
        assert_eq!(shift_ms, 3_970);
        assert_eq!(
            spans(&chapters),
            vec![(0, 37_850), (37_850, 598_031), (598_031, 32_260_246)]
        );
    }

    #[test]
    fn same_count_from_another_edition_offers_titles_only() {
        let current: Vec<Chapter> = (0..3)
            .map(|i| {
                Chapter::new(
                    i,
                    format!("Track {:02}", i + 1),
                    i as u64 * 1_000_000,
                    (i as u64 + 1) * 1_000_000,
                )
            })
            .collect();
        let ChapterOffer::TitlesOnly { chapters } = offer(&dune_messiah(), 33_120_000, &current)
        else {
            panic!("expected titles only");
        };
        assert_eq!(chapters[2].title, "Analysis of History");
        assert_eq!(spans(&chapters), spans(&current));
    }

    #[test]
    fn different_edition_and_count_says_why() {
        let mut found = dune_messiah();
        found.chapters = (0..28)
            .map(|i| FoundChapter {
                title: format!("Chapter {}", i + 1),
                start_ms: i * 1_152_000,
            })
            .collect();
        let current: Vec<Chapter> = (0..6)
            .map(|i| {
                Chapter::new(
                    i,
                    format!("Part {}", i + 1),
                    i as u64 * 5_000_000,
                    (i as u64 + 1) * 5_000_000,
                )
            })
            .collect();

        let ChapterOffer::Unavailable { reason } = offer(&found, 33_120_000, &current) else {
            panic!("expected unavailable");
        };
        assert_eq!(
            reason,
            "The found edition runs 8h 57m with 28 chapters; this book runs 9h 12m with 6."
        );
    }

    #[test]
    fn timings_that_do_not_fit_the_book_are_not_offered() {
        // Within tolerance, but the last chapter would start after the book ends.
        let mut found = dune_messiah();
        found.chapters.push(FoundChapter {
            title: "Closing Credits".to_string(),
            start_ms: 32_268_500,
        });
        assert!(matches!(
            offer(&found, 32_268_000, &[]),
            ChapterOffer::Unavailable { .. }
        ));
    }

    #[test]
    fn no_chapter_list_is_unavailable() {
        let mut found = dune_messiah();
        found.chapters.clear();
        assert!(matches!(
            offer(&found, 32_269_161, &[]),
            ChapterOffer::Unavailable { .. }
        ));
    }
}
