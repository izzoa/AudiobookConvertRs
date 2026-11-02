//! Audible as a lookup source: the public catalog for search, Audnexus for
//! the full record and chapters of the book the user picks.
//!
//! Searching asks the catalog once; Audnexus is only called for the chosen
//! book, so a search costs one request instead of one per result.

use crate::error::{AppError, AppResult};
use crate::metadata::chapter_match::{FoundChapter, FoundChapters};
use crate::metadata::lookup::{
    html_to_text, join_names, language_name, series_position, LookupCandidate, LookupSource,
};
use crate::models::{normalize_asin, Metadata};
use reqwest::{Client, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

/// Audible stores by region code, with the domain suffix each store's catalog
/// API lives on (`api.audible.<suffix>`).
pub const REGIONS: [(&str, &str); 10] = [
    ("us", "com"),
    ("uk", "co.uk"),
    ("ca", "ca"),
    ("au", "com.au"),
    ("de", "de"),
    ("fr", "fr"),
    ("it", "it"),
    ("es", "es"),
    ("jp", "co.jp"),
    ("in", "in"),
];

pub const DEFAULT_REGION: &str = "us";

const AUDNEXUS_URL: &str = "https://api.audnex.us";
const CATALOG: &str = "Audible";
const AUDNEXUS: &str = "Audnexus";
const RESPONSE_GROUPS: &str = "contributors,media,product_desc,product_attrs,series";
const SEARCH_RESULTS: &str = "10";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = concat!("AudiobookConvertRs/", env!("CARGO_PKG_VERSION"));

/// `https://api.audible.co.uk` for "uk"; `None` for an unknown region.
pub fn catalog_base_url(region: &str) -> Option<String> {
    REGIONS
        .iter()
        .find(|(code, _)| code.eq_ignore_ascii_case(region))
        .map(|(_, suffix)| format!("https://api.audible.{}", suffix))
}

pub fn is_region(region: &str) -> bool {
    catalog_base_url(region).is_some()
}

/// Everything one Audible book offers, before its chapters are fitted to a book.
#[derive(Debug, Clone)]
pub struct AudibleRecord {
    pub fields: Metadata,
    pub cover_url: Option<String>,
    pub runtime_ms: Option<u64>,
    /// The chapter list, or why there is none.
    pub chapters: Result<FoundChapters, String>,
}

/// Searches the region's catalog by title and, optionally, author.
pub async fn search(title: &str, author: Option<&str>, region: &str) -> AppResult<Vec<LookupCandidate>> {
    let base = catalog_base_url(region).ok_or_else(|| lookup_error(CATALOG, format!("unknown region \"{}\"", region)))?;
    let mut url = Url::parse(&format!("{}/1.0/catalog/products", base)).expect("catalog url");
    url.query_pairs_mut()
        .append_pair("title", title.trim())
        .append_pair("num_results", SEARCH_RESULTS)
        .append_pair("products_sort_by", "Relevance")
        .append_pair("response_groups", RESPONSE_GROUPS);
    if let Some(author) = author.map(str::trim).filter(|a| !a.is_empty()) {
        url.query_pairs_mut().append_pair("author", author);
    }

    debug!("Searching Audible ({}): title={:?} author={:?}", region, title, author);
    let found: CatalogSearch = get_json(&client()?, CATALOG, url.as_str()).await?;
    Ok(found.products.iter().filter_map(CatalogProduct::candidate).collect())
}

/// The full record for one ASIN: Audnexus when it answers, otherwise the
/// catalog's own product data with chapters marked unavailable.
pub async fn fetch_record(asin: &str, region: &str) -> AppResult<AudibleRecord> {
    let asin = normalize_asin(asin).ok_or_else(|| lookup_error(AUDNEXUS, "no ASIN given".to_string()))?;
    let region = region.to_lowercase();
    let client = client()?;

    let book_url = format!("{}/books/{}?region={}", AUDNEXUS_URL, asin, region);
    match get_json::<AudnexusBook>(&client, AUDNEXUS, &book_url).await {
        Ok(book) => {
            let chapters_url = format!("{}/books/{}/chapters?region={}", AUDNEXUS_URL, asin, region);
            let chapters = match get_json_or_missing::<AudnexusChapters>(&client, AUDNEXUS, &chapters_url).await {
                Ok(Some(chapters)) => Ok(chapters.into_found()),
                Ok(None) => Err(NO_CHAPTERS.to_string()),
                Err(e) => Err(format!("Chapters could not be fetched ({}).", e)),
            };
            Ok(record_from_book(&book, chapters))
        }
        Err(audnexus_error) => {
            warn!("Audnexus lookup failed for {}: {}; falling back to the catalog", asin, audnexus_error);
            let base = catalog_base_url(&region).ok_or(audnexus_error)?;
            let url = format!("{}/1.0/catalog/products/{}?response_groups={}", base, asin, RESPONSE_GROUPS);
            let item: CatalogItem = get_json(&client, CATALOG, &url).await?;
            Ok(record_from_catalog(&item.product))
        }
    }
}

const NO_CHAPTERS: &str = "This record has no chapter list.";

fn record_from_book(book: &AudnexusBook, chapters: Result<FoundChapters, String>) -> AudibleRecord {
    let series = book.series_primary.as_ref();
    AudibleRecord {
        fields: Metadata {
            title: Some(book.title.clone()),
            artist: join_names(&names(&book.authors)),
            narrator: join_names(&names(&book.narrators)),
            series: series.map(|s| s.name.clone()),
            series_index: series.and_then(|s| s.position.as_deref()).and_then(series_position),
            year: book.release_date.as_deref().and_then(year_of),
            genre: book.genres.iter().find(|g| g.kind == "genre").map(|g| g.name.clone()),
            publisher: book.publisher_name.clone(),
            copyright: book.copyright.map(|year| format!("© {}", year)),
            description: book.summary.as_deref().and_then(html_to_text),
            language: book.language.as_deref().and_then(language_name),
            asin: normalize_asin(&book.asin),
            ..Default::default()
        },
        cover_url: book.image.clone(),
        runtime_ms: book.runtime_length_min.map(|min| min * 60_000),
        chapters,
    }
}

fn record_from_catalog(product: &CatalogProduct) -> AudibleRecord {
    let series = product.main_series();
    AudibleRecord {
        fields: Metadata {
            title: product.title.clone(),
            artist: join_names(&names(&product.authors)),
            narrator: join_names(&names(&product.narrators)),
            series: series.and_then(|s| s.title.clone()),
            series_index: series.and_then(|s| s.sequence.as_deref()).and_then(series_position),
            year: product.release_date.as_deref().and_then(year_of),
            publisher: product.publisher_name.clone(),
            description: product.merchandising_summary.as_deref().and_then(html_to_text),
            language: product.language.as_deref().and_then(language_name),
            asin: normalize_asin(&product.asin),
            ..Default::default()
        },
        cover_url: product.cover_url(),
        runtime_ms: product.runtime_length_min.map(|min| min * 60_000),
        chapters: Err("Chapter details could not be fetched; showing catalog details only.".to_string()),
    }
}

fn names(people: &[Person]) -> Vec<&str> {
    people.iter().map(|p| p.name.as_str()).collect()
}

/// "2007-10-01" or "2007-10-01T00:00:00.000Z" -> 2007.
fn year_of(date: &str) -> Option<u16> {
    date.get(..4)?.parse().ok()
}

// ---- HTTP ----------------------------------------------------------------

fn client() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))
}

async fn get_json<T: DeserializeOwned>(client: &Client, provider: &str, url: &str) -> AppResult<T> {
    get_json_or_missing(client, provider, url)
        .await?
        .ok_or_else(|| lookup_error(provider, "not found".to_string()))
}

/// Like `get_json`, but a 404 is `Ok(None)`: Audnexus answers 404 for books
/// that simply have no chapter list.
async fn get_json_or_missing<T: DeserializeOwned>(client: &Client, provider: &str, url: &str) -> AppResult<Option<T>> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| lookup_error(provider, format!("request failed: {}", e)))?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        return Err(error_for_status(provider, status, reset.as_deref()));
    }

    response
        .json::<T>()
        .await
        .map(Some)
        .map_err(|e| lookup_error(provider, format!("unexpected response: {}", e)))
}

fn error_for_status(provider: &str, status: StatusCode, ratelimit_reset: Option<&str>) -> AppError {
    let reason = if status == StatusCode::TOO_MANY_REQUESTS {
        match ratelimit_reset.and_then(|s| s.trim().parse::<u64>().ok()) {
            Some(seconds) => format!("the service is busy; try again in {} s", seconds),
            None => "the service is busy; try again in a minute".to_string(),
        }
    } else {
        format!("HTTP {}", status)
    };
    lookup_error(provider, reason)
}

fn lookup_error(provider: &str, reason: String) -> AppError {
    AppError::MetadataLookupError { provider: provider.to_string(), reason }
}

// ---- Response shapes -------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CatalogSearch {
    #[serde(default)]
    products: Vec<CatalogProduct>,
}

#[derive(Debug, Deserialize)]
struct CatalogItem {
    product: CatalogProduct,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogProduct {
    asin: String,
    title: Option<String>,
    subtitle: Option<String>,
    #[serde(default)]
    authors: Vec<Person>,
    #[serde(default)]
    narrators: Vec<Person>,
    #[serde(default)]
    series: Vec<CatalogSeries>,
    runtime_length_min: Option<u64>,
    release_date: Option<String>,
    language: Option<String>,
    publisher_name: Option<String>,
    merchandising_summary: Option<String>,
    #[serde(default)]
    product_images: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Person {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogSeries {
    title: Option<String>,
    sequence: Option<String>,
}

impl CatalogProduct {
    fn candidate(&self) -> Option<LookupCandidate> {
        let series = self.main_series();
        Some(LookupCandidate {
            source: LookupSource::Audible,
            id: self.asin.clone(),
            title: self.title.clone()?,
            subtitle: self.subtitle.clone(),
            authors: names(&self.authors).into_iter().map(String::from).collect(),
            narrators: names(&self.narrators).into_iter().map(String::from).collect(),
            series: series.and_then(|s| s.title.clone()),
            series_index: series.and_then(|s| s.sequence.as_deref()).and_then(series_position),
            runtime_ms: self.runtime_length_min.map(|min| min * 60_000),
            year: self.release_date.as_deref().and_then(year_of),
            cover_url: self.cover_url(),
        })
    }

    /// A book in several series ("Dune" #2, "The Dune Sequence" #13) lists
    /// them in no stable order; the lowest position is the book's own series.
    fn main_series(&self) -> Option<&CatalogSeries> {
        let position = |s: &CatalogSeries| {
            s.sequence
                .as_deref()
                .and_then(series_position)
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(f64::MAX)
        };
        self.series
            .iter()
            .filter(|s| s.title.is_some())
            .min_by(|a, b| position(a).total_cmp(&position(b)))
    }

    fn cover_url(&self) -> Option<String> {
        self.product_images.get("500").cloned()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusBook {
    asin: String,
    title: String,
    #[serde(default)]
    authors: Vec<Person>,
    #[serde(default)]
    narrators: Vec<Person>,
    series_primary: Option<AudnexusSeries>,
    publisher_name: Option<String>,
    release_date: Option<String>,
    language: Option<String>,
    #[serde(default)]
    genres: Vec<AudnexusGenre>,
    copyright: Option<u32>,
    image: Option<String>,
    summary: Option<String>,
    runtime_length_min: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AudnexusSeries {
    name: String,
    position: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusGenre {
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusChapters {
    #[serde(default)]
    brand_intro_duration_ms: u64,
    #[serde(default)]
    brand_outro_duration_ms: u64,
    runtime_length_ms: u64,
    #[serde(default)]
    chapters: Vec<AudnexusChapter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusChapter {
    title: String,
    start_offset_ms: u64,
}

impl AudnexusChapters {
    fn into_found(self) -> FoundChapters {
        FoundChapters {
            chapters: self
                .chapters
                .into_iter()
                .map(|c| FoundChapter { title: c.title, start_ms: c.start_offset_ms })
                .collect(),
            runtime_ms: self.runtime_length_ms,
            intro_ms: self.brand_intro_duration_ms,
            outro_ms: self.brand_outro_duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/audible/{}", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
    }

    #[test]
    fn every_region_has_a_catalog_host() {
        let hosts: Vec<String> = REGIONS.iter().map(|(code, _)| catalog_base_url(code).unwrap()).collect();
        assert_eq!(
            hosts,
            [
                "https://api.audible.com",
                "https://api.audible.co.uk",
                "https://api.audible.ca",
                "https://api.audible.com.au",
                "https://api.audible.de",
                "https://api.audible.fr",
                "https://api.audible.it",
                "https://api.audible.es",
                "https://api.audible.co.jp",
                "https://api.audible.in",
            ]
        );
        assert_eq!(catalog_base_url("UK").as_deref(), Some("https://api.audible.co.uk"));
        assert!(!is_region("nz"));
    }

    #[test]
    fn catalog_search_becomes_candidates() {
        let found: CatalogSearch = serde_json::from_str(&fixture("catalog_dune_messiah.json")).unwrap();
        let candidates: Vec<LookupCandidate> = found.products.iter().filter_map(CatalogProduct::candidate).collect();

        let messiah = &candidates[0];
        assert_eq!(messiah.id, "B002V8H660");
        assert_eq!(messiah.title, "Dune Messiah");
        assert_eq!(messiah.authors, ["Frank Herbert"]);
        assert_eq!(messiah.narrators, ["Scott Brick", "Euan Morton", "Simon Vance", "Katherine Kellgren"]);
        assert_eq!(messiah.series.as_deref(), Some("Dune"));
        assert_eq!(messiah.series_index.as_deref(), Some("2"));
        assert_eq!(messiah.runtime_ms, Some(537 * 60_000));
        assert_eq!(messiah.year, Some(2007));
        assert!(messiah.cover_url.as_deref().is_some_and(|u| u.starts_with("https://")));
    }

    #[test]
    fn main_series_is_the_lowest_position_whatever_the_order() {
        // The single-product response lists "The Dune Sequence" #13 before "Dune" #2.
        let item: CatalogItem = serde_json::from_str(&fixture("product_dune_messiah.json")).unwrap();
        assert_eq!(item.product.series[0].title.as_deref(), Some("The Dune Sequence"));
        let series = item.product.main_series().unwrap();
        assert_eq!((series.title.as_deref(), series.sequence.as_deref()), (Some("Dune"), Some("2")));
    }

    #[test]
    fn audnexus_book_and_chapters_become_a_record() {
        let book: AudnexusBook = serde_json::from_str(&fixture("audnexus_book_dune_messiah.json")).unwrap();
        let chapters: AudnexusChapters = serde_json::from_str(&fixture("audnexus_chapters_dune_messiah.json")).unwrap();
        let record = record_from_book(&book, Ok(chapters.into_found()));

        let f = &record.fields;
        assert_eq!(f.title.as_deref(), Some("Dune Messiah"));
        assert_eq!(f.artist.as_deref(), Some("Frank Herbert"));
        assert_eq!(f.narrator.as_deref(), Some("Scott Brick, Euan Morton, Simon Vance, Katherine Kellgren"));
        assert_eq!((f.series.as_deref(), f.series_index.as_deref()), (Some("Dune"), Some("2")));
        assert_eq!(f.year, Some(2007));
        assert_eq!(f.genre.as_deref(), Some("Literature & Fiction"));
        assert_eq!(f.publisher.as_deref(), Some("Macmillan Audio"));
        assert_eq!(f.copyright.as_deref(), Some("© 1969"));
        assert_eq!(f.language.as_deref(), Some("English"));
        assert_eq!(f.description.as_deref(), Some("A sample summary.\n\nSecond paragraph."));
        assert_eq!(f.asin.as_deref(), Some("B002V8H660"));

        let found = record.chapters.unwrap();
        assert_eq!(found.chapters.len(), 28);
        assert_eq!(found.runtime_ms, 32_269_161);
        assert_eq!((found.intro_ms, found.outro_ms), (3_970, 4_945));
        assert_eq!(found.chapters[1].start_ms, 41_820);
    }

    #[test]
    fn catalog_only_record_keeps_the_details_and_explains_the_chapters() {
        let item: CatalogItem = serde_json::from_str(&fixture("product_dune_messiah.json")).unwrap();
        let record = record_from_catalog(&item.product);

        let f = &record.fields;
        assert_eq!(f.title.as_deref(), Some("Dune Messiah"));
        assert_eq!(f.artist.as_deref(), Some("Frank Herbert"));
        assert!(f.narrator.as_deref().is_some_and(|n| n.starts_with("Scott Brick")));
        assert_eq!((f.series.as_deref(), f.series_index.as_deref()), (Some("Dune"), Some("2")));
        assert_eq!(f.year, Some(2007));
        assert_eq!(f.language.as_deref(), Some("English"));
        assert!(record.cover_url.is_some());
        assert!(record.chapters.is_err());
    }

    #[test]
    fn rate_limit_names_when_to_retry() {
        let busy = error_for_status(AUDNEXUS, StatusCode::TOO_MANY_REQUESTS, Some("42")).to_string();
        assert!(busy.contains("try again in 42 s"), "{busy}");
        let busy = error_for_status(AUDNEXUS, StatusCode::TOO_MANY_REQUESTS, None).to_string();
        assert!(busy.contains("try again in a minute"), "{busy}");
        let down = error_for_status(CATALOG, StatusCode::SERVICE_UNAVAILABLE, None).to_string();
        assert!(down.contains("Audible") && down.contains("503"), "{down}");
    }

    #[test]
    fn release_dates_give_the_year() {
        assert_eq!(year_of("2007-10-01"), Some(2007));
        assert_eq!(year_of("2007-10-01T00:00:00.000Z"), Some(2007));
        assert_eq!(year_of("n/a"), None);
    }
}
