//! Find details: search a catalog, fetch one record with its chapters
//! already fitted to the open book, and apply the reviewed fields.

use crate::commands::undo::UndoState;
use crate::config;
use crate::metadata::chapter_match::offer_chapters;
use crate::metadata::lookup::{self, ChapterOffer, LookupCandidate, LookupRecord, LookupSource};
use crate::metadata::{audible, open_library};
use crate::models::{AudioBook, Chapter, Metadata};
use crate::state::AppState;
use tauri::State;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
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
    state: State<'_, AppState>,
) -> Result<LookupRecord, String> {
    let book = state
        .get_audiobook(&audiobook_id)
        .ok_or_else(|| format!("Audiobook not found: {}", audiobook_id))?;

    match source {
        LookupSource::Audible => {
            let region = region.unwrap_or_else(|| config::get_settings().audible_region);
            let found = audible::fetch_record(&id, &region).await.map_err(to_string_err)?;
            let chapters = match &found.chapters {
                Ok(list) => offer_chapters(list, book.total_duration_ms, &book.chapters),
                Err(reason) => ChapterOffer::Unavailable { reason: reason.clone() },
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
            let (fields, cover_url) = open_library::fetch_record(&id).await.map_err(to_string_err)?;
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
    let edit = lookup::apply_edit(&book, source, &fields, cover, chapters).map_err(to_string_err)?;
    undo_state.execute(&mut book, edit).map_err(to_string_err)?;
    state.update_audiobook(book.clone()).map_err(to_string_err)?;
    Ok(book)
}
