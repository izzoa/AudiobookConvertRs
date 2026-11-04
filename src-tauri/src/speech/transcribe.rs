//! Words and their times from a short slice of audio, with whisper.cpp.

use crate::error::{AppError, AppResult};
use camino::Utf8Path;
use whisper_rs::{
    DtwMode, DtwModelPreset, DtwParameters, FullParams, SamplingStrategy, WhisperContext,
    WhisperContextParameters,
};

/// The sample rate whisper.cpp expects.
pub const SAMPLE_RATE: u32 = 16_000;

/// One recognised word, timed from the start of the slice it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// A loaded speech model. Loading takes about half a second, so one is kept
/// for a whole fit and asked about each window in turn.
pub struct Transcriber {
    context: WhisperContext,
}

impl Transcriber {
    pub fn load(model: &Utf8Path) -> AppResult<Self> {
        // whisper.cpp writes progress to stderr; this sends it to tracing instead
        whisper_rs::install_logging_hooks();
        let mut params = WhisperContextParameters::default();
        // Plain token times drift by a second or more; DTW alignment, set up
        // for the base model, times each word to a tenth of a second or so
        params.dtw_parameters(DtwParameters {
            mode: DtwMode::ModelPreset {
                model_preset: DtwModelPreset::Base,
            },
            ..DtwParameters::default()
        });
        let context = WhisperContext::new_with_params(model.as_str(), params)
            .map_err(|e| AppError::Speech(format!("could not load the speech model: {}", e)))?;
        Ok(Self { context })
    }

    /// The words in `samples` (16 kHz mono). `language` is a Whisper code such
    /// as `en`; `None` lets the model detect it.
    pub fn words(&self, samples: &[f32], language: Option<&str>) -> AppResult<Vec<Word>> {
        let mut state = self
            .context
            .create_state()
            .map_err(|e| AppError::Speech(format!("could not start transcription: {}", e)))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_n_threads(threads());
        params.set_token_timestamps(true);
        // Each window stands alone; earlier text must not steer this one
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        state
            .full(params, samples)
            .map_err(|e| AppError::Speech(format!("transcription failed: {}", e)))?;

        // Tokens are pieces of words; one that starts with a space starts a word
        let first_special = self.context.token_eot();
        let mut words: Vec<Word> = Vec::new();
        for segment in state.as_iter() {
            for i in 0..segment.n_tokens() {
                let Some(token) = segment.get_token(i) else {
                    continue;
                };
                let data = token.token_data();
                if data.id >= first_special {
                    continue;
                }
                let piece = token
                    .to_str_lossy()
                    .map(|t| t.to_string())
                    .unwrap_or_default();
                // Times are in centiseconds; DTW's when it has one
                let at = if data.t_dtw >= 0 { data.t_dtw } else { data.t0 };
                let at_ms = at.max(0) as u64 * 10;
                let end_ms = (data.t1.max(0) as u64 * 10).max(at_ms);
                match words.last_mut() {
                    Some(word) if !piece.starts_with(' ') => {
                        word.text.push_str(&piece);
                        word.end_ms = word.end_ms.max(end_ms);
                    }
                    _ => {
                        let text = piece.trim().to_string();
                        if !text.is_empty() {
                            words.push(Word {
                                text,
                                start_ms: at_ms,
                                end_ms,
                            });
                        }
                    }
                }
            }
        }
        Ok(words)
    }
}

/// Up to eight threads; more gives little for a minute of audio.
fn threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Speech made by macOS `say`, as 16 kHz mono samples through FFmpeg.
    #[cfg(target_os = "macos")]
    fn spoken(text: &str) -> Vec<f32> {
        let dir = tempfile::tempdir().unwrap();
        let aiff = dir.path().join("speech.aiff");
        assert!(Command::new("say")
            .arg("-o")
            .arg(&aiff)
            .arg(text)
            .status()
            .unwrap()
            .success());
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let out = Command::new(ffmpeg.path.as_str())
            .args(["-v", "error", "-i"])
            .arg(&aiff)
            .args(["-ac", "1", "-ar", "16000", "-f", "f32le", "-"])
            .output()
            .unwrap();
        assert!(out.status.success());
        out.stdout
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }

    /// Needs the model file: `SPEECH_MODEL=<path to ggml-base-q5_1.bin>
    /// cargo test spoken_words -- --ignored`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn spoken_words_come_back_with_times() {
        let model = std::env::var("SPEECH_MODEL").expect("set SPEECH_MODEL");
        let transcriber = Transcriber::load(Utf8Path::new(&model)).unwrap();
        let samples = spoken("The end of the last chapter. Chapter four. Outlaws.");
        let started = std::time::Instant::now();
        let words = transcriber.words(&samples, Some("en")).unwrap();
        println!("{:?} in {:?}", words, started.elapsed());

        // The title word itself may be misheard ("Outlaws" came back as "August"),
        // which is why "chapter" plus the number is enough to match
        let text: Vec<String> = words.iter().map(|w| w.text.to_lowercase()).collect();
        let chapter = text
            .iter()
            .rposition(|w| w.starts_with("chapter"))
            .expect("heard 'chapter'");
        let number = text.get(chapter + 1).expect("a word after 'chapter'");
        assert!(
            number.starts_with('4') || number.starts_with("four"),
            "{text:?}"
        );
        assert!(words.windows(2).all(|w| w[0].start_ms <= w[1].start_ms));
        assert!(
            words[chapter].start_ms > 500,
            "the announcement follows other words"
        );
    }
}
