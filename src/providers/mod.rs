//! Providers module - External system integrations
//!
//! This module contains providers that interface with external systems:
//! - `winget`: Windows Package Manager for package installation
//! - `filesystem`: File operations with backup and hashing
//! - `script`: PowerShell script execution

pub mod backup;
pub mod filesystem;
pub mod script;
pub mod template;
pub mod winget;

// Re-export commonly used types
pub use filesystem::FilesystemProvider;
pub use script::ScriptProvider;
pub use winget::WingetProvider;

use thiserror::Error;

/// Errors that can occur in providers
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Package-related errors from winget
    #[error("Winget error: {0}")]
    Winget(#[from] winget::WingetError),

    /// File system operation errors
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] filesystem::FilesystemError),

    /// Template processing errors
    #[error("Template error: {0}")]
    Template(#[from] template::TemplateError),

    /// Backup operation errors
    #[error("Backup error: {0}")]
    Backup(#[from] backup::BackupError),

    /// Script execution errors
    #[error("Script error: {0}")]
    Script(#[from] script::ScriptError),

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Operation was cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Operation timed out
    #[error("Operation timed out after {0} seconds")]
    Timeout(u64),
}

/// Result type alias for provider operations
#[allow(dead_code)]
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Trait for providers that can report their status
#[allow(dead_code)]
pub trait ProviderStatus {
    /// Check if the provider is available and functional
    fn is_available(&self) -> bool;

    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get provider version information (if applicable)
    fn version(&self) -> Option<String>;
}

/// Common configuration for providers
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Enable verbose output
    pub verbose: bool,

    /// Dry run mode - don't make actual changes
    pub dry_run: bool,

    /// Default timeout in seconds for operations
    pub timeout_seconds: u64,

    /// Number of retry attempts for transient failures
    pub retry_count: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            dry_run: false,
            timeout_seconds: 300,
            retry_count: 3,
        }
    }
}

#[allow(dead_code)]
impl ProviderConfig {
    /// Create a new provider configuration with verbose output
    pub fn verbose() -> Self {
        Self {
            verbose: true,
            ..Default::default()
        }
    }

    /// Create a new provider configuration for dry run
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set the retry count
    pub fn with_retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}
