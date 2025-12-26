//! Audio quality detection and normalization module
//!
//! Provides functionality for:
//! - Analyzing audio levels (peak, RMS, loudness)
//! - Suggesting normalization settings
//! - Applying normalization using FFmpeg loudnorm filter

use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

/// Audio level analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLevels {
    /// Input integrated loudness in LUFS
    pub integrated_loudness_lufs: f32,
    /// Input loudness range in LU
    pub loudness_range_lu: f32,
    /// Input true peak in dBTP
    pub true_peak_dbtp: f32,
    /// Input threshold in LUFS
    pub threshold_lufs: f32,
    /// Target offset from -24 LUFS
    pub target_offset: f32,
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            integrated_loudness_lufs: -24.0,
            loudness_range_lu: 7.0,
            true_peak_dbtp: -2.0,
            threshold_lufs: -34.0,
            target_offset: 0.0,
        }
    }
}

/// Normalization suggestion based on audio analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationSuggestion {
    /// Whether normalization is recommended
    pub recommended: bool,
    /// Reason for the recommendation
    pub reason: String,
    /// Target integrated loudness in LUFS (typically -16 to -24)
    pub target_lufs: f32,
    /// Target true peak in dBTP (typically -1 to -2)
    pub target_true_peak: f32,
    /// Target loudness range in LU
    pub target_lra: f32,
}

impl Default for NormalizationSuggestion {
    fn default() -> Self {
        Self {
            recommended: false,
            reason: "Audio levels are within acceptable range".to_string(),
            target_lufs: -16.0,
            target_true_peak: -1.5,
            target_lra: 11.0,
        }
    }
}

/// Normalization settings for apply_normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationSettings {
    /// Target integrated loudness in LUFS
    pub target_lufs: f32,
    /// Target true peak in dBTP
    pub target_true_peak: f32,
    /// Target loudness range in LU
    pub target_lra: f32,
    /// Whether to use two-pass normalization for better accuracy
    pub two_pass: bool,
}

/// Progress event emitted during normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationProgress {
    /// Progress percentage (0-100)
    pub percent: f32,
    /// Current stage: "analyzing" or "normalizing"
    pub stage: String,
}

impl Default for NormalizationSettings {
    fn default() -> Self {
        Self {
            target_lufs: -16.0,
            target_true_peak: -1.5,
            target_lra: 11.0,
            two_pass: true,
        }
    }
}

/// Analyze audio levels using FFmpeg's loudnorm filter in measurement mode
///
/// This performs a first-pass analysis to measure:
/// - Integrated loudness (LUFS)
/// - Loudness range (LU)
/// - True peak (dBTP)
pub fn analyze_audio_levels(file_path: &Utf8Path) -> AppResult<AudioLevels> {
    if !file_path.exists() {
        return Err(AppError::FileNotFound(file_path.as_std_path().to_path_buf()));
    }

    info!("Analyzing audio levels: {}", file_path);

    let output = Command::new("ffmpeg")
        .args([
            "-i", file_path.as_str(),
            "-af", "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json",
            "-f", "null",
            "-"
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::FFmpegNotFound
            } else {
                AppError::FFmpegExecutionError {
                    reason: e.to_string(),
                }
            }
        })?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    debug!("FFmpeg loudnorm output: {}", stderr);

    parse_loudnorm_output(&stderr)
}

/// Parse the JSON output from FFmpeg's loudnorm filter
fn parse_loudnorm_output(output: &str) -> AppResult<AudioLevels> {
    let re_input_i = Regex::new(r#""input_i"\s*:\s*"(-?[\d.]+)""#).unwrap();
    let re_input_lra = Regex::new(r#""input_lra"\s*:\s*"(-?[\d.]+)""#).unwrap();
    let re_input_tp = Regex::new(r#""input_tp"\s*:\s*"(-?[\d.]+)""#).unwrap();
    let re_input_thresh = Regex::new(r#""input_thresh"\s*:\s*"(-?[\d.]+)""#).unwrap();
    let re_target_offset = Regex::new(r#""target_offset"\s*:\s*"(-?[\d.]+)""#).unwrap();

    let integrated_loudness = re_input_i
        .captures(output)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(-24.0);

    let loudness_range = re_input_lra
        .captures(output)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(7.0);

    let true_peak = re_input_tp
        .captures(output)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(-2.0);

    let threshold = re_input_thresh
        .captures(output)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(-34.0);

    let target_offset = re_target_offset
        .captures(output)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(0.0);

    Ok(AudioLevels {
        integrated_loudness_lufs: integrated_loudness,
        loudness_range_lu: loudness_range,
        true_peak_dbtp: true_peak,
        threshold_lufs: threshold,
        target_offset,
    })
}

/// Suggest normalization settings based on audio level analysis
///
/// Recommends normalization if:
/// - Loudness is significantly different from target (-16 LUFS for streaming)
/// - True peak exceeds -1 dBTP (risk of clipping)
/// - Loudness range is very high (inconsistent volume)
pub fn suggest_normalization(levels: &AudioLevels) -> NormalizationSuggestion {
    let target_lufs = -16.0;
    let target_true_peak = -1.5;
    let target_lra = 11.0;

    let loudness_diff = (levels.integrated_loudness_lufs - target_lufs).abs();
    let peak_issue = levels.true_peak_dbtp > -1.0;
    let range_issue = levels.loudness_range_lu > 20.0;

    let mut reasons = Vec::new();

    if loudness_diff > 3.0 {
        reasons.push(format!(
            "Loudness ({:.1} LUFS) differs from target ({:.1} LUFS) by {:.1} LU",
            levels.integrated_loudness_lufs, target_lufs, loudness_diff
        ));
    }

    if peak_issue {
        reasons.push(format!(
            "True peak ({:.1} dBTP) exceeds safe limit (-1.0 dBTP)",
            levels.true_peak_dbtp
        ));
    }

    if range_issue {
        reasons.push(format!(
            "Loudness range ({:.1} LU) is very high, indicating inconsistent volume",
            levels.loudness_range_lu
        ));
    }

    let recommended = !reasons.is_empty();
    let reason = if recommended {
        reasons.join("; ")
    } else {
        "Audio levels are within acceptable range".to_string()
    };

    NormalizationSuggestion {
        recommended,
        reason,
        target_lufs,
        target_true_peak,
        target_lra,
    }
}

/// Apply audio normalization using FFmpeg's loudnorm filter
///
/// Uses two-pass normalization for better accuracy:
/// 1. First pass: analyze audio levels
/// 2. Second pass: apply normalization with measured values
pub fn apply_normalization(
    input: &Utf8Path,
    output: &Utf8Path,
    settings: &NormalizationSettings,
) -> AppResult<()> {
    if !input.exists() {
        return Err(AppError::FileNotFound(input.as_std_path().to_path_buf()));
    }

    info!("Applying normalization: {} -> {}", input, output);
    info!(
        "Target: {:.1} LUFS, {:.1} dBTP, {:.1} LU range",
        settings.target_lufs, settings.target_true_peak, settings.target_lra
    );

    if settings.two_pass {
        apply_two_pass_normalization(input, output, settings)
    } else {
        apply_single_pass_normalization(input, output, settings)
    }
}

/// Single-pass normalization (faster but less accurate)
fn apply_single_pass_normalization(
    input: &Utf8Path,
    output: &Utf8Path,
    settings: &NormalizationSettings,
) -> AppResult<()> {
    let filter = format!(
        "loudnorm=I={}:TP={}:LRA={}",
        settings.target_lufs, settings.target_true_peak, settings.target_lra
    );

    let result = Command::new("ffmpeg")
        .args([
            "-i", input.as_str(),
            "-af", &filter,
            "-ar", "44100",
            "-y",
            output.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: e.to_string(),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        warn!("FFmpeg normalization failed: {}", stderr);
        return Err(AppError::FFmpegExecutionError {
            reason: format!("Normalization failed: {}", stderr),
        });
    }

    info!("Single-pass normalization complete: {}", output);
    Ok(())
}

/// Two-pass normalization (more accurate)
fn apply_two_pass_normalization(
    input: &Utf8Path,
    output: &Utf8Path,
    settings: &NormalizationSettings,
) -> AppResult<()> {
    // First pass: analyze
    let levels = analyze_audio_levels(input)?;

    // Second pass: apply with measured values
    let filter = format!(
        "loudnorm=I={}:TP={}:LRA={}:measured_I={}:measured_LRA={}:measured_TP={}:measured_thresh={}:offset={}:linear=true",
        settings.target_lufs,
        settings.target_true_peak,
        settings.target_lra,
        levels.integrated_loudness_lufs,
        levels.loudness_range_lu,
        levels.true_peak_dbtp,
        levels.threshold_lufs,
        levels.target_offset,
    );

    debug!("Two-pass loudnorm filter: {}", filter);

    let result = Command::new("ffmpeg")
        .args([
            "-i", input.as_str(),
            "-af", &filter,
            "-ar", "44100",
            "-y",
            output.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: e.to_string(),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        warn!("FFmpeg two-pass normalization failed: {}", stderr);
        return Err(AppError::FFmpegExecutionError {
            reason: format!("Two-pass normalization failed: {}", stderr),
        });
    }

    info!("Two-pass normalization complete: {}", output);
    Ok(())
}

/// Apply audio normalization with progress events
pub fn apply_normalization_with_progress(
    input: &Utf8Path,
    output: &Utf8Path,
    settings: &NormalizationSettings,
    total_duration_ms: u64,
    handle: &AppHandle,
) -> AppResult<()> {
    if !input.exists() {
        return Err(AppError::FileNotFound(input.as_std_path().to_path_buf()));
    }

    info!("Applying normalization with progress: {} -> {}", input, output);

    let _ = handle.emit("normalization_progress", NormalizationProgress {
        percent: 0.0,
        stage: "analyzing".to_string(),
    });

    let levels = analyze_audio_levels(input)?;

    let _ = handle.emit("normalization_progress", NormalizationProgress {
        percent: 10.0,
        stage: "normalizing".to_string(),
    });

    let filter = format!(
        "loudnorm=I={}:TP={}:LRA={}:measured_I={}:measured_LRA={}:measured_TP={}:measured_thresh={}:offset={}:linear=true",
        settings.target_lufs,
        settings.target_true_peak,
        settings.target_lra,
        levels.integrated_loudness_lufs,
        levels.loudness_range_lu,
        levels.true_peak_dbtp,
        levels.threshold_lufs,
        levels.target_offset,
    );

    let mut child = Command::new("ffmpeg")
        .args([
            "-i", input.as_str(),
            "-af", &filter,
            "-ar", "44100",
            "-y",
            output.as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::FFmpegExecutionError {
            reason: e.to_string(),
        })?;

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let time_regex = Regex::new(r"time=(\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(caps) = time_regex.captures(&line) {
                let hours: u64 = caps[1].parse().unwrap_or(0);
                let mins: u64 = caps[2].parse().unwrap_or(0);
                let secs: u64 = caps[3].parse().unwrap_or(0);
                let centis: u64 = caps[4].parse().unwrap_or(0);
                let current_ms = hours * 3600000 + mins * 60000 + secs * 1000 + centis * 10;

                let percent = if total_duration_ms > 0 {
                    (current_ms as f32 / total_duration_ms as f32 * 90.0) + 10.0
                } else {
                    50.0
                };

                let _ = handle.emit("normalization_progress", NormalizationProgress {
                    percent: percent.min(99.0),
                    stage: "normalizing".to_string(),
                });
            }
        }
    }

    let result = child.wait().map_err(|e| AppError::FFmpegExecutionError {
        reason: e.to_string(),
    })?;

    if !result.success() {
        return Err(AppError::FFmpegExecutionError {
            reason: "Normalization failed".to_string(),
        });
    }

    let _ = handle.emit("normalization_progress", NormalizationProgress {
        percent: 100.0,
        stage: "complete".to_string(),
    });

    info!("Normalization with progress complete: {}", output);
    Ok(())
}

/// Analyze multiple files and return average levels
pub fn analyze_batch_levels(file_paths: &[Utf8PathBuf]) -> AppResult<AudioLevels> {
    if file_paths.is_empty() {
        return Ok(AudioLevels::default());
    }

    let mut total_loudness = 0.0;
    let mut total_range = 0.0;
    let mut max_peak = f32::MIN;
    let mut count = 0;

    for path in file_paths {
        match analyze_audio_levels(path) {
            Ok(levels) => {
                total_loudness += levels.integrated_loudness_lufs;
                total_range += levels.loudness_range_lu;
                if levels.true_peak_dbtp > max_peak {
                    max_peak = levels.true_peak_dbtp;
                }
                count += 1;
            }
            Err(e) => {
                warn!("Failed to analyze {}: {}", path, e);
            }
        }
    }

    if count == 0 {
        return Err(AppError::ValidationError("No files could be analyzed".to_string()));
    }

    Ok(AudioLevels {
        integrated_loudness_lufs: total_loudness / count as f32,
        loudness_range_lu: total_range / count as f32,
        true_peak_dbtp: max_peak,
        threshold_lufs: -34.0,
        target_offset: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization_suggestion_quiet_audio() {
        let levels = AudioLevels {
            integrated_loudness_lufs: -30.0, // Very quiet
            loudness_range_lu: 8.0,
            true_peak_dbtp: -10.0,
            threshold_lufs: -40.0,
            target_offset: 0.0,
        };

        let suggestion = suggest_normalization(&levels);
        assert!(suggestion.recommended);
        assert!(suggestion.reason.contains("differs from target"));
    }

    #[test]
    fn test_normalization_suggestion_clipping() {
        let levels = AudioLevels {
            integrated_loudness_lufs: -16.0,
            loudness_range_lu: 8.0,
            true_peak_dbtp: 0.5, // Clipping
            threshold_lufs: -26.0,
            target_offset: 0.0,
        };

        let suggestion = suggest_normalization(&levels);
        assert!(suggestion.recommended);
        assert!(suggestion.reason.contains("True peak"));
    }

    #[test]
    fn test_normalization_suggestion_ok() {
        let levels = AudioLevels {
            integrated_loudness_lufs: -16.0,
            loudness_range_lu: 8.0,
            true_peak_dbtp: -2.0,
            threshold_lufs: -26.0,
            target_offset: 0.0,
        };

        let suggestion = suggest_normalization(&levels);
        assert!(!suggestion.recommended);
    }

    #[test]
    fn test_parse_loudnorm_output() {
        let output = r#"
        {
            "input_i" : "-24.50",
            "input_tp" : "-3.20",
            "input_lra" : "12.30",
            "input_thresh" : "-35.00",
            "output_i" : "-16.00",
            "output_tp" : "-1.50",
            "output_lra" : "11.00",
            "output_thresh" : "-26.50",
            "normalization_type" : "dynamic",
            "target_offset" : "0.50"
        }
        "#;

        let levels = parse_loudnorm_output(output).unwrap();
        assert!((levels.integrated_loudness_lufs - (-24.5)).abs() < 0.01);
        assert!((levels.true_peak_dbtp - (-3.2)).abs() < 0.01);
        assert!((levels.loudness_range_lu - 12.3).abs() < 0.01);
    }
}
