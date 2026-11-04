//! A short stretch of a book's audio drawn as a waveform, with its pauses,
//! for checking where a chapter starts.
//!
//! The stretch is in book time and may cross from one file into the next.

use crate::error::AppResult;
use crate::ffmpeg::silence::{find_silences, Silence};
use crate::models::AudioFile;
use crate::speech::audio::read_window;
use crate::speech::transcribe::SAMPLE_RATE;
use camino::Utf8Path;
use serde::Serialize;

/// Columns in a waveform; enough for a wide window, light to send
pub const COLUMNS: usize = 600;

/// A pause, in book time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Pause {
    pub start_ms: u64,
    pub end_ms: u64,
}

/// The audio from `from_ms` to `to_ms` (book time) as peak levels, 0 to 1,
/// one per column, and the pauses in it
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Waveform {
    pub from_ms: u64,
    pub to_ms: u64,
    pub peaks: Vec<f32>,
    pub pauses: Vec<Pause>,
}

/// One file's part of a stretch: the file, and the stretch's part of it in
/// file time, starting `book_ms` into the book
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Piece {
    file: usize,
    from_ms: u64,
    len_ms: u64,
    book_ms: u64,
}

/// The files a stretch of the book crosses, and which part of each
fn pieces(files: &[AudioFile], from_ms: u64, to_ms: u64) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut start = 0u64;
    for (i, file) in files.iter().enumerate() {
        let end = start + file.duration_ms;
        let (a, b) = (from_ms.max(start), to_ms.min(end));
        if a < b {
            out.push(Piece {
                file: i,
                from_ms: a - start,
                len_ms: b - a,
                book_ms: a,
            });
        }
        start = end;
    }
    out
}

/// The loudest sample in each of `columns` equal parts of `samples`
pub fn peaks(samples: &[f32], columns: usize) -> Vec<f32> {
    if columns == 0 {
        return Vec::new();
    }
    (0..columns)
        .map(|c| {
            let a = c * samples.len() / columns;
            let b = ((c + 1) * samples.len() / columns).max(a);
            samples[a..b]
                .iter()
                .fold(0.0f32, |m, s| m.max(s.abs()))
                .min(1.0)
        })
        .collect()
}

/// The waveform of the book between `from_ms` and `to_ms`. A file that
/// cannot be read shows as silence rather than failing the whole view.
pub fn waveform(
    ffmpeg: &Utf8Path,
    files: &[AudioFile],
    from_ms: u64,
    to_ms: u64,
) -> AppResult<Waveform> {
    let mut samples: Vec<f32> = Vec::new();
    let mut pauses = Vec::new();
    for piece in pieces(files, from_ms, to_ms) {
        let file = &files[piece.file].path;
        let mut read = read_window(ffmpeg, file, piece.from_ms, piece.len_ms).unwrap_or_default();
        // Exactly the piece's length, so later pieces line up
        read.resize((piece.len_ms * SAMPLE_RATE as u64 / 1000) as usize, 0.0);
        samples.extend(read);
        let found: Vec<Silence> =
            find_silences(ffmpeg, file, piece.from_ms, piece.len_ms).unwrap_or_default();
        pauses.extend(found.into_iter().map(|s| Pause {
            start_ms: piece.book_ms + (s.start_ms - piece.from_ms),
            end_ms: piece.book_ms + (s.end_ms - piece.from_ms),
        }));
    }
    let to_ms = from_ms + samples.len() as u64 * 1000 / SAMPLE_RATE as u64;
    Ok(Waveform {
        from_ms,
        to_ms,
        peaks: peaks(&samples, COLUMNS),
        pauses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioFormat, Metadata};

    fn file(duration_ms: u64) -> AudioFile {
        AudioFile {
            path: "/book/part.mp3".into(),
            format: AudioFormat::Mp3,
            duration_ms,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            file_size_bytes: 0,
            metadata: Metadata::default(),
            has_artwork: false,
        }
    }

    #[test]
    fn a_stretch_inside_one_file_is_one_piece() {
        let files = [file(60_000), file(60_000)];
        assert_eq!(
            pieces(&files, 70_000, 90_000),
            [Piece {
                file: 1,
                from_ms: 10_000,
                len_ms: 20_000,
                book_ms: 70_000
            }]
        );
    }

    #[test]
    fn a_stretch_across_a_file_end_takes_both_files() {
        let files = [file(60_000), file(60_000)];
        let got = pieces(&files, 50_000, 70_000);
        assert_eq!(got.len(), 2);
        assert_eq!(
            (got[0].file, got[0].from_ms, got[0].len_ms),
            (0, 50_000, 10_000)
        );
        assert_eq!(
            (got[1].file, got[1].from_ms, got[1].len_ms, got[1].book_ms),
            (1, 0, 10_000, 60_000)
        );
    }

    #[test]
    fn a_stretch_past_the_end_stops_at_the_book_end() {
        let files = [file(60_000)];
        assert_eq!(pieces(&files, 55_000, 80_000)[0].len_ms, 5_000);
        assert!(pieces(&files, 60_000, 80_000).is_empty());
    }

    #[test]
    fn peaks_take_the_loudest_sample_of_each_column() {
        let samples = [0.1, -0.5, 0.2, 0.0, 0.0, 0.0, 0.9, -1.5];
        assert_eq!(peaks(&samples, 4), [0.5, 0.2, 0.0, 1.0]);
        assert_eq!(peaks(&[], 3), [0.0, 0.0, 0.0]);
    }

    /// Two files, each a second of tone, a second of silence and a second of
    /// tone; the stretch crosses from one into the other. Needs FFmpeg:
    /// `cargo test waveform_of_real -- --ignored`.
    #[test]
    #[ignore]
    fn waveform_of_real_files_shows_tone_and_pauses() {
        use std::process::Command;
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let dir = tempfile::tempdir().unwrap();
        let mut files = Vec::new();
        for name in ["a.wav", "b.wav"] {
            let out = camino::Utf8PathBuf::from_path_buf(dir.path().join(name)).unwrap();
            let status = Command::new(ffmpeg.path.as_str())
                .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
                .arg("sine=frequency=440:duration=3,volume='if(between(t,1,2),0,1)':eval=frame")
                .arg(out.as_str())
                .status()
                .unwrap();
            assert!(status.success());
            files.push(AudioFile {
                path: out,
                ..file(3_000)
            });
        }
        // 1.5 s to 4.5 s: a's pause and last tone, then b's first tone and pause
        // (FFmpeg's sine is at 1/8 of full scale)
        let view = waveform(&ffmpeg.path, &files, 1_500, 4_500).unwrap();
        assert_eq!((view.from_ms, view.to_ms), (1_500, 4_500));
        assert_eq!(view.peaks.len(), COLUMNS);
        let at = |ms: u64| view.peaks[((ms - 1_500) * COLUMNS as u64 / 3_000) as usize];
        assert!(at(1_700) < 0.01, "silence in a");
        assert!(at(2_500) > 0.08, "tone in a");
        assert!(at(3_500) > 0.08, "tone in b");
        assert!(at(4_300) < 0.01, "silence in b");
        // a's pause, which the view starts in; b's runs past its end, so
        // where it ends is not known and it is left out
        assert_eq!(view.pauses.len(), 1, "{:?}", view.pauses);
        assert!(view.pauses[0].end_ms.abs_diff(2_000) < 100);
    }
}
