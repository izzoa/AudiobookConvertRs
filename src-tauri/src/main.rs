//! AudiobookConvertRs - Main Entry Point
//!
//! A cross-platform audiobook converter built with Tauri.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiobookconvert_rs::logging::init_logging;
use audiobookconvert_rs::state::AppState;
use audiobookconvert_rs::commands::undo::UndoState;
use audiobookconvert_rs::conversion::runner::create_shared_manager;
use audiobookconvert_rs::ffmpeg::detect_ffmpeg;
use audiobookconvert_rs::config::UserSettings;
use audiobookconvert_rs::audio::player::create_shared_player;
use camino::Utf8PathBuf;
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

    // Create conversion manager with persistence
    let config_dir = UserSettings::config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let queue_path = Utf8PathBuf::from(config_dir.join("conversion_queue.json").to_string_lossy().to_string());
    let manager = create_shared_manager(Some(queue_path))
        .expect("Failed to create conversion manager");

    // Create audio player for preview functionality
    let player = create_shared_player();

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .manage(UndoState::new())
        .manage(manager)
        .manage(player)
        .invoke_handler(tauri::generate_handler![
            // Audiobook
            audiobookconvert_rs::commands::audiobook::create_audiobook_from_files,
            audiobookconvert_rs::commands::audiobook::load_audiobook_file,
            audiobookconvert_rs::commands::audiobook::save_audiobook_project,
            audiobookconvert_rs::commands::audiobook::clear_current_audiobook,
            audiobookconvert_rs::commands::audiobook::get_active_audiobook,
            // Metadata
            audiobookconvert_rs::commands::metadata::update_audiobook_metadata,
            audiobookconvert_rs::commands::metadata::add_chapter,
            audiobookconvert_rs::commands::metadata::update_chapter,
            audiobookconvert_rs::commands::metadata::delete_chapter,
            audiobookconvert_rs::commands::metadata::reorder_chapters,
            audiobookconvert_rs::commands::metadata::auto_create_chapters,
            audiobookconvert_rs::commands::metadata::upload_artwork,
            audiobookconvert_rs::commands::metadata::set_artwork_base64,
            audiobookconvert_rs::commands::metadata::clear_artwork,
            audiobookconvert_rs::commands::metadata::import_chapters,
            audiobookconvert_rs::commands::metadata::export_chapters,
            audiobookconvert_rs::commands::metadata::set_chapters,
            audiobookconvert_rs::commands::metadata::generate_chapters_from_source,
            audiobookconvert_rs::commands::metadata::rename_chapters_with_regex,
            audiobookconvert_rs::commands::metadata::get_chapter_regex_presets,
            audiobookconvert_rs::commands::metadata::import_chapters_from_cue,
            audiobookconvert_rs::commands::metadata::parse_cue_file_info,
            audiobookconvert_rs::commands::metadata::discover_artwork,
            audiobookconvert_rs::commands::metadata::select_discovered_artwork,
            // Files
            audiobookconvert_rs::commands::files::pick_files,
            audiobookconvert_rs::commands::files::pick_directory,
            audiobookconvert_rs::commands::files::validate_files,
            audiobookconvert_rs::commands::files::get_file_info,
            audiobookconvert_rs::commands::files::pick_save_project_file,
            audiobookconvert_rs::commands::files::pick_open_project_file,
            audiobookconvert_rs::commands::files::pick_export_summary_file,
            audiobookconvert_rs::commands::files::write_text_file,
            audiobookconvert_rs::commands::files::pick_artwork_file,
            audiobookconvert_rs::commands::files::pick_chapters_file,
            audiobookconvert_rs::commands::files::read_text_file,
            audiobookconvert_rs::commands::files::pick_output_file,
            audiobookconvert_rs::commands::files::detect_cue_file,
            audiobookconvert_rs::commands::files::find_cue_for_audio,
            audiobookconvert_rs::commands::files::pick_cue_file,
            // Settings
            audiobookconvert_rs::commands::settings::get_settings,
            audiobookconvert_rs::commands::settings::update_settings,
            audiobookconvert_rs::commands::settings::get_default_settings,
            audiobookconvert_rs::commands::settings::check_ffmpeg_available,
            audiobookconvert_rs::commands::settings::get_ffmpeg_version,
            audiobookconvert_rs::commands::settings::get_ffmpeg_info,
            audiobookconvert_rs::commands::settings::reset_settings,
            // Conversion
            audiobookconvert_rs::commands::conversion::start_conversion,
            audiobookconvert_rs::commands::conversion::enqueue_conversion,
            audiobookconvert_rs::commands::conversion::start_conversion_queue,
            audiobookconvert_rs::commands::conversion::stop_conversion_queue,
            audiobookconvert_rs::commands::conversion::list_conversion_jobs,
            audiobookconvert_rs::commands::conversion::remove_conversion_job,
            audiobookconvert_rs::commands::conversion::clear_completed_jobs,
            audiobookconvert_rs::commands::conversion::clear_pending_jobs,
            audiobookconvert_rs::commands::conversion::cancel_conversion,
            audiobookconvert_rs::commands::conversion::get_conversion_status,
            audiobookconvert_rs::commands::conversion::pause_conversion,
            audiobookconvert_rs::commands::conversion::resume_conversion,
            // Quality
            audiobookconvert_rs::commands::quality::analyze_audio_levels,
            audiobookconvert_rs::commands::quality::suggest_normalization,
            audiobookconvert_rs::commands::quality::apply_audio_normalization,
            // Presets
            audiobookconvert_rs::commands::presets::list_presets,
            audiobookconvert_rs::commands::presets::get_preset,
            audiobookconvert_rs::commands::presets::save_preset,
            audiobookconvert_rs::commands::presets::delete_preset,
            audiobookconvert_rs::commands::presets::dry_run_conversion,
            // Project
            audiobookconvert_rs::commands::project::save_project,
            audiobookconvert_rs::commands::project::load_project,
            audiobookconvert_rs::commands::project::list_recent_projects,
            audiobookconvert_rs::commands::project::clear_recent_projects,
            audiobookconvert_rs::commands::project::export_project_summary,
            audiobookconvert_rs::commands::project::clear_project,
            // Undo
            audiobookconvert_rs::commands::undo::undo,
            audiobookconvert_rs::commands::undo::redo,
            audiobookconvert_rs::commands::undo::can_undo,
            audiobookconvert_rs::commands::undo::can_redo,
            audiobookconvert_rs::commands::undo::get_undo_description,
            audiobookconvert_rs::commands::undo::get_redo_description,
            audiobookconvert_rs::commands::undo::get_undo_state,
            // Open Library
            audiobookconvert_rs::commands::open_library::open_library_search,
            audiobookconvert_rs::commands::open_library::open_library_apply,
            // Playback
            audiobookconvert_rs::commands::playback::play_audio,
            audiobookconvert_rs::commands::playback::pause_audio,
            audiobookconvert_rs::commands::playback::resume_audio,
            audiobookconvert_rs::commands::playback::stop_audio,
            audiobookconvert_rs::commands::playback::seek_audio,
            audiobookconvert_rs::commands::playback::set_audio_volume,
            audiobookconvert_rs::commands::playback::get_playback_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
