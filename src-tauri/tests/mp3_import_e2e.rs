//! Import checks against real files generated with FFmpeg: ID3v2 tags and
//! chapter markers must be read, a FLAC cue sheet must become chapters, and a
//! multi-disc folder must expand and keep disc order.
//!
//! Requires FFmpeg on PATH. Run explicitly with:
//!   cargo test --test mp3_import_e2e -- --ignored
use audiobookconvert_rs::audio::batch::load_audiobook_from_files;
use audiobookconvert_rs::audio::loader::load_audio_file;
use audiobookconvert_rs::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use audiobookconvert_rs::ffmpeg::detection::detect_ffmpeg;
use audiobookconvert_rs::metadata::{extract_chapters_from_flac_cuesheet, extract_chapters_from_mp3};
use audiobookconvert_rs::models::{ConversionJob, ConversionSettings};
use camino::{Utf8Path, Utf8PathBuf};
use std::process::Command;

/// Writes a short tagged MP3. FFmpeg puts the tags in an ID3v2 block ahead of
/// the stream — the layout that used to be missed on import.
fn write_tagged_mp3(path: &Utf8Path, title: &str, track: u32) {
    let ffmpeg = detect_ffmpeg().expect("FFmpeg must be available for this test");
    let status = Command::new(ffmpeg.path.as_str())
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=2"])
        .args(["-metadata", &format!("title={title}")])
        .args(["-metadata", "artist=Ada Wrenfield"])
        .args(["-metadata", "album=The Clockwork Chronicle"])
        .args(["-metadata", "date=2024"])
        .args(["-metadata", "genre=Steampunk"])
        .args(["-metadata", &format!("track={track}")])
        // FFmpeg has no ID3 frame for ASIN, so it lands in a TXXX user text frame.
        .args(["-metadata", "ASIN=b002v8h660"])
        .args(["-metadata", "narrator=Lena Holloway"])
        .args(["-metadata", "series=Clockwork Cycle"])
        .args(["-metadata", "series-part=1.5"])
        // These three do have ID3 frames: TPUB, TLAN, TCOP.
        .args(["-metadata", "publisher=Brassworks Audio"])
        .args(["-metadata", "language=eng"])
        .args(["-metadata", "copyright=© 2024 Ada Wrenfield"])
        .args(["-b:a", "64k", path.as_str()])
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg failed to write {path}");
}

#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn mp3_import_reads_id3v2_tags() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
    let mp3 = dir.join("01 - Opening.mp3");
    write_tagged_mp3(&mp3, "Opening", 1);

    let file = load_audio_file(&mp3).expect("load mp3");
    let meta = &file.metadata;

    // The filename stem is the fallback; getting the tag proves ID3 was read.
    assert_eq!(meta.title.as_deref(), Some("Opening"));
    assert_eq!(meta.artist.as_deref(), Some("Ada Wrenfield"));
    assert_eq!(meta.album.as_deref(), Some("The Clockwork Chronicle"));
    assert_eq!(meta.genre.as_deref(), Some("Steampunk"));
    assert_eq!(meta.year, Some(2024));
    assert_eq!(meta.track_number, Some(1));
    assert_eq!(meta.asin.as_deref(), Some("B002V8H660"));
    assert_eq!(meta.narrator.as_deref(), Some("Lena Holloway"));
    assert_eq!(meta.series.as_deref(), Some("Clockwork Cycle"));
    assert_eq!(meta.series_index.as_deref(), Some("1.5"));
    assert_eq!(meta.publisher.as_deref(), Some("Brassworks Audio"));
    assert_eq!(meta.language.as_deref(), Some("eng"));
    assert_eq!(meta.copyright.as_deref(), Some("© 2024 Ada Wrenfield"));
}

#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn folder_import_expands_discs_in_order_and_carries_tags() {
    let temp = tempfile::tempdir().unwrap();
    let book = Utf8PathBuf::from_path_buf(temp.path().to_path_buf())
        .unwrap()
        .join("The Clockwork Chronicle");

    // Each disc restarts at 01, so sorting by filename alone would interleave them.
    for (disc, parts) in [("CD1", [1, 2]), ("CD2", [3, 4])] {
        let disc_dir = book.join(disc);
        std::fs::create_dir_all(&disc_dir).unwrap();
        for (i, part) in parts.into_iter().enumerate() {
            let name = format!("{:02} - Part {part}.mp3", i + 1);
            write_tagged_mp3(&disc_dir.join(name), &format!("Part {part}"), part);
        }
    }
    // Non-audio files beside the book must be ignored, not fail the import.
    std::fs::write(book.join("notes.txt"), "not audio").unwrap();
    std::fs::write(book.join(".DS_Store"), "junk").unwrap();

    let audiobook = load_audiobook_from_files(&[book.clone()]).expect("import folder");

    let titles: Vec<_> = audiobook
        .files
        .iter()
        .map(|f| f.metadata.title.clone().unwrap_or_default())
        .collect();
    assert_eq!(titles, ["Part 1", "Part 2", "Part 3", "Part 4"]);
    assert_eq!(audiobook.chapters.len(), 4);

    // Merged book metadata comes from the tags, not the first filename. The
    // book is named by the album tag; each file's title tag names a chapter.
    assert_eq!(audiobook.title, "The Clockwork Chronicle");
    assert_eq!(audiobook.output_metadata.title.as_deref(), Some("The Clockwork Chronicle"));
    let chapter_titles: Vec<_> = audiobook.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(chapter_titles, ["Part 1", "Part 2", "Part 3", "Part 4"]);
    assert_eq!(audiobook.output_metadata.artist.as_deref(), Some("Ada Wrenfield"));
    assert_eq!(audiobook.output_metadata.album.as_deref(), Some("The Clockwork Chronicle"));
}

/// The whole import path: a tagged folder must come out of the real pipeline
/// as an M4B that still carries the source tags. Before the loader read ID3,
/// the output held only a title, and that title was a filename.
#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn tagged_folder_converts_to_m4b_with_source_tags() {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
    let book = root.join("book");
    std::fs::create_dir_all(&book).unwrap();
    for part in 1..=3 {
        write_tagged_mp3(&book.join(format!("{part:02}.mp3")), &format!("Part {part}"), part);
    }

    let audiobook = load_audiobook_from_files(&[book]).expect("import folder");
    let output = root.join("out.m4b");
    let mut settings = ConversionSettings::default();
    settings.output_path = output.clone();

    let ffmpeg_info = detect_ffmpeg().expect("FFmpeg must be available for this test");
    let pipeline = ConversionPipeline::new(ffmpeg_info, PipelineConfig::default());
    pipeline
        .run(&ConversionJob::new(audiobook, settings), |_| {}, || false)
        .expect("pipeline run");

    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format_tags:chapter_tags", "-of", "json", output.as_str()])
        .output()
        .expect("run ffprobe");
    assert!(probe.status.success(), "ffprobe failed: {:?}", probe.stderr);
    let report: serde_json::Value = serde_json::from_slice(&probe.stdout).expect("ffprobe json");

    let tags = report["format"]["tags"].to_string().to_ascii_lowercase();
    for expected in ["ada wrenfield", "the clockwork chronicle", "steampunk", "2024"] {
        assert!(tags.contains(expected), "M4B missing source tag {expected:?}:\n{tags}");
    }

    // One chapter per file, named by each file's title tag.
    let chapters: Vec<_> = report["chapters"]
        .as_array()
        .expect("chapters array")
        .iter()
        .map(|c| c["tags"]["title"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(chapters, ["Part 1", "Part 2", "Part 3"]);
}

/// Runs the detected FFmpeg with `args`, writing a 4-second tone to `output`.
fn write_tone(output: &Utf8Path, args: &[&str]) {
    let ffmpeg = detect_ffmpeg().expect("FFmpeg must be available for this test");
    let status = Command::new(ffmpeg.path.as_str())
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=4"])
        .args(args)
        .arg(output.as_str())
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg failed to write {output}");
}

/// MP3 chapter markers (ID3 CHAP frames) are read through the detected FFmpeg,
/// not an `ffprobe` found on PATH.
#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn mp3_chapter_markers_are_read() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
    let meta = dir.join("chapters.txt");
    std::fs::write(
        &meta,
        ";FFMETADATA1\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=2000\ntitle=Opening\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=2000\nEND=4000\ntitle=Second Part\n",
    )
    .unwrap();
    let mp3 = dir.join("book.mp3");
    write_tone(&mp3, &["-i", meta.as_str(), "-map", "0:a", "-map_chapters", "1", "-b:a", "64k"]);

    let chapters = extract_chapters_from_mp3(&mp3).expect("read chapters");
    let titles: Vec<_> = chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Opening", "Second Part"]);
    // FFmpeg shifts chapter times by the encoder delay (about 25 ms at 44.1 kHz)
    let starts: Vec<_> = chapters.iter().map(|c| c.start_time_ms).collect();
    assert!(starts[0] == 0 && starts[1].abs_diff(2000) <= 50, "starts {starts:?}");
}

/// A FLAC's CUESHEET tag is read through Symphonia, with no FFmpeg tool.
#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn flac_cue_sheet_becomes_chapters() {
    let temp = tempfile::tempdir().unwrap();
    let flac = Utf8PathBuf::from_path_buf(temp.path().join("book.flac")).unwrap();
    let cuesheet = "FILE \"book.flac\" WAVE\n  TRACK 01 AUDIO\n    TITLE \"Opening\"\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    TITLE \"Second Part\"\n    INDEX 01 00:02:00\n";
    write_tone(&flac, &["-metadata", &format!("CUESHEET={cuesheet}")]);

    let chapters = extract_chapters_from_flac_cuesheet(&flac).expect("read cue sheet");
    let found: Vec<_> = chapters.iter().map(|c| (c.title.as_str(), c.start_time_ms)).collect();
    assert_eq!(found, [("Opening", 0), ("Second Part", 2000)]);
}
