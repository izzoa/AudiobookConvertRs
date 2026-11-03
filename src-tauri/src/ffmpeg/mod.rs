//! FFmpeg integration module
//!
//! This module handles:
//! - FFmpeg detection on the system
//! - Building FFmpeg commands
//! - Executing FFmpeg processes
//! - Parsing FFmpeg output for progress

pub mod command_builder;
pub mod detection;
pub mod download;
pub mod executor;
pub mod silence;

pub use command_builder::*;
pub use detection::*;
pub use executor::*;
