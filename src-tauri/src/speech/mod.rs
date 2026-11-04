//! Speech: hearing chapter titles in a book's audio, so fitted chapters can
//! start where the narrator announces them.
//!
//! Everything runs on this computer with whisper.cpp. The model is an
//! optional download; without it, fitting uses track starts and pauses only.

pub mod audio;
pub mod hear;
pub mod matcher;
pub mod model;
pub mod transcribe;

pub use transcribe::{Transcriber, Word};

/// The Whisper language code for a book language as the lookup names it
/// ("English"), so the model need not guess
pub fn whisper_language(name: &str) -> Option<&'static str> {
    let code = match name.trim().to_lowercase().as_str() {
        "english" => "en",
        "german" | "deutsch" => "de",
        "french" | "français" | "francais" => "fr",
        "spanish" | "español" | "espanol" => "es",
        "italian" | "italiano" => "it",
        "japanese" | "日本語" => "ja",
        "portuguese" | "português" => "pt",
        "dutch" | "nederlands" => "nl",
        _ => return None,
    };
    Some(code)
}
