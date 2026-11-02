//! Open Library as a lookup source, for books that are not on Audible
//! (public-domain recordings, LibriVox). Results map into the shared lookup
//! shapes; Open Library has no narrators, series or chapters.

use crate::error::{AppError, AppResult};
use crate::metadata::lookup::{join_names, LookupCandidate, LookupSource};
use crate::models::Metadata;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};

const OPENLIBRARY_SEARCH_URL: &str = "https://openlibrary.org/search.json";
const OPENLIBRARY_COVERS_URL: &str = "https://covers.openlibrary.org/b/id";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = "AudiobookConvertRs/1.0";

/// Response from Open Library search API
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    docs: Vec<OpenLibraryDoc>,
    #[serde(default)]
    #[allow(dead_code)]
    num_found: i32,
}

/// Document in Open Library search results
#[derive(Debug, Deserialize)]
pub(crate) struct OpenLibraryDoc {
    key: String,
    title: String,
    author_name: Option<Vec<String>>,
    first_publish_year: Option<i32>,
    cover_i: Option<i64>,
    publisher: Option<Vec<String>>,
    subject: Option<Vec<String>>,
}

/// Create HTTP client with timeout and user agent
fn create_client() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))
}

/// Only the fields the lookup uses; full documents are large.
const SEARCH_FIELDS: &str = "key,title,author_name,first_publish_year,cover_i,publisher,subject";

/// Searches Open Library by free text.
pub async fn search_candidates(query: &str, limit: Option<u32>) -> AppResult<Vec<LookupCandidate>> {
    let limit = limit.unwrap_or(10).min(100);
    debug!("Searching Open Library: query='{}', limit={}", query, limit);
    let docs = fetch_docs(query, limit).await?;
    info!("Found {} results from Open Library", docs.len());
    Ok(docs.iter().map(candidate).collect())
}

/// The fields and cover for one work, by its key ("/works/OL893461W").
pub async fn fetch_record(key: &str) -> AppResult<(Metadata, Option<String>)> {
    let doc = fetch_docs(&format!("key:{}", key), 1)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("no work with key {}", key),
        })?;
    Ok(record_fields(&doc))
}

async fn fetch_docs(query: &str, limit: u32) -> AppResult<Vec<OpenLibraryDoc>> {
    let client = create_client()?;
    let url = format!(
        "{}?q={}&limit={}&fields={}",
        OPENLIBRARY_SEARCH_URL,
        urlencoding::encode(query),
        limit,
        SEARCH_FIELDS
    );

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("Request failed: {}", e),
        })?;

    if !response.status().is_success() {
        return Err(AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("HTTP error: {}", response.status()),
        });
    }

    let search_response: SearchResponse = response.json()
        .await
        .map_err(|e| AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("Failed to parse response: {}", e),
        })?;

    Ok(search_response.docs)
}

fn candidate(doc: &OpenLibraryDoc) -> LookupCandidate {
    LookupCandidate {
        source: LookupSource::OpenLibrary,
        id: doc.key.clone(),
        title: doc.title.clone(),
        subtitle: None,
        authors: doc.author_name.clone().unwrap_or_default(),
        narrators: Vec::new(),
        series: None,
        series_index: None,
        runtime_ms: None,
        year: doc.first_publish_year.and_then(valid_year),
        cover_url: doc.cover_i.map(cover_url),
    }
}

/// Title, authors, first publication year, first publisher, first subject
/// as genre, and the large cover.
fn record_fields(doc: &OpenLibraryDoc) -> (Metadata, Option<String>) {
    let first = |list: &Option<Vec<String>>| list.as_ref().and_then(|l| l.first()).cloned();
    let metadata = Metadata {
        title: Some(doc.title.clone()),
        artist: doc.author_name.as_deref().and_then(join_names),
        year: doc.first_publish_year.and_then(valid_year),
        publisher: first(&doc.publisher),
        genre: first(&doc.subject),
        ..Default::default()
    };
    (metadata, doc.cover_i.map(cover_url))
}

fn cover_url(cover_id: i64) -> String {
    format!("{}/{}-L.jpg", OPENLIBRARY_COVERS_URL, cover_id)
}

fn valid_year(year: i32) -> Option<u16> {
    (1..=9999).contains(&year).then_some(year as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dune_docs() -> Vec<OpenLibraryDoc> {
        let json = std::fs::read_to_string(format!(
            "{}/tests/fixtures/open_library/search_dune_messiah.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        serde_json::from_str::<SearchResponse>(&json).unwrap().docs
    }

    #[test]
    fn search_docs_become_candidates_and_records() {
        let docs = dune_docs();
        let found = candidate(&docs[0]);
        assert_eq!(found.source, LookupSource::OpenLibrary);
        assert_eq!(found.id, "/works/OL893461W");
        assert_eq!(found.authors, ["Frank Herbert"]);
        assert_eq!(found.year, Some(1969));
        assert_eq!(found.cover_url.as_deref(), Some("https://covers.openlibrary.org/b/id/980253-L.jpg"));
        assert!(found.narrators.is_empty() && found.series.is_none());

        let (fields, cover) = record_fields(&docs[0]);
        assert_eq!(fields.title.as_deref(), Some("Dune Messiah"));
        assert_eq!(fields.artist.as_deref(), Some("Frank Herbert"));
        assert_eq!(fields.year, Some(1969));
        assert_eq!(fields.publisher.as_deref(), Some("Orion Publishing Group, Limited"));
        assert_eq!(fields.genre.as_deref(), Some("American Science fiction"));
        assert_eq!((fields.narrator, fields.series, fields.description), (None, None, None));
        assert_eq!(cover, found.cover_url);
    }
}

