//! A window of a book's audio as whisper.cpp wants it: 16 kHz mono samples,
//! read through the resolved FFmpeg so every input format works.

use crate::error::{AppError, AppResult};
use crate::ffmpeg::silence::seconds;
use crate::speech::transcribe::SAMPLE_RATE;
use camino::Utf8Path;
use std::process::Command;

/// `len_ms` of `file` from `from_ms`, as 16 kHz mono f32 samples.
pub fn read_window(
    ffmpeg: &Utf8Path,
    file: &Utf8Path,
    from_ms: u64,
    len_ms: u64,
) -> AppResult<Vec<f32>> {
    let rate = SAMPLE_RATE.to_string();
    let output = Command::new(ffmpeg.as_str())
        .args([
            "-v",
            "error",
            "-ss",
            &seconds(from_ms),
            "-t",
            &seconds(len_ms),
        ])
        .args(["-i", file.as_str()])
        .args(["-ac", "1", "-ar", &rate, "-f", "f32le", "-"])
        .output()
        .map_err(|e| AppError::Speech(format!("could not read {}: {}", file, e)))?;
    if !output.status.success() {
        return Err(AppError::Speech(format!(
            "could not read {}: {}",
            file,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(parse_samples(&output.stdout))
}

/// Little-endian f32 samples; a trailing partial sample is dropped.
pub fn parse_samples(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_read_as_little_endian_floats() {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.5, -1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0x7f);
        assert_eq!(parse_samples(&bytes), [0.0, 0.5, -1.0]);
    }

    /// Run with `cargo test read_window -- --ignored` (needs FFmpeg).
    #[test]
    #[ignore]
    fn read_window_returns_sixteen_thousand_samples_a_second() {
        let ffmpeg = crate::ffmpeg::detection::detect_ffmpeg().expect("FFmpeg is needed");
        let dir = tempfile::tempdir().unwrap();
        let file = camino::Utf8PathBuf::from_path_buf(dir.path().join("tone.mp3")).unwrap();
        let status = Command::new(ffmpeg.path.as_str())
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=10",
            ])
            .arg(file.as_str())
            .status()
            .unwrap();
        assert!(status.success());
        let samples = read_window(&ffmpeg.path, &file, 2_000, 5_000).unwrap();
        assert_eq!(samples.len(), 80_000);
        assert!(samples.iter().any(|s| s.abs() > 0.1), "the tone is there");
    }
}
