use crate::metadata::lookup::LookupCandidate;
use crate::metadata::open_library;

fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

/// Searches Open Library by free text. Applying a result goes through
/// `apply_lookup`, like every other lookup source.
#[tauri::command]
pub async fn open_library_search(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<LookupCandidate>, String> {
    open_library::search_candidates(&query, limit)
        .await
        .map_err(to_string_err)
}
