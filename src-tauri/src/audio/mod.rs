//! Audio processing module using Symphonia
//!
//! This module handles all audio file operations:
//! - Format detection and validation
//! - Audio file loading with Symphonia
//! - Duration calculation
//! - Metadata extraction
//! - Batch file processing

pub mod loader;
pub mod format;
pub mod timing;
pub mod batch;

pub use loader::*;
pub use format::*;
pub use timing::*;
pub use batch::*;

