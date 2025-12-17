use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use camino::Utf8PathBuf;
use crate::models::{AudioFile, AudioFormat};
use crate::audio::{loader, format};

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
