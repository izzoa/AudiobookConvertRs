//! Pauses in speech, found with FFmpeg's `silencedetect`, for placing a fitted
//! chapter where no track starts near it.
//!
//! Only a short window of one file is read per chapter, so a book of many
//! tracks needs a handful of runs and a single long file one per chapter.

use crate::error::{AppError, AppResult};
use camino::Utf8Path;
use std::process::Command;

/// Quieter than this counts as silence.
pub const SILENCE_NOISE_DB: i32 = -40;
/// The shortest pause worth starting a chapter in.
pub const MIN_PAUSE_MS: u64 = 500;
/// How long before the speech resumes a chapter starts, so it opens on a
/// short breath rather than a long silence.
pub const LEAD_IN_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Silence {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl Silence {
    fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    /// Where a chapter would start in this pause.
    fn chapter_start_ms(&self) -> u64 {
        self.end_ms.saturating_sub(LEAD_IN_MS).max(self.start_ms)
    }
}

/// Silences in `file` between `from_ms` and `from_ms + len_ms`, in file time.
pub fn find_silences(
    ffmpeg: &Utf8Path,
    file: &Utf8Path,
    from_ms: u64,
    len_ms: u64,
) -> AppResult<Vec<Silence>> {
    let filter = format!(
        "silencedetect=noise={}dB:d={}",
        SILENCE_NOISE_DB,
        seconds(MIN_PAUSE_MS)
    );
    let output = Command::new(ffmpeg.as_str())
        .args(["-hide_banner", "-nostats", "-ss", &seconds(from_ms)])
        .args(["-t", &seconds(len_ms), "-i", file.as_str()])
        .args(["-af", &filter, "-f", "null", "-"])
        .output()
        .map_err(|e| AppError::ffmpeg_error(format!("could not search for pauses: {}", e)))?;
    if !output.status.success() {
        return Err(AppError::ffmpeg_error(format!(
            "could not search {} for pauses: {}",
            file,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(parse_silences(
        &String::from_utf8_lossy(&output.stderr),
        from_ms,
    ))
}

/// The silences `silencedetect` logged, in file time. Its times count from the
/// seek point, so `from_ms` is added back. A silence the window starts in
/// begins at the window; one still open when the window ends is left out,
/// since where its speech resumes is not known.
pub fn parse_silences(log: &str, from_ms: u64) -> Vec<Silence> {
    let value = |line: &str, key: &str| -> Option<u64> {
        let rest = &line[line.find(key)? + key.len()..];
        let number = rest.split_whitespace().next()?;
        let secs: f64 = number.parse().ok()?;
        Some(from_ms + (secs.max(0.0) * 1000.0).round() as u64)
    };
    let mut silences = Vec::new();
    let mut open: Option<u64> = None;
    for line in log.lines().filter(|l| l.contains("silencedetect")) {
        if let Some(start) = value(line, "silence_start:") {
            open = Some(start);
        } else if let Some(end) = value(line, "silence_end:") {
            silences.push(Silence {
                start_ms: open.take().unwrap_or(from_ms),
                end_ms: end,
            });
        }
    }
    silences
}

/// Where a chapter expected at `estimate_ms` should start among `silences`:
/// in the longest pause of at least [`MIN_PAUSE_MS`] whose start point is
/// within `window_ms` of the estimate, ties going to the nearest.
pub fn choose_pause(silences: &[Silence], estimate_ms: u64, window_ms: u64) -> Option<u64> {
    silences
        .iter()
        .filter(|s| s.duration_ms() >= MIN_PAUSE_MS)
        .map(|s| (s.duration_ms(), s.chapter_start_ms()))
        .filter(|&(_, start)| start.abs_diff(estimate_ms) <= window_ms)
        .max_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.1.abs_diff(estimate_ms).cmp(&a.1.abs_diff(estimate_ms)))
        })
        .map(|(_, start)| start)
}

/// The file `book_ms` falls in and how far into it, given where each file
/// starts on the book's timeline. `None` past the last file's start when
/// `track_starts` is empty.
pub fn locate(track_starts: &[u64], book_ms: u64) -> Option<(usize, u64)> {
    let index = track_starts.iter().rposition(|&start| start <= book_ms)?;
    Some((index, book_ms - track_starts[index]))
}

/// "12.345" for FFmpeg's time options.
fn seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "\
Input #0, wav, from 'book.wav':
[silencedetect @ 0x600] silence_start: 3.2
[silencedetect @ 0x600] silence_end: 3.8 | silence_duration: 0.6
size=N/A time=00:01:00.00 bitrate=N/A speed= 512x
[silencedetect @ 0x600] silence_start: 21.05
[silencedetect @ 0x600] silence_end: 23.15 | silence_duration: 2.1
[silencedetect @ 0x600] silence_start: 58.9
";

    #[test]
    fn silences_are_read_in_file_time() {
        assert_eq!(
            parse_silences(LOG, 3_710_000),
            [
                Silence {
                    start_ms: 3_713_200,
                    end_ms: 3_713_800
                },
                Silence {
                    start_ms: 3_731_050,
                    end_ms: 3_733_150
                },
            ]
        );
    }

    #[test]
    fn a_window_that_starts_in_silence_begins_it_at_the_window() {
        let log = "[silencedetect @ 0x1] silence_end: 1.25 | silence_duration: 1.25\n";
        assert_eq!(
            parse_silences(log, 60_000),
            [Silence {
                start_ms: 60_000,
                end_ms: 61_250
            }]
        );
    }

    #[test]
    fn no_silence_logged_means_none() {
        assert!(parse_silences("size=N/A time=00:01:00.00\n", 0).is_empty());
    }

    #[test]
    fn the_longest_pause_wins_and_the_chapter_starts_before_speech() {
        let silences = parse_silences(LOG, 0);
        // The spec's example: a 0.6 s pause nearer the estimate, a 2.1 s one further away
        assert_eq!(choose_pause(&silences, 10_000, 30_000), Some(22_950));
    }

    #[test]
    fn short_or_distant_pauses_are_not_used() {
        let short = [Silence {
            start_ms: 10_000,
            end_ms: 10_400,
        }];
        assert_eq!(choose_pause(&short, 10_000, 30_000), None);
        let far = [Silence {
            start_ms: 50_000,
            end_ms: 52_000,
        }];
        assert_eq!(choose_pause(&far, 10_000, 30_000), None);
    }

    #[test]
    fn equal_pauses_go_to_the_nearest() {
        let silences = [
            Silence {
                start_ms: 1_000,
                end_ms: 2_000,
            },
            Silence {
                start_ms: 13_000,
                end_ms: 14_000,
            },
        ];
        assert_eq!(choose_pause(&silences, 12_000, 30_000), Some(13_800));
    }

    #[test]
    fn book_time_maps_to_a_file_and_back() {
        // Files of 90 s, 45 s and 300 s
        let starts = [0, 90_000, 135_000];
        assert_eq!(locate(&starts, 0), Some((0, 0)));
        assert_eq!(locate(&starts, 89_999), Some((0, 89_999)));
        assert_eq!(locate(&starts, 90_000), Some((1, 0)));
        assert_eq!(locate(&starts, 200_000), Some((2, 65_000)));
        let (index, in_file) = locate(&starts, 200_000).unwrap();
        assert_eq!(starts[index] + in_file, 200_000);
        assert_eq!(locate(&[], 5), None);
    }

    #[test]
    fn times_are_written_as_seconds() {
        assert_eq!(seconds(0), "0.000");
        assert_eq!(seconds(61_250), "61.250");
    }

    /// Builds a WAV of 5 s tone, 2 s silence and 5 s tone with FFmpeg, then
    /// finds the pause. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn finds_a_known_silence_in_real_audio() {
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let dir = tempfile::tempdir().unwrap();
        let file = camino::Utf8PathBuf::from_path_buf(dir.path().join("pause.wav")).unwrap();
        let status = Command::new(ffmpeg.path.as_str())
            .args(["-v", "error", "-y"])
            .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=5"])
            .args(["-f", "lavfi", "-i", "anullsrc=r=44100:cl=mono:d=2"])
            .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=5"])
            .args(["-filter_complex", "[0:a][1:a][2:a]concat=n=3:v=0:a=1"])
            .arg(file.as_str())
            .status()
            .unwrap();
        assert!(status.success());

        let silences = find_silences(&ffmpeg.path, &file, 1_000, 10_000).unwrap();
        let start = choose_pause(&silences, 6_000, 30_000).expect("a pause");
        // Speech resumes at 7.0 s, so the chapter starts at 6.8 s
        assert!(start.abs_diff(6_800) <= 100, "started at {start} ms");
    }
}
