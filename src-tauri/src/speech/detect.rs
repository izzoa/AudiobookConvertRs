//! Making chapters from the audio alone: listening after every long pause
//! for a narrator announcing a heading ("Chapter four.", "Lesson 21",
//! "Prologue"), with no list of titles to look for.
//!
//! Each file is searched once for pauses, then a few seconds after each long
//! one (and at the start of every file) are transcribed. Whisper always works
//! on 30 seconds of audio, so several such windows go to it at a time.

use crate::error::{AppError, AppResult};
use crate::ffmpeg::silence::{find_silences, Silence};
use crate::models::{AudioFile, Chapter};
use crate::speech::audio::read_window;
use crate::speech::hear::{heard_start, BREAK_BEFORE_MS};
use crate::speech::matcher::heading_at_start;
use crate::speech::transcribe::SAMPLE_RATE;
use crate::speech::{Transcriber, Word};
use camino::Utf8Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

/// A pause at least this long may come before a chapter
pub const CHAPTER_PAUSE_MS: u64 = 1_800;
/// How long is listened to after a pause
const WINDOW_MS: u64 = 7_000;
/// Listening starts this long before the pause ends, so the first word is whole
const WINDOW_LEAD_MS: u64 = 400;
/// The heading must start this soon after listening starts
const HEADING_WITHIN_MS: u64 = 3_000;
/// Silence put between windows sent to Whisper together
const GAP_MS: u64 = 1_000;
/// Windows sent to Whisper together; with their gaps they fit in 30 seconds
const WINDOWS_PER_PASS: usize = 3;
/// A heading this close after the last chapter start is the same one
const SAME_CHAPTER_MS: u64 = 15_000;
/// A first heading heard this soon starts the book; a later one gets an
/// opening chapter before it
const OPENING_MS: u64 = 60_000;

/// What the detection is doing, for the progress shown while it runs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Searching files for pauses; `done` and `total` count files
    Pauses,
    /// Listening after pauses; `done` and `total` count places
    Listening,
}

/// One place to listen: a file, where listening starts, and the silences
/// around it
struct Place {
    file: usize,
    from_ms: u64,
    len_ms: u64,
}

/// A heading heard in the book, in book time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub title: String,
    /// The heading alone ("Chapter 4"), which says whether two are the same
    pub heading: String,
    pub start_ms: u64,
}

/// Where listening starts in a file with these silences: its start, and the
/// end of every pause long enough to come before a chapter.
fn places_in(silences: &[Silence], file_duration_ms: u64) -> Vec<u64> {
    let mut starts = vec![0];
    for s in silences {
        if s.end_ms - s.start_ms >= CHAPTER_PAUSE_MS && s.end_ms + 1_000 < file_duration_ms {
            let from = s.end_ms.saturating_sub(WINDOW_LEAD_MS);
            if from > starts.last().copied().unwrap_or(0) + WINDOW_MS / 2 {
                starts.push(from);
            }
        }
    }
    starts
}

/// The chapters for headings found in a book of `total_ms`, in book time:
/// repeats dropped, each ending where the next starts, and the book's start
/// covered.
pub fn chapters_from(mut found: Vec<Found>, total_ms: u64) -> Vec<Chapter> {
    found.sort_by_key(|f| f.start_ms);
    let mut kept: Vec<Found> = Vec::new();
    for f in found {
        let repeat = kept.last().is_some_and(|last| {
            f.start_ms < last.start_ms + SAME_CHAPTER_MS || f.heading == last.heading
        });
        if !repeat {
            kept.push(f);
        }
    }
    if let Some(first) = kept.first_mut() {
        if first.start_ms <= OPENING_MS {
            first.start_ms = 0;
        } else {
            kept.insert(
                0,
                Found {
                    title: "Opening".to_string(),
                    heading: "Opening".to_string(),
                    start_ms: 0,
                },
            );
        }
    }
    let ends: Vec<u64> = kept
        .iter()
        .skip(1)
        .map(|f| f.start_ms)
        .chain(std::iter::once(total_ms))
        .collect();
    kept.into_iter()
        .zip(ends)
        .enumerate()
        .map(|(index, (f, end))| Chapter {
            index,
            title: f.title,
            start_time_ms: f.start_ms,
            end_time_ms: end,
            auto_detected: true,
            placement: None,
        })
        .collect()
}

/// Splits the words of windows sent together back into each window, timed
/// from that window's start. `offsets` are where each window began in the
/// joined audio.
fn split_words(words: &[Word], offsets: &[u64]) -> Vec<Vec<Word>> {
    let mut per: Vec<Vec<Word>> = vec![Vec::new(); offsets.len()];
    for word in words {
        let k = offsets
            .iter()
            .rposition(|&o| word.start_ms + GAP_MS / 2 >= o)
            .unwrap_or(0);
        let offset = offsets[k];
        per[k].push(Word {
            text: word.text.clone(),
            start_ms: word.start_ms.saturating_sub(offset),
            end_ms: word.end_ms.saturating_sub(offset),
        });
    }
    per
}

/// Everything needed to listen through a book
pub struct Detection<'a> {
    pub transcriber: &'a Transcriber,
    pub ffmpeg: &'a Utf8Path,
    pub files: &'a [AudioFile],
    /// Whisper language code, or `None` to detect it
    pub language: Option<&'a str>,
    /// Set to stop; the detection then returns "cancelled"
    pub cancel: &'a AtomicBool,
}

/// The message a cancelled detection returns
pub const CANCELLED: &str = "cancelled";

impl Detection<'_> {
    /// The book's chapters, from the headings heard in it. `progress` is told
    /// how far along each stage is.
    pub fn run(&self, mut progress: impl FnMut(Stage, usize, usize)) -> AppResult<Vec<Chapter>> {
        let cancelled = || {
            if self.cancel.load(Ordering::SeqCst) {
                Err(AppError::Speech(CANCELLED.to_string()))
            } else {
                Ok(())
            }
        };

        // Every file's pauses, and where to listen in it
        let mut silences: Vec<Vec<Silence>> = Vec::with_capacity(self.files.len());
        let mut places: Vec<Place> = Vec::new();
        progress(Stage::Pauses, 0, self.files.len());
        for (i, file) in self.files.iter().enumerate() {
            cancelled()?;
            let found = find_silences(self.ffmpeg, &file.path, 0, file.duration_ms)
                .map_err(|e| warn!("Pause search failed in {}: {}", file.path, e))
                .unwrap_or_default();
            for from_ms in places_in(&found, file.duration_ms) {
                let len_ms = WINDOW_MS.min(file.duration_ms.saturating_sub(from_ms));
                if len_ms > 0 {
                    places.push(Place {
                        file: i,
                        from_ms,
                        len_ms,
                    });
                }
            }
            silences.push(found);
            progress(Stage::Pauses, i + 1, self.files.len());
        }
        info!("Listening for headings at {} places", places.len());

        let file_starts: Vec<u64> = self
            .files
            .iter()
            .scan(0u64, |at, f| {
                let start = *at;
                *at += f.duration_ms;
                Some(start)
            })
            .collect();
        let total_ms: u64 = self.files.iter().map(|f| f.duration_ms).sum();

        let mut found = Vec::new();
        let mut done = 0;
        progress(Stage::Listening, 0, places.len());
        for group in places.chunks(WINDOWS_PER_PASS) {
            cancelled()?;
            let mut samples: Vec<f32> = Vec::new();
            let mut offsets = Vec::with_capacity(group.len());
            for place in group {
                let file = &self.files[place.file];
                let window = read_window(self.ffmpeg, &file.path, place.from_ms, place.len_ms)
                    .map_err(|e| warn!("Could not read audio to listen: {}", e))
                    .unwrap_or_default();
                offsets.push(samples.len() as u64 * 1000 / SAMPLE_RATE as u64);
                samples.extend_from_slice(&window);
                samples.extend(std::iter::repeat_n(
                    0.0,
                    (GAP_MS * SAMPLE_RATE as u64 / 1000) as usize,
                ));
            }
            let words = self
                .transcriber
                .words(&samples, self.language)
                .map_err(|e| warn!("Could not listen: {}", e))
                .unwrap_or_default();
            for (place, words) in group.iter().zip(split_words(&words, &offsets)) {
                if let Some(heading) = heading_at_start(&words, HEADING_WITHIN_MS) {
                    let title_ms = place.from_ms + heading.start_ms;
                    let start = heard_start(title_ms, BREAK_BEFORE_MS, &silences[place.file]);
                    let title = match &heading.subtitle {
                        Some(subtitle) => format!("{}: {}", heading.title, subtitle),
                        None => heading.title.clone(),
                    };
                    found.push(Found {
                        title,
                        heading: heading.title,
                        start_ms: file_starts[place.file] + start,
                    });
                }
            }
            done += group.len();
            progress(Stage::Listening, done, places.len());
        }

        if found.is_empty() {
            return Err(AppError::Speech(
                "no chapter titles were heard in the audio".to_string(),
            ));
        }
        info!("Heard {} headings", found.len());
        Ok(chapters_from(found, total_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence(start_ms: u64, end_ms: u64) -> Silence {
        Silence { start_ms, end_ms }
    }

    fn found(title: &str, start_ms: u64) -> Found {
        Found {
            title: title.to_string(),
            heading: title.split(':').next().unwrap().to_string(),
            start_ms,
        }
    }

    #[test]
    fn listening_starts_at_the_file_start_and_after_long_pauses() {
        let silences = [
            silence(10_000, 10_600),   // a breath
            silence(60_000, 62_500),   // a chapter break
            silence(62_800, 65_000),   // too soon after the last
            silence(119_000, 121_000), // leaves too little to hear
        ];
        assert_eq!(places_in(&silences, 121_500), vec![0, 62_100]);
    }

    #[test]
    fn chapters_cover_the_book_without_repeats() {
        let chapters = chapters_from(
            vec![
                found("Chapter 2", 600_000),
                found("Chapter 1", 4_000),
                found("Chapter 2: The river", 900_000),
                found("Chapter 3", 1_200_000),
                found("Chapter 4", 1_205_000),
            ],
            2_000_000,
        );
        let got: Vec<(&str, u64, u64)> = chapters
            .iter()
            .map(|c| (c.title.as_str(), c.start_time_ms, c.end_time_ms))
            .collect();
        assert_eq!(
            got,
            [
                ("Chapter 1", 0, 600_000),
                ("Chapter 2", 600_000, 1_200_000),
                ("Chapter 3", 1_200_000, 2_000_000),
            ]
        );
        assert!(chapters.iter().enumerate().all(|(i, c)| c.index == i));
    }

    #[test]
    fn a_late_first_heading_gets_an_opening_before_it() {
        let chapters = chapters_from(vec![found("Chapter 1", 300_000)], 1_000_000);
        assert_eq!(chapters[0].title, "Opening");
        assert_eq!(chapters[0].end_time_ms, 300_000);
        assert_eq!(chapters[1].start_time_ms, 300_000);
    }

    #[test]
    fn words_sent_together_go_back_to_their_windows() {
        let word = |text: &str, start_ms: u64| Word {
            text: text.to_string(),
            start_ms,
            end_ms: start_ms + 300,
        };
        let words = [word("a", 100), word("Chapter", 8_200), word("b", 16_300)];
        let per = split_words(&words, &[0, 8_000, 16_000]);
        assert_eq!(per[0][0].start_ms, 100);
        assert_eq!(
            (per[1][0].text.as_str(), per[1][0].start_ms),
            ("Chapter", 200)
        );
        assert_eq!(per[2][0].start_ms, 300);
    }

    /// Two files made by macOS `say`, chapters announced after pauses. Needs
    /// FFmpeg and the model:
    /// `SPEECH_MODEL=<ggml-base-q5_1.bin> cargo test spoken_headings -- --ignored --nocapture`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn spoken_headings_in_real_audio_become_chapters() {
        use std::process::Command;
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let model = std::env::var("SPEECH_MODEL").expect("set SPEECH_MODEL");
        let transcriber = Transcriber::load(Utf8Path::new(&model)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = |name: &str| camino::Utf8PathBuf::from_path_buf(dir.path().join(name)).unwrap();
        let say = |text: &str, out: &camino::Utf8PathBuf| {
            assert!(Command::new("say")
                .arg("-o")
                .arg(out.as_str())
                .arg(text)
                .status()
                .unwrap()
                .success());
        };
        // Speech, three seconds of silence, speech, joined into one file
        let book_file = |name: &str, first: &str, second: &str| {
            let (a, b, out) = (
                path(&format!("{name}a.aiff")),
                path(&format!("{name}b.aiff")),
                path(&format!("{name}.m4a")),
            );
            say(first, &a);
            say(second, &b);
            let status = Command::new(ffmpeg.path.as_str())
                .args(["-v", "error", "-y", "-i", a.as_str()])
                .args(["-f", "lavfi", "-t", "3", "-i", "anullsrc=r=22050:cl=mono"])
                .args(["-i", b.as_str()])
                .args([
                    "-filter_complex",
                    "[0:a]aresample=22050[a];[2:a]aresample=22050[c];[a][1:a][c]concat=n=3:v=0:a=1",
                ])
                .arg(out.as_str())
                .status()
                .unwrap();
            assert!(status.success());
            let ms = (read_window(&ffmpeg.path, &out, 0, 600_000).unwrap().len() as u64) * 1000
                / SAMPLE_RATE as u64;
            AudioFile {
                path: out,
                format: crate::models::AudioFormat::M4a,
                duration_ms: ms,
                bitrate_kbps: None,
                sample_rate_hz: None,
                channels: None,
                file_size_bytes: 0,
                metadata: crate::models::Metadata::default(),
                has_artwork: false,
            }
        };
        let files = vec![
            book_file(
                "one",
                "Chapter one. The road. [[slnc 1000]] They set out at dawn, and the road was long. As we will see in chapter three, it did not end well.",
                "Chapter two. The river. [[slnc 1000]] By noon they had reached the water.",
            ),
            book_file(
                "two",
                "They crossed in silence and slept on the far bank.",
                "Chapter three. Outlaws. [[slnc 1000]] In the spring the men came down from the hills.",
            ),
        ];
        let cancel = AtomicBool::new(false);
        let detection = Detection {
            transcriber: &transcriber,
            ffmpeg: &ffmpeg.path,
            files: &files,
            language: Some("en"),
            cancel: &cancel,
        };
        let chapters = detection.run(|_, _, _| {}).unwrap();
        for c in &chapters {
            println!("{} at {} ms", c.title, c.start_time_ms);
        }
        let titles: Vec<&str> = chapters.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Chapter 1: The road",
                "Chapter 2: The river",
                "Chapter 3: Outlaws"
            ]
        );
        assert!(chapters[2].start_time_ms > files[0].duration_ms);
    }
}
