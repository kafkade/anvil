//! Operations module for Anvil CLI
//!
//! This module contains the implementation of each CLI command.
//! Each submodule corresponds to a command and contains an `execute` function
//! that performs the actual operation.

pub mod backup;
pub mod config;
pub mod health;
pub mod init;
pub mod install;
pub mod list;
pub mod show;
pub mod status;
pub mod validate;

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::workload::Workload;

/// Represents the result of an operation
#[allow(dead_code)]
#[derive(Debug)]
pub struct OperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of items processed
    pub processed: usize,
    /// Number of items that failed
    pub failed: usize,
    /// Number of items skipped
    pub skipped: usize,
    /// Optional message describing the result
    pub message: Option<String>,
}

#[allow(dead_code)]
impl OperationResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            processed: 0,
            failed: 0,
            skipped: 0,
            message: None,
        }
    }

    /// Create a successful result with counts
    pub fn success_with_counts(processed: usize, skipped: usize) -> Self {
        Self {
            success: true,
            processed,
            failed: 0,
            skipped,
            message: None,
        }
    }

    /// Create a failed result
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            processed: 0,
            failed: 1,
            skipped: 0,
            message: Some(message.into()),
        }
    }

    /// Create a failed result with counts
    pub fn failure_with_counts(
        processed: usize,
        failed: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            success: false,
            processed,
            failed,
            skipped: 0,
            message: Some(message.into()),
        }
    }

    /// Add a message to the result
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Common context for operations
pub struct OperationContext<'a> {
    /// The resolved workload to operate on
    pub workload: &'a Workload,
    /// Path to the workload directory
    pub workload_path: PathBuf,
    /// Whether to run in dry-run mode
    pub dry_run: bool,
    /// Verbosity level (0 = quiet, 1 = normal, 2+ = verbose)
    pub verbosity: u8,
    /// Whether to use colored output
    pub use_color: bool,
}

impl<'a> OperationContext<'a> {
    /// Create a new operation context
    pub fn new(
        workload: &'a Workload,
        workload_path: impl Into<PathBuf>,
        dry_run: bool,
        verbosity: u8,
        use_color: bool,
    ) -> Self {
        Self {
            workload,
            workload_path: workload_path.into(),
            dry_run,
            verbosity,
            use_color,
        }
    }

    /// Log a debug message if verbosity is high enough
    pub fn debug(&self, message: &str) {
        if self.verbosity >= 3 {
            tracing::debug!("{}", message);
        }
    }

    /// Log an info message if not in quiet mode
    #[allow(dead_code)]
    pub fn info(&self, message: &str) {
        if self.verbosity >= 1 {
            tracing::info!("{}", message);
        }
    }

    /// Log a warning message
    pub fn warn(&self, message: &str) {
        tracing::warn!("{}", message);
    }

    /// Log an error message
    #[allow(dead_code)]
    pub fn error(&self, message: &str) {
        tracing::error!("{}", message);
    }
}

/// Find workloads directory
///
/// Searches for workloads in the following order:
/// 1. Custom path if provided
/// 2. Current directory's `workloads/` folder
/// 3. Executable directory's `workloads/` folder
/// 4. User's home directory under `.anvil/workloads/`
pub fn find_workloads_dir(custom_path: Option<&Path>) -> Result<PathBuf> {
    // Check custom path first
    if let Some(path) = custom_path {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }

    // Check current directory
    let current_dir = std::env::current_dir()?;
    let current_workloads = current_dir.join("workloads");
    if current_workloads.exists() {
        return Ok(current_workloads);
    }

    // Check executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_workloads = exe_dir.join("workloads");
            if exe_workloads.exists() {
                return Ok(exe_workloads);
            }
        }
    }

    // Check home directory
    if let Some(home) = dirs::home_dir() {
        let home_workloads = home.join(".anvil").join("workloads");
        if home_workloads.exists() {
            return Ok(home_workloads);
        }
    }

    // Fall back to current directory workloads (even if it doesn't exist yet)
    Ok(current_workloads)
}

/// Resolve a workload name or path to the actual workload file
///
/// If the input is a path to a file or directory, use it directly.
/// Otherwise, search for the workload by name in the workloads directory.
pub fn resolve_workload_path(workload: &str, custom_search_path: Option<&Path>) -> Result<PathBuf> {
    let path = Path::new(workload);

    // If it's an absolute path or starts with ./ or ../, use directly
    if path.is_absolute() || workload.starts_with('.') {
        if path.is_file() {
            return Ok(path.to_path_buf());
        } else if path.is_dir() {
            let yaml_path = path.join("workload.yaml");
            if yaml_path.exists() {
                return Ok(yaml_path);
            }
            let yml_path = path.join("workload.yml");
            if yml_path.exists() {
                return Ok(yml_path);
            }
            anyhow::bail!("No workload.yaml found in directory: {}", path.display());
        } else {
            anyhow::bail!("Path does not exist: {}", path.display());
        }
    }

    // Search in workloads directory
    let workloads_dir = find_workloads_dir(custom_search_path)?;
    let workload_dir = workloads_dir.join(workload);

    if workload_dir.is_dir() {
        let yaml_path = workload_dir.join("workload.yaml");
        if yaml_path.exists() {
            return Ok(yaml_path);
        }
        let yml_path = workload_dir.join("workload.yml");
        if yml_path.exists() {
            return Ok(yml_path);
        }
        anyhow::bail!(
            "No workload.yaml found in workload directory: {}",
            workload_dir.display()
        );
    }

    anyhow::bail!(
        "Workload '{}' not found. Searched in: {}",
        workload,
        workloads_dir.display()
    )
}
