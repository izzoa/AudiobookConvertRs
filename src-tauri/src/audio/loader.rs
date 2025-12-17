//! Audio file loading with Symphonia
//!
//! Provides functions to load audio files and extract their properties
//! using the Symphonia library for format detection and decoding.

use crate::error::{AppError, AppResult};
use crate::models::{AudioFile, AudioFormat, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs::File;
use std::io::BufReader;
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

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AppError::FormatDetectionFailed(path.as_std_path().to_path_buf()))?;

    let mut format_reader = probed.format;

    // Get the default audio track
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AppError::NoAudioStream(path.as_std_path().to_path_buf()))?;

    // Extract codec parameters
    let codec_params = &track.codec_params;

    // Calculate duration
    let duration_ms = calculate_duration_from_track(track, &format_reader)?;

    // Detect format
    let format = detect_format_from_extension(path)?;

    // Extract sample rate and channels
    let sample_rate = codec_params.sample_rate;
    let channels = codec_params.channels.map(|c| c.count() as u8);

    // Estimate bitrate (if not directly available)
    let bitrate = estimate_bitrate(file_size, duration_ms);

    // Extract metadata
    let metadata = extract_metadata_from_reader(&mut format_reader, path);
    let has_artwork = metadata.artwork_data.is_some() || metadata.artwork_path.is_some();

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
    track: &symphonia::core::formats::Track,
    format_reader: &Box<dyn symphonia::core::formats::FormatReader>,
) -> AppResult<u64> {
    let codec_params = &track.codec_params;

    // Try to get duration from codec params
    if let Some(n_frames) = codec_params.n_frames {
        if let Some(sample_rate) = codec_params.sample_rate {
            let duration_sec = n_frames as f64 / sample_rate as f64;
            return Ok((duration_sec * 1000.0) as u64);
        }
    }

    // Try time base
    if let Some(time_base) = codec_params.time_base {
        if let Some(n_frames) = codec_params.n_frames {
            let duration_sec = time_base.calc_time(n_frames).seconds as f64;
            return Ok((duration_sec * 1000.0) as u64);
        }
    }

    // Fallback: estimate from file
    warn!("Could not determine exact duration, using estimate");
    Ok(0) // Will be estimated later if needed
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

/// Extract metadata from format reader
fn extract_metadata_from_reader(
    format_reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    path: &Utf8Path,
) -> Metadata {
    let mut metadata = Metadata::default();

    // Try to get metadata from the reader
    if let Some(meta_rev) = format_reader.metadata().current() {
        for tag in meta_rev.tags() {
            match tag.std_key {
                Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                    metadata.title = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Artist) => {
                    metadata.artist = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Album) => {
                    metadata.album = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::AlbumArtist) => {
                    metadata.album_artist = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Composer) => {
                    metadata.composer = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Genre) => {
                    metadata.genre = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Date) => {
                    // Try to parse year from date
                    if let Ok(year) = tag.value.to_string().parse::<u16>() {
                        metadata.year = Some(year);
                    } else if tag.value.to_string().len() >= 4 {
                        // Try first 4 characters as year
                        if let Ok(year) = tag.value.to_string()[..4].parse::<u16>() {
                            metadata.year = Some(year);
                        }
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::Comment) => {
                    metadata.comments = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::Description) => {
                    metadata.description = Some(tag.value.to_string());
                }
                Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                    if let Ok(num) = tag.value.to_string().parse::<u32>() {
                        metadata.track_number = Some(num);
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::TrackTotal) => {
                    if let Ok(num) = tag.value.to_string().parse::<u32>() {
                        metadata.total_tracks = Some(num);
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::DiscNumber) => {
                    if let Ok(num) = tag.value.to_string().parse::<u32>() {
                        metadata.disc_number = Some(num);
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::DiscTotal) => {
                    if let Ok(num) = tag.value.to_string().parse::<u32>() {
                        metadata.total_discs = Some(num);
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::Label) => {
                    metadata.publisher = Some(tag.value.to_string());
                }
                _ => {}
            }
        }

        // Extract artwork
        for visual in meta_rev.visuals() {
            if !visual.data.is_empty() {
                // Convert to base64 for IPC transfer
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                metadata.artwork_data = Some(STANDARD.encode(&visual.data));
                break; // Just take the first artwork
            }
        }
    }

    // If no title from metadata, use filename
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
}

