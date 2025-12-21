//! Tauri commands for project management (save/load/recent)

use camino::Utf8PathBuf;
use tauri::State;

use crate::project::{
    Project, ProjectExport, RecentProject, RecentProjectsManager, export_project_metadata,
};
use crate::state::AppState;

/// Get the recent projects file path in user config directory
fn get_recent_projects_path() -> Utf8PathBuf {
    dirs::config_dir()
        .map(|p| Utf8PathBuf::from_path_buf(p).unwrap_or_else(|_| Utf8PathBuf::from(".")))
        .unwrap_or_else(|| Utf8PathBuf::from("."))
        .join("audiobookconverter")
        .join("recent_projects.json")
}

/// Save the current audiobook as a project file
#[tauri::command]
pub async fn save_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let audiobook = state
        .get_active_audiobook()
        .ok_or("No active audiobook to save")?;

    let project_path = Utf8PathBuf::from(&path);
    let mut project = Project::new(audiobook.clone());
    project.save(&project_path).map_err(|e| e.to_string())?;

    // Add to recent projects
    let recent_path = get_recent_projects_path();
    if let Ok(mut manager) = RecentProjectsManager::load(&recent_path) {
        manager.add_project(project_path, audiobook.title);
        let _ = manager.save(&recent_path);
    }

    state.mark_clean();
    Ok(())
}

/// Load a project file
#[tauri::command]
pub async fn load_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let project_path = Utf8PathBuf::from(&path);
    let project = Project::load(&project_path).map_err(|e| e.to_string())?;

    // Add audiobook to state
    state.add_audiobook(project.audiobook.clone());

    // Update recent projects
    let recent_path = get_recent_projects_path();
    if let Ok(mut manager) = RecentProjectsManager::load(&recent_path) {
        manager.add_project(project_path, project.audiobook.title.clone());
        let _ = manager.save(&recent_path);
    }

    Ok(project)
}

/// List recent projects
#[tauri::command]
pub async fn list_recent_projects(
    _state: State<'_, AppState>,
) -> Result<Vec<RecentProject>, String> {
    let path = get_recent_projects_path();
    let mut manager = RecentProjectsManager::load(&path).map_err(|e| e.to_string())?;
    manager.refresh_all();
    Ok(manager.list().to_vec())
}

/// Clear all recent projects
#[tauri::command]
pub async fn clear_recent_projects(_state: State<'_, AppState>) -> Result<(), String> {
    let path = get_recent_projects_path();
    let mut manager = RecentProjectsManager::load(&path).map_err(|e| e.to_string())?;
    manager.clear();
    manager.save(&path).map_err(|e| e.to_string())
}

/// Export project metadata summary (for display or export)
#[tauri::command]
pub async fn export_project_summary(
    state: State<'_, AppState>,
) -> Result<ProjectExport, String> {
    let audiobook = state
        .get_active_audiobook()
        .ok_or("No active audiobook loaded")?;

    let project = Project::new(audiobook);
    Ok(export_project_metadata(&project))
}
