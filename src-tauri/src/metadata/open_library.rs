//! Open Library API client for fetching audiobook metadata
//!
//! Provides functions to search for books and fetch metadata from OpenLibrary.org

use crate::error::{AppError, AppResult};
use crate::models::Metadata;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

const OPENLIBRARY_SEARCH_URL: &str = "https://openlibrary.org/search.json";
const OPENLIBRARY_COVERS_URL: &str = "https://covers.openlibrary.org/b/id";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = "AudiobookConvertRs/1.0";

/// Search result from Open Library API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLibrarySearchResult {
    pub key: String,
    pub title: String,
    pub author_name: Option<Vec<String>>,
    pub first_publish_year: Option<i32>,
    pub cover_i: Option<i64>,
    pub publisher: Option<Vec<String>>,
    pub subject: Option<Vec<String>>,
}

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
struct OpenLibraryDoc {
    key: String,
    title: String,
    author_name: Option<Vec<String>>,
    first_publish_year: Option<i32>,
    cover_i: Option<i64>,
    publisher: Option<Vec<String>>,
    subject: Option<Vec<String>>,
}

/// Selection made by user from search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLibrarySelection {
    pub key: String,
    pub title: String,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub cover_id: Option<i64>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
}

/// Create HTTP client with timeout and user agent
fn create_client() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))
}

/// Search Open Library for books
pub async fn search(query: &str, limit: Option<u32>) -> AppResult<Vec<OpenLibrarySearchResult>> {
    let limit = limit.unwrap_or(10).min(100); // Cap at 100 results
    
    debug!("Searching Open Library: query='{}', limit={}", query, limit);

    let client = create_client()?;
    
    let url = format!("{}?q={}&limit={}", OPENLIBRARY_SEARCH_URL, 
        urlencoding::encode(query), limit);

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

    info!("Found {} results from Open Library", search_response.docs.len());

    let results = search_response.docs.into_iter()
        .map(|doc| OpenLibrarySearchResult {
            key: doc.key,
            title: doc.title,
            author_name: doc.author_name,
            first_publish_year: doc.first_publish_year,
            cover_i: doc.cover_i,
            publisher: doc.publisher,
            subject: doc.subject,
        })
        .collect();

    Ok(results)
}

/// Download cover image from Open Library
pub async fn download_cover(cover_id: i64, size: &str) -> AppResult<Vec<u8>> {
    debug!("Downloading cover {} from Open Library (size: {})", cover_id, size);

    let client = create_client()?;
    
    let url = format!("{}/{}-{}.jpg", OPENLIBRARY_COVERS_URL, cover_id, size);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("Cover download failed: {}", e),
        })?;

    if !response.status().is_success() {
        return Err(AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("Cover HTTP error: {}", response.status()),
        });
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| AppError::MetadataLookupError {
            provider: "Open Library".to_string(),
            reason: format!("Failed to read cover bytes: {}", e),
        })?
        .to_vec();

    info!("Downloaded cover {} ({} bytes)", cover_id, bytes.len());

    Ok(bytes)
}

/// Convert OpenLibrary selection to internal Metadata format
pub fn selection_to_metadata(selection: &OpenLibrarySelection) -> Metadata {
    let mut metadata = Metadata::default();

    metadata.title = Some(selection.title.clone());
    metadata.artist = selection.author.clone();
    
    if let Some(year) = selection.year {
        if year > 0 && year <= 9999 {
            metadata.year = Some(year as u16);
        }
    }

    metadata.publisher = selection.publisher.clone();
    
    // Use genre if available (take first subject as genre)
    metadata.genre = selection.genre.clone();

    debug!("Converted selection to metadata: title={:?}, artist={:?}", 
        metadata.title, metadata.artist);

    metadata
}

/// Apply Open Library selection to metadata, optionally downloading cover
pub async fn apply_selection(
    selection: &OpenLibrarySelection,
    download_cover: bool,
) -> AppResult<(Metadata, Option<Vec<u8>>)> {
    let metadata = selection_to_metadata(selection);

    let cover_bytes = if download_cover {
        if let Some(cover_id) = selection.cover_id {
            match self::download_cover(cover_id, "L").await {
                Ok(bytes) => {
                    info!("Successfully downloaded cover for: {}", selection.title);
                    Some(bytes)
                }
                Err(e) => {
                    warn!("Failed to download cover: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok((metadata, cover_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_to_metadata() {
        let selection = OpenLibrarySelection {
            key: "/works/OL123W".to_string(),
            title: "Test Book".to_string(),
            author: Some("Test Author".to_string()),
            year: Some(2020),
            cover_id: Some(12345),
            publisher: Some("Test Publisher".to_string()),
            genre: Some("Fiction".to_string()),
        };

        let metadata = selection_to_metadata(&selection);

        assert_eq!(metadata.title, Some("Test Book".to_string()));
        assert_eq!(metadata.artist, Some("Test Author".to_string()));
        assert_eq!(metadata.year, Some(2020));
        assert_eq!(metadata.publisher, Some("Test Publisher".to_string()));
        assert_eq!(metadata.genre, Some("Fiction".to_string()));
    }

    #[test]
    fn test_selection_to_metadata_invalid_year() {
        let selection = OpenLibrarySelection {
            key: "/works/OL123W".to_string(),
            title: "Test Book".to_string(),
            author: Some("Test Author".to_string()),
            year: Some(99999), // Invalid year
            cover_id: None,
            publisher: None,
            genre: None,
        };

        let metadata = selection_to_metadata(&selection);

        assert_eq!(metadata.year, None); // Should be filtered out
    }
}

