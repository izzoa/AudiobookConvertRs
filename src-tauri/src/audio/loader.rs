//! Audio file loading with Symphonia
//!
//! Provides functions to load audio files and extract their properties
//! using the Symphonia library for format detection and decoding.

use crate::error::{AppError, AppResult};
use crate::metadata::extraction::{extract_from_revision, normalize_metadata};
use crate::models::{AudioFile, AudioFormat, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use regex::Regex;
use std::fs::File;
use std::io::BufReader;
use std::process::Command;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{debug, info, warn};

/// Load an audio file and extract all properties
pub fn load_audio_file(path: &Utf8Path) -> AppResult<AudioFile> {
    debug!("Loading audio file: {}", path);

    // Verify file exists
    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    // Get file size
    let file_size = std::fs::metadata(path.as_std_path())
        .map_err(|e| AppError::file_read(path, e.to_string()))?
        .len();

    // Open file for reading
    let file = File::open(path.as_std_path())
        .map_err(|e| AppError::file_read(path, e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint using the file extension
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext);
    }

    // Probe the file to get format
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AppError::FormatDetectionFailed(path.as_std_path().to_path_buf()))?;

    let mut format_reader = probed.format;

    // Get the default audio track
    let track_params = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .map(|t| (t.id, t.codec_params.clone()))
        .ok_or_else(|| AppError::NoAudioStream(path.as_std_path().to_path_buf()))?;

    let (track_id, codec_params) = track_params;

    // Extract metadata before any packet scanning consumes the reader
    let metadata = extract_metadata_from_reader(&mut format_reader, &mut probed.metadata, path);
    let has_artwork = metadata.artwork_data.is_some() || metadata.artwork_path.is_some();

    // Calculate duration
    let duration_ms = calculate_duration_from_track(&codec_params, track_id, &mut format_reader, path, file_size)?;

    // Detect format
    let format = detect_format_from_extension(path)?;

    // Extract sample rate and channels
    let sample_rate = codec_params.sample_rate;
    let channels = codec_params.channels.map(|c| c.count() as u8);

    // Estimate bitrate (if not directly available)
    let bitrate = estimate_bitrate(file_size, duration_ms);

    let audio_file = AudioFile {
        path: path.to_owned(),
        format,
        duration_ms,
        bitrate_kbps: bitrate,
        sample_rate_hz: sample_rate,
        channels,
        file_size_bytes: file_size,
        metadata,
        has_artwork,
    };

    info!(
        "Loaded audio file: {} ({}, {})",
        path,
        format,
        audio_file.formatted_duration()
    );

    Ok(audio_file)
}

/// Calculate duration from track information
fn calculate_duration_from_track(
    codec_params: &symphonia::core::codecs::CodecParameters,
    track_id: u32,
    format_reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    path: &Utf8Path,
    file_size: u64,
) -> AppResult<u64> {
    // Try to get duration from codec params
    if let Some(n_frames) = codec_params.n_frames {
        if n_frames > 0 {
            if let Some(sample_rate) = codec_params.sample_rate {
                if sample_rate > 0 {
                    let duration_sec = n_frames as f64 / sample_rate as f64;
                    return Ok((duration_sec * 1000.0) as u64);
                }
            }
            if let Some(time_base) = codec_params.time_base {
                let time = time_base.calc_time(n_frames);
                let duration_ms = (time.seconds as f64 + time.frac) * 1000.0;
                if duration_ms > 0.0 {
                    return Ok(duration_ms as u64);
                }
            }
        }
    }

    // Fallback 1: Probe duration via FFmpeg header output
    if let Some(ffmpeg_duration_ms) = probe_duration_with_ffmpeg(path) {
        if ffmpeg_duration_ms > 0 {
            debug!("Resolved duration via FFmpeg probe for {}: {} ms", path, ffmpeg_duration_ms);
            return Ok(ffmpeg_duration_ms);
        }
    }

    // Fallback 2: Scan packet durations from Symphonia FormatReader
    if let Some(packet_duration_ms) = scan_packets_duration_ms(format_reader, track_id, codec_params) {
        if packet_duration_ms > 0 {
            debug!("Resolved duration via packet scan for {}: {} ms", path, packet_duration_ms);
            return Ok(packet_duration_ms);
        }
    }

    // Fallback 3: Estimate from file size assuming 128 kbps nominal bitrate
    if let Some(estimated_ms) = estimate_duration_from_file_size(file_size, 128) {
        warn!(
            "Could not determine exact duration for {}, using file-size estimate ({} ms)",
            path, estimated_ms
        );
        return Ok(estimated_ms);
    }

    warn!("Could not determine duration for {}", path);
    Ok(0)
}

/// Probe duration using FFmpeg `-i` output
fn probe_duration_with_ffmpeg(path: &Utf8Path) -> Option<u64> {
    let ffmpeg_bin = crate::ffmpeg::detection::detect_ffmpeg()
        .map(|info| info.path)
        .unwrap_or_else(|_| Utf8PathBuf::from("ffmpeg"));

    let output = Command::new(ffmpeg_bin.as_str())
        .args(["-hide_banner", "-i", path.as_str()])
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_ffmpeg_duration_ms(&stderr)
}

/// Parse `Duration: HH:MM:SS.xx` from FFmpeg stderr output into milliseconds
pub fn parse_ffmpeg_duration_ms(stderr: &str) -> Option<u64> {
    let re = Regex::new(r"Duration:\s*(\d+):(\d{2}):(\d{2})(?:\.(\d+))?").ok()?;
    let caps = re.captures(stderr)?;
    let hours: u64 = caps.get(1)?.as_str().parse().ok()?;
    let minutes: u64 = caps.get(2)?.as_str().parse().ok()?;
    let seconds: u64 = caps.get(3)?.as_str().parse().ok()?;
    let millis: u64 = if let Some(frac_match) = caps.get(4) {
        let frac_str = frac_match.as_str();
        let truncated = if frac_str.len() > 3 { &frac_str[..3] } else { frac_str };
        let padding = 3 - truncated.len();
        truncated.parse::<u64>().ok()? * 10u64.pow(padding as u32)
    } else {
        0
    };
    let total_ms = hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis;
    if total_ms > 0 { Some(total_ms) } else { None }
}

/// Scan packets in the format reader to sum track duration
fn scan_packets_duration_ms(
    format_reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    track_id: u32,
    codec_params: &symphonia::core::codecs::CodecParameters,
) -> Option<u64> {
    let mut total_dur: u64 = 0;
    let mut max_end_ts: u64 = 0;
    while let Ok(packet) = format_reader.next_packet() {
        if packet.track_id() == track_id {
            total_dur = total_dur.saturating_add(packet.dur());
            let end_ts = packet.ts().saturating_add(packet.dur());
            if end_ts > max_end_ts {
                max_end_ts = end_ts;
            }
        }
    }
    let units = total_dur.max(max_end_ts);
    if units == 0 {
        return None;
    }
    if let Some(time_base) = codec_params.time_base {
        let time = time_base.calc_time(units);
        let ms = ((time.seconds as f64 + time.frac) * 1000.0) as u64;
        if ms > 0 {
            return Some(ms);
        }
    }
    if let Some(sample_rate) = codec_params.sample_rate {
        if sample_rate > 0 {
            let ms = (units as f64 / sample_rate as f64 * 1000.0) as u64;
            if ms > 0 {
                return Some(ms);
            }
        }
    }
    None
}

/// Estimate duration in milliseconds from file size and nominal bitrate (kbps)
pub fn estimate_duration_from_file_size(file_size: u64, nominal_bitrate_kbps: u32) -> Option<u64> {
    if file_size == 0 || nominal_bitrate_kbps == 0 {
        return None;
    }
    let bits = file_size as f64 * 8.0;
    let bits_per_ms = nominal_bitrate_kbps as f64;
    let duration_ms = (bits / bits_per_ms).round() as u64;
    Some(duration_ms.max(1))
}

/// Detect format from file extension
fn detect_format_from_extension(path: &Utf8Path) -> AppResult<AudioFormat> {
    let ext = path
        .extension()
        .ok_or_else(|| AppError::FormatDetectionFailed(path.as_std_path().to_path_buf()))?;

    AudioFormat::from_extension(ext)
        .ok_or_else(|| AppError::UnsupportedFormat(ext.to_string()))
}

/// Estimate bitrate from file size and duration
fn estimate_bitrate(file_size: u64, duration_ms: u64) -> Option<u32> {
    if duration_ms > 0 {
        // bitrate = (file_size * 8) / duration_sec / 1000
        let duration_sec = duration_ms as f64 / 1000.0;
        let bitrate = (file_size as f64 * 8.0 / duration_sec / 1000.0) as u32;
        Some(bitrate)
    } else {
        None
    }
}

/// Extract metadata from a probed file.
///
/// Symphonia exposes tags in two places and which one is populated depends on
/// the container: MP4/FLAC surface them on the format reader, while an MP3's
/// ID3v2 block sits ahead of the stream and lands on the probe result instead.
/// Reading only the reader is why MP3 imports used to come back untagged, with
/// every field falling through to the filename.
fn extract_metadata_from_reader(
    format_reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    probed_metadata: &mut symphonia::core::probe::ProbedMetadata,
    path: &Utf8Path,
) -> Metadata {
    let mut metadata = Metadata::default();

    if let Some(meta_rev) = format_reader.metadata().current() {
        extract_from_revision(&mut metadata, meta_rev);
    }

    if let Some(probe_meta) = probed_metadata.get() {
        if let Some(meta_rev) = probe_meta.current() {
            extract_from_revision(&mut metadata, meta_rev);
        }
    }

    normalize_metadata(&mut metadata);

    // Only once real tags have been exhausted is the filename worth guessing from.
    if metadata.title.is_none() {
        metadata.title = path.file_stem().map(|s| s.to_string());
    }

    metadata
}

/// Load multiple audio files
pub fn load_audio_files(paths: &[Utf8PathBuf]) -> Vec<AppResult<AudioFile>> {
    paths.iter().map(|p| load_audio_file(p)).collect()
}

/// Quick duration check without full file loading
pub fn get_duration_ms(path: &Utf8Path) -> AppResult<u64> {
    let audio_file = load_audio_file(path)?;
    Ok(audio_file.duration_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_from_extension() {
        let path = Utf8Path::new("/test/file.mp3");
        let format = detect_format_from_extension(path).unwrap();
        assert_eq!(format, AudioFormat::Mp3);

        let path = Utf8Path::new("/test/file.flac");
        let format = detect_format_from_extension(path).unwrap();
        assert_eq!(format, AudioFormat::Flac);

        let path = Utf8Path::new("/test/file.unknown");
        assert!(detect_format_from_extension(path).is_err());
    }

    #[test]
    fn test_estimate_bitrate() {
        // 128 kbps for 60 seconds = 960,000 bytes
        let bitrate = estimate_bitrate(960_000, 60_000);
        assert!(bitrate.is_some());
        // Allow some tolerance for rounding
        let br = bitrate.unwrap();
        assert!(br > 120 && br < 140);
    }

    #[test]
    fn test_parse_ffmpeg_duration_ms() {
        let stderr = "  Duration: 01:02:03.45, start: 0.000000, bitrate: 128 kb/s";
        let ms = parse_ffmpeg_duration_ms(stderr).unwrap();
        assert_eq!(ms, (3600 + 120 + 3) * 1000 + 450);

        let stderr_short = "  Duration: 00:00:12.08, start: 0.025057, bitrate: 64 kb/s";
        assert_eq!(parse_ffmpeg_duration_ms(stderr_short), Some(12_080));

        let stderr_na = "  Duration: N/A, bitrate: N/A";
        assert_eq!(parse_ffmpeg_duration_ms(stderr_na), None);
    }

    #[test]
    fn test_estimate_duration_from_file_size() {
        // 960,000 bytes at 128 kbps = 60,000 ms
        assert_eq!(estimate_duration_from_file_size(960_000, 128), Some(60_000));
        assert_eq!(estimate_duration_from_file_size(0, 128), None);
        assert_eq!(estimate_duration_from_file_size(100, 128), Some(6));
    }
}

