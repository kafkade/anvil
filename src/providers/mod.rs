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
#[cfg(target_os = "windows")]
pub use winget::WingetProvider;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in providers
#[derive(Error, Debug)]
#[allow(dead_code)] // Variants cover all expected error conditions
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

/// Trait for providers that can report their status
#[allow(dead_code)] // Test-only: used in module tests
pub trait ProviderStatus {
    /// Check if the provider is available and functional
    fn is_available(&self) -> bool;

    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get provider version information (if applicable)
    fn version(&self) -> Option<String>;
}

/// Common configuration for providers
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Enable verbose output
    pub verbose: bool,

    /// Dry run mode - don't make actual changes
    pub dry_run: bool,

    /// Number of retry attempts for transient failures
    pub retry_count: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            dry_run: false,
            retry_count: 3,
        }
    }
}
// ---------------------------------------------------------------------------
// Package-manager abstraction layer
// ---------------------------------------------------------------------------

/// A package specification that is manager-agnostic.
/// Each package manager can interpret these fields as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    /// Package identifier (e.g. "Git.Git" for winget, "git" for brew)
    pub id: String,
    /// Specific version to install (None = latest)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Package source or tap
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Override arguments passed to the installer (e.g. `--override "/VERYSILENT"`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_args: Option<Vec<String>>,
}

/// Result of installing a package
#[derive(Debug, Clone)]
pub struct PackageInstallResult {
    /// Package that was installed
    #[allow(dead_code)] // Test-only: used in module tests
    pub package_id: String,
    /// Whether the installation succeeded
    pub success: bool,
    /// Whether the package was already installed (no-op)
    #[allow(dead_code)] // Test-only: used in module tests
    pub already_installed: bool,
    /// Version that was installed
    pub installed_version: Option<String>,
    /// Whether a reboot is required
    pub needs_reboot: bool,
    /// Human-readable message
    pub message: String,
}

/// Information about an installed package
#[derive(Debug, Clone, Serialize)]
pub struct InstalledPackage {
    /// Package identifier
    pub id: String,
    /// Installed version
    pub version: Option<String>,
    /// Available version (if upgrade available)
    pub available_version: Option<String>,
    /// Package source
    pub source: Option<String>,
}

/// Information about a package manager provider (version and compatibility)
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    /// Provider version string (e.g. "v1.7.10")
    pub version: String,
    /// Whether the version meets the minimum requirements
    pub meets_minimum: bool,
}

/// Trait abstracting a system package manager (winget, brew, apt, etc.)
///
/// Implementations provide package installation, querying, and listing
/// capabilities. Only `winget` is implemented today; `brew` and `apt`
/// are planned for future cross-platform support.
pub trait PackageManager: Send + Sync {
    /// Return the manager's human-readable name (e.g., "winget", "brew", "apt")
    fn name(&self) -> &str;

    /// Check whether the manager binary is available on this system
    #[allow(dead_code)] // Test-only: used in module tests
    fn is_available(&self) -> bool;

    /// Check availability and return version / compatibility info
    fn check_availability(&self) -> Result<ProviderInfo, ProviderError>;

    /// Install a package from a specification
    fn install(&self, package: &PackageSpec) -> Result<PackageInstallResult, ProviderError>;

    /// Upgrade an already-installed package to its latest version
    fn upgrade(&self, package_id: &str) -> Result<PackageInstallResult, ProviderError>;

    /// Check whether a package is installed, returning its version if so
    fn is_installed(&self, package_id: &str) -> Result<Option<String>, ProviderError>;

    /// List all packages managed by this provider
    fn list_installed(&self) -> Result<Vec<InstalledPackage>, ProviderError>;
}

/// Registry of available package managers.
/// At runtime, managers are detected and only available ones are used.
pub struct PackageManagerRegistry {
    managers: Vec<Box<dyn PackageManager>>,
}
impl PackageManagerRegistry {
    /// Create a new registry with no managers
    pub fn new() -> Self {
        Self {
            managers: Vec::new(),
        }
    }

    /// Register a package manager
    #[allow(dead_code)] // Called inside #[cfg(target_os)] blocks and tests
    pub fn register(&mut self, manager: Box<dyn PackageManager>) {
        self.managers.push(manager);
    }

    /// Get a manager by name
    pub fn get(&self, name: &str) -> Option<&dyn PackageManager> {
        self.managers
            .iter()
            .find(|m| m.name() == name)
            .map(|m| m.as_ref())
    }

    /// Get all available managers (those whose is_available() returns true)
    #[allow(dead_code)] // Test-only: used in module tests
    pub fn available(&self) -> Vec<&dyn PackageManager> {
        self.managers
            .iter()
            .filter(|m| m.is_available())
            .map(|m| m.as_ref())
            .collect()
    }

    /// Get all registered manager names
    #[allow(dead_code)] // Test-only: used in module tests
    pub fn registered_names(&self) -> Vec<&str> {
        self.managers.iter().map(|m| m.name()).collect()
    }
}

// ---------------------------------------------------------------------------
// Conversion: WingetPackage → PackageSpec (lossless)
// ---------------------------------------------------------------------------

impl From<&crate::config::workload::WingetPackage> for PackageSpec {
    fn from(wp: &crate::config::workload::WingetPackage) -> Self {
        // Merge override_args and override_str into a single list
        let mut args: Vec<String> = Vec::new();
        if let Some(ref oa) = wp.override_args {
            args.extend(oa.iter().cloned());
        }
        if let Some(ref os) = wp.override_str {
            args.extend(os.iter().cloned());
        }

        PackageSpec {
            id: wp.id.clone(),
            version: wp.version.clone(),
            source: wp.source.clone(),
            override_args: if args.is_empty() { None } else { Some(args) },
        }
    }
}

// ---------------------------------------------------------------------------
// Factory: config-aware registry
// ---------------------------------------------------------------------------

/// Build a [`PackageManagerRegistry`] with all providers appropriate
/// for the current platform, configured with the given settings.
pub fn create_registry(_config: &ProviderConfig) -> PackageManagerRegistry {
    #[allow(unused_mut)]
    let mut registry = PackageManagerRegistry::new();

    #[cfg(target_os = "windows")]
    {
        registry.register(Box::new(WingetProvider::with_config(_config.clone())));
    }

    // Future: register Homebrew / APT providers on their respective platforms

    registry
}

/// Generate a provider-scoped cache key to prevent collisions across
/// package managers (e.g. `winget:git.git` vs `brew:git`).
pub fn cache_key(provider: &str, package_id: &str) -> String {
    format!("{}:{}", provider, package_id.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock package manager for testing the trait and registry.
    struct MockManager {
        manager_name: String,
        available: bool,
        packages: Vec<InstalledPackage>,
    }

    impl MockManager {
        fn new(name: &str, available: bool) -> Self {
            Self {
                manager_name: name.to_string(),
                available,
                packages: Vec::new(),
            }
        }

        fn with_package(mut self, id: &str, version: &str) -> Self {
            self.packages.push(InstalledPackage {
                id: id.to_string(),
                version: Some(version.to_string()),
                available_version: None,
                source: None,
            });
            self
        }
    }

    impl PackageManager for MockManager {
        fn name(&self) -> &str {
            &self.manager_name
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn check_availability(&self) -> Result<ProviderInfo, ProviderError> {
            if self.available {
                Ok(ProviderInfo {
                    version: "1.0.0".to_string(),
                    meets_minimum: true,
                })
            } else {
                Err(ProviderError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not available",
                )))
            }
        }

        fn install(&self, package: &PackageSpec) -> Result<PackageInstallResult, ProviderError> {
            Ok(PackageInstallResult {
                package_id: package.id.clone(),
                success: true,
                already_installed: false,
                installed_version: package.version.clone(),
                needs_reboot: false,
                message: format!("Installed {}", package.id),
            })
        }

        fn upgrade(&self, package_id: &str) -> Result<PackageInstallResult, ProviderError> {
            Ok(PackageInstallResult {
                package_id: package_id.to_string(),
                success: true,
                already_installed: false,
                installed_version: None,
                needs_reboot: false,
                message: format!("Upgraded {}", package_id),
            })
        }

        fn is_installed(&self, package_id: &str) -> Result<Option<String>, ProviderError> {
            Ok(self
                .packages
                .iter()
                .find(|p| p.id == package_id)
                .and_then(|p| p.version.clone()))
        }

        fn list_installed(&self) -> Result<Vec<InstalledPackage>, ProviderError> {
            Ok(self.packages.clone())
        }
    }

    // -- PackageSpec tests --

    #[test]
    fn test_package_spec_creation() {
        let spec = PackageSpec {
            id: "Git.Git".to_string(),
            version: Some("2.42.0".to_string()),
            source: None,
            override_args: None,
        };
        assert_eq!(spec.id, "Git.Git");
        assert_eq!(spec.version.as_deref(), Some("2.42.0"));
        assert!(spec.source.is_none());
        assert!(spec.override_args.is_none());
    }

    #[test]
    fn test_package_spec_serde_roundtrip_full() {
        let spec = PackageSpec {
            id: "Git.Git".to_string(),
            version: Some("2.42.0".to_string()),
            source: Some("winget".to_string()),
            override_args: None,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: PackageSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "Git.Git");
        assert_eq!(deserialized.version.as_deref(), Some("2.42.0"));
        assert_eq!(deserialized.source.as_deref(), Some("winget"));
    }

    #[test]
    fn test_package_spec_serde_roundtrip_minimal() {
        let json = r#"{"id":"ripgrep"}"#;
        let spec: PackageSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.id, "ripgrep");
        assert!(spec.version.is_none());
        assert!(spec.source.is_none());

        // Re-serialize should omit None fields
        let reserialized = serde_json::to_string(&spec).unwrap();
        assert!(!reserialized.contains("version"));
        assert!(!reserialized.contains("source"));
    }

    // -- PackageManagerRegistry tests --

    #[test]
    fn test_registry_empty() {
        let registry = PackageManagerRegistry::new();
        assert!(registry.get("winget").is_none());
        assert!(registry.available().is_empty());
        assert!(registry.registered_names().is_empty());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = PackageManagerRegistry::new();
        registry.register(Box::new(MockManager::new("winget", true)));
        registry.register(Box::new(MockManager::new("brew", false)));

        assert!(registry.get("winget").is_some());
        assert_eq!(registry.get("winget").unwrap().name(), "winget");
        assert!(registry.get("brew").is_some());
        assert!(registry.get("apt").is_none());
    }

    #[test]
    fn test_registry_available_filters_unavailable() {
        let mut registry = PackageManagerRegistry::new();
        registry.register(Box::new(MockManager::new("winget", true)));
        registry.register(Box::new(MockManager::new("brew", false)));
        registry.register(Box::new(MockManager::new("apt", true)));

        let available = registry.available();
        assert_eq!(available.len(), 2);
        let names: Vec<&str> = available.iter().map(|m| m.name()).collect();
        assert!(names.contains(&"winget"));
        assert!(names.contains(&"apt"));
        assert!(!names.contains(&"brew"));
    }

    #[test]
    fn test_registry_registered_names() {
        let mut registry = PackageManagerRegistry::new();
        registry.register(Box::new(MockManager::new("winget", true)));
        registry.register(Box::new(MockManager::new("brew", false)));

        let names = registry.registered_names();
        assert_eq!(names, vec!["winget", "brew"]);
    }

    // -- Mock PackageManager behaviour --

    #[test]
    fn test_mock_install() {
        let manager = MockManager::new("test", true);
        let spec = PackageSpec {
            id: "Pkg.Test".to_string(),
            version: Some("1.0.0".to_string()),
            source: None,
            override_args: None,
        };
        let result = manager.install(&spec).unwrap();
        assert!(result.success);
        assert!(!result.already_installed);
        assert_eq!(result.package_id, "Pkg.Test");
        assert_eq!(result.installed_version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn test_mock_is_installed() {
        let manager = MockManager::new("test", true).with_package("Git.Git", "2.42.0");
        assert_eq!(
            manager.is_installed("Git.Git").unwrap().as_deref(),
            Some("2.42.0")
        );
        assert!(manager.is_installed("Unknown.Pkg").unwrap().is_none());
    }

    #[test]
    fn test_mock_list_installed() {
        let manager = MockManager::new("test", true)
            .with_package("Git.Git", "2.42.0")
            .with_package("Rustlang.Rust", "1.75.0");

        let packages = manager.list_installed().unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "Git.Git");
        assert_eq!(packages[1].id, "Rustlang.Rust");
    }
}
