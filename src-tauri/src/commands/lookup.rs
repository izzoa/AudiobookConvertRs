//! Find details: search a catalog, fetch one record with its chapters
//! already fitted to the open book, and apply the reviewed fields.

use crate::commands::undo::UndoState;
use crate::config;
use crate::ffmpeg::detection::detect_ffmpeg;
use crate::ffmpeg::silence::{choose_pause, find_silences, locate, LEAD_IN_MS};
use crate::metadata::chapter_match::{
    offer_chapters, track_starts, FoundChapters, PauseFinder, SNAP_WINDOW_MS,
};
use crate::metadata::lookup::{self, ChapterOffer, LookupCandidate, LookupRecord, LookupSource};
use crate::metadata::{audible, open_library};
use crate::models::{AudioBook, AudioFile, Chapter, Metadata};
use crate::state::AppState;
use camino::Utf8Path;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::{error, warn};

/// Brings a fit whose pause searches ran after the record was returned.
pub const CHAPTERS_FITTED_EVENT: &str = "lookup_chapters_fitted";

#[derive(Debug, Clone, Serialize)]
struct ChaptersFitted {
    request_id: String,
    offer: ChapterOffer,
}

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

/// The chapters row as soon as the record is fetched.
enum FirstOffer {
    Ready(ChapterOffer),
    /// The fit needs pause searches; `estimates` is the result without them.
    NeedsPauses {
        estimates: ChapterOffer,
    },
}

fn first_offer(found: &FoundChapters, book: &AudioBook) -> FirstOffer {
    let mut needs_pauses = false;
    let estimates = fit_with(found, book, &mut |_| {
        needs_pauses = true;
        None
    });
    if needs_pauses {
        FirstOffer::NeedsPauses { estimates }
    } else {
        FirstOffer::Ready(estimates)
    }
}

fn fit_with(found: &FoundChapters, book: &AudioBook, find_pause: &mut PauseFinder) -> ChapterOffer {
    offer_chapters(
        found,
        book.total_duration_ms,
        &book.chapters,
        &track_starts(&book.files),
        find_pause,
    )
}

/// A chapter start in a pause near `estimate_ms`, searching the one file it
/// falls in. A failed search counts as no pause.
fn pause_near(
    ffmpeg: &Utf8Path,
    files: &[AudioFile],
    starts: &[u64],
    estimate_ms: u64,
) -> Option<u64> {
    let (index, in_file) = locate(starts, estimate_ms)?;
    let file = files.get(index)?;
    let from = in_file.saturating_sub(SNAP_WINDOW_MS);
    let to = (in_file + SNAP_WINDOW_MS + LEAD_IN_MS).min(file.duration_ms);
    let silences = find_silences(ffmpeg, &file.path, from, to.saturating_sub(from))
        .map_err(|e| warn!("Pause search failed: {}", e))
        .ok()?;
    choose_pause(&silences, in_file, SNAP_WINDOW_MS).map(|start| starts[index] + start)
}

/// Runs the fit with real pause searches, one after another, and emits the
/// result for `request_id`.
fn finish_fit_in_background(
    app: AppHandle,
    request_id: String,
    found: FoundChapters,
    book: AudioBook,
    ffmpeg: camino::Utf8PathBuf,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let starts = track_starts(&book.files);
        let offer = fit_with(&found, &book, &mut |estimate| {
            pause_near(&ffmpeg, &book.files, &starts, estimate)
        });
        let payload = ChaptersFitted { request_id, offer };
        if let Err(e) = app.emit(CHAPTERS_FITTED_EVENT, payload) {
            error!("Failed to emit fitted chapters: {}", e);
        }
    });
}

/// Searches the Audible catalog; `region` defaults to the one in settings.
#[tauri::command]
pub async fn audible_search(
    title: String,
    author: Option<String>,
    region: Option<String>,
) -> Result<Vec<LookupCandidate>, String> {
    let region = region.unwrap_or_else(|| config::get_settings().audible_region);
    audible::search(&title, author.as_deref(), &region)
        .await
        .map_err(to_string_err)
}

/// The full record for one search result (or a typed ASIN), with the
/// source's chapters fitted to the book `audiobook_id`.
#[tauri::command]
pub async fn lookup_record(
    source: LookupSource,
    id: String,
    region: Option<String>,
    audiobook_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LookupRecord, String> {
    let book = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    match source {
        LookupSource::Audible => {
            let region = region.unwrap_or_else(|| config::get_settings().audible_region);
            let found = audible::fetch_record(&id, &region)
                .await
                .map_err(to_string_err)?;
            // Pause searches take seconds, so the review opens without them
            // and the finished fit follows as an event.
            let chapters = match &found.chapters {
                Ok(list) => match first_offer(list, &book) {
                    FirstOffer::Ready(offer) => offer,
                    FirstOffer::NeedsPauses { estimates } => match detect_ffmpeg() {
                        Ok(ffmpeg) => {
                            let request_id = uuid::Uuid::new_v4().to_string();
                            finish_fit_in_background(
                                app,
                                request_id.clone(),
                                list.clone(),
                                book.clone(),
                                ffmpeg.path,
                            );
                            ChapterOffer::Fitting { request_id }
                        }
                        // Without FFmpeg the estimates stand, flagged
                        Err(_) => estimates,
                    },
                },
                Err(reason) => ChapterOffer::Unavailable {
                    reason: reason.clone(),
                },
            };
            Ok(LookupRecord {
                source,
                id: found.fields.asin.clone().unwrap_or(id),
                fields: found.fields,
                cover_url: found.cover_url,
                runtime_ms: found.runtime_ms,
                chapters,
            })
        }
        LookupSource::OpenLibrary => {
            let (fields, cover_url) = open_library::fetch_record(&id)
                .await
                .map_err(to_string_err)?;
            Ok(LookupRecord {
                source,
                id,
                fields,
                cover_url,
                runtime_ms: None,
                chapters: ChapterOffer::Unavailable {
                    reason: "Open Library has no chapter lists.".to_string(),
                },
            })
        }
    }
}

/// Applies the fields, cover and chapters the user selected in the review as
/// one undo step. The cover is downloaded first; if that fails nothing changes.
#[tauri::command]
pub async fn apply_lookup(
    audiobook_id: String,
    source: LookupSource,
    fields: Metadata,
    cover_url: Option<String>,
    chapters: Option<Vec<Chapter>>,
    state: State<'_, AppState>,
    undo_state: State<'_, UndoState>,
) -> Result<AudioBook, String> {
    let cover = match cover_url {
        Some(url) => Some(lookup::download_cover(&url).await.map_err(to_string_err)?),
        None => None,
    };

    // Read the book after the download so edits made meanwhile are not lost.
    let mut book = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;
    let edit =
        lookup::apply_edit(&book, source, &fields, cover, chapters).map_err(to_string_err)?;
    undo_state.execute(&mut book, edit).map_err(to_string_err)?;
    state
        .update_audiobook(book.clone())
        .map_err(to_string_err)?;
    Ok(book)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::chapter_match::FoundChapter;
    use crate::metadata::lookup::PlacementKind;
    use crate::models::{AudioFormat, ChapterPlacement};

    /// `n` chapters every 100 s of an edition with no branding.
    fn edition(n: u64) -> FoundChapters {
        FoundChapters {
            chapters: (0..n)
                .map(|i| FoundChapter {
                    title: format!("Chapter {}", i + 1),
                    start_ms: i * 100_000,
                })
                .collect(),
            runtime_ms: n * 100_000,
            intro_ms: 0,
            outro_ms: 0,
        }
    }

    /// A book of files with these durations, one generated chapter per file.
    fn book(durations: &[u64]) -> AudioBook {
        let mut book = AudioBook::new("Test".to_string());
        book.files = durations
            .iter()
            .enumerate()
            .map(|(i, &duration_ms)| AudioFile {
                path: camino::Utf8PathBuf::from(format!("/test/{:02}.mp3", i + 1)),
                format: AudioFormat::Mp3,
                duration_ms,
                bitrate_kbps: None,
                sample_rate_hz: None,
                channels: None,
                file_size_bytes: 0,
                metadata: Metadata::default(),
                has_artwork: false,
            })
            .collect();
        book.total_duration_ms = durations.iter().sum();
        book.chapters = crate::metadata::chapters::create_chapters_from_files(&book.files);
        book
    }

    #[test]
    fn a_matching_edition_needs_no_pause_search() {
        assert!(matches!(
            first_offer(&edition(4), &book(&[400_000])),
            FirstOffer::Ready(ChapterOffer::Exact { .. })
        ));
    }

    #[test]
    fn chapters_that_all_start_tracks_need_no_pause_search() {
        // 404 s in eight tracks; every scaled chapter start begins one
        let offer = first_offer(&edition(4), &book(&[50_500; 8]));
        let FirstOffer::Ready(ChapterOffer::Fitted { counts, .. }) = offer else {
            panic!("expected a finished fit");
        };
        assert_eq!(counts.track, 4);
    }

    #[test]
    fn a_single_file_waits_for_pause_searches() {
        let FirstOffer::NeedsPauses { estimates } = first_offer(&edition(4), &book(&[404_000]))
        else {
            panic!("expected pause searches");
        };
        // What stands if no FFmpeg is found: every later chapter estimated
        let ChapterOffer::Fitted { counts, .. } = estimates else {
            panic!("expected fitted estimates");
        };
        assert_eq!(counts.estimate, 3);
    }

    #[test]
    fn the_finished_fit_places_chapters_in_pauses() {
        let book = book(&[404_000]);
        let offer = fit_with(&edition(4), &book, &mut |estimate| Some(estimate + 1_500));
        let ChapterOffer::Fitted {
            chapters,
            placements,
            counts,
        } = offer
        else {
            panic!("expected fitted chapters");
        };
        assert_eq!(counts.pause, 3);
        assert_eq!(placements[1].kind, PlacementKind::Pause);
        assert_eq!(chapters[1].start_time_ms, 102_500);
        assert_eq!(chapters[1].placement, Some(ChapterPlacement::Pause));
    }
}
