//! Backup management for Winforge
//!
//! This module provides comprehensive backup management including:
//! - Backup creation with hash verification
//! - Backup index tracking (stored in JSON)
//! - Backup restoration with integrity checks
//! - Rotation policies (by count, age, and total size)
//! - CLI commands support (list, show, restore, clean)

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors that can occur during backup operations
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum BackupError {
    /// Backup file not found
    #[error("Backup not found: {0}")]
    NotFound(String),

    /// Backup is corrupted (hash mismatch)
    #[error("Backup corrupted: expected hash {expected}, got {actual}")]
    Corrupted { expected: String, actual: String },

    /// Original file for backup doesn't exist
    #[error("Original file not found: {0}")]
    OriginalNotFound(PathBuf),

    /// Failed to create backup directory
    #[error("Failed to create backup directory: {path}")]
    CreateDirFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Failed to read/write index file
    #[error("Failed to access backup index: {0}")]
    IndexError(String),

    /// IO error during backup operation
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for backup operations
pub type BackupResult<T> = Result<T, BackupError>;

/// Configuration for backup rotation
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Maximum number of backups per file (default: 5)
    pub max_backups_per_file: usize,
    /// Maximum age of backups in days (default: 30)
    pub max_age_days: u32,
    /// Maximum total backup size in bytes (default: 1GB)
    pub max_total_size: u64,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_backups_per_file: 5,
            max_age_days: 30,
            max_total_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// A single backup entry
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Unique backup identifier (short hash)
    pub id: String,
    /// When the backup was created
    pub timestamp: DateTime<Utc>,
    /// Workload that created this backup
    pub workload: String,
    /// Original file path (expanded)
    pub original_path: PathBuf,
    /// Path to the backup file
    pub backup_path: PathBuf,
    /// SHA-256 hash of the backup file
    pub hash: String,
    /// Size of the backup file in bytes
    pub size: u64,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[allow(dead_code)]
impl BackupEntry {
    /// Generate a short ID from the hash
    pub fn generate_id(hash: &str) -> String {
        // Take first 6 characters of the hash (after "sha256:" prefix)
        if let Some(hex) = hash.strip_prefix("sha256:") {
            hex.chars().take(6).collect()
        } else {
            hash.chars().take(6).collect()
        }
    }

    /// Format the size for human display
    pub fn formatted_size(&self) -> String {
        format_size(self.size)
    }

    /// Check if this backup is older than the given duration
    pub fn is_older_than(&self, days: u32) -> bool {
        let cutoff = Utc::now() - Duration::days(days as i64);
        self.timestamp < cutoff
    }
}

/// The backup index containing all backup entries
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupIndex {
    /// Version of the index format
    pub version: u32,
    /// All backup entries
    pub backups: Vec<BackupEntry>,
    /// When the index was last updated
    pub last_updated: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
impl BackupIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            version: 1,
            backups: Vec::new(),
            last_updated: Some(Utc::now()),
        }
    }

    /// Find a backup by ID
    pub fn find_by_id(&self, id: &str) -> Option<&BackupEntry> {
        self.backups.iter().find(|b| b.id == id)
    }

    /// Find all backups for a specific workload
    pub fn find_by_workload(&self, workload: &str) -> Vec<&BackupEntry> {
        self.backups
            .iter()
            .filter(|b| b.workload == workload)
            .collect()
    }

    /// Find all backups for a specific original path
    pub fn find_by_path(&self, path: &Path) -> Vec<&BackupEntry> {
        self.backups
            .iter()
            .filter(|b| b.original_path == path)
            .collect()
    }

    /// Get total size of all backups
    pub fn total_size(&self) -> u64 {
        self.backups.iter().map(|b| b.size).sum()
    }

    /// Get backup count
    pub fn count(&self) -> usize {
        self.backups.len()
    }

    /// Group backups by original path
    pub fn group_by_path(&self) -> HashMap<PathBuf, Vec<&BackupEntry>> {
        let mut groups: HashMap<PathBuf, Vec<&BackupEntry>> = HashMap::new();
        for backup in &self.backups {
            groups
                .entry(backup.original_path.clone())
                .or_default()
                .push(backup);
        }
        groups
    }
}

/// Backup manager handles all backup operations
#[allow(dead_code)]
pub struct BackupManager {
    /// Directory where backups are stored
    backup_dir: PathBuf,
    /// Path to the index file
    index_file: PathBuf,
    /// Rotation configuration
    rotation_config: RotationConfig,
    /// Dry run mode
    dry_run: bool,
}

#[allow(dead_code)]
impl BackupManager {
    /// Create a new backup manager with default paths
    pub fn new() -> BackupResult<Self> {
        let backup_dir = Self::default_backup_dir()?;
        Self::with_dir(backup_dir)
    }

    /// Create a backup manager with a specific backup directory
    pub fn with_dir(backup_dir: PathBuf) -> BackupResult<Self> {
        let index_file = backup_dir.join("index.json");

        // Create backup directory if it doesn't exist
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).map_err(|e| BackupError::CreateDirFailed {
                path: backup_dir.clone(),
                source: e,
            })?;
        }

        Ok(Self {
            backup_dir,
            index_file,
            rotation_config: RotationConfig::default(),
            dry_run: false,
        })
    }

    /// Get the default backup directory (~/.winforge/backups)
    pub fn default_backup_dir() -> BackupResult<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| BackupError::IndexError("Could not determine home directory".into()))?;
        Ok(home.join(".winforge").join("backups"))
    }

    /// Set dry run mode
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Set rotation configuration
    pub fn with_rotation_config(mut self, config: RotationConfig) -> Self {
        self.rotation_config = config;
        self
    }

    /// Get the backup directory path
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Load the backup index from disk
    pub fn load_index(&self) -> BackupResult<BackupIndex> {
        if !self.index_file.exists() {
            return Ok(BackupIndex::new());
        }

        let content = fs::read_to_string(&self.index_file)?;
        let index: BackupIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Save the backup index to disk
    pub fn save_index(&self, index: &BackupIndex) -> BackupResult<()> {
        if self.dry_run {
            tracing::info!("Would save backup index to: {}", self.index_file.display());
            return Ok(());
        }

        let mut index = index.clone();
        index.last_updated = Some(Utc::now());

        let content = serde_json::to_string_pretty(&index)?;
        fs::write(&self.index_file, content)?;
        Ok(())
    }

    /// Create a backup of a file
    pub fn backup_file(&self, original_path: &Path, workload: &str) -> BackupResult<BackupEntry> {
        if !original_path.exists() {
            return Err(BackupError::OriginalNotFound(original_path.to_path_buf()));
        }

        let timestamp = Utc::now();
        let timestamp_str = timestamp.format("%Y%m%d_%H%M%S").to_string();

        // Create backup filename
        let filename = original_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Create a unique backup filename
        let backup_filename = format!("{}_{}", filename, timestamp_str);
        let backup_path = self.backup_dir.join(&backup_filename);

        // Compute hash of original file
        let hash = compute_file_hash(original_path)?;

        // Get file size
        let metadata = fs::metadata(original_path)?;
        let size = metadata.len();

        // Generate backup ID
        let id = BackupEntry::generate_id(&hash);

        if self.dry_run {
            tracing::info!(
                "Would backup {} -> {}",
                original_path.display(),
                backup_path.display()
            );
        } else {
            // Copy file to backup location
            fs::copy(original_path, &backup_path)?;
            tracing::debug!(
                "Backed up {} -> {}",
                original_path.display(),
                backup_path.display()
            );
        }

        let entry = BackupEntry {
            id,
            timestamp,
            workload: workload.to_string(),
            original_path: original_path.to_path_buf(),
            backup_path,
            hash,
            size,
            description: None,
        };

        // Update index
        let mut index = self.load_index()?;
        index.backups.push(entry.clone());
        self.save_index(&index)?;

        Ok(entry)
    }

    /// Restore a backup by ID
    pub fn restore_by_id(&self, id: &str) -> BackupResult<PathBuf> {
        let index = self.load_index()?;
        let entry = index
            .find_by_id(id)
            .ok_or_else(|| BackupError::NotFound(id.to_string()))?;

        self.restore_entry(entry)
    }

    /// Restore a backup entry
    pub fn restore_entry(&self, entry: &BackupEntry) -> BackupResult<PathBuf> {
        if !entry.backup_path.exists() {
            return Err(BackupError::NotFound(
                entry.backup_path.display().to_string(),
            ));
        }

        // Verify backup integrity
        let current_hash = compute_file_hash(&entry.backup_path)?;
        if current_hash != entry.hash {
            return Err(BackupError::Corrupted {
                expected: entry.hash.clone(),
                actual: current_hash,
            });
        }

        if self.dry_run {
            tracing::info!(
                "Would restore {} -> {}",
                entry.backup_path.display(),
                entry.original_path.display()
            );
        } else {
            // Create parent directory if needed
            if let Some(parent) = entry.original_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            // Copy backup to original location
            fs::copy(&entry.backup_path, &entry.original_path)?;
            tracing::info!(
                "Restored {} -> {}",
                entry.backup_path.display(),
                entry.original_path.display()
            );
        }

        Ok(entry.original_path.clone())
    }

    /// Restore all backups for a workload
    pub fn restore_workload(&self, workload: &str) -> BackupResult<Vec<PathBuf>> {
        let index = self.load_index()?;
        let entries = index.find_by_workload(workload);

        if entries.is_empty() {
            return Err(BackupError::NotFound(format!(
                "No backups found for workload: {}",
                workload
            )));
        }

        // Group by original path and get the most recent backup for each
        let mut latest_by_path: HashMap<PathBuf, &BackupEntry> = HashMap::new();
        for entry in entries {
            let current = latest_by_path.get(&entry.original_path);
            if current.is_none() || entry.timestamp > current.unwrap().timestamp {
                latest_by_path.insert(entry.original_path.clone(), entry);
            }
        }

        let mut restored = Vec::new();
        for entry in latest_by_path.values() {
            let path = self.restore_entry(entry)?;
            restored.push(path);
        }

        Ok(restored)
    }

    /// Delete a backup by ID
    pub fn delete_by_id(&self, id: &str) -> BackupResult<()> {
        let mut index = self.load_index()?;
        let pos = index
            .backups
            .iter()
            .position(|b| b.id == id)
            .ok_or_else(|| BackupError::NotFound(id.to_string()))?;

        let entry = &index.backups[pos];

        if self.dry_run {
            tracing::info!("Would delete backup: {}", entry.backup_path.display());
        } else {
            if entry.backup_path.exists() {
                fs::remove_file(&entry.backup_path)?;
            }
            index.backups.remove(pos);
            self.save_index(&index)?;
        }

        Ok(())
    }

    /// Apply rotation policy to clean up old backups
    pub fn apply_rotation(&self) -> BackupResult<RotationResult> {
        let mut index = self.load_index()?;
        let mut result = RotationResult::default();

        // Track backups to remove
        let mut to_remove: Vec<usize> = Vec::new();

        // 1. Remove backups older than max_age_days
        let cutoff = Utc::now() - Duration::days(self.rotation_config.max_age_days as i64);
        for (i, backup) in index.backups.iter().enumerate() {
            if backup.timestamp < cutoff {
                to_remove.push(i);
                result.removed_by_age += 1;
                result.bytes_freed += backup.size;
            }
        }

        // 2. Keep only max_backups_per_file for each file
        let groups = index.group_by_path();
        for (_path, mut backups) in groups {
            if backups.len() > self.rotation_config.max_backups_per_file {
                // Sort by timestamp, newest first
                backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

                // Mark excess backups for removal
                for backup in backups
                    .iter()
                    .skip(self.rotation_config.max_backups_per_file)
                {
                    if let Some(i) = index.backups.iter().position(|b| b.id == backup.id) {
                        if !to_remove.contains(&i) {
                            to_remove.push(i);
                            result.removed_by_count += 1;
                            result.bytes_freed += backup.size;
                        }
                    }
                }
            }
        }

        // 3. Check total size limit
        let mut total_size: u64 = index.backups.iter().map(|b| b.size).sum();
        if total_size > self.rotation_config.max_total_size {
            // Sort by timestamp, oldest first
            let mut sorted_indices: Vec<usize> = (0..index.backups.len()).collect();
            sorted_indices
                .sort_by(|&a, &b| index.backups[a].timestamp.cmp(&index.backups[b].timestamp));

            for i in sorted_indices {
                if total_size <= self.rotation_config.max_total_size {
                    break;
                }
                if !to_remove.contains(&i) {
                    to_remove.push(i);
                    result.removed_by_size += 1;
                    result.bytes_freed += index.backups[i].size;
                    total_size -= index.backups[i].size;
                }
            }
        }

        // Remove backups (in reverse order to maintain indices)
        to_remove.sort_by(|a, b| b.cmp(a));
        for i in to_remove {
            let backup = &index.backups[i];
            if self.dry_run {
                tracing::info!("Would remove backup: {}", backup.backup_path.display());
            } else {
                if backup.backup_path.exists() {
                    fs::remove_file(&backup.backup_path)?;
                }
                index.backups.remove(i);
            }
        }

        if !self.dry_run {
            self.save_index(&index)?;
        }

        Ok(result)
    }

    /// Clean backups older than a specific duration
    pub fn clean_older_than(&self, days: u32) -> BackupResult<CleanResult> {
        let mut index = self.load_index()?;
        let cutoff = Utc::now() - Duration::days(days as i64);

        let mut result = CleanResult::default();
        let mut to_remove: Vec<usize> = Vec::new();

        for (i, backup) in index.backups.iter().enumerate() {
            if backup.timestamp < cutoff {
                to_remove.push(i);
                result.removed_count += 1;
                result.bytes_freed += backup.size;
            }
        }

        // Remove in reverse order
        to_remove.sort_by(|a, b| b.cmp(a));
        for i in to_remove {
            let backup = &index.backups[i];
            if self.dry_run {
                tracing::info!(
                    "Would remove backup: {} (age: {} days)",
                    backup.backup_path.display(),
                    (Utc::now() - backup.timestamp).num_days()
                );
            } else {
                if backup.backup_path.exists() {
                    fs::remove_file(&backup.backup_path)?;
                }
                index.backups.remove(i);
            }
        }

        if !self.dry_run {
            self.save_index(&index)?;
        }

        Ok(result)
    }

    /// List all backups
    pub fn list(&self) -> BackupResult<Vec<BackupEntry>> {
        let index = self.load_index()?;
        Ok(index.backups)
    }

    /// List backups for a specific workload
    pub fn list_for_workload(&self, workload: &str) -> BackupResult<Vec<BackupEntry>> {
        let index = self.load_index()?;
        Ok(index
            .find_by_workload(workload)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get a specific backup by ID
    pub fn get(&self, id: &str) -> BackupResult<BackupEntry> {
        let index = self.load_index()?;
        index
            .find_by_id(id)
            .cloned()
            .ok_or_else(|| BackupError::NotFound(id.to_string()))
    }

    /// Verify integrity of all backups
    pub fn verify_all(&self) -> BackupResult<VerifyResult> {
        let index = self.load_index()?;
        let mut result = VerifyResult::default();

        for backup in &index.backups {
            result.total += 1;

            if !backup.backup_path.exists() {
                result.missing.push(backup.id.clone());
                continue;
            }

            match compute_file_hash(&backup.backup_path) {
                Ok(hash) if hash == backup.hash => {
                    result.valid += 1;
                }
                Ok(hash) => {
                    result.corrupted.push((backup.id.clone(), hash));
                }
                Err(_) => {
                    result.errors.push(backup.id.clone());
                }
            }
        }

        Ok(result)
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default BackupManager")
    }
}

/// Result of a rotation operation
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct RotationResult {
    /// Backups removed due to age
    pub removed_by_age: usize,
    /// Backups removed due to per-file count limit
    pub removed_by_count: usize,
    /// Backups removed due to total size limit
    pub removed_by_size: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
}

#[allow(dead_code)]
impl RotationResult {
    /// Total backups removed
    pub fn total_removed(&self) -> usize {
        self.removed_by_age + self.removed_by_count + self.removed_by_size
    }

    /// Format bytes freed for display
    pub fn formatted_bytes_freed(&self) -> String {
        format_size(self.bytes_freed)
    }
}

/// Result of a clean operation
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct CleanResult {
    /// Number of backups removed
    pub removed_count: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
}

impl CleanResult {
    /// Format bytes freed for display
    pub fn formatted_bytes_freed(&self) -> String {
        format_size(self.bytes_freed)
    }
}

/// Result of a verification operation
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct VerifyResult {
    /// Total backups checked
    pub total: usize,
    /// Number of valid backups
    pub valid: usize,
    /// IDs of missing backup files
    pub missing: Vec<String>,
    /// IDs and actual hashes of corrupted backups
    pub corrupted: Vec<(String, String)>,
    /// IDs of backups that couldn't be verified due to errors
    pub errors: Vec<String>,
}

#[allow(dead_code)]
impl VerifyResult {
    /// Check if all backups are valid
    pub fn all_valid(&self) -> bool {
        self.missing.is_empty() && self.corrupted.is_empty() && self.errors.is_empty()
    }
}

/// Compute SHA-256 hash of a file
pub fn compute_file_hash(path: &Path) -> BackupResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("sha256:{:x}", hash))
}

/// Format a size in bytes for human display
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let files_dir = temp_dir.path().join("files");
        fs::create_dir_all(&files_dir).unwrap();

        let manager = BackupManager::with_dir(backup_dir).unwrap();

        // Create a test file
        let test_file = create_test_file(&files_dir, "test.txt", "original content");

        // Create backup
        let entry = manager.backup_file(&test_file, "test-workload").unwrap();
        assert!(!entry.id.is_empty());
        assert!(entry.backup_path.exists());

        // Modify original file
        fs::write(&test_file, "modified content").unwrap();
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "modified content");

        // Restore backup
        manager.restore_by_id(&entry.id).unwrap();
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "original content");
    }

    #[test]
    fn test_backup_index() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let files_dir = temp_dir.path().join("files");
        fs::create_dir_all(&files_dir).unwrap();

        let manager = BackupManager::with_dir(backup_dir).unwrap();

        // Create multiple backups
        let file1 = create_test_file(&files_dir, "file1.txt", "content 1");
        let file2 = create_test_file(&files_dir, "file2.txt", "content 2");

        manager.backup_file(&file1, "workload-a").unwrap();
        manager.backup_file(&file2, "workload-a").unwrap();
        manager.backup_file(&file1, "workload-b").unwrap();

        // List all backups
        let all = manager.list().unwrap();
        assert_eq!(all.len(), 3);

        // List by workload
        let workload_a = manager.list_for_workload("workload-a").unwrap();
        assert_eq!(workload_a.len(), 2);
    }

    #[test]
    fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let files_dir = temp_dir.path().join("files");
        fs::create_dir_all(&files_dir).unwrap();

        let manager = BackupManager::with_dir(backup_dir).unwrap();

        let test_file = create_test_file(&files_dir, "test.txt", "content");
        let entry = manager.backup_file(&test_file, "test-workload").unwrap();

        assert!(entry.backup_path.exists());

        manager.delete_by_id(&entry.id).unwrap();

        assert!(!entry.backup_path.exists());
        assert!(manager.get(&entry.id).is_err());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_backup_entry_id() {
        let id = BackupEntry::generate_id("sha256:abc123def456");
        assert_eq!(id, "abc123");
    }

    #[test]
    fn test_verify_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let files_dir = temp_dir.path().join("files");
        fs::create_dir_all(&files_dir).unwrap();

        let manager = BackupManager::with_dir(backup_dir).unwrap();

        let test_file = create_test_file(&files_dir, "test.txt", "content");
        manager.backup_file(&test_file, "test-workload").unwrap();

        let result = manager.verify_all().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.valid, 1);
        assert!(result.all_valid());
    }
}
