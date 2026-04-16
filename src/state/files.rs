//! File state tracking for Anvil
//!
//! This module tracks all files managed by Anvil, enabling:
//! - Detection of file drift (changes since installation)
//! - Tracking which workload manages which file
//! - Conflict detection when multiple workloads target the same file
//! - Restoration recommendations

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::providers::backup::compute_file_hash;

/// Errors that can occur during file state operations
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum FileStateError {
    /// State file not found
    #[error("State file not found: {0}")]
    NotFound(PathBuf),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// File conflict detected
    #[error("File conflict: {path} is managed by workloads: {workloads:?}")]
    Conflict {
        path: PathBuf,
        workloads: Vec<String>,
    },
}

/// Result type for file state operations
pub type FileStateResult<T> = Result<T, FileStateError>;

/// State information for a single managed file
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// Destination path of the file
    pub path: PathBuf,
    /// Hash of the source file when installed
    pub source_hash: String,
    /// Hash of the file after installation
    pub installed_hash: String,
    /// When the file was installed
    pub installed_at: DateTime<Utc>,
    /// Workload that manages this file
    pub workload: String,
    /// Whether the file was processed as a template
    pub was_templated: bool,
    /// Associated backup ID (if file was backed up before overwriting)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    /// Source file path (relative to workload)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Size of the installed file in bytes
    #[serde(default)]
    pub size: u64,
}

#[allow(dead_code)]
impl FileState {
    /// Create a new file state entry
    pub fn new(
        path: PathBuf,
        source_hash: String,
        installed_hash: String,
        workload: String,
        was_templated: bool,
    ) -> Self {
        Self {
            path,
            source_hash,
            installed_hash,
            installed_at: Utc::now(),
            workload,
            was_templated,
            backup_id: None,
            source_path: None,
            size: 0,
        }
    }

    /// Set the backup ID
    pub fn with_backup_id(mut self, id: String) -> Self {
        self.backup_id = Some(id);
        self
    }

    /// Set the source path
    pub fn with_source_path(mut self, path: String) -> Self {
        self.source_path = Some(path);
        self
    }

    /// Set the file size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// Check if the file still exists at the destination
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Check if the file has drifted from its installed state
    pub fn has_drifted(&self) -> FileStateResult<bool> {
        if !self.path.exists() {
            return Ok(true); // File was deleted = drifted
        }

        let current_hash = compute_file_hash(&self.path).map_err(|e| {
            FileStateError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
        Ok(current_hash != self.installed_hash)
    }

    /// Get the current hash of the file (if it exists)
    pub fn current_hash(&self) -> FileStateResult<Option<String>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let hash = compute_file_hash(&self.path).map_err(|e| {
            FileStateError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
        Ok(Some(hash))
    }

    /// Format the file size for display
    pub fn formatted_size(&self) -> String {
        crate::providers::backup::format_size(self.size)
    }
}

/// Status of a file check
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCheckStatus {
    /// File matches installed state
    Ok,
    /// File has been modified since installation
    Modified,
    /// File is missing
    Missing,
    /// File was never installed (source missing)
    NotInstalled,
    /// Error checking file
    Error(String),
}

impl std::fmt::Display for FileCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileCheckStatus::Ok => write!(f, "OK"),
            FileCheckStatus::Modified => write!(f, "Modified"),
            FileCheckStatus::Missing => write!(f, "Missing"),
            FileCheckStatus::NotInstalled => write!(f, "Not installed"),
            FileCheckStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Result of checking a single file
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileCheckResult {
    /// Path to the file
    pub path: PathBuf,
    /// Status of the check
    pub status: FileCheckStatus,
    /// Expected hash (from state)
    pub expected_hash: Option<String>,
    /// Current hash (if file exists)
    pub current_hash: Option<String>,
    /// Associated workload
    pub workload: String,
    /// Size of the file
    pub size: u64,
}

/// The file state index containing all managed files
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileStateIndex {
    /// Version of the index format
    pub version: u32,
    /// All file state entries, keyed by normalized path
    pub files: HashMap<String, FileState>,
    /// When the index was last updated
    pub last_updated: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
impl FileStateIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            version: 1,
            files: HashMap::new(),
            last_updated: Some(Utc::now()),
        }
    }

    /// Normalize a path for use as a key
    fn normalize_path(path: &Path) -> String {
        // Convert to lowercase on Windows for case-insensitive comparison
        #[cfg(windows)]
        {
            path.to_string_lossy().to_lowercase().replace('/', "\\")
        }
        #[cfg(not(windows))]
        {
            path.to_string_lossy().to_string()
        }
    }

    /// Get a file state by path
    pub fn get(&self, path: &Path) -> Option<&FileState> {
        let key = Self::normalize_path(path);
        self.files.get(&key)
    }

    /// Get a mutable file state by path
    pub fn get_mut(&mut self, path: &Path) -> Option<&mut FileState> {
        let key = Self::normalize_path(path);
        self.files.get_mut(&key)
    }

    /// Insert or update a file state
    pub fn set(&mut self, state: FileState) {
        let key = Self::normalize_path(&state.path);
        self.files.insert(key, state);
        self.last_updated = Some(Utc::now());
    }

    /// Remove a file state
    pub fn remove(&mut self, path: &Path) -> Option<FileState> {
        let key = Self::normalize_path(path);
        let removed = self.files.remove(&key);
        if removed.is_some() {
            self.last_updated = Some(Utc::now());
        }
        removed
    }

    /// Check if a file is tracked
    pub fn contains(&self, path: &Path) -> bool {
        let key = Self::normalize_path(path);
        self.files.contains_key(&key)
    }

    /// Get all files for a specific workload
    pub fn files_for_workload(&self, workload: &str) -> Vec<&FileState> {
        self.files
            .values()
            .filter(|f| f.workload == workload)
            .collect()
    }

    /// Get all tracked files
    pub fn all_files(&self) -> Vec<&FileState> {
        self.files.values().collect()
    }

    /// Get the count of tracked files
    pub fn count(&self) -> usize {
        self.files.len()
    }

    /// Find files that are managed by multiple workloads (conflicts)
    pub fn find_conflicts(&self) -> HashMap<PathBuf, Vec<String>> {
        let mut path_workloads: HashMap<String, Vec<String>> = HashMap::new();

        for state in self.files.values() {
            let key = Self::normalize_path(&state.path);
            path_workloads
                .entry(key)
                .or_default()
                .push(state.workload.clone());
        }

        // Note: With current implementation, each path has exactly one entry
        // This function is more useful when checking potential conflicts
        // before installing a new workload
        HashMap::new()
    }

    /// Check if installing a file would conflict with existing tracked files
    pub fn would_conflict(&self, path: &Path, workload: &str) -> Option<String> {
        if let Some(state) = self.get(path) {
            if state.workload != workload {
                return Some(state.workload.clone());
            }
        }
        None
    }

    /// Check all files and return their status
    pub fn check_all(&self) -> Vec<FileCheckResult> {
        self.files
            .values()
            .map(|state| self.check_file(state))
            .collect()
    }

    /// Check files for a specific workload
    pub fn check_workload(&self, workload: &str) -> Vec<FileCheckResult> {
        self.files_for_workload(workload)
            .iter()
            .map(|state| self.check_file(state))
            .collect()
    }

    /// Check a single file state
    fn check_file(&self, state: &FileState) -> FileCheckResult {
        if !state.path.exists() {
            return FileCheckResult {
                path: state.path.clone(),
                status: FileCheckStatus::Missing,
                expected_hash: Some(state.installed_hash.clone()),
                current_hash: None,
                workload: state.workload.clone(),
                size: state.size,
            };
        }

        match compute_file_hash(&state.path) {
            Ok(current_hash) => {
                let status = if current_hash == state.installed_hash {
                    FileCheckStatus::Ok
                } else {
                    FileCheckStatus::Modified
                };

                FileCheckResult {
                    path: state.path.clone(),
                    status,
                    expected_hash: Some(state.installed_hash.clone()),
                    current_hash: Some(current_hash),
                    workload: state.workload.clone(),
                    size: state.size,
                }
            }
            Err(e) => FileCheckResult {
                path: state.path.clone(),
                status: FileCheckStatus::Error(e.to_string()),
                expected_hash: Some(state.installed_hash.clone()),
                current_hash: None,
                workload: state.workload.clone(),
                size: state.size,
            },
        }
    }
}

/// Manager for file state tracking
#[allow(dead_code)]
pub struct FileStateManager {
    /// Path to the state file
    state_file: PathBuf,
    /// Cached index
    index: FileStateIndex,
    /// Whether changes have been made since last save
    dirty: bool,
}

#[allow(dead_code)]
impl FileStateManager {
    /// Create a new file state manager
    pub fn new() -> FileStateResult<Self> {
        let state_file = Self::default_state_file()?;
        Self::with_file(state_file)
    }

    /// Create a file state manager with a specific state file
    pub fn with_file(state_file: PathBuf) -> FileStateResult<Self> {
        let index = if state_file.exists() {
            let content = fs::read_to_string(&state_file)?;
            serde_json::from_str(&content)?
        } else {
            FileStateIndex::new()
        };

        Ok(Self {
            state_file,
            index,
            dirty: false,
        })
    }

    /// Get the default state file path (~/.anvil/state/files.json)
    pub fn default_state_file() -> FileStateResult<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            FileStateError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        })?;
        let state_dir = home.join(".anvil").join("state");

        if !state_dir.exists() {
            fs::create_dir_all(&state_dir)?;
        }

        Ok(state_dir.join("files.json"))
    }

    /// Save the state to disk
    pub fn save(&mut self) -> FileStateResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Create parent directory if needed
        if let Some(parent) = self.state_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        self.index.last_updated = Some(Utc::now());
        let content = serde_json::to_string_pretty(&self.index)?;
        fs::write(&self.state_file, content)?;
        self.dirty = false;
        Ok(())
    }

    /// Force save (even if not dirty)
    pub fn force_save(&mut self) -> FileStateResult<()> {
        self.dirty = true;
        self.save()
    }

    /// Record a file installation
    pub fn record_install(
        &mut self,
        path: &Path,
        source_hash: &str,
        installed_hash: &str,
        workload: &str,
        was_templated: bool,
    ) -> FileStateResult<()> {
        let state = FileState::new(
            path.to_path_buf(),
            source_hash.to_string(),
            installed_hash.to_string(),
            workload.to_string(),
            was_templated,
        );

        self.index.set(state);
        self.dirty = true;
        Ok(())
    }

    /// Record a file installation with full details
    pub fn record_install_full(&mut self, state: FileState) -> FileStateResult<()> {
        self.index.set(state);
        self.dirty = true;
        Ok(())
    }

    /// Remove tracking for a file
    pub fn remove(&mut self, path: &Path) -> Option<FileState> {
        let removed = self.index.remove(path);
        if removed.is_some() {
            self.dirty = true;
        }
        removed
    }

    /// Get file state
    pub fn get(&self, path: &Path) -> Option<&FileState> {
        self.index.get(path)
    }

    /// Check if a file is tracked
    pub fn is_tracked(&self, path: &Path) -> bool {
        self.index.contains(path)
    }

    /// Get all files for a workload
    pub fn files_for_workload(&self, workload: &str) -> Vec<&FileState> {
        self.index.files_for_workload(workload)
    }

    /// Check all tracked files
    pub fn check_all(&self) -> Vec<FileCheckResult> {
        self.index.check_all()
    }

    /// Check files for a workload
    pub fn check_workload(&self, workload: &str) -> Vec<FileCheckResult> {
        self.index.check_workload(workload)
    }

    /// Check if a file would conflict with existing tracked files
    pub fn would_conflict(&self, path: &Path, workload: &str) -> Option<String> {
        self.index.would_conflict(path, workload)
    }

    /// Get statistics about tracked files
    pub fn stats(&self) -> FileStateStats {
        let mut stats = FileStateStats::default();
        let mut workloads = std::collections::HashSet::new();

        for state in self.index.all_files() {
            stats.total_files += 1;
            stats.total_size += state.size;
            workloads.insert(state.workload.clone());

            if state.was_templated {
                stats.templated_files += 1;
            }
        }

        stats.workload_count = workloads.len();
        stats
    }

    /// Get access to the underlying index
    pub fn index(&self) -> &FileStateIndex {
        &self.index
    }

    /// Remove all files for a workload
    pub fn remove_workload(&mut self, workload: &str) -> Vec<FileState> {
        let paths: Vec<PathBuf> = self
            .index
            .files_for_workload(workload)
            .iter()
            .map(|f| f.path.clone())
            .collect();

        let mut removed = Vec::new();
        for path in paths {
            if let Some(state) = self.index.remove(&path) {
                removed.push(state);
            }
        }

        if !removed.is_empty() {
            self.dirty = true;
        }

        removed
    }
}

impl Default for FileStateManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default FileStateManager")
    }
}

impl Drop for FileStateManager {
    fn drop(&mut self) {
        // Try to save on drop, but don't panic on error
        let _ = self.save();
    }
}

/// Statistics about tracked files
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct FileStateStats {
    /// Total number of tracked files
    pub total_files: usize,
    /// Number of templated files
    pub templated_files: usize,
    /// Total size of all tracked files
    pub total_size: u64,
    /// Number of workloads with tracked files
    pub workload_count: usize,
}

#[allow(dead_code)]
impl FileStateStats {
    /// Format the total size for display
    pub fn formatted_size(&self) -> String {
        crate::providers::backup::format_size(self.total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_state_creation() {
        let state = FileState::new(
            PathBuf::from("/path/to/file"),
            "sha256:abc".to_string(),
            "sha256:abc".to_string(),
            "test-workload".to_string(),
            false,
        );

        assert_eq!(state.path, PathBuf::from("/path/to/file"));
        assert_eq!(state.workload, "test-workload");
        assert!(!state.was_templated);
    }

    #[test]
    fn test_file_state_index() {
        let mut index = FileStateIndex::new();

        let state = FileState::new(
            PathBuf::from("/path/to/file"),
            "sha256:abc".to_string(),
            "sha256:abc".to_string(),
            "workload-a".to_string(),
            false,
        );

        index.set(state);
        assert_eq!(index.count(), 1);
        assert!(index.contains(Path::new("/path/to/file")));

        let files = index.files_for_workload("workload-a");
        assert_eq!(files.len(), 1);

        let files = index.files_for_workload("workload-b");
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_file_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("files.json");

        let mut manager = FileStateManager::with_file(state_file.clone()).unwrap();

        // Record a file
        manager
            .record_install(
                Path::new("/test/file.txt"),
                "sha256:source",
                "sha256:installed",
                "test-workload",
                false,
            )
            .unwrap();

        assert!(manager.is_tracked(Path::new("/test/file.txt")));

        // Save and reload
        manager.save().unwrap();

        let manager2 = FileStateManager::with_file(state_file).unwrap();
        assert!(manager2.is_tracked(Path::new("/test/file.txt")));
    }

    #[test]
    fn test_would_conflict() {
        let mut index = FileStateIndex::new();

        let state = FileState::new(
            PathBuf::from("/shared/file"),
            "sha256:abc".to_string(),
            "sha256:abc".to_string(),
            "workload-a".to_string(),
            false,
        );

        index.set(state);

        // Same workload should not conflict
        assert!(index
            .would_conflict(Path::new("/shared/file"), "workload-a")
            .is_none());

        // Different workload should conflict
        assert_eq!(
            index.would_conflict(Path::new("/shared/file"), "workload-b"),
            Some("workload-a".to_string())
        );
    }

    #[test]
    fn test_remove_workload() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("files.json");

        let mut manager = FileStateManager::with_file(state_file).unwrap();

        // Add files for multiple workloads
        manager
            .record_install(Path::new("/a/file1.txt"), "h1", "h1", "workload-a", false)
            .unwrap();
        manager
            .record_install(Path::new("/a/file2.txt"), "h2", "h2", "workload-a", false)
            .unwrap();
        manager
            .record_install(Path::new("/b/file1.txt"), "h3", "h3", "workload-b", false)
            .unwrap();

        assert_eq!(manager.index().count(), 3);

        // Remove workload-a
        let removed = manager.remove_workload("workload-a");
        assert_eq!(removed.len(), 2);
        assert_eq!(manager.index().count(), 1);

        // workload-b should still be there
        assert!(manager.is_tracked(Path::new("/b/file1.txt")));
    }

    #[test]
    fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("files.json");

        let mut manager = FileStateManager::with_file(state_file).unwrap();

        let mut state1 = FileState::new(
            PathBuf::from("/file1"),
            "h1".to_string(),
            "h1".to_string(),
            "workload-a".to_string(),
            false,
        );
        state1.size = 1000;

        let mut state2 = FileState::new(
            PathBuf::from("/file2"),
            "h2".to_string(),
            "h2".to_string(),
            "workload-a".to_string(),
            true,
        );
        state2.size = 2000;

        manager.record_install_full(state1).unwrap();
        manager.record_install_full(state2).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.templated_files, 1);
        assert_eq!(stats.total_size, 3000);
        assert_eq!(stats.workload_count, 1);
    }
}
