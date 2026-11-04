//! Listening near a fitted chapter's estimate for its title, and starting the
//! chapter just before the announcement.

use crate::ffmpeg::silence::{find_silences, Silence, LEAD_IN_MS};
use crate::speech::audio::read_window;
use crate::speech::matcher::find_announcement;
use crate::speech::Transcriber;
use camino::Utf8Path;
use tracing::warn;

/// How far from the estimate the title is first listened for; most land here
const NEAR_MS: u64 = 30_000;
/// How far it is listened for when it was not heard nearer
pub const LISTEN_MS: u64 = 90_000;
/// A silence must end this close before the title to be the chapter break
pub const BREAK_BEFORE_MS: u64 = 1_500;
/// How much further back the break may be when the title was matched on its
/// words and the "chapter 4" before them was not recognized
const UNHEARD_LEAD_IN_MS: u64 = 1_000;
/// Whisper may time a word's start a little early, inside the silence
const WORD_START_SLACK_MS: u64 = 300;
/// Where the chapter starts when no silence comes before the title
const NO_BREAK_LEAD_MS: u64 = 300;

/// Where a chapter whose spoken title starts at `title_ms` should start: in
/// the last silence ending at most `break_before_ms` before the title,
/// shortly before speech resumes, or a little before the title when there is
/// none.
pub fn heard_start(title_ms: u64, break_before_ms: u64, silences: &[Silence]) -> u64 {
    silences
        .iter()
        .filter(|s| {
            s.end_ms <= title_ms + WORD_START_SLACK_MS
                && title_ms.saturating_sub(s.end_ms) <= break_before_ms
        })
        .max_by_key(|s| s.end_ms)
        .map(|s| s.end_ms.saturating_sub(LEAD_IN_MS).max(s.start_ms))
        .unwrap_or_else(|| title_ms.saturating_sub(NO_BREAK_LEAD_MS))
}

/// One file to listen in
pub struct Listening<'a> {
    pub transcriber: &'a Transcriber,
    pub ffmpeg: &'a Utf8Path,
    pub file: &'a Utf8Path,
    pub file_duration_ms: u64,
    /// Whisper language code, or `None` to detect it
    pub language: Option<&'a str>,
}

impl Listening<'_> {
    /// Where the chapter `title` starts in the file, when it is heard within
    /// [`LISTEN_MS`] of `estimate_ms` (file time). Any failure counts as not
    /// heard, so the fit falls back to pauses.
    pub fn hear(&self, title: &str, estimate_ms: u64) -> Option<u64> {
        self.hear_within(title, estimate_ms, NEAR_MS)
            .or_else(|| self.hear_within(title, estimate_ms, LISTEN_MS))
    }

    fn hear_within(&self, title: &str, estimate_ms: u64, reach_ms: u64) -> Option<u64> {
        let from = estimate_ms.saturating_sub(reach_ms);
        let to = (estimate_ms + reach_ms).min(self.file_duration_ms);
        if to <= from {
            return None;
        }
        let samples = read_window(self.ffmpeg, self.file, from, to - from)
            .map_err(|e| warn!("Could not read audio to listen: {}", e))
            .ok()?;
        let mut words = self
            .transcriber
            .words(&samples, self.language)
            .map_err(|e| warn!("Could not listen: {}", e))
            .ok()?;
        for word in &mut words {
            word.start_ms += from;
            word.end_ms += from;
        }
        let heard = find_announcement(title, &words, estimate_ms)?;
        let silences = find_silences(self.ffmpeg, self.file, from, to - from).unwrap_or_default();
        let break_before_ms = if heard.lead_in_unheard {
            BREAK_BEFORE_MS + UNHEARD_LEAD_IN_MS
        } else {
            BREAK_BEFORE_MS
        };
        Some(heard_start(heard.start_ms, break_before_ms, &silences))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence(start_ms: u64, end_ms: u64) -> Silence {
        Silence { start_ms, end_ms }
    }

    #[test]
    fn the_chapter_starts_in_the_silence_before_the_title() {
        let silences = [silence(10_000, 11_000), silence(20_000, 21_300)];
        assert_eq!(heard_start(21_500, BREAK_BEFORE_MS, &silences), 21_100);
    }

    #[test]
    fn a_title_timed_slightly_early_still_uses_its_silence() {
        // Whisper put the first word inside the silence
        assert_eq!(
            heard_start(21_200, BREAK_BEFORE_MS, &[silence(20_000, 21_300)]),
            21_100
        );
    }

    #[test]
    fn a_distant_silence_is_not_the_break() {
        // The last silence ended 4 s before the title
        assert_eq!(
            heard_start(25_000, BREAK_BEFORE_MS, &[silence(20_000, 21_000)]),
            24_700
        );
    }

    #[test]
    fn an_unheard_lead_in_lets_the_break_come_earlier() {
        // "Number 5. China…": the title words came 1.6 s after the break
        let silences = [silence(20_000, 23_976)];
        assert_eq!(heard_start(25_620, BREAK_BEFORE_MS, &silences), 25_320);
        assert_eq!(
            heard_start(25_620, BREAK_BEFORE_MS + UNHEARD_LEAD_IN_MS, &silences),
            23_776
        );
    }

    #[test]
    fn with_no_silence_the_chapter_starts_just_before_the_title() {
        assert_eq!(heard_start(5_000, BREAK_BEFORE_MS, &[]), 4_700);
        assert_eq!(heard_start(100, BREAK_BEFORE_MS, &[]), 0);
    }

    /// The end of one chapter, 2 s of silence, then an announcement, all made
    /// by macOS `say`. Needs FFmpeg and the model:
    /// `SPEECH_MODEL=<ggml-base-q5_1.bin> cargo test announcement -- --ignored`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn an_announcement_in_real_audio_is_placed_at_its_break() {
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
        let (before, after, book) = (path("before.aiff"), path("after.aiff"), path("book.wav"));
        say(
            "They rode on into the night, and that was the end of the old life.",
            &before,
        );
        say(
            "Chapter Four. Outlaws. In the spring the men came down from the hills.",
            &after,
        );
        let status = Command::new(ffmpeg.path.as_str())
            .args(["-v", "error", "-y", "-i", before.as_str()])
            .args(["-f", "lavfi", "-t", "2", "-i", "anullsrc=r=22050:cl=mono"])
            .args(["-i", after.as_str()])
            .args([
                "-filter_complex",
                "[0:a]aresample=22050[a];[2:a]aresample=22050[c];[a][1:a][c]concat=n=3:v=0:a=1",
            ])
            .arg(book.as_str())
            .status()
            .unwrap();
        assert!(status.success());

        let before_ms =
            (read_window(&ffmpeg.path, &before, 0, 60_000).unwrap().len() as u64) * 1000 / 16_000;
        let book_ms =
            (read_window(&ffmpeg.path, &book, 0, 120_000).unwrap().len() as u64) * 1000 / 16_000;
        let announcement_ms = before_ms + 2_000;
        let listening = Listening {
            transcriber: &transcriber,
            ffmpeg: &ffmpeg.path,
            file: &book,
            file_duration_ms: book_ms,
            language: Some("en"),
        };
        // An estimate 3 s off, as a scaled start would be
        let start = listening
            .hear("Chapter 4: Outlaws", announcement_ms + 3_000)
            .expect("heard");
        println!("announcement at {announcement_ms} ms, chapter starts at {start} ms");
        assert!(
            start.abs_diff(announcement_ms) <= 500,
            "start {start} vs {announcement_ms}"
        );
    }
}
