//! Artwork handling module
//!
//! Provides functions for detecting, extracting, and processing artwork
//! for audiobook cover images. Supports extracting embedded artwork from
//! audio files and finding external artwork files in directories.

use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat as ImgFormat};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{debug, info, warn};

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Bmp,
    Webp,
}

impl ImageFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Webp => "webp",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Webp => "image/webp",
        }
    }
}

/// Default maximum dimension for artwork (500x500)
pub const DEFAULT_MAX_ARTWORK_DIMENSION: u32 = 500;

/// Common artwork filenames to search for
const ARTWORK_FILENAMES: &[&str] = &[
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "artwork.jpg",
    "artwork.jpeg",
    "artwork.png",
    "album.jpg",
    "album.jpeg",
    "album.png",
    "front.jpg",
    "front.jpeg",
    "front.png",
];

/// Extract embedded artwork from an audio file
///
/// Uses Symphonia to extract any embedded cover art from the audio file.
/// Returns the raw image bytes if artwork is found.
pub fn extract_embedded_artwork(path: &Utf8Path) -> AppResult<Option<Vec<u8>>> {
    debug!("Extracting embedded artwork from: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let file = File::open(path.as_std_path())
        .map_err(|e| AppError::ArtworkError(format!("Failed to open file: {}", e)))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AppError::ArtworkError(format!("Failed to probe file: {}", e)))?;

    let mut format_reader = probed.format;

    // Check metadata in format reader
    if let Some(meta_rev) = format_reader.metadata().current() {
        for visual in meta_rev.visuals() {
            if !visual.data.is_empty() {
                info!("Found embedded artwork in: {} ({} bytes)", path, visual.data.len());
                return Ok(Some(visual.data.to_vec()));
            }
        }
    }

    // Also check probed metadata
    if let Some(probe_meta) = probed.metadata.get() {
        if let Some(meta_rev) = probe_meta.current() {
            for visual in meta_rev.visuals() {
                if !visual.data.is_empty() {
                    info!("Found embedded artwork in: {} ({} bytes)", path, visual.data.len());
                    return Ok(Some(visual.data.to_vec()));
                }
            }
        }
    }

    debug!("No embedded artwork found in: {}", path);
    Ok(None)
}

/// Scan a directory for common artwork files
///
/// Searches for files like cover.jpg, folder.jpg, etc. in the given directory.
/// Returns the path to the first matching artwork file found.
pub fn scan_directory_for_artwork(dir: &Utf8Path) -> AppResult<Option<Utf8PathBuf>> {
    debug!("Scanning directory for artwork: {}", dir);

    if !dir.is_dir() {
        return Err(AppError::InvalidPath(format!(
            "Not a directory: {}",
            dir
        )));
    }

    // Check for common artwork filenames (case-insensitive)
    for filename in ARTWORK_FILENAMES {
        let path = dir.join(filename);
        if path.exists() {
            info!("Found artwork file: {}", path);
            return Ok(Some(path));
        }

        // Also try uppercase
        let path_upper = dir.join(filename.to_uppercase());
        if path_upper.exists() {
            info!("Found artwork file: {}", path_upper);
            return Ok(Some(path_upper));
        }
    }

    // If no common names found, look for any image file
    if let Ok(entries) = std::fs::read_dir(dir.as_std_path()) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let name_lower = name.to_lowercase();
                if name_lower.ends_with(".jpg")
                    || name_lower.ends_with(".jpeg")
                    || name_lower.ends_with(".png")
                {
                    if let Ok(path) = Utf8PathBuf::try_from(entry.path()) {
                        info!("Found image file: {}", path);
                        return Ok(Some(path));
                    }
                }
            }
        }
    }

    debug!("No artwork found in directory: {}", dir);
    Ok(None)
}

/// Detect the image format from raw bytes
///
/// Checks magic bytes to identify the image format.
pub fn detect_image_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 4 {
        return None;
    }

    // Check magic bytes
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(ImageFormat::Png)
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if data.starts_with(&[0x42, 0x4D]) {
        Some(ImageFormat::Bmp)
    } else if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
        Some(ImageFormat::Webp)
    } else {
        None
    }
}

/// Resize artwork if it exceeds the maximum dimension
///
/// Maintains aspect ratio and resizes the image if either dimension
/// exceeds the specified maximum. Returns JPEG-encoded bytes.
pub fn resize_artwork_if_needed(data: &[u8], max_dimension: u32) -> AppResult<Vec<u8>> {
    let img = image::load_from_memory(data)
        .map_err(|e| AppError::ArtworkError(format!("Failed to load image: {}", e)))?;

    let (width, height) = img.dimensions();
    debug!("Original artwork dimensions: {}x{}", width, height);

    // Check if resizing is needed
    if width <= max_dimension && height <= max_dimension {
        // No resize needed, but still convert to JPEG for consistency
        return convert_to_jpeg(data);
    }

    // Calculate new dimensions maintaining aspect ratio
    let (new_width, new_height) = if width > height {
        let ratio = max_dimension as f32 / width as f32;
        (max_dimension, (height as f32 * ratio) as u32)
    } else {
        let ratio = max_dimension as f32 / height as f32;
        ((width as f32 * ratio) as u32, max_dimension)
    };

    debug!("Resizing artwork to: {}x{}", new_width, new_height);

    // Resize using Lanczos3 filter for best quality
    let resized = img.resize(new_width, new_height, FilterType::Lanczos3);

    // Encode as JPEG
    let mut buffer = Cursor::new(Vec::new());
    resized
        .write_to(&mut buffer, ImgFormat::Jpeg)
        .map_err(|e| AppError::ArtworkError(format!("Failed to encode image: {}", e)))?;

    info!(
        "Resized artwork from {}x{} to {}x{} ({} bytes -> {} bytes)",
        width,
        height,
        new_width,
        new_height,
        data.len(),
        buffer.get_ref().len()
    );

    Ok(buffer.into_inner())
}

/// Convert image data to JPEG format
///
/// Takes image data in any supported format and returns JPEG-encoded bytes.
pub fn convert_to_jpeg(data: &[u8]) -> AppResult<Vec<u8>> {
    let img = image::load_from_memory(data)
        .map_err(|e| AppError::ArtworkError(format!("Failed to load image: {}", e)))?;

    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImgFormat::Jpeg)
        .map_err(|e| AppError::ArtworkError(format!("Failed to encode JPEG: {}", e)))?;

    Ok(buffer.into_inner())
}

/// Load artwork from a file path
///
/// Reads the file and optionally resizes it.
pub fn load_artwork_from_file(path: &Utf8Path, resize: bool) -> AppResult<Vec<u8>> {
    debug!("Loading artwork from file: {}", path);

    if !path.exists() {
        return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
    }

    let mut file = File::open(path.as_std_path())
        .map_err(|e| AppError::ArtworkError(format!("Failed to open file: {}", e)))?;

    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| AppError::ArtworkError(format!("Failed to read file: {}", e)))?;

    if resize {
        resize_artwork_if_needed(&data, DEFAULT_MAX_ARTWORK_DIMENSION)
    } else {
        convert_to_jpeg(&data)
    }
}

/// Get artwork dimensions
pub fn get_artwork_dimensions(data: &[u8]) -> AppResult<(u32, u32)> {
    let img = image::load_from_memory(data)
        .map_err(|e| AppError::ArtworkError(format!("Failed to load image: {}", e)))?;
    Ok(img.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid JPEG (1x1 red pixel)
    const MINIMAL_JPEG: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
        0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09,
        0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
        0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
        0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
        0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00,
        0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
        0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
        0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08,
        0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72,
        0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
        0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
        0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3,
        0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
        0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
        0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
        0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
        0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01,
        0x00, 0x00, 0x3F, 0x00, 0xFB, 0xD5, 0xDB, 0x20, 0xA8, 0xBA, 0xB3, 0x32,
        0xAD, 0xBC, 0x84, 0x6E, 0x70, 0x00, 0xCF, 0xFF, 0xD9,
    ];

    #[test]
    fn test_detect_image_format_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_image_format(&data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_detect_image_format_png() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image_format(&data), Some(ImageFormat::Png));
    }

    #[test]
    fn test_detect_image_format_gif() {
        let data = b"GIF89a";
        assert_eq!(detect_image_format(data), Some(ImageFormat::Gif));
    }

    #[test]
    fn test_detect_image_format_bmp() {
        let data = [0x42, 0x4D, 0x00, 0x00];
        assert_eq!(detect_image_format(&data), Some(ImageFormat::Bmp));
    }

    #[test]
    fn test_detect_image_format_unknown() {
        let data = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_image_format(&data), None);
    }

    #[test]
    fn test_detect_image_format_too_short() {
        let data = [0xFF, 0xD8];
        assert_eq!(detect_image_format(&data), None);
    }

    #[test]
    fn test_image_format_extension() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.extension(), "png");
    }

    #[test]
    fn test_image_format_mime_type() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    }

    #[test]
    fn test_scan_directory_not_a_directory() {
        let path = Utf8Path::new("/nonexistent/path");
        let result = scan_directory_for_artwork(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_embedded_artwork_file_not_found() {
        let path = Utf8Path::new("/nonexistent/file.mp3");
        let result = extract_embedded_artwork(path);
        assert!(result.is_err());
    }
}
