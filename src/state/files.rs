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

/// Errors that can occur during file state operations
#[derive(Error, Debug)]
pub enum FileStateError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for file state operations
pub type FileStateResult<T> = Result<T, FileStateError>;

/// State information for a single managed file
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
}

/// The file state index containing all managed files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileStateIndex {
    /// Version of the index format
    pub version: u32,
    /// All file state entries, keyed by normalized path
    pub files: HashMap<String, FileState>,
    /// When the index was last updated
    pub last_updated: Option<DateTime<Utc>>,
}
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

    /// Insert or update a file state
    pub fn set(&mut self, state: FileState) {
        let key = Self::normalize_path(&state.path);
        self.files.insert(key, state);
        self.last_updated = Some(Utc::now());
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
}

/// Manager for file state tracking
pub struct FileStateManager {
    /// Path to the state file
    state_file: PathBuf,
    /// Cached index
    index: FileStateIndex,
    /// Whether changes have been made since last save
    dirty: bool,
}
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

    /// Record a file installation with full details
    pub fn record_install_full(&mut self, state: FileState) -> FileStateResult<()> {
        self.index.set(state);
        self.dirty = true;
        Ok(())
    }

    /// Get file state
    pub fn get(&self, path: &Path) -> Option<&FileState> {
        self.index.get(path)
    }

    /// Check if a file would conflict with existing tracked files
    pub fn would_conflict(&self, path: &Path, workload: &str) -> Option<String> {
        self.index.would_conflict(path, workload)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
