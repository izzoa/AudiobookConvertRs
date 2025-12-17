//! FFmpeg integration module
//!
//! This module handles:
//! - FFmpeg detection on the system
//! - Building FFmpeg commands
//! - Executing FFmpeg processes
//! - Parsing FFmpeg output for progress

pub mod detection;
pub mod command_builder;

pub use detection::*;
pub use command_builder::*;

