//! Project management module
//!
//! Provides functionality for:
//! - Saving audiobook projects to disk
//! - Loading projects to resume editing
//! - Managing recent projects list
//! - Project file versioning for compatibility

use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, ConversionDefaults, ConversionSettings};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{debug, info, warn};

/// Current project file format version
const PROJECT_VERSION: u32 = 1;

/// Project file extension
pub const PROJECT_EXTENSION: &str = "abcproj";

/// A saved audiobook project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    /// Project file format version
    pub version: u32,
    /// The audiobook data
    pub audiobook: AudioBook,
    /// Optional saved conversion settings (legacy, kept for backwards compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversion_settings: Option<ConversionSettings>,
    /// Project-level conversion defaults (overrides global defaults)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversion_defaults: Option<ConversionDefaults>,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// When the project was last saved
    pub saved_at: DateTime<Utc>,
    /// Application version that created this project
    pub app_version: String,
    /// Optional notes/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Default for Project {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            version: PROJECT_VERSION,
            audiobook: AudioBook::new("Untitled".to_string()),
            conversion_settings: None,
            conversion_defaults: None,
            created_at: now,
            saved_at: now,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            notes: None,
        }
    }
}

impl Project {
    /// Create a new project from an audiobook
    pub fn new(audiobook: AudioBook) -> Self {
        let now = Utc::now();
        Self {
            version: PROJECT_VERSION,
            audiobook,
            conversion_settings: None,
            conversion_defaults: None,
            created_at: now,
            saved_at: now,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            notes: None,
        }
    }

    /// Create a project with conversion settings
    pub fn with_settings(audiobook: AudioBook, settings: ConversionSettings) -> Self {
        let mut project = Self::new(audiobook);
        project.conversion_settings = Some(settings);
        project
    }

    /// Save the project to a file
    pub fn save(&mut self, path: &Utf8Path) -> AppResult<()> {
        self.saved_at = Utc::now();

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent.as_std_path())
                .map_err(|e| AppError::FileWriteError {
                    path: path.as_std_path().to_path_buf(),
                    reason: e.to_string(),
                })?;
        }

        fs::write(path.as_std_path(), json)
            .map_err(|e| AppError::FileWriteError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        info!("Saved project to: {}", path);
        Ok(())
    }

    /// Load a project from a file
    pub fn load(path: &Utf8Path) -> AppResult<Self> {
        if !path.exists() {
            return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
        }

        let content = fs::read_to_string(path.as_std_path())
            .map_err(|e| AppError::FileReadError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        let project: Project = serde_json::from_str(&content)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        // Check version compatibility
        if project.version > PROJECT_VERSION {
            warn!(
                "Project version {} is newer than supported version {}",
                project.version, PROJECT_VERSION
            );
        }

        info!("Loaded project from: {}", path);
        Ok(project)
    }

    /// Check if project needs migration to newer format
    pub fn needs_migration(&self) -> bool {
        self.version < PROJECT_VERSION
    }

    /// Migrate project to current version (if needed)
    pub fn migrate(&mut self) -> bool {
        if !self.needs_migration() {
            return false;
        }

        // Add migration logic here as versions change
        // For now, just update version number
        self.version = PROJECT_VERSION;
        true
    }
}

/// Recent project entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    /// Path to the project file
    pub path: Utf8PathBuf,
    /// Project title (for display)
    pub title: String,
    /// When it was last opened
    pub last_opened: DateTime<Utc>,
    /// Whether the file still exists
    #[serde(skip_deserializing, default)]
    pub exists: bool,
}

impl RecentProject {
    /// Create a new recent project entry
    pub fn new(path: Utf8PathBuf, title: String) -> Self {
        Self {
            exists: path.exists(),
            path,
            title,
            last_opened: Utc::now(),
        }
    }

    /// Update the exists field by checking the file system
    pub fn refresh_exists(&mut self) {
        self.exists = self.path.exists();
    }
}

/// Manager for recent projects list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentProjectsManager {
    /// List of recent projects (most recent first)
    projects: Vec<RecentProject>,
    /// Maximum number of recent projects to track
    #[serde(default = "default_max_recent")]
    max_recent: usize,
}

fn default_max_recent() -> usize {
    10
}

impl RecentProjectsManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
            max_recent: 10,
        }
    }

    /// Load recent projects from config file
    pub fn load(path: &Utf8Path) -> AppResult<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path.as_std_path())
            .map_err(|e| AppError::FileReadError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        let mut manager: RecentProjectsManager = serde_json::from_str(&content)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        // Refresh exists status for all entries
        manager.refresh_all();

        Ok(manager)
    }

    /// Save recent projects to config file
    pub fn save(&self, path: &Utf8Path) -> AppResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent.as_std_path())
                .map_err(|e| AppError::FileWriteError {
                    path: path.as_std_path().to_path_buf(),
                    reason: e.to_string(),
                })?;
        }

        fs::write(path.as_std_path(), json)
            .map_err(|e| AppError::FileWriteError {
                path: path.as_std_path().to_path_buf(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Add or update a project in the recent list
    pub fn add_project(&mut self, path: Utf8PathBuf, title: String) {
        // Remove if already exists
        self.projects.retain(|p| p.path != path);

        // Add to front
        self.projects.insert(0, RecentProject::new(path, title));

        // Trim to max
        self.projects.truncate(self.max_recent);

        debug!("Added to recent projects: {}", self.projects[0].title);
    }

    /// Remove a project from the list
    pub fn remove_project(&mut self, path: &Utf8Path) {
        self.projects.retain(|p| p.path != path);
    }

    /// Get the list of recent projects
    pub fn list(&self) -> &[RecentProject] {
        &self.projects
    }

    /// Get only projects that still exist
    pub fn list_existing(&self) -> Vec<&RecentProject> {
        self.projects.iter().filter(|p| p.exists).collect()
    }

    /// Refresh exists status for all entries
    pub fn refresh_all(&mut self) {
        for project in &mut self.projects {
            project.refresh_exists();
        }
    }

    /// Remove entries for files that no longer exist
    pub fn cleanup_missing(&mut self) {
        self.refresh_all();
        let before = self.projects.len();
        self.projects.retain(|p| p.exists);
        let removed = before - self.projects.len();
        if removed > 0 {
            debug!("Cleaned up {} missing recent projects", removed);
        }
    }

    /// Clear all recent projects
    pub fn clear(&mut self) {
        self.projects.clear();
    }
}

/// Export project metadata to a summary format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExport {
    pub title: String,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub duration_formatted: String,
    pub chapter_count: usize,
    pub file_count: usize,
    pub chapters: Vec<ChapterExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterExport {
    pub index: usize,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub duration: String,
}

/// Export project metadata to a summary
pub fn export_project_metadata(project: &Project) -> ProjectExport {
    use crate::models::format_duration_ms;

    let ab = &project.audiobook;
    let chapters: Vec<ChapterExport> = ab.chapters.iter().map(|ch| {
        ChapterExport {
            index: ch.index,
            title: ch.title.clone(),
            start_time: format_duration_ms(ch.start_time_ms),
            end_time: format_duration_ms(ch.end_time_ms),
            duration: format_duration_ms(ch.end_time_ms.saturating_sub(ch.start_time_ms)),
        }
    }).collect();

    ProjectExport {
        title: ab.title.clone(),
        author: ab.output_metadata.artist.clone(),
        narrator: ab.output_metadata.narrator.clone(),
        duration_formatted: format_duration_ms(ab.total_duration_ms),
        chapter_count: ab.chapters.len(),
        file_count: ab.files.len(),
        chapters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AudioBook;

    #[test]
    fn test_project_creation() {
        let audiobook = AudioBook::new("Test Book".to_string());
        let project = Project::new(audiobook);

        assert_eq!(project.version, PROJECT_VERSION);
        assert_eq!(project.audiobook.title, "Test Book");
        assert!(project.conversion_settings.is_none());
    }

    #[test]
    fn test_recent_projects_manager() {
        let mut manager = RecentProjectsManager::new();
        
        manager.add_project("/path/to/project1.abcproj".into(), "Book 1".to_string());
        manager.add_project("/path/to/project2.abcproj".into(), "Book 2".to_string());
        
        assert_eq!(manager.list().len(), 2);
        assert_eq!(manager.list()[0].title, "Book 2"); // Most recent first
        
        // Adding same path updates and moves to front
        manager.add_project("/path/to/project1.abcproj".into(), "Book 1 Updated".to_string());
        assert_eq!(manager.list().len(), 2);
        assert_eq!(manager.list()[0].title, "Book 1 Updated");
    }

    #[test]
    fn test_recent_projects_max() {
        let mut manager = RecentProjectsManager::new();
        manager.max_recent = 3;
        
        for i in 0..5 {
            manager.add_project(format!("/path/to/project{}.abcproj", i).into(), format!("Book {}", i));
        }
        
        assert_eq!(manager.list().len(), 3);
        assert_eq!(manager.list()[0].title, "Book 4"); // Most recent
    }
}
