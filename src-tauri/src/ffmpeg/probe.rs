//! What audio a file holds, read from FFmpeg's description of its input,
//! and whether a book's audio can go into an M4B as it is.
//!
//! An M4B holds AAC. Files that already hold AAC can be copied into one
//! without re-encoding, which takes seconds and loses nothing; several files
//! can be joined that way only when they agree on sample rate and channels.

use crate::error::{AppError, AppResult};
use crate::models::AudioFile;
use camino::Utf8Path;
use serde::Serialize;
use std::process::Command;

/// The first audio stream of a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStream {
    /// FFmpeg's codec name: "aac", "mp3", "alac", "flac"…
    pub codec: String,
    pub sample_rate_hz: Option<u32>,
    /// "mono", "stereo", "5.1"…
    pub channels: Option<String>,
    pub bitrate_kbps: Option<u32>,
}

/// Whether the book's audio can be copied into the M4B, and why not
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyCheck {
    pub possible: bool,
    /// Why not, naming the file; `None` when possible
    pub reason: Option<String>,
    /// What would be kept, such as "AAC, 64 kb/s, 44.1 kHz, stereo"
    pub audio: Option<String>,
}

/// The first audio stream FFmpeg describes in `log`, the output of
/// `ffmpeg -i file`, such as
/// `Stream #0:0(eng): Audio: aac (LC) (mp4a / 0x6134706D), 44100 Hz, stereo, fltp, 64 kb/s (default)`.
pub fn parse_audio_stream(log: &str) -> Option<AudioStream> {
    let line = log
        .lines()
        .find(|l| l.trim_start().starts_with("Stream #") && l.contains(": Audio: "))?;
    let description = &line[line.find(": Audio: ")? + ": Audio: ".len()..];
    let codec = description
        .split(|c: char| c == ' ' || c == ',')
        .next()?
        .to_lowercase();
    let mut sample_rate_hz = None;
    let mut channels = None;
    let mut bitrate_kbps = None;
    let fields: Vec<&str> = description.split(", ").map(str::trim).collect();
    for (i, field) in fields.iter().enumerate() {
        if let Some(hz) = field.strip_suffix(" Hz") {
            sample_rate_hz = hz.trim().parse().ok();
            // The channel layout follows the sample rate
            channels = fields.get(i + 1).map(|c| c.to_string());
        }
        if let Some(kbps) = field
            .split(" kb/s")
            .next()
            .filter(|_| field.contains(" kb/s"))
        {
            bitrate_kbps = kbps.trim().parse().ok();
        }
    }
    Some(AudioStream {
        codec,
        sample_rate_hz,
        channels,
        bitrate_kbps,
    })
}

/// The first audio stream of `file`
pub fn audio_stream(ffmpeg: &Utf8Path, file: &Utf8Path) -> AppResult<AudioStream> {
    // With no output FFmpeg describes the input, then exits with an error
    let output = Command::new(ffmpeg.as_str())
        .args(["-hide_banner", "-nostdin", "-i", file.as_str()])
        .output()
        .map_err(|e| AppError::ffmpeg_error(format!("could not read {}: {}", file, e)))?;
    parse_audio_stream(&String::from_utf8_lossy(&output.stderr))
        .ok_or_else(|| AppError::ffmpeg_error(format!("no audio found in {}", file)))
}

/// Whether streams already read from `files` can be copied into an M4B
pub fn copy_check_of(files: &[AudioFile], streams: &[AudioStream]) -> CopyCheck {
    let no = |reason: String| CopyCheck {
        possible: false,
        reason: Some(reason),
        audio: None,
    };
    let (Some(first), Some(first_file)) = (streams.first(), files.first()) else {
        return no("There are no files.".to_string());
    };
    for (file, stream) in files.iter().zip(streams) {
        if stream.codec != "aac" {
            return no(format!(
                "“{}” holds {}, not AAC, so it has to be converted.",
                file.filename(),
                codec_name(&stream.codec)
            ));
        }
    }
    for (file, stream) in files.iter().zip(streams).skip(1) {
        if stream.sample_rate_hz != first.sample_rate_hz || stream.channels != first.channels {
            return no(format!(
                "“{}” and “{}” differ in sample rate or channels, so they cannot be joined as they are.",
                first_file.filename(),
                file.filename()
            ));
        }
    }
    let mut parts = vec!["AAC".to_string()];
    let rates: Vec<u32> = streams.iter().filter_map(|s| s.bitrate_kbps).collect();
    if !rates.is_empty() {
        let average = rates.iter().sum::<u32>() / rates.len() as u32;
        parts.push(format!("{} kb/s", average));
    }
    if let Some(hz) = first.sample_rate_hz {
        parts.push(format!("{} kHz", hz as f64 / 1000.0));
    }
    if let Some(channels) = &first.channels {
        parts.push(channels.clone());
    }
    CopyCheck {
        possible: true,
        reason: None,
        audio: Some(parts.join(", ")),
    }
}

/// Whether the audio of `files` can be copied into an M4B
pub fn copy_check(ffmpeg: &Utf8Path, files: &[AudioFile]) -> AppResult<CopyCheck> {
    let streams = files
        .iter()
        .map(|f| audio_stream(ffmpeg, &f.path))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(copy_check_of(files, &streams))
}

fn codec_name(codec: &str) -> String {
    match codec {
        "mp3" | "mp3float" => "MP3".to_string(),
        "alac" => "Apple Lossless".to_string(),
        "flac" => "FLAC".to_string(),
        "opus" => "Opus".to_string(),
        "vorbis" => "Vorbis".to_string(),
        c if c.starts_with("pcm") => "uncompressed audio".to_string(),
        other => other.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioFormat, Metadata};

    const M4B_LOG: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'book.m4b':
  Duration: 07:01:02.37, start: 0.000000, bitrate: 65 kb/s
  Chapters:
    Chapter #0:0: start 0.000000, end 1200.000000
  Stream #0:0[0x1](eng): Audio: aac (LC) (mp4a / 0x6134706D), 44100 Hz, stereo, fltp, 62 kb/s (default)
  Stream #0:1[0x0]: Video: mjpeg (Baseline), yuvj420p(pc, bt470bg/unknown/unknown), 500x500, 90k tbr (attached pic)
At least one output file must be specified";

    fn file(name: &str) -> AudioFile {
        AudioFile {
            path: format!("/book/{name}").into(),
            format: AudioFormat::M4b,
            duration_ms: 1_000,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            file_size_bytes: 0,
            metadata: Metadata::default(),
            has_artwork: false,
        }
    }

    fn aac(hz: u32, channels: &str, kbps: u32) -> AudioStream {
        AudioStream {
            codec: "aac".into(),
            sample_rate_hz: Some(hz),
            channels: Some(channels.into()),
            bitrate_kbps: Some(kbps),
        }
    }

    #[test]
    fn the_audio_stream_is_read_from_the_input_description() {
        assert_eq!(parse_audio_stream(M4B_LOG), Some(aac(44_100, "stereo", 62)));
        let mp3 = "  Stream #0:0: Audio: mp3 (mp3float), 22050 Hz, mono, fltp, 32 kb/s";
        assert_eq!(
            parse_audio_stream(mp3),
            Some(AudioStream {
                codec: "mp3".into(),
                sample_rate_hz: Some(22_050),
                channels: Some("mono".into()),
                bitrate_kbps: Some(32),
            })
        );
        assert_eq!(parse_audio_stream("  Stream #0:0: Video: h264"), None);
    }

    #[test]
    fn aac_files_that_agree_can_be_copied() {
        let files = [file("1.m4a"), file("2.m4a")];
        let check = copy_check_of(
            &files,
            &[aac(44_100, "stereo", 64), aac(44_100, "stereo", 62)],
        );
        assert!(check.possible);
        assert_eq!(
            check.audio.as_deref(),
            Some("AAC, 63 kb/s, 44.1 kHz, stereo")
        );
    }

    #[test]
    fn other_codecs_and_mismatched_files_must_be_converted() {
        let mut mp3 = aac(44_100, "stereo", 64);
        mp3.codec = "mp3".into();
        let check = copy_check_of(&[file("1.mp3")], &[mp3]);
        assert!(!check.possible);
        assert_eq!(
            check.reason.as_deref(),
            Some("“1.mp3” holds MP3, not AAC, so it has to be converted.")
        );
        let check = copy_check_of(
            &[file("1.m4a"), file("2.m4a")],
            &[aac(44_100, "stereo", 64), aac(22_050, "mono", 32)],
        );
        assert!(!check.possible);
        assert!(check.reason.unwrap().contains("differ in sample rate"));
    }
}
