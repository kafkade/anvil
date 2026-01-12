//! Winget provider module
//!
//! This module provides an interface to the Windows Package Manager (winget)
//! for package installation, removal, and verification.

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;

use super::{ProviderConfig, ProviderStatus};
use crate::config::workload::WingetPackage;

/// Winget exit codes
/// Note: Winget returns both HRESULT values (negative 32-bit) and friendly codes (small positive).
/// We handle both variants for compatibility across winget versions.
pub mod exit_codes {
    /// Success
    pub const SUCCESS: i32 = 0;
    /// Generic error
    #[allow(dead_code)]
    pub const GENERIC_ERROR: i32 = 1;
    /// Package already installed (0x8A150011)
    pub const ALREADY_INSTALLED: i32 = -1978335215;
    /// No applicable installer (0x8A15002B) - HRESULT version
    pub const NO_APPLICABLE_INSTALLER: i32 = -1978335189;
    /// Package not found (0x8A150014)
    pub const PACKAGE_NOT_FOUND: i32 = -1978335212;
    /// Installer hash mismatch (0x8A150008)
    pub const HASH_MISMATCH: i32 = -1978335224;
    /// Cancelled by user
    pub const CANCELLED: i32 = -1978335199;
    /// Needs reboot (0x8A15002C)
    pub const NEEDS_REBOOT: i32 = -1978335188;
    /// Access denied
    pub const ACCESS_DENIED: i32 = 5;
    /// Upgrade not available (0x8A15001A)
    pub const NO_UPGRADE_AVAILABLE: i32 = -1978335206;

    // Friendly exit codes (newer winget versions)
    // These are smaller positive codes that map to the same errors
    /// No applicable installer / No upgrade available (friendly code)
    pub const NO_APPLICABLE_INSTALLER_FRIENDLY: i32 = 43;
    /// Package not found (friendly code)
    pub const PACKAGE_NOT_FOUND_FRIENDLY: i32 = 4;
    /// Already installed (friendly code)
    pub const ALREADY_INSTALLED_FRIENDLY: i32 = 17;
    /// Needs reboot (friendly code)
    pub const NEEDS_REBOOT_FRIENDLY: i32 = 44;
}

/// Errors that can occur when interacting with winget
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum WingetError {
    /// Package was not found in any source
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    /// Package installation failed
    #[error("Installation failed for {package}: exit code {exit_code}, {stderr}")]
    InstallationFailed {
        package: String,
        exit_code: i32,
        stderr: String,
    },

    /// Package version doesn't match expected
    #[error("Version mismatch for {package}: expected {expected}, found {actual}")]
    VersionMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    /// Network error during operation
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Access denied (may need elevation)
    #[error("Access denied: {0}. Try running as administrator.")]
    AccessDenied(String),

    /// Operation timed out
    #[error("Operation timed out for {package} after {timeout_seconds}s")]
    Timeout {
        package: String,
        timeout_seconds: u64,
    },

    /// Winget is not installed or not in PATH
    #[error("Winget is not available: {0}")]
    NotAvailable(String),

    /// Failed to parse winget output
    #[error("Failed to parse winget output: {0}")]
    ParseError(String),

    /// Package already installed
    #[error("Package already installed: {0}")]
    AlreadyInstalled(String),

    /// No applicable installer found
    #[error("No applicable installer found for {package}: {details}")]
    NoApplicableInstaller { package: String, details: String },

    /// Operation was cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Reboot required
    #[error("Reboot required to complete installation of {0}")]
    RebootRequired(String),

    /// Upgrade not available
    #[error("No upgrade available for {0}")]
    NoUpgradeAvailable(String),

    /// Version not available
    #[error("Version {version} not available for {package}. Available versions: {available}")]
    VersionNotAvailable {
        package: String,
        version: String,
        available: String,
    },

    /// All retry attempts failed
    #[error("All {attempts} retry attempts failed for {package}: {last_error}")]
    RetryExhausted {
        package: String,
        attempts: u32,
        last_error: String,
    },

    /// Generic IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl WingetError {
    /// Check if this error is retryable (transient)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            WingetError::NetworkError(_) | WingetError::Timeout { .. } | WingetError::IoError(_)
        )
    }

    /// Get a user-friendly suggestion for fixing the error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            WingetError::PackageNotFound(_) => {
                Some("Try searching with 'winget search <package>' to find similar packages")
            }
            WingetError::AccessDenied(_) => {
                Some("Run the command as Administrator or check your permissions")
            }
            WingetError::NetworkError(_) => Some("Check your internet connection and try again"),
            WingetError::NotAvailable(_) => Some(
                "Install winget from the Microsoft Store (App Installer) or from GitHub releases",
            ),
            WingetError::NoApplicableInstaller { .. } => {
                Some("This package may not support your system architecture or Windows version")
            }
            WingetError::RebootRequired(_) => {
                Some("Restart your computer to complete the installation")
            }
            WingetError::VersionNotAvailable { .. } => {
                Some("Try 'winget show <package> --versions' to see available versions")
            }
            _ => None,
        }
    }
}

/// Information about an installed package
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    /// Package ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Available version (if upgrade exists)
    pub available_version: Option<String>,
    /// Source (e.g., "winget", "msstore")
    pub source: Option<String>,
}

/// Result of a package installation
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Package that was installed
    pub package_id: String,
    /// Whether installation was successful
    pub success: bool,
    /// Version that was installed (if known)
    pub installed_version: Option<String>,
    /// Any messages from the installation
    pub message: Option<String>,
    /// Whether a reboot is required
    pub reboot_required: bool,
    /// Duration of installation
    pub duration: Duration,
    /// Exit code from winget
    pub exit_code: i32,
}

/// Progress callback type for installation operations
#[allow(dead_code)]
pub type ProgressCallback = Arc<dyn Fn(ProgressEvent) + Send + Sync>;

/// Progress events during package operations
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Starting a package operation
    Starting { package_id: String },
    /// Download progress (if available)
    Downloading { package_id: String, message: String },
    /// Installation in progress
    Installing { package_id: String, message: String },
    /// Operation completed
    Completed {
        package_id: String,
        success: bool,
        duration: Duration,
    },
    /// Retry attempt
    Retrying {
        package_id: String,
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
    },
}

/// Winget provider for package management
#[allow(dead_code)]
pub struct WingetProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// Cached version of winget
    cached_version: Option<String>,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
}

#[allow(dead_code)]
impl WingetProvider {
    /// Create a new winget provider with default configuration
    pub fn new() -> Self {
        Self {
            config: ProviderConfig::default(),
            cached_version: None,
            progress_callback: None,
        }
    }

    /// Create a new winget provider with custom configuration
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            cached_version: None,
            progress_callback: None,
        }
    }

    /// Set the progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Emit a progress event if a callback is set
    fn emit_progress(&self, event: ProgressEvent) {
        if let Some(ref callback) = self.progress_callback {
            callback(event);
        }
    }

    /// Check if winget is available on the system
    pub fn check_availability(&mut self) -> Result<WingetInfo, WingetError> {
        let output = Command::new("winget")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                WingetError::NotAvailable(format!(
                    "Failed to execute winget: {}. {}",
                    e,
                    Self::get_installation_instructions()
                ))
            })?;

        if !output.status.success() {
            return Err(WingetError::NotAvailable(
                "winget command failed".to_string(),
            ));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.cached_version = Some(version.clone());

        // Parse version to check if it meets minimum requirements
        let min_version = "1.6";
        let version_num = version.trim_start_matches('v');

        Ok(WingetInfo {
            version: version.clone(),
            meets_minimum: version_num >= min_version,
            minimum_version: min_version.to_string(),
        })
    }

    /// Get installation instructions for winget
    pub fn get_installation_instructions() -> &'static str {
        r#"
To install Windows Package Manager (winget):

1. Open Microsoft Store
2. Search for "App Installer"
3. Install or update "App Installer" by Microsoft

Alternatively, download from:
https://github.com/microsoft/winget-cli/releases
"#
    }

    /// Execute a winget command with retry logic
    fn execute_with_retry<F, T>(&self, package_id: &str, operation: F) -> Result<T, WingetError>
    where
        F: Fn() -> Result<T, WingetError>,
    {
        let max_retries = self.config.retry_count;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() || attempt == max_retries {
                        last_error = Some(e);
                        break;
                    }

                    // Calculate exponential backoff delay
                    let delay_ms = 1000 * 2u64.pow(attempt);

                    self.emit_progress(ProgressEvent::Retrying {
                        package_id: package_id.to_string(),
                        attempt: attempt + 1,
                        max_attempts: max_retries,
                        delay_ms,
                    });

                    if self.config.verbose {
                        tracing::warn!(
                            "Retry {}/{} for {} after {}ms: {}",
                            attempt + 1,
                            max_retries,
                            package_id,
                            delay_ms,
                            e
                        );
                    }

                    std::thread::sleep(Duration::from_millis(delay_ms));
                    last_error = Some(e);
                }
            }
        }

        Err(match last_error {
            Some(e) if max_retries > 0 => WingetError::RetryExhausted {
                package: package_id.to_string(),
                attempts: max_retries + 1,
                last_error: e.to_string(),
            },
            Some(e) => e,
            None => WingetError::InstallationFailed {
                package: package_id.to_string(),
                exit_code: -1,
                stderr: "Unknown error".to_string(),
            },
        })
    }

    /// Map winget exit code to error
    fn map_exit_code(
        &self,
        exit_code: i32,
        package_id: &str,
        stdout: &str,
        stderr: &str,
    ) -> Result<(), WingetError> {
        let combined = format!("{}\n{}", stdout, stderr);

        match exit_code {
            exit_codes::SUCCESS => Ok(()),
            exit_codes::ALREADY_INSTALLED | exit_codes::ALREADY_INSTALLED_FRIENDLY => {
                Err(WingetError::AlreadyInstalled(package_id.to_string()))
            }
            exit_codes::PACKAGE_NOT_FOUND | exit_codes::PACKAGE_NOT_FOUND_FRIENDLY => {
                Err(WingetError::PackageNotFound(package_id.to_string()))
            }
            exit_codes::NO_APPLICABLE_INSTALLER => {
                // Check if package is already installed and up to date
                if combined.contains("No available upgrade")
                    || combined.contains("already installed")
                    || combined.contains("No newer package versions")
                {
                    Err(WingetError::AlreadyInstalled(package_id.to_string()))
                } else {
                    Err(WingetError::NoApplicableInstaller {
                        package: package_id.to_string(),
                        details: stderr.to_string(),
                    })
                }
            }
            // Friendly code 43 can mean "no applicable installer" OR "no upgrade available"
            // Check the output to determine which case it is
            exit_codes::NO_APPLICABLE_INSTALLER_FRIENDLY => {
                // Check if package is already installed and up to date
                if combined.contains("No available upgrade")
                    || combined.contains("already installed")
                    || combined.contains("No newer package versions")
                {
                    Err(WingetError::AlreadyInstalled(package_id.to_string()))
                } else {
                    Err(WingetError::NoApplicableInstaller {
                        package: package_id.to_string(),
                        details: stderr.to_string(),
                    })
                }
            }
            exit_codes::ACCESS_DENIED => Err(WingetError::AccessDenied(package_id.to_string())),
            exit_codes::CANCELLED => Err(WingetError::Cancelled),
            exit_codes::NEEDS_REBOOT | exit_codes::NEEDS_REBOOT_FRIENDLY => {
                Err(WingetError::RebootRequired(package_id.to_string()))
            }
            exit_codes::NO_UPGRADE_AVAILABLE => {
                Err(WingetError::NoUpgradeAvailable(package_id.to_string()))
            }
            exit_codes::HASH_MISMATCH => Err(WingetError::InstallationFailed {
                package: package_id.to_string(),
                exit_code,
                stderr:
                    "Installer hash verification failed. The package may have been tampered with."
                        .to_string(),
            }),
            _ => {
                // Check for common error patterns in output
                if combined.contains("No package found") || combined.contains("did not match") {
                    return Err(WingetError::PackageNotFound(package_id.to_string()));
                }

                if combined.contains("Access is denied") || combined.contains("administrator") {
                    return Err(WingetError::AccessDenied(package_id.to_string()));
                }

                if combined.contains("network") || combined.contains("connection") {
                    return Err(WingetError::NetworkError(stderr.to_string()));
                }

                // Check if already installed (may come with various exit codes)
                if combined.contains("already installed")
                    || combined.contains("No available upgrade")
                {
                    return Err(WingetError::AlreadyInstalled(package_id.to_string()));
                }

                Err(WingetError::InstallationFailed {
                    package: package_id.to_string(),
                    exit_code,
                    stderr: stderr.to_string(),
                })
            }
        }
    }

    /// Install a package
    pub fn install(&self, package: &WingetPackage) -> Result<InstallResult, WingetError> {
        self.emit_progress(ProgressEvent::Starting {
            package_id: package.id.clone(),
        });

        let start_time = Instant::now();

        if self.config.dry_run {
            return Ok(InstallResult {
                package_id: package.id.clone(),
                success: true,
                installed_version: package.version.clone(),
                message: Some("Dry run - would install package".to_string()),
                reboot_required: false,
                duration: start_time.elapsed(),
                exit_code: 0,
            });
        }

        let result =
            self.execute_with_retry(&package.id, || self.install_internal(package, start_time));

        let duration = start_time.elapsed();

        match &result {
            Ok(r) => {
                self.emit_progress(ProgressEvent::Completed {
                    package_id: package.id.clone(),
                    success: r.success,
                    duration,
                });
            }
            Err(_) => {
                self.emit_progress(ProgressEvent::Completed {
                    package_id: package.id.clone(),
                    success: false,
                    duration,
                });
            }
        }

        result
    }

    /// Internal installation logic
    fn install_internal(
        &self,
        package: &WingetPackage,
        start_time: Instant,
    ) -> Result<InstallResult, WingetError> {
        self.emit_progress(ProgressEvent::Installing {
            package_id: package.id.clone(),
            message: "Preparing installation...".to_string(),
        });

        let mut cmd = Command::new("winget");
        cmd.arg("install")
            .arg("--id")
            .arg(&package.id)
            .arg("--exact")
            .arg("--accept-package-agreements")
            .arg("--accept-source-agreements");

        // Add version if specified
        if let Some(ref version) = package.version {
            cmd.arg("--version").arg(version);
        }

        // Add source if specified
        if let Some(ref source) = package.source {
            cmd.arg("--source").arg(source);
        }

        // Add override arguments if specified
        if let Some(ref override_args) = package.override_args {
            for arg in override_args {
                cmd.arg(arg);
            }
        }

        if let Some(ref override_str) = package.override_str {
            for arg in override_str {
                cmd.arg(arg);
            }
        }

        // Disable interactive prompts
        cmd.arg("--silent").arg("--disable-interactivity");

        if self.config.verbose {
            tracing::debug!("Running: {:?}", cmd);
        }

        let output = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        if self.config.verbose {
            tracing::debug!("Exit code: {}", exit_code);
            if !stdout.is_empty() {
                tracing::debug!("stdout: {}", stdout);
            }
            if !stderr.is_empty() {
                tracing::debug!("stderr: {}", stderr);
            }
        }

        // Handle special case: already installed is success
        // Check both HRESULT and friendly exit codes
        if exit_code == exit_codes::ALREADY_INSTALLED
            || exit_code == exit_codes::ALREADY_INSTALLED_FRIENDLY
        {
            return Ok(InstallResult {
                package_id: package.id.clone(),
                success: true,
                installed_version: package.version.clone(),
                message: Some("Package is already installed".to_string()),
                reboot_required: false,
                duration: start_time.elapsed(),
                exit_code,
            });
        }

        // Handle "no applicable installer" which can mean "already installed at latest version"
        // This applies to both HRESULT (-1978335189) and friendly code (43)
        if exit_code == exit_codes::NO_APPLICABLE_INSTALLER
            || exit_code == exit_codes::NO_APPLICABLE_INSTALLER_FRIENDLY
        {
            let combined = format!("{}\n{}", stdout, stderr);
            if combined.contains("No available upgrade")
                || combined.contains("already installed")
                || combined.contains("No newer package versions")
            {
                return Ok(InstallResult {
                    package_id: package.id.clone(),
                    success: true,
                    installed_version: package.version.clone(),
                    message: Some("Package is already installed at the latest version".to_string()),
                    reboot_required: false,
                    duration: start_time.elapsed(),
                    exit_code,
                });
            }
        }

        // Handle reboot required as success with flag
        if exit_code == exit_codes::NEEDS_REBOOT
            || exit_code == exit_codes::NEEDS_REBOOT_FRIENDLY
            || stdout.contains("reboot")
            || stdout.contains("restart")
        {
            return Ok(InstallResult {
                package_id: package.id.clone(),
                success: true,
                installed_version: package.version.clone(),
                message: Some("Installation complete - reboot required".to_string()),
                reboot_required: true,
                duration: start_time.elapsed(),
                exit_code,
            });
        }

        // Check for errors
        self.map_exit_code(exit_code, &package.id, &stdout, &stderr)?;

        Ok(InstallResult {
            package_id: package.id.clone(),
            success: true,
            installed_version: package.version.clone(),
            message: Some(stdout.trim().to_string()),
            reboot_required: false,
            duration: start_time.elapsed(),
            exit_code,
        })
    }

    /// Uninstall a package
    pub fn uninstall(&self, package_id: &str) -> Result<(), WingetError> {
        if self.config.dry_run {
            tracing::info!("Dry run - would uninstall: {}", package_id);
            return Ok(());
        }

        let output = Command::new("winget")
            .arg("uninstall")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--silent")
            .arg("--disable-interactivity")
            .arg("--accept-source-agreements")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.map_exit_code(exit_code, package_id, &stdout, &stderr)
    }

    /// Check if a package is installed
    pub fn is_installed(&self, package_id: &str) -> Result<bool, WingetError> {
        let output = Command::new("winget")
            .arg("list")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check if the package ID appears in the output (case-insensitive)
        Ok(stdout.to_lowercase().contains(&package_id.to_lowercase()))
    }

    /// Get the installed version of a package
    pub fn get_installed_version(&self, package_id: &str) -> Result<Option<String>, WingetError> {
        let output = Command::new("winget")
            .arg("list")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_installed_version(&stdout, package_id)
    }

    /// Get detailed information about an installed package
    pub fn get_package_info(
        &self,
        package_id: &str,
    ) -> Result<Option<InstalledPackage>, WingetError> {
        let output = Command::new("winget")
            .arg("list")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if !stdout.to_lowercase().contains(&package_id.to_lowercase()) {
            return Ok(None);
        }

        let version = parse_installed_version(&stdout, package_id)?;
        let available = self.get_available_upgrade(package_id)?;

        Ok(Some(InstalledPackage {
            id: package_id.to_string(),
            name: package_id.to_string(), // Could parse from show command
            version: version.unwrap_or_else(|| "unknown".to_string()),
            available_version: available,
            source: Some("winget".to_string()),
        }))
    }

    /// Check if an upgrade is available for a package
    pub fn get_available_upgrade(&self, package_id: &str) -> Result<Option<String>, WingetError> {
        let output = Command::new("winget")
            .arg("upgrade")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // If the package appears in upgrade list, parse the available version
        if stdout.to_lowercase().contains(&package_id.to_lowercase()) {
            return parse_upgrade_version(&stdout, package_id);
        }

        Ok(None)
    }

    /// List all installed packages
    pub fn list_installed(&self) -> Result<Vec<InstalledPackage>, WingetError> {
        let output = Command::new("winget")
            .arg("list")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WingetError::ParseError(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = parse_winget_list_output(&stdout)?;

        Ok(packages)
    }

    /// Search for packages
    pub fn search(&self, query: &str) -> Result<Vec<InstalledPackage>, WingetError> {
        let output = Command::new("winget")
            .arg("search")
            .arg(query)
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WingetError::ParseError(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = parse_winget_list_output(&stdout)?;

        Ok(packages)
    }

    /// Upgrade a package to the latest version
    pub fn upgrade(&self, package_id: &str) -> Result<InstallResult, WingetError> {
        self.emit_progress(ProgressEvent::Starting {
            package_id: package_id.to_string(),
        });

        let start_time = Instant::now();

        if self.config.dry_run {
            return Ok(InstallResult {
                package_id: package_id.to_string(),
                success: true,
                installed_version: None,
                message: Some("Dry run - would upgrade package".to_string()),
                reboot_required: false,
                duration: start_time.elapsed(),
                exit_code: 0,
            });
        }

        self.emit_progress(ProgressEvent::Installing {
            package_id: package_id.to_string(),
            message: "Upgrading...".to_string(),
        });

        let output = Command::new("winget")
            .arg("upgrade")
            .arg("--id")
            .arg(package_id)
            .arg("--exact")
            .arg("--silent")
            .arg("--accept-package-agreements")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        let duration = start_time.elapsed();

        self.emit_progress(ProgressEvent::Completed {
            package_id: package_id.to_string(),
            success: output.status.success(),
            duration,
        });

        // Handle no upgrade available
        if exit_code == exit_codes::NO_UPGRADE_AVAILABLE {
            return Ok(InstallResult {
                package_id: package_id.to_string(),
                success: true,
                installed_version: None,
                message: Some("Already at latest version".to_string()),
                reboot_required: false,
                duration,
                exit_code,
            });
        }

        self.map_exit_code(exit_code, package_id, &stdout, &stderr)?;

        Ok(InstallResult {
            package_id: package_id.to_string(),
            success: true,
            installed_version: None,
            message: Some(stdout.trim().to_string()),
            reboot_required: stdout.contains("reboot") || stdout.contains("restart"),
            duration,
            exit_code,
        })
    }

    /// Upgrade all packages in a list
    pub fn upgrade_all(&self, package_ids: &[String]) -> Vec<Result<InstallResult, WingetError>> {
        package_ids.iter().map(|id| self.upgrade(id)).collect()
    }

    /// Export installed packages to a file
    pub fn export(&self, output_path: &std::path::Path) -> Result<(), WingetError> {
        let output = Command::new("winget")
            .arg("export")
            .arg("-o")
            .arg(output_path)
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.map_exit_code(exit_code, "export", &stdout, &stderr)
    }

    /// Import packages from a file
    pub fn import(&self, input_path: &std::path::Path) -> Result<(), WingetError> {
        if self.config.dry_run {
            tracing::info!("Dry run - would import from: {}", input_path.display());
            return Ok(());
        }

        let output = Command::new("winget")
            .arg("import")
            .arg("-i")
            .arg(input_path)
            .arg("--accept-package-agreements")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.map_exit_code(exit_code, "import", &stdout, &stderr)
    }

    /// Get available versions for a package
    pub fn get_available_versions(&self, package_id: &str) -> Result<Vec<String>, WingetError> {
        let output = Command::new("winget")
            .arg("show")
            .arg("--id")
            .arg(package_id)
            .arg("--versions")
            .arg("--accept-source-agreements")
            .arg("--disable-interactivity")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(WingetError::PackageNotFound(package_id.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();

        let mut in_versions = false;
        for line in stdout.lines() {
            if line.starts_with("Version") {
                in_versions = true;
                continue;
            }
            if line.starts_with("---") {
                continue;
            }
            if in_versions && !line.trim().is_empty() {
                versions.push(line.trim().to_string());
            }
        }

        Ok(versions)
    }
}

impl Default for WingetProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderStatus for WingetProvider {
    fn is_available(&self) -> bool {
        Command::new("winget")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn name(&self) -> &'static str {
        "winget"
    }

    fn version(&self) -> Option<String> {
        self.cached_version.clone().or_else(|| {
            Command::new("winget")
                .arg("--version")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
    }
}

/// Information about the winget installation
#[derive(Debug, Clone)]
pub struct WingetInfo {
    /// Winget version string
    pub version: String,
    /// Whether the version meets minimum requirements
    pub meets_minimum: bool,
    /// Minimum recommended version
    pub minimum_version: String,
}

/// Parse installed version from winget list output
fn parse_installed_version(output: &str, package_id: &str) -> Result<Option<String>, WingetError> {
    // Clean the output first to remove progress indicators (same as parse_winget_list_output)
    let cleaned_output = clean_winget_output(output);
    let lines: Vec<&str> = cleaned_output.lines().collect();

    // Find the header line to determine column positions
    let header_idx = lines
        .iter()
        .position(|line| line.contains("Name") && (line.contains("Id") || line.contains("ID")));

    if header_idx.is_none() {
        return Ok(None);
    }

    let header_line = lines[header_idx.unwrap()];

    // Find column positions
    let _id_pos = header_line.find("Id").or_else(|| header_line.find("ID"));
    let version_pos = header_line.find("Version");

    if _id_pos.is_none() || version_pos.is_none() {
        return Ok(None);
    }

    let _id_pos = _id_pos.unwrap();
    let version_pos = version_pos.unwrap();

    // Find the separator line to determine exact column widths
    let separator_idx = header_idx.unwrap() + 1;
    if separator_idx >= lines.len() {
        return Ok(None);
    }

    // Find the line containing our package
    for line in lines.iter().skip(separator_idx + 1) {
        let line_lower = line.to_lowercase();
        let package_lower = package_id.to_lowercase();

        if line_lower.contains(&package_lower) {
            // Parse the version from this line
            // The version typically comes after the ID column
            let chars: Vec<char> = line.chars().collect();

            // Extract the substring starting at version_pos
            if version_pos < chars.len() {
                let version_str: String = chars[version_pos..].iter().collect();
                let version = version_str.split_whitespace().next();

                if let Some(v) = version {
                    // Validate it looks like a version
                    if v.chars().any(|c| c.is_ascii_digit()) {
                        return Ok(Some(v.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Parse upgrade version from winget upgrade output
fn parse_upgrade_version(output: &str, package_id: &str) -> Result<Option<String>, WingetError> {
    let lines: Vec<&str> = output.lines().collect();

    // Find the header line
    let header_idx = lines
        .iter()
        .position(|line| line.contains("Name") && line.contains("Available"));

    if header_idx.is_none() {
        return Ok(None);
    }

    let header_line = lines[header_idx.unwrap()];
    let available_pos = header_line.find("Available");

    if available_pos.is_none() {
        return Ok(None);
    }

    let available_pos = available_pos.unwrap();

    // Find the line containing our package
    for line in lines.iter().skip(header_idx.unwrap() + 2) {
        let line_lower = line.to_lowercase();
        let package_lower = package_id.to_lowercase();

        if line_lower.contains(&package_lower) {
            let chars: Vec<char> = line.chars().collect();

            if available_pos < chars.len() {
                let version_str: String = chars[available_pos..].iter().collect();
                let version = version_str.split_whitespace().next();

                if let Some(v) = version {
                    if v.chars().any(|c| c.is_ascii_digit()) {
                        return Ok(Some(v.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Check if a character is a progress indicator character
fn is_progress_char(c: char) -> bool {
    // Spinner characters
    if c == '-' || c == '\\' || c == '|' || c == '/' {
        return true;
    }
    // Progress bar block characters
    if c == '█' || c == '▒' || c == '░' || c == '▓' {
        return true;
    }
    // Percentage and size indicators are part of progress
    false
}

/// Check if a string segment is just progress/spinner content
fn is_progress_segment(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return true;
    }
    // If it contains progress bar characters, it's progress content
    if trimmed
        .chars()
        .any(|c| c == '█' || c == '▒' || c == '░' || c == '▓')
    {
        return true;
    }
    // If it's just spinner characters and whitespace, BUT not a long separator line
    // The separator line in winget output is a long line of dashes (e.g., "-------...")
    // We don't want to filter those out as they're used for column parsing
    if trimmed
        .chars()
        .all(|c| is_progress_char(c) || c.is_whitespace())
    {
        // If it's a long line of just dashes, it's likely the separator line, not progress
        if trimmed.len() > 20 && trimmed.chars().all(|c| c == '-') {
            return false;
        }
        return true;
    }
    // If it looks like a percentage (e.g., "45%", "100%")
    if trimmed.ends_with('%') && trimmed.len() <= 4 {
        return true;
    }
    // If it looks like a size indicator (e.g., "2.51 MB", "1024 KB")
    if trimmed.contains(" KB") || trimmed.contains(" MB") || trimmed.contains(" GB") {
        if trimmed.split_whitespace().count() <= 3 {
            return true;
        }
    }
    false
}

/// Clean winget output by removing progress indicators and ANSI sequences
fn clean_winget_output(output: &str) -> String {
    // Split on carriage returns first (spinner uses \r to overwrite)
    // Take only the last segment of each \r-separated chunk
    let mut result_lines: Vec<String> = Vec::new();

    for line in output.lines() {
        // Split by \r and take the last non-empty, non-progress segment
        let segments: Vec<&str> = line.split('\r').collect();
        let mut best_segment = "";
        for seg in segments.iter().rev() {
            if !is_progress_segment(seg) {
                best_segment = seg;
                break;
            }
        }

        if !best_segment.is_empty() {
            // Remove ANSI escape sequences
            let mut cleaned_seg = String::new();
            let mut in_escape = false;
            for ch in best_segment.chars() {
                if ch == '\x1b' {
                    in_escape = true;
                    continue;
                }
                if in_escape {
                    if ch.is_ascii_alphabetic() {
                        in_escape = false;
                    }
                    continue;
                }
                cleaned_seg.push(ch);
            }

            let trimmed = cleaned_seg.trim();
            if !trimmed.is_empty() && !is_progress_segment(trimmed) {
                result_lines.push(cleaned_seg);
            }
        }
    }

    result_lines.join("\n")
}

/// Parse winget list/search output into structured data
#[allow(dead_code)]
fn parse_winget_list_output(output: &str) -> Result<Vec<InstalledPackage>, WingetError> {
    let mut packages = Vec::new();

    // Clean the output first to remove progress indicators
    let cleaned_output = clean_winget_output(output);
    let lines: Vec<&str> = cleaned_output.lines().collect();

    // Find the header line to determine column positions
    let header_idx = lines
        .iter()
        .position(|line| line.contains("Name") && (line.contains("Id") || line.contains("ID")));

    if header_idx.is_none() {
        return Ok(packages); // No packages found
    }

    let header_line = lines[header_idx.unwrap()];

    // Convert header to chars to find char-based column positions
    let header_chars: Vec<char> = header_line.chars().collect();
    let header_str: String = header_chars.iter().collect();

    // Find column positions based on header (using char indices)
    let name_pos = header_str
        .find("Name")
        .map(|pos| header_str[..pos].chars().count())
        .unwrap_or(0);
    let id_pos = header_str
        .find("Id")
        .or_else(|| header_str.find("ID"))
        .map(|pos| header_str[..pos].chars().count())
        .unwrap_or(0);
    let version_pos = header_str
        .find("Version")
        .map(|pos| header_str[..pos].chars().count())
        .unwrap_or(0);
    let available_pos = header_str
        .find("Available")
        .map(|pos| header_str[..pos].chars().count());
    let source_pos = header_str
        .find("Source")
        .map(|pos| header_str[..pos].chars().count());

    // Skip header and separator lines
    let start_idx = header_idx.unwrap() + 2;

    for line in lines.iter().skip(start_idx) {
        if line.trim().is_empty() || line.starts_with('-') {
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let line_len = chars.len();

        // Skip lines that are too short to contain valid data
        if line_len < 10 {
            continue;
        }

        // Safe extraction helper
        let extract = |start: usize, end: usize| -> String {
            let safe_start = start.min(line_len);
            let safe_end = end.min(line_len);
            if safe_start >= safe_end {
                return String::new();
            }
            chars[safe_start..safe_end]
                .iter()
                .collect::<String>()
                .trim()
                .to_string()
        };

        // Extract name (from start to id_pos)
        let name_end = if id_pos > name_pos { id_pos } else { line_len };
        let name = extract(name_pos, name_end);

        // Extract ID
        let id_end = if version_pos > id_pos {
            version_pos
        } else {
            line_len
        };
        let id = extract(id_pos, id_end);

        // Extract version
        let version_end = available_pos.or(source_pos).unwrap_or(line_len);
        let version = extract(version_pos, version_end);

        // Extract available version if present
        let available_version = if let Some(avail_pos) = available_pos {
            let avail_end = source_pos.unwrap_or(line_len);
            let v = extract(avail_pos, avail_end);
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        };

        // Extract source if present
        let source = if let Some(src_pos) = source_pos {
            let v = extract(src_pos, line_len);
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        };

        // Only add if we have a valid ID
        // Accept IDs with dots (e.g., ShareX.ShareX) or alphanumeric MS Store IDs (e.g., 9PFXXSHC64H3)
        let is_valid_id =
            !id.is_empty() && (id.contains('.') || id.chars().all(|c| c.is_alphanumeric()));
        if is_valid_id {
            packages.push(InstalledPackage {
                id,
                name,
                version,
                available_version,
                source,
            });
        }
    }

    Ok(packages)
}

/// Version comparison utilities
pub mod version {
    use std::cmp::Ordering;

    /// Compare two version strings
    /// Returns Ordering based on semantic version comparison
    pub fn compare(a: &str, b: &str) -> Ordering {
        let a_parts = parse_version(a);
        let b_parts = parse_version(b);

        for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
            match a_part.cmp(b_part) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        a_parts.len().cmp(&b_parts.len())
    }

    /// Parse a version string into comparable parts
    fn parse_version(version: &str) -> Vec<u64> {
        version
            .split(|c: char| c == '.' || c == '-' || c == '_')
            .filter_map(|part| {
                // Extract leading digits
                let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse().ok()
            })
            .collect()
    }

    /// Check if version meets minimum requirement
    pub fn meets_minimum(version: &str, minimum: &str) -> bool {
        compare(version, minimum) != Ordering::Less
    }

    /// Check if version is in range [min, max)
    #[allow(dead_code)]
    pub fn in_range(version: &str, min: &str, max: &str) -> bool {
        compare(version, min) != Ordering::Less && compare(version, max) == Ordering::Less
    }

    /// Check if version matches a constraint (e.g., ">=1.0.0", ">=1.0.0,<2.0.0", "1.2.3")
    pub fn matches_constraint(version: &str, constraint: &str) -> bool {
        let constraint = constraint.trim();

        // Handle exact version match
        if !constraint.contains('>') && !constraint.contains('<') {
            return version == constraint;
        }

        // Handle range constraint
        if constraint.contains(',') {
            let parts: Vec<&str> = constraint.split(',').collect();
            return parts.iter().all(|part| matches_constraint(version, part));
        }

        // Handle individual constraints
        if let Some(min) = constraint.strip_prefix(">=") {
            return meets_minimum(version, min.trim());
        }
        if let Some(min) = constraint.strip_prefix(">") {
            return compare(version, min.trim()) == Ordering::Greater;
        }
        if let Some(max) = constraint.strip_prefix("<=") {
            return compare(version, max.trim()) != Ordering::Greater;
        }
        if let Some(max) = constraint.strip_prefix("<") {
            return compare(version, max.trim()) == Ordering::Less;
        }

        // Default: exact match
        version == constraint
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_version_compare() {
            assert_eq!(compare("1.0.0", "1.0.0"), Ordering::Equal);
            assert_eq!(compare("1.0.1", "1.0.0"), Ordering::Greater);
            assert_eq!(compare("1.0.0", "1.0.1"), Ordering::Less);
            assert_eq!(compare("2.0.0", "1.9.9"), Ordering::Greater);
            assert_eq!(compare("1.10.0", "1.9.0"), Ordering::Greater);
        }

        #[test]
        fn test_meets_minimum() {
            assert!(meets_minimum("2.0.0", "1.0.0"));
            assert!(meets_minimum("1.0.0", "1.0.0"));
            assert!(!meets_minimum("0.9.0", "1.0.0"));
        }

        #[test]
        fn test_matches_constraint() {
            assert!(matches_constraint("1.5.0", ">=1.0.0"));
            assert!(matches_constraint("1.5.0", ">=1.0.0,<2.0.0"));
            assert!(!matches_constraint("2.1.0", ">=1.0.0,<2.0.0"));
            assert!(matches_constraint("1.0.0", "1.0.0"));
            assert!(!matches_constraint("1.0.1", "1.0.0"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winget_package_creation() {
        let package = WingetPackage::new("Git.Git");
        assert_eq!(package.id, "Git.Git");
        assert!(package.version.is_none());
    }

    #[test]
    fn test_winget_package_with_version() {
        let package = WingetPackage::with_version("Git.Git", "2.43.0");
        assert_eq!(package.id, "Git.Git");
        assert_eq!(package.version, Some("2.43.0".to_string()));
    }

    #[test]
    fn test_provider_name() {
        let provider = WingetProvider::new();
        assert_eq!(provider.name(), "winget");
    }

    #[test]
    fn test_error_is_retryable() {
        assert!(WingetError::NetworkError("test".to_string()).is_retryable());
        assert!(WingetError::Timeout {
            package: "test".to_string(),
            timeout_seconds: 60
        }
        .is_retryable());
        assert!(!WingetError::PackageNotFound("test".to_string()).is_retryable());
        assert!(!WingetError::AccessDenied("test".to_string()).is_retryable());
    }

    #[test]
    fn test_error_suggestions() {
        let err = WingetError::PackageNotFound("test".to_string());
        assert!(err.suggestion().is_some());

        let err = WingetError::AccessDenied("test".to_string());
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(exit_codes::SUCCESS, 0);
        assert_eq!(exit_codes::ALREADY_INSTALLED, -1978335215);
    }
}
