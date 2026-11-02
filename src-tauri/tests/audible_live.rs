//! Live checks against the Audible catalog and Audnexus. They need network
//! access and real services, so they only run when asked:
//!   cargo test --test audible_live -- --ignored
use audiobookconvert_rs::metadata::audible::{fetch_record, search};

#[tokio::test]
#[ignore = "calls api.audible.com and api.audnex.us; run with -- --ignored"]
async fn search_finds_dune_messiah() {
    let found = search("Dune Messiah", Some("Frank Herbert"), "us").await.expect("catalog search");
    let messiah = found.iter().find(|c| c.id == "B002V8H660").expect("Dune Messiah in the results");
    assert_eq!(messiah.series.as_deref(), Some("Dune"));
    assert!(messiah.narrators.iter().any(|n| n == "Scott Brick"));
}

#[tokio::test]
#[ignore = "calls api.audnex.us; run with -- --ignored"]
async fn record_has_details_and_chapters() {
    let record = fetch_record("b002v8h660", "us").await.expect("record");
    assert_eq!(record.fields.asin.as_deref(), Some("B002V8H660"));
    assert_eq!(record.fields.series_index.as_deref(), Some("2"));
    assert!(record.fields.description.as_deref().is_some_and(|d| !d.contains('<')));
    let chapters = record.chapters.expect("chapter list");
    assert!(chapters.chapters.len() > 20);
    assert!(chapters.runtime_ms > 32_000_000);
}
