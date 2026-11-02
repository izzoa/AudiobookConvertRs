//! Audio processing module using Symphonia
//!
//! This module handles all audio file operations:
//! - Format detection and validation
//! - Audio file loading with Symphonia
//! - Duration calculation
//! - Metadata extraction
//! - Batch file processing
//! - Audio playback preview

pub mod batch;
pub mod format;
pub mod loader;
pub mod player;
pub mod quality;
pub mod timing;

pub use batch::*;
pub use format::*;
pub use loader::*;
pub use player::*;
pub use quality::*;
pub use timing::*;
