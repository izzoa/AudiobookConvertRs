//! End-to-end check that the conversion pipeline carries every configured
//! metadata field into the output M4B and that ffprobe can see it.
//!
//! Requires FFmpeg on PATH. Run explicitly with:
//!   cargo test --test pipeline_metadata_e2e -- --ignored
use audiobookconvert_rs::conversion::pipeline::{ConversionPipeline, PipelineConfig};
use audiobookconvert_rs::ffmpeg::detection::detect_ffmpeg;
use audiobookconvert_rs::models::{
    AudioBook, AudioFile, AudioFormat, ConversionJob, ConversionSettings, Metadata,
};
use camino::Utf8PathBuf;

#[test]
#[ignore = "requires FFmpeg; run with cargo test -- --ignored"]
fn pipeline_output_carries_all_metadata_fields() {
    let temp = tempfile::tempdir().unwrap();
    let temp_path = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

    // Real input: the committed 1 s silent fixture.
    let input = temp_path.join("input.m4b");
    std::fs::copy(
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.m4b"),
        &input,
    )
    .unwrap();
    let output = temp_path.join("output.m4b");

    let mut audiobook = AudioBook::new("E2E Metadata".to_string());
    audiobook.files.push(AudioFile {
        path: input.clone(),
        format: AudioFormat::M4b,
        duration_ms: 1000,
        bitrate_kbps: Some(16),
        sample_rate_hz: Some(44100),
        channels: Some(1),
        file_size_bytes: 1000,
        metadata: Metadata::default(),
        has_artwork: false,
    });
    audiobook.output_metadata = Metadata {
        title: Some("The Title".to_string()),
        artist: Some("The Author".to_string()),
        narrator: Some("The Narrator".to_string()),
        series: Some("Pimsleur Italian".to_string()),
        series_index: Some("1".to_string()),
        language: Some("it".to_string()),
        publisher: Some("The Publisher".to_string()),
        description: Some("A description".to_string()),
        ..Default::default()
    };

    let mut settings = ConversionSettings::default();
    settings.output_path = output.clone();

    let job = ConversionJob::new(audiobook, settings);
    let ffmpeg_info = detect_ffmpeg().expect("FFmpeg must be available for this test");
    let pipeline = ConversionPipeline::new(ffmpeg_info, PipelineConfig::default());

    let produced = pipeline.run(&job, |_| {}, || false).expect("pipeline run");
    assert_eq!(produced, output);
    assert!(output.exists());

    // ffprobe must see the standard and freeform tags (case-insensitive:
    // ffmpeg reports freeform atom names with varying casing).
    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format_tags",
            "-of",
            "json",
            output.as_str(),
        ])
        .output()
        .expect("run ffprobe");
    assert!(probe.status.success(), "ffprobe failed: {:?}", probe.stderr);
    let tags = String::from_utf8_lossy(&probe.stdout).to_ascii_lowercase();
    println!("ffprobe format_tags: {}", tags);

    for expected in [
        "the title",
        "the author",
        "the narrator",
        "pimsleur italian",
        "the publisher",
        "a description",
    ] {
        assert!(tags.contains(expected), "ffprobe tags missing {:?}:\n{}", expected, tags);
    }
    for expected_key in ["narrator", "series", "series-part", "language", "description"] {
        assert!(tags.contains(expected_key), "ffprobe tags missing key {:?}:\n{}", expected_key, tags);
    }
    // series index 1 is written to SERIES-PART
    assert!(tags.contains("\"1\"") || tags.contains("=1") || tags.contains(": 1") || tags.contains("part\":\"1"),
        "ffprobe tags missing series index 1:\n{}", tags);
}
