//! AudiobookConvertRs - A cross-platform audiobook converter
//!
//! This library provides the core functionality for converting audio files
//! to M4B audiobook format using Symphonia for reading and FFmpeg for encoding.

pub mod audio;
pub mod config;
pub mod conversion;
pub mod error;
pub mod ffmpeg;
pub mod logging;
pub mod metadata;
pub mod models;
pub mod state;

pub use audio::*;
pub use config::*;
pub use error::*;
pub use models::*;
pub use state::*;
