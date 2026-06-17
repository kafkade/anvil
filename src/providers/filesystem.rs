//! Filesystem provider for file operations
//!
//! This module handles all file-related operations including:
//! - Copying files from workload to target destinations
//! - Backing up existing files before overwriting
//! - Computing and comparing file hashes for verification
//! - Variable expansion in paths
//! - Atomic file writes (write to temp, then rename)
//! - Directory copy with glob pattern support
//! - Rollback capability for failed operations
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use glob::Pattern;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;
use walkdir::WalkDir;

use super::ProviderConfig;

/// Errors that can occur during filesystem operations
#[derive(Error, Debug)]
#[allow(dead_code)] // Variants cover all expected error conditions
pub enum FilesystemError {
    /// File not found
    #[error("File not found: {0}")]
    NotFound(PathBuf),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    /// Directory creation failed
    #[error("Failed to create directory: {path}")]
    CreateDirectoryFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// File copy failed
    #[error("Failed to copy file from {source} to {destination}")]
    CopyFailed {
        source: PathBuf,
        destination: PathBuf,
        #[source]
        error: io::Error,
    },

    /// Backup failed
    #[error("Failed to backup file: {path}")]
    BackupFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Hash computation failed
    #[error("Failed to compute hash for: {path}")]
    HashFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Hash verification failed after copy
    #[error("Hash verification failed for {path}: expected {expected}, got {actual}")]
    HashVerificationFailed {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Glob pattern error
    #[error("Invalid glob pattern: {pattern}")]
    InvalidGlobPattern {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },

    /// Atomic write failed
    #[error("Atomic write failed for {path}")]
    AtomicWriteFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// File in use
    #[error("File is in use by another process: {0}")]
    FileInUse(PathBuf),

    /// Path too long (Windows limitation)
    #[error("Path too long: {0}")]
    PathTooLong(PathBuf),

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Result of a file comparison
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCompareResult {
    /// Files are identical
    Match,
    /// Files have different content
    Different {
        expected_hash: String,
        actual_hash: String,
    },
    /// Target file does not exist
    TargetMissing,
    /// Source file does not exist
    SourceMissing,
}

/// Information about a backup file
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Original file path
    #[allow(dead_code)] // Part of data structure API
    pub original_path: PathBuf,
    /// Backup file path
    pub backup_path: PathBuf,
    /// Timestamp of the backup
    #[allow(dead_code)] // Part of data structure API
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Hash of the backed up file
    pub hash: String,
}

/// Result of a file copy operation
#[derive(Debug, Clone)]
pub struct CopyResult {
    /// Source path
    #[allow(dead_code)] // Part of data structure API
    pub source: PathBuf,
    /// Destination path
    #[allow(dead_code)] // Part of data structure API
    pub destination: PathBuf,
    /// Hash of the copied file
    pub hash: String,
    /// Size of the file in bytes
    pub size: u64,
    /// Whether a backup was created
    pub backup_info: Option<BackupInfo>,
    /// Whether the file was skipped (identical)
    pub skipped: bool,
}

/// A recorded file operation for rollback
#[derive(Debug, Clone)]
pub struct FileOperation {
    /// Type of operation
    #[allow(dead_code)] // Part of data structure API
    pub op_type: FileOperationType,
    /// Path affected
    #[allow(dead_code)] // Part of data structure API
    pub path: PathBuf,
    /// Backup path (if applicable)
    #[allow(dead_code)] // Part of data structure API
    pub backup_path: Option<PathBuf>,
    /// Timestamp
    #[allow(dead_code)] // Part of data structure API
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Type of file operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOperationType {
    /// File was created (new)
    Created,
    /// File was overwritten (backup exists)
    Overwritten,
    /// Directory was created
    DirectoryCreated,
}

/// Options for file copy operations
#[derive(Debug, Clone)]
pub struct CopyOptions {
    /// Create backup of existing files
    pub backup: bool,
    /// Verify hash after copy
    pub verify: bool,
    /// Use atomic writes
    pub atomic: bool,
    /// Overwrite existing files
    #[allow(dead_code)] // Part of data structure API
    pub overwrite: bool,
    /// Preserve file attributes
    pub preserve_attributes: bool,
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            backup: true,
            verify: true,
            atomic: true,
            overwrite: true,
            preserve_attributes: true,
        }
    }
}

/// Options for directory copy operations
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Default)]
pub struct DirectoryCopyOptions {
    /// Base copy options
    pub copy_options: CopyOptions,
    /// Include patterns (glob)
    pub include: Vec<String>,
    /// Exclude patterns (glob)
    pub exclude: Vec<String>,
    /// Recursive copy
    pub recursive: bool,
}

#[allow(dead_code)] // Test-only: used in module tests
impl DirectoryCopyOptions {
    /// Create new directory copy options with default settings
    pub fn new() -> Self {
        Self {
            copy_options: CopyOptions::default(),
            include: Vec::new(),
            exclude: Vec::new(),
            recursive: true,
        }
    }

    /// Set include patterns
    pub fn with_include(mut self, patterns: Vec<String>) -> Self {
        self.include = patterns;
        self
    }

    /// Set exclude patterns
    pub fn with_exclude(mut self, patterns: Vec<String>) -> Self {
        self.exclude = patterns;
        self
    }
}

/// Filesystem provider for file operations
#[derive(Debug)]
pub struct FilesystemProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// Directory for storing backups
    backup_dir: PathBuf,
    /// Recorded operations for potential rollback
    operations: Vec<FileOperation>,
}
impl FilesystemProvider {
    /// Create a new filesystem provider with default configuration
    pub fn new() -> Self {
        let backup_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anvil")
            .join("backups");

        Self {
            config: ProviderConfig::default(),
            backup_dir,
            operations: Vec::new(),
        }
    }

    /// Create a new filesystem provider with custom configuration
    pub fn with_config(config: ProviderConfig) -> Self {
        let mut provider = Self::new();
        provider.config = config;
        provider
    }

    /// Set the backup directory
    #[allow(dead_code)]
    pub fn with_backup_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.backup_dir = path.into();
        self
    }

    /// Get the backup directory
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Check if in dry-run mode
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn is_dry_run(&self) -> bool {
        self.config.dry_run
    }

    /// Copy a file from source to destination with full options
    pub fn copy_file_with_options(
        &mut self,
        source: &Path,
        destination: &Path,
        options: &CopyOptions,
    ) -> Result<CopyResult, FilesystemError> {
        // Validate source exists
        if !source.exists() {
            return Err(FilesystemError::NotFound(source.to_path_buf()));
        }

        // Check for path length on Windows
        #[cfg(windows)]
        {
            if destination.to_string_lossy().len() > 260 {
                // Try with long path prefix
                let dest_str = format!("\\\\?\\{}", destination.display());
                if dest_str.len() > 32767 {
                    return Err(FilesystemError::PathTooLong(destination.to_path_buf()));
                }
            }
        }

        // Check if destination exists and is identical
        if destination.exists() {
            let source_hash = self.compute_hash(source)?;
            let dest_hash = self.compute_hash(destination)?;

            if source_hash == dest_hash {
                // Files are identical, skip
                let metadata = fs::metadata(source)?;
                return Ok(CopyResult {
                    source: source.to_path_buf(),
                    destination: destination.to_path_buf(),
                    hash: source_hash,
                    size: metadata.len(),
                    backup_info: None,
                    skipped: true,
                });
            }
        }

        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            if !parent.exists() {
                if self.config.dry_run {
                    tracing::info!("Would create directory: {}", parent.display());
                } else {
                    fs::create_dir_all(parent).map_err(|e| {
                        FilesystemError::CreateDirectoryFailed {
                            path: parent.to_path_buf(),
                            source: e,
                        }
                    })?;
                    self.operations.push(FileOperation {
                        op_type: FileOperationType::DirectoryCreated,
                        path: parent.to_path_buf(),
                        backup_path: None,
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }

        // Backup existing file if requested
        let backup_info = if options.backup && destination.exists() {
            Some(self.backup_file(destination)?)
        } else {
            None
        };

        // Compute source hash
        let source_hash = self.compute_hash(source)?;
        let source_metadata = fs::metadata(source)?;
        let source_size = source_metadata.len();

        if self.config.dry_run {
            tracing::info!(
                "Would copy {} -> {}",
                source.display(),
                destination.display()
            );
            return Ok(CopyResult {
                source: source.to_path_buf(),
                destination: destination.to_path_buf(),
                hash: source_hash,
                size: source_size,
                backup_info,
                skipped: false,
            });
        }

        // Perform the copy
        if options.atomic {
            self.atomic_copy(source, destination)?;
        } else {
            fs::copy(source, destination).map_err(|e| FilesystemError::CopyFailed {
                source: source.to_path_buf(),
                destination: destination.to_path_buf(),
                error: e,
            })?;
        }

        // Verify hash if requested
        if options.verify {
            let dest_hash = self.compute_hash(destination)?;
            if dest_hash != source_hash {
                return Err(FilesystemError::HashVerificationFailed {
                    path: destination.to_path_buf(),
                    expected: source_hash,
                    actual: dest_hash,
                });
            }
        }

        // Preserve attributes if requested
        if options.preserve_attributes {
            self.preserve_attributes(source, destination)?;
        }

        // Record the operation
        let op_type = if backup_info.is_some() {
            FileOperationType::Overwritten
        } else {
            FileOperationType::Created
        };

        self.operations.push(FileOperation {
            op_type,
            path: destination.to_path_buf(),
            backup_path: backup_info.as_ref().map(|b| b.backup_path.clone()),
            timestamp: chrono::Utc::now(),
        });

        tracing::debug!("Copied {} -> {}", source.display(), destination.display());

        Ok(CopyResult {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            hash: source_hash,
            size: source_size,
            backup_info,
            skipped: false,
        })
    }

    /// Copy a file from source to destination (simplified interface)
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn copy_file(
        &mut self,
        source: &Path,
        destination: &Path,
        backup: bool,
    ) -> Result<Option<BackupInfo>, FilesystemError> {
        let options = CopyOptions {
            backup,
            ..Default::default()
        };

        let result = self.copy_file_with_options(source, destination, &options)?;
        Ok(result.backup_info)
    }

    /// Perform an atomic copy (write to temp file, then rename)
    fn atomic_copy(&self, source: &Path, destination: &Path) -> Result<(), FilesystemError> {
        // Get the parent directory of destination for temp file
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));

        // Create a temp file in the same directory
        let temp_file =
            NamedTempFile::new_in(parent).map_err(|e| FilesystemError::AtomicWriteFailed {
                path: destination.to_path_buf(),
                source: e,
            })?;

        // Copy content to temp file
        fs::copy(source, temp_file.path()).map_err(|e| FilesystemError::CopyFailed {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            error: e,
        })?;

        // Persist (rename) the temp file to destination
        temp_file
            .persist(destination)
            .map_err(|e| FilesystemError::AtomicWriteFailed {
                path: destination.to_path_buf(),
                source: e.error,
            })?;

        Ok(())
    }

    /// Preserve file attributes (read-only, hidden on Windows)
    fn preserve_attributes(
        &self,
        _source: &Path,
        _destination: &Path,
    ) -> Result<(), FilesystemError> {
        // On Windows, preserve read-only and hidden attributes
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            let source_meta = fs::metadata(_source)?;
            let attrs = source_meta.file_attributes();

            // FILE_ATTRIBUTE_READONLY = 0x1
            // FILE_ATTRIBUTE_HIDDEN = 0x2
            const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

            let is_readonly = (attrs & FILE_ATTRIBUTE_READONLY) != 0;
            let is_hidden = (attrs & FILE_ATTRIBUTE_HIDDEN) != 0;

            if is_readonly || is_hidden {
                // Use SetFileAttributes via std::process::Command
                // For now, just set read-only through std::fs
                if is_readonly {
                    let mut perms = fs::metadata(_destination)?.permissions();
                    perms.set_readonly(true);
                    fs::set_permissions(_destination, perms)?;
                }
            }
        }

        Ok(())
    }

    /// Copy a directory recursively with options
    #[allow(dead_code)]
    pub fn copy_directory(
        &mut self,
        source: &Path,
        destination: &Path,
        options: &DirectoryCopyOptions,
    ) -> Result<Vec<CopyResult>, FilesystemError> {
        if !source.exists() {
            return Err(FilesystemError::NotFound(source.to_path_buf()));
        }

        if !source.is_dir() {
            return Err(FilesystemError::InvalidPath(format!(
                "Not a directory: {}",
                source.display()
            )));
        }

        // Compile glob patterns
        let include_patterns: Vec<Pattern> = options
            .include
            .iter()
            .map(|p| Pattern::new(p))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| FilesystemError::InvalidGlobPattern {
                pattern: e.msg.to_string(),
                source: e,
            })?;

        let exclude_patterns: Vec<Pattern> = options
            .exclude
            .iter()
            .map(|p| Pattern::new(p))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| FilesystemError::InvalidGlobPattern {
                pattern: e.msg.to_string(),
                source: e,
            })?;

        let mut results = Vec::new();
        let walker = if options.recursive {
            WalkDir::new(source)
        } else {
            WalkDir::new(source).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip the root directory itself
            if path == source {
                continue;
            }

            // Skip directories (we'll create them as needed)
            if path.is_dir() {
                continue;
            }

            // Get relative path
            let relative = path.strip_prefix(source).unwrap_or(path);
            let relative_str = relative.to_string_lossy();

            // Check include patterns (if any)
            if !include_patterns.is_empty() {
                let matches = include_patterns.iter().any(|p| p.matches(&relative_str));
                if !matches {
                    continue;
                }
            }

            // Check exclude patterns
            if exclude_patterns.iter().any(|p| p.matches(&relative_str)) {
                continue;
            }

            // Build destination path
            let dest_path = destination.join(relative);

            // Copy the file
            let result = self.copy_file_with_options(path, &dest_path, &options.copy_options)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Expand glob patterns in a source path and return matching files
    #[allow(dead_code)]
    pub fn expand_glob(
        &self,
        pattern: &str,
        base_dir: &Path,
    ) -> Result<Vec<PathBuf>, FilesystemError> {
        let full_pattern = base_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let paths: Vec<PathBuf> = glob::glob(&pattern_str)
            .map_err(|e| FilesystemError::InvalidGlobPattern {
                pattern: pattern.to_string(),
                source: e,
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(paths)
    }

    /// Backup a file to the backup directory
    pub fn backup_file(&self, path: &Path) -> Result<BackupInfo, FilesystemError> {
        let timestamp = chrono::Utc::now();
        let timestamp_str = timestamp.format("%Y%m%d_%H%M%S").to_string();

        // Create backup filename
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| FilesystemError::InvalidPath(path.display().to_string()))?;

        let backup_filename = format!("{}_{}", filename, timestamp_str);
        let backup_path = self.backup_dir.join(&backup_filename);

        // Ensure backup directory exists
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir).map_err(|e| {
                FilesystemError::CreateDirectoryFailed {
                    path: self.backup_dir.clone(),
                    source: e,
                }
            })?;
        }

        // Compute hash before backing up
        let hash = self.compute_hash(path)?;

        // Copy to backup location
        if self.config.dry_run {
            tracing::info!(
                "Would backup {} -> {}",
                path.display(),
                backup_path.display()
            );
        } else {
            fs::copy(path, &backup_path).map_err(|e| FilesystemError::BackupFailed {
                path: path.to_path_buf(),
                source: e,
            })?;
            tracing::debug!("Backed up {} -> {}", path.display(), backup_path.display());
        }

        Ok(BackupInfo {
            original_path: path.to_path_buf(),
            backup_path,
            timestamp,
            hash,
        })
    }

    /// Restore a file from backup
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn restore_file(&self, backup: &BackupInfo) -> Result<(), FilesystemError> {
        if !backup.backup_path.exists() {
            return Err(FilesystemError::NotFound(backup.backup_path.clone()));
        }

        if self.config.dry_run {
            tracing::info!(
                "Would restore {} -> {}",
                backup.backup_path.display(),
                backup.original_path.display()
            );
        } else {
            // Create parent directory if needed
            if let Some(parent) = backup.original_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            fs::copy(&backup.backup_path, &backup.original_path).map_err(|e| {
                FilesystemError::CopyFailed {
                    source: backup.backup_path.clone(),
                    destination: backup.original_path.clone(),
                    error: e,
                }
            })?;
            tracing::debug!(
                "Restored {} -> {}",
                backup.backup_path.display(),
                backup.original_path.display()
            );
        }

        Ok(())
    }

    /// Rollback all operations recorded in this session
    #[allow(dead_code)]
    pub fn rollback(&mut self) -> Result<RollbackResult, FilesystemError> {
        let mut result = RollbackResult::default();

        // Process operations in reverse order
        for op in self.operations.iter().rev() {
            match op.op_type {
                FileOperationType::Created => {
                    // Remove the created file
                    if op.path.exists() {
                        if self.config.dry_run {
                            tracing::info!("Would remove created file: {}", op.path.display());
                        } else {
                            fs::remove_file(&op.path)?;
                            result.files_removed += 1;
                        }
                    }
                }
                FileOperationType::Overwritten => {
                    // Restore from backup
                    if let Some(backup_path) = &op.backup_path {
                        if backup_path.exists() {
                            if self.config.dry_run {
                                tracing::info!(
                                    "Would restore {} from {}",
                                    op.path.display(),
                                    backup_path.display()
                                );
                            } else {
                                fs::copy(backup_path, &op.path)?;
                                result.files_restored += 1;
                            }
                        }
                    }
                }
                FileOperationType::DirectoryCreated => {
                    // Try to remove the directory if empty
                    if op.path.exists()
                        && op.path.is_dir()
                        && fs::read_dir(&op.path)?.next().is_none()
                    {
                        if self.config.dry_run {
                            tracing::info!("Would remove empty directory: {}", op.path.display());
                        } else {
                            fs::remove_dir(&op.path)?;
                            result.directories_removed += 1;
                        }
                    }
                }
            }
        }

        // Clear the operations log
        self.operations.clear();

        Ok(result)
    }

    /// Clear the operations log without rolling back
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn clear_operations(&mut self) {
        self.operations.clear();
    }

    /// Get the current operations log
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn operations(&self) -> &[FileOperation] {
        &self.operations
    }

    /// Compute SHA-256 hash of a file
    pub fn compute_hash(&self, path: &Path) -> Result<String, FilesystemError> {
        let mut file = fs::File::open(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                FilesystemError::NotFound(path.to_path_buf())
            } else {
                FilesystemError::HashFailed {
                    path: path.to_path_buf(),
                    source: e,
                }
            }
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| FilesystemError::HashFailed {
                    path: path.to_path_buf(),
                    source: e,
                })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!(
            "sha256:{}",
            hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
    }

    /// Compare two files by their content hash
    #[allow(dead_code)]
    pub fn compare_files(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<FileCompareResult, FilesystemError> {
        // Check if source exists
        if !source.exists() {
            return Ok(FileCompareResult::SourceMissing);
        }

        // Check if target exists
        if !target.exists() {
            return Ok(FileCompareResult::TargetMissing);
        }

        let source_hash = self.compute_hash(source)?;
        let target_hash = self.compute_hash(target)?;

        if source_hash == target_hash {
            Ok(FileCompareResult::Match)
        } else {
            Ok(FileCompareResult::Different {
                expected_hash: source_hash,
                actual_hash: target_hash,
            })
        }
    }

    /// Check if a file exists
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// Check if a path is a file
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    /// Check if a path is a directory
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    /// Get file metadata
    pub fn metadata(&self, path: &Path) -> Result<fs::Metadata, FilesystemError> {
        fs::metadata(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                FilesystemError::NotFound(path.to_path_buf())
            } else {
                FilesystemError::Io(e)
            }
        })
    }

    /// Get file size
    pub fn file_size(&self, path: &Path) -> Result<u64, FilesystemError> {
        let metadata = self.metadata(path)?;
        Ok(metadata.len())
    }

    /// Read a file's contents to a string
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn read_to_string(&self, path: &Path) -> Result<String, FilesystemError> {
        fs::read_to_string(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                FilesystemError::NotFound(path.to_path_buf())
            } else {
                FilesystemError::Io(e)
            }
        })
    }

    /// Write content to a file (atomic by default)
    pub fn write(&self, path: &Path, content: &str) -> Result<(), FilesystemError> {
        self.write_bytes(path, content.as_bytes())
    }

    /// Write bytes to a file (atomic by default)
    pub fn write_bytes(&self, path: &Path, content: &[u8]) -> Result<(), FilesystemError> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| FilesystemError::CreateDirectoryFailed {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }

        if self.config.dry_run {
            tracing::info!("Would write {} bytes to {}", content.len(), path.display());
            return Ok(());
        }

        // Use atomic write
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file =
            NamedTempFile::new_in(parent).map_err(|e| FilesystemError::AtomicWriteFailed {
                path: path.to_path_buf(),
                source: e,
            })?;

        temp_file
            .as_file()
            .write_all(content)
            .map_err(|e| FilesystemError::AtomicWriteFailed {
                path: path.to_path_buf(),
                source: e,
            })?;

        temp_file
            .persist(path)
            .map_err(|e| FilesystemError::AtomicWriteFailed {
                path: path.to_path_buf(),
                source: e.error,
            })?;

        Ok(())
    }

    /// Write content to a file without atomic write
    #[allow(dead_code)] // Module API: used in tests and future CLI integration
    pub fn write_direct(&self, path: &Path, content: &str) -> Result<(), FilesystemError> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| FilesystemError::CreateDirectoryFailed {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }

        if self.config.dry_run {
            tracing::info!("Would write {} bytes to {}", content.len(), path.display());
            Ok(())
        } else {
            fs::write(path, content).map_err(FilesystemError::Io)
        }
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a rollback operation
#[allow(dead_code)] // Part of data structure API
#[derive(Debug, Default)]
pub struct RollbackResult {
    /// Number of files removed (created during session)
    pub files_removed: usize,
    /// Number of files restored from backup
    pub files_restored: usize,
    /// Number of directories removed
    pub directories_removed: usize,
}
#[allow(dead_code)] // Part of data structure API
impl RollbackResult {
    /// Total number of changes reverted
    pub fn total(&self) -> usize {
        self.files_removed + self.files_restored + self.directories_removed
    }

    /// Check if any rollback actions were taken
    pub fn any_changes(&self) -> bool {
        self.total() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let provider = FilesystemProvider::new();
        let hash = provider.compute_hash(&file_path).unwrap();

        assert!(hash.starts_with("sha256:"));
        // SHA256 of "hello world"
        assert!(hash.contains("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"));
    }

    #[test]
    fn test_compare_identical_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let provider = FilesystemProvider::new();
        let result = provider.compare_files(&file1, &file2).unwrap();

        assert_eq!(result, FileCompareResult::Match);
    }

    #[test]
    fn test_compare_different_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "content A").unwrap();
        fs::write(&file2, "content B").unwrap();

        let provider = FilesystemProvider::new();
        let result = provider.compare_files(&file1, &file2).unwrap();

        match result {
            FileCompareResult::Different { .. } => {}
            _ => panic!("Expected Different result"),
        }
    }

    #[test]
    fn test_target_missing() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("nonexistent.txt");

        fs::write(&file1, "content").unwrap();

        let provider = FilesystemProvider::new();
        let result = provider.compare_files(&file1, &file2).unwrap();

        assert_eq!(result, FileCompareResult::TargetMissing);
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("atomic_test.txt");

        let provider = FilesystemProvider::new();
        provider.write(&file_path, "atomic content").unwrap();

        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "atomic content");
    }

    #[test]
    fn test_copy_with_skip() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "same content").unwrap();
        fs::write(&dest, "same content").unwrap();

        let mut provider = FilesystemProvider::new();
        let result = provider
            .copy_file_with_options(&source, &dest, &CopyOptions::default())
            .unwrap();

        assert!(result.skipped);
    }

    #[test]
    fn test_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "new content").unwrap();

        let mut provider = FilesystemProvider::with_config(ProviderConfig::default())
            .with_backup_dir(temp_dir.path().join("backups"));

        // Copy file
        provider
            .copy_file_with_options(
                &source,
                &dest,
                &CopyOptions {
                    backup: false,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new content");

        // Rollback
        let result = provider.rollback().unwrap();
        assert_eq!(result.files_removed, 1);
        assert!(!dest.exists());
    }

    #[test]
    fn test_expand_glob() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
        fs::write(temp_dir.path().join("other.json"), "").unwrap();

        let provider = FilesystemProvider::new();
        let matches = provider.expand_glob("*.txt", temp_dir.path()).unwrap();

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|p| p.extension().unwrap() == "txt"));
    }

    #[test]
    fn test_directory_copy() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        // Create source directory structure
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.join("file2.json"), "content2").unwrap();
        fs::write(src_dir.join("sub/file3.txt"), "content3").unwrap();

        let mut provider = FilesystemProvider::new();
        let results = provider
            .copy_directory(&src_dir, &dst_dir, &DirectoryCopyOptions::new())
            .unwrap();

        assert_eq!(results.len(), 3);
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("file2.json").exists());
        assert!(dst_dir.join("sub/file3.txt").exists());
    }

    #[test]
    fn test_directory_copy_with_filters() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        // Create source directory structure
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.join("file2.json"), "content2").unwrap();
        fs::write(src_dir.join("file3.txt"), "content3").unwrap();

        let mut provider = FilesystemProvider::new();
        let options = DirectoryCopyOptions::new()
            .with_include(vec!["*.txt".to_string()])
            .with_exclude(vec!["file1*".to_string()]);

        let results = provider
            .copy_directory(&src_dir, &dst_dir, &options)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(!dst_dir.join("file1.txt").exists());
        assert!(!dst_dir.join("file2.json").exists());
        assert!(dst_dir.join("file3.txt").exists());
    }
}
