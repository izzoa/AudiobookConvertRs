//! Downloads every fixed FFmpeg build and checks it against the table the app
//! ships: size, SHA-256, and the one entry the installer keeps. A failure means
//! a host moved or replaced a file, and the table needs a new row.
//!
//! Needs network access (about 100 MB). Run explicitly with:
//!   cargo test --test ffmpeg_sources_live -- --ignored
//! `.github/workflows/ffmpeg-links.yml` runs it weekly.
use audiobookconvert_rs::ffmpeg::download::{extract_entry, fetch_verified, FIXED_BUILDS};
use camino::Utf8Path;
use std::sync::atomic::AtomicBool;

#[tokio::test]
#[ignore = "downloads about 100 MB; run with cargo test -- --ignored"]
async fn every_fixed_build_is_online_and_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(temp.path()).unwrap();
    let keep_going = AtomicBool::new(false);
    let mut failures = Vec::new();

    for build in FIXED_BUILDS {
        let checked = async {
            let archive = fetch_verified(build, dir, &keep_going, |_, _| {}).await?;
            let entry = dir.join(format!("{}-entry", build.target));
            extract_entry(&archive, build, &entry)?;
            let size = std::fs::metadata(&entry)?.len();
            std::fs::remove_file(&archive)?;
            std::fs::remove_file(&entry)?;
            Ok::<_, audiobookconvert_rs::ffmpeg::download::DownloadError>(size)
        };
        match checked.await {
            Ok(size) => println!("ok   {} ({} bytes unpacked)", build.target, size),
            Err(e) => failures.push(format!("{}: {} ({})", build.target, e, build.url)),
        }
    }

    assert!(
        failures.is_empty(),
        "fixed FFmpeg builds changed:\n{}",
        failures.join("\n")
    );
}

/// The speech model is fixed the same way; a failure means Hugging Face moved
/// or replaced it and `speech::model::MODEL` needs a new entry.
#[tokio::test]
#[ignore = "downloads about 60 MB; run with cargo test -- --ignored"]
async fn the_speech_model_is_online_and_unchanged() {
    use audiobookconvert_rs::speech::model::{download, installed, MODEL};
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().join("speech");
    let keep_going = AtomicBool::new(false);
    let path = download(&MODEL, &root, &keep_going, |_, _| {})
        .await
        .unwrap_or_else(|e| panic!("speech model changed: {} ({})", e, MODEL.url));
    assert_eq!(installed(&root, &MODEL), Some(path));
    println!("ok   speech model ({} bytes)", MODEL.size_bytes);
}
