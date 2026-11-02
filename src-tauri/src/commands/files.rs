use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use camino::Utf8PathBuf;
use crate::models::{AudioFile, AudioFormat};
use crate::audio::{loader, format};
use crate::metadata::cue_parser::{CueDetectionResult, detect_cue_for_files, find_cue_file};

// Helper to convert Result to String for IPC
fn to_string_err<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn pick_files(app: AppHandle) -> Result<Vec<String>, String> {
    let file_paths = app.dialog()
        .file()
        .add_filter("Audio Files", &["mp3", "flac", "m4a", "wav", "ogg"])
        .blocking_pick_files();

    match file_paths {
        Some(paths) => {
             let strings = paths.iter()
                .map(|p| p.as_path().map(|path| path.to_string_lossy().to_string()).unwrap_or_default())
                .collect();
             Ok(strings)
        },
        None => Ok(vec![]) // User cancelled
    }
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    let dir_path = app.dialog()
        .file()
        .blocking_pick_folder();
        
    match dir_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[derive(serde::Serialize)]
pub struct FileValidation {
    pub path: String,
    pub is_valid: bool,
    pub format: Option<AudioFormat>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn validate_files(paths: Vec<String>) -> Result<Vec<FileValidation>, String> {
    let validations = paths.into_iter().map(|path_str| {
        let path = Utf8PathBuf::from(&path_str);
        
        match format::is_supported_format(&path) {
            true => {
                // Try to detect specific format
                let format = path.extension()
                    .and_then(AudioFormat::from_extension);
                    
                FileValidation {
                    path: path_str,
                    is_valid: true,
                    format,
                    error: None,
                }
            },
            false => {
                 FileValidation {
                    path: path_str,
                    is_valid: false,
                    format: None,
                    error: Some("Unsupported format or file not found".to_string()),
                }
            }
        }
    }).collect();
    
    Ok(validations)
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<AudioFile, String> {
    let path_buf = Utf8PathBuf::from(path);
    loader::load_audio_file(&path_buf).map_err(to_string_err)
}

#[tauri::command]
pub async fn pick_save_project_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("Audiobook Project", &["abcproj"])
        .set_file_name("project.abcproj")
        .blocking_save_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn pick_open_project_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("Audiobook Project", &["abcproj"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn pick_export_summary_file(app: AppHandle, default_name: String) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("Text File", &["txt", "json"])
        .set_file_name(&format!("{}.txt", default_name))
        .blocking_save_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Pick the FFmpeg executable to use
#[tauri::command]
pub async fn pick_ffmpeg_file(app: AppHandle) -> Result<Option<String>, String> {
    let dialog = app.dialog().file().set_title("Choose FFmpeg");
    // Elsewhere the executable has no extension, so nothing can be filtered
    #[cfg(windows)]
    let dialog = dialog.add_filter("Programs", &["exe"]);
    Ok(dialog
        .blocking_pick_file()
        .and_then(|path| path.as_path().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
pub async fn pick_artwork_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("Image Files", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn pick_chapters_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("Chapter Files", &["txt", "csv", "ffmeta"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_output_file(app: AppHandle, default_name: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = app.dialog()
        .file()
        .add_filter("M4B Audiobook", &["m4b"]);

    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }

    let file_path = dialog.blocking_save_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn detect_cue_file(paths: Vec<String>) -> Result<Option<CueDetectionResult>, String> {
    let utf8_paths: Vec<Utf8PathBuf> = paths.iter().map(|p| Utf8PathBuf::from(p)).collect();
    Ok(detect_cue_for_files(&utf8_paths))
}

#[tauri::command]
pub async fn find_cue_for_audio(path: String) -> Result<Option<String>, String> {
    let utf8_path = Utf8PathBuf::from(path);
    Ok(find_cue_file(&utf8_path).map(|p| p.to_string()))
}

#[tauri::command]
pub async fn pick_cue_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog()
        .file()
        .add_filter("CUE Sheet", &["cue"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

#[tauri::command]
pub async fn pick_normalized_output_file(app: AppHandle, default_name: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = app.dialog()
        .file()
        .add_filter("Audio Files", &["m4a", "mp3", "wav", "flac", "aac"]);

    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }

    let file_path = dialog.blocking_save_file();

    match file_path {
        Some(path) => {
            Ok(Some(path.as_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
        },
        None => Ok(None)
    }
}

/// Returns the given paths that already exist, so a batch can warn before it
/// overwrites finished books.
#[tauri::command]
pub async fn existing_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    Ok(paths.into_iter().filter(|p| std::path::Path::new(p).exists()).collect())
}

#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path_buf.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to open file manager: {}", e))?;
        }
    }

    Ok(())
}
