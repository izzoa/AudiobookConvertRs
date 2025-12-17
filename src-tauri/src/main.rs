//! AudiobookConvertRs - Main Entry Point
//!
//! A cross-platform audiobook converter built with Tauri.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiobookconvert_rs::logging::init_logging;
use audiobookconvert_rs::state::AppState;
use audiobookconvert_rs::ffmpeg::detect_ffmpeg;
use tracing::{info, warn};

fn main() {
    // Initialize logging
    if let Err(e) = init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
    }

    info!("Starting AudiobookConvertRs");

    // Check FFmpeg availability
    match detect_ffmpeg() {
        Ok(ffmpeg_info) => {
            info!(
                "FFmpeg detected: {} at {}",
                ffmpeg_info.version, ffmpeg_info.path
            );
        }
        Err(e) => {
            warn!("FFmpeg not found: {}. Some features will be unavailable.", e);
        }
    }

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
