//! Audio processing module using Symphonia
//!
//! This module handles all audio file operations:
//! - Format detection and validation
//! - Audio file loading with Symphonia
//! - Duration calculation
//! - Metadata extraction
//! - Batch file processing
//! - Audio playback preview

pub mod loader;
pub mod format;
pub mod timing;
pub mod batch;
pub mod quality;
pub mod player;

pub use loader::*;
pub use format::*;
pub use timing::*;
pub use batch::*;
pub use quality::*;
pub use player::*;

