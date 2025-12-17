//! Temporary file management module
//!
//! Provides RAII-based temporary directory management for conversion jobs.
//! Ensures cleanup happens even on errors or panics.

use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Default temp directory name prefix
const TEMP_DIR_PREFIX: &str = "audiobook_convert_";

/// A temporary directory that is automatically cleaned up on drop
///
/// Uses RAII pattern to ensure cleanup happens even on panic.
#[derive(Debug)]
pub struct TempDirectory {
    /// Path to the temporary directory
    path: Utf8PathBuf,
    /// Job ID associated with this temp directory
    job_id: String,
    /// Whether to delete on drop (can be disabled to preserve for debugging)
    delete_on_drop: bool,
}

impl TempDirectory {
    /// Create a new temporary directory for a conversion job
    pub fn new(job_id: &str) -> AppResult<Self> {
        let temp_base = std::env::temp_dir();
        let temp_base = Utf8PathBuf::try_from(temp_base.clone()).map_err(|_| {
            AppError::InternalError(format!(
                "Temp directory path is not valid UTF-8: {:?}",
                temp_base
            ))
        })?;

        let dir_name = format!("{}{}", TEMP_DIR_PREFIX, job_id);
        let path = temp_base.join(dir_name);

        info!("Creating temp directory: {}", path);
        fs::create_dir_all(&path).map_err(|e| AppError::FileWriteError {
            path: path.clone().into(),
            reason: format!("Failed to create temp directory: {}", e),
        })?;

        Ok(Self {
            path,
            job_id: job_id.to_string(),
            delete_on_drop: true,
        })
    }

    /// Create a new temporary directory with an auto-generated ID
    pub fn new_auto() -> AppResult<Self> {
        let job_id = Uuid::new_v4().to_string();
        Self::new(&job_id)
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    /// Get the job ID
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// Create a file path within the temp directory
    pub fn file_path(&self, filename: &str) -> Utf8PathBuf {
        self.path.join(filename)
    }

    /// Create a unique file path within the temp directory
    pub fn unique_file_path(&self, extension: &str) -> Utf8PathBuf {
        let unique_name = format!("{}_{}.{}", self.job_id, Uuid::new_v4(), extension);
        self.path.join(unique_name)
    }

    /// List all files in the temp directory
    pub fn list_files(&self) -> AppResult<Vec<Utf8PathBuf>> {
        let mut files = Vec::new();

        if !self.path.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(&self.path).map_err(|e| AppError::FileReadError {
            path: self.path.clone().into(),
            reason: format!("Failed to read temp directory: {}", e),
        })? {
            let entry = entry.map_err(|e| AppError::FileReadError {
                path: self.path.clone().into(),
                reason: format!("Failed to read directory entry: {}", e),
            })?;

            let path = entry.path();
            if path.is_file() {
                if let Ok(utf8_path) = Utf8PathBuf::try_from(path.clone()) {
                    files.push(utf8_path);
                } else {
                    warn!("Skipping non-UTF8 path: {:?}", path);
                }
            }
        }

        Ok(files)
    }

    /// Disable automatic cleanup on drop
    ///
    /// Useful for debugging or when you want to manually control cleanup.
    pub fn preserve(&mut self) {
        self.delete_on_drop = false;
    }

    /// Manually cleanup the temporary directory
    pub fn cleanup(&mut self) -> AppResult<()> {
        if !self.path.exists() {
            debug!("Temp directory already cleaned up: {}", self.path);
            return Ok(());
        }

        info!("Cleaning up temp directory: {}", self.path);
        cleanup_directory_with_retry(&self.path, 3)?;
        self.delete_on_drop = false; // Prevent double cleanup
        Ok(())
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        if self.delete_on_drop && self.path.exists() {
            debug!("Auto-cleaning temp directory: {}", self.path);
            if let Err(e) = cleanup_directory_with_retry(&self.path, 3) {
                error!("Failed to cleanup temp directory {}: {}", self.path, e);
            }
        }
    }
}

/// Create a temporary directory for a conversion job
pub fn create_temp_directory(job_id: &str) -> AppResult<Utf8PathBuf> {
    let temp = TempDirectory::new(job_id)?;
    let path = temp.path().to_owned();
    // Preserve the directory so it doesn't get cleaned up when temp is dropped
    std::mem::forget(temp);
    Ok(path)
}

/// Cleanup a temporary directory with retry logic
///
/// On Windows, files may be locked briefly after being closed.
/// This function retries cleanup a few times with delays.
pub fn cleanup_temp_directory(path: &Utf8Path) -> AppResult<()> {
    cleanup_directory_with_retry(path, 3)
}

/// Cleanup directory with retry logic for file locking issues
fn cleanup_directory_with_retry(path: &Utf8Path, max_retries: usize) -> AppResult<()> {
    let mut last_error = None;

    for attempt in 0..max_retries {
        match fs::remove_dir_all(path) {
            Ok(()) => {
                debug!("Successfully removed directory: {}", path);
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries - 1 {
                    warn!(
                        "Failed to remove directory {} (attempt {}/{}), retrying...",
                        path,
                        attempt + 1,
                        max_retries
                    );
                    // Wait a bit before retrying (Windows file locking)
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    Err(AppError::FileWriteError {
        path: path.to_path_buf().into(),
        reason: format!(
            "Failed to remove directory after {} attempts: {}",
            max_retries,
            last_error.map(|e| e.to_string()).unwrap_or_default()
        ),
    })
}

/// List all temp files for a specific job
pub fn list_temp_files(job_id: &str) -> AppResult<Vec<Utf8PathBuf>> {
    let temp_base = std::env::temp_dir();
    let temp_base = Utf8PathBuf::try_from(temp_base.clone()).map_err(|_| {
        AppError::InternalError(format!(
            "Temp directory path is not valid UTF-8: {:?}",
            temp_base
        ))
    })?;

    let dir_name = format!("{}{}", TEMP_DIR_PREFIX, job_id);
    let path = temp_base.join(dir_name);

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&path).map_err(|e| AppError::FileReadError {
        path: path.clone().into(),
        reason: format!("Failed to read temp directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AppError::FileReadError {
            path: path.clone().into(),
            reason: format!("Failed to read directory entry: {}", e),
        })?;

        let path = entry.path();
        if let Ok(utf8_path) = Utf8PathBuf::try_from(path) {
            files.push(utf8_path);
        }
    }

    Ok(files)
}

/// Get the temp directory path for a job (without creating it)
pub fn get_temp_directory_path(job_id: &str) -> AppResult<Utf8PathBuf> {
    let temp_base = std::env::temp_dir();
    let temp_base = Utf8PathBuf::try_from(temp_base.clone()).map_err(|_| {
        AppError::InternalError(format!(
            "Temp directory path is not valid UTF-8: {:?}",
            temp_base
        ))
    })?;

    let dir_name = format!("{}{}", TEMP_DIR_PREFIX, job_id);
    Ok(temp_base.join(dir_name))
}

/// Find all orphaned temp directories (for cleanup on startup)
pub fn find_orphaned_temp_directories() -> AppResult<Vec<Utf8PathBuf>> {
    let temp_base = std::env::temp_dir();
    let temp_base = Utf8PathBuf::try_from(temp_base.clone()).map_err(|_| {
        AppError::InternalError(format!(
            "Temp directory path is not valid UTF-8: {:?}",
            temp_base
        ))
    })?;

    let mut orphaned = Vec::new();

    if let Ok(entries) = fs::read_dir(&temp_base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(TEMP_DIR_PREFIX) {
                        if let Ok(utf8_path) = Utf8PathBuf::try_from(path) {
                            orphaned.push(utf8_path);
                        }
                    }
                }
            }
        }
    }

    Ok(orphaned)
}

/// Cleanup all orphaned temp directories
pub fn cleanup_orphaned_temp_directories() -> AppResult<usize> {
    let orphaned = find_orphaned_temp_directories()?;
    let count = orphaned.len();

    for path in orphaned {
        info!("Cleaning up orphaned temp directory: {}", path);
        if let Err(e) = cleanup_directory_with_retry(&path, 3) {
            warn!("Failed to cleanup orphaned directory {}: {}", path, e);
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_directory_creation() {
        let temp = TempDirectory::new("test-job-123").expect("Should create temp dir");
        assert!(temp.path().exists());
        assert_eq!(temp.job_id(), "test-job-123");

        // Get the path before drop
        let path = temp.path().to_owned();
        drop(temp);

        // Should be cleaned up on drop
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_directory_preserve() {
        let mut temp = TempDirectory::new("test-preserve-456").expect("Should create temp dir");
        let path = temp.path().to_owned();
        temp.preserve();
        drop(temp);

        // Should NOT be cleaned up because we called preserve()
        assert!(path.exists());

        // Manual cleanup
        fs::remove_dir_all(&path).expect("Should cleanup");
    }

    #[test]
    fn test_temp_directory_file_path() {
        let temp = TempDirectory::new("test-file-789").expect("Should create temp dir");
        let file_path = temp.file_path("output.m4b");
        assert!(file_path.ends_with("output.m4b"));
        assert!(file_path.starts_with(temp.path()));
    }

    #[test]
    fn test_temp_directory_unique_file_path() {
        let temp = TempDirectory::new("test-unique-001").expect("Should create temp dir");
        let path1 = temp.unique_file_path("m4b");
        let path2 = temp.unique_file_path("m4b");
        assert_ne!(path1, path2); // Should be unique
    }

    #[test]
    fn test_temp_directory_list_files() {
        let temp = TempDirectory::new("test-list-002").expect("Should create temp dir");

        // Create some test files
        let file1 = temp.file_path("file1.txt");
        let file2 = temp.file_path("file2.txt");
        fs::write(&file1, "test").expect("Should write file");
        fs::write(&file2, "test").expect("Should write file");

        let files = temp.list_files().expect("Should list files");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_get_temp_directory_path() {
        let path = get_temp_directory_path("my-job-id").expect("Should get path");
        assert!(path.as_str().contains("audiobook_convert_my-job-id"));
    }
}
