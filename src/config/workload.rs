//! Workload configuration structures
//!
//! This module defines the data structures for parsing and representing
//! workload definition files (workload.yaml).

use serde::{Deserialize, Serialize};

/// A complete workload definition
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    /// Unique workload identifier
    pub name: String,

    /// Semantic version (e.g., "1.0.0")
    pub version: String,

    /// Human-readable description
    pub description: String,

    /// List of parent workloads to extend
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<Vec<String>>,

    /// Package definitions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packages: Option<Packages>,

    /// File definitions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileEntry>>,

    /// Script definitions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scripts: Option<Scripts>,

    /// Environment configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,

    /// Health check configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthConfig>,
}

#[allow(dead_code)]
impl Workload {
    /// Create a new minimal workload
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            extends: None,
            packages: None,
            files: None,
            scripts: None,
            environment: None,
            health: None,
        }
    }

    /// Create an empty workload (used as a base for merging)
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: String::new(),
            extends: None,
            packages: None,
            files: None,
            scripts: None,
            environment: None,
            health: None,
        }
    }

    /// Check if this workload extends other workloads
    pub fn has_parents(&self) -> bool {
        self.extends
            .as_ref()
            .map(|e| !e.is_empty())
            .unwrap_or(false)
    }

    /// Get the list of parent workload names
    pub fn parent_names(&self) -> Vec<&str> {
        self.extends
            .as_ref()
            .map(|e| e.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get the total number of packages
    pub fn package_count(&self) -> usize {
        self.packages
            .as_ref()
            .and_then(|p| p.winget.as_ref())
            .map(|w| w.len())
            .unwrap_or(0)
    }

    /// Get the total number of files
    pub fn file_count(&self) -> usize {
        self.files.as_ref().map(|f| f.len()).unwrap_or(0)
    }
}

/// Package manager definitions
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Packages {
    /// Winget package definitions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winget: Option<Vec<WingetPackage>>,
}

/// A single winget package definition
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingetPackage {
    /// Winget package ID (required)
    pub id: String,

    /// Specific version to install (optional, default: latest)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Package source (optional, default: winget)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Additional winget arguments (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_args: Option<Vec<String>>,

    /// Override string for installer (shorthand for common patterns)
    #[serde(rename = "override", default, skip_serializing_if = "Option::is_none")]
    pub override_str: Option<Vec<String>>,
}

#[allow(dead_code)]
impl WingetPackage {
    /// Create a new package with just an ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: None,
            source: None,
            override_args: None,
            override_str: None,
        }
    }

    /// Create a new package with ID and version
    pub fn with_version(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: Some(version.into()),
            source: None,
            override_args: None,
            override_str: None,
        }
    }
}

/// A file to be copied to the target system
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from workload's files/ directory
    pub source: String,

    /// Absolute path or path with variables for destination
    pub destination: String,

    /// Whether to backup existing file (default: true)
    #[serde(default = "default_true")]
    pub backup: bool,

    /// File permissions (future use)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,

    /// Whether to process as a template (default: false)
    #[serde(default)]
    pub template: bool,
}

fn default_true() -> bool {
    true
}

#[allow(dead_code)]
impl FileEntry {
    /// Create a new file entry
    pub fn new(source: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            backup: true,
            permissions: None,
            template: false,
        }
    }
}

/// Script definitions for various phases
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scripts {
    /// Scripts to run before package installation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<Vec<ScriptEntry>>,

    /// Scripts to run after package installation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_install: Option<Vec<ScriptEntry>>,

    /// Scripts for health validation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check: Option<Vec<HealthCheckScript>>,
}

/// A script to execute
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    /// Relative path from workload's scripts/ directory
    pub path: String,

    /// Execution shell (default: "powershell")
    #[serde(default = "default_shell")]
    pub shell: String,

    /// Description of what the script does
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether to require admin privileges (default: false)
    #[serde(default)]
    pub elevated: bool,

    /// Timeout in seconds (default: 300)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_shell() -> String {
    "powershell".to_string()
}

fn default_timeout() -> u64 {
    300
}

#[allow(dead_code)]
impl ScriptEntry {
    /// Create a new script entry
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            shell: default_shell(),
            description: None,
            elevated: false,
            timeout: default_timeout(),
        }
    }
}

/// A health check script with metadata
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckScript {
    /// Relative path from workload's scripts/ directory
    pub path: String,

    /// Display name for the check
    pub name: String,

    /// Description of what this check validates
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Execution shell (default: "powershell")
    #[serde(default = "default_shell")]
    pub shell: String,
}

#[allow(dead_code)]
impl HealthCheckScript {
    /// Create a new health check script
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            description: None,
            shell: default_shell(),
        }
    }
}

/// Environment configuration
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    /// Environment variables to set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<EnvVariable>>,

    /// Paths to add to PATH
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_additions: Option<Vec<String>>,
}

/// An environment variable to set
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    /// Variable name
    pub name: String,

    /// Variable value
    pub value: String,

    /// Scope: "user" or "machine" (default: "user")
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_scope() -> String {
    "user".to_string()
}

#[allow(dead_code)]
impl EnvVariable {
    /// Create a new user-scoped environment variable
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            scope: default_scope(),
        }
    }

    /// Create a new machine-scoped environment variable
    pub fn machine(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            scope: "machine".to_string(),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Whether to verify packages are installed (default: true)
    #[serde(default = "default_true")]
    pub package_check: bool,

    /// Whether to verify files match (default: true)
    #[serde(default = "default_true")]
    pub file_check: bool,

    /// Whether to run health check scripts (default: true)
    #[serde(default = "default_true")]
    pub script_check: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            package_check: true,
            file_check: true,
            script_check: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_minimal_workload() {
        let yaml = r#"
name: test-workload
version: "1.0.0"
description: "A test workload"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workload.name, "test-workload");
        assert_eq!(workload.version, "1.0.0");
        assert!(!workload.has_parents());
    }

    #[test]
    fn test_deserialize_full_workload() {
        let yaml = r#"
name: full-workload
version: "1.0.0"
description: "A full workload"
extends:
  - base-workload
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
      version: "1.85.0"
files:
  - source: config.toml
    destination: "~/.config/app/config.toml"
scripts:
  post_install:
    - path: setup.ps1
      description: "Run setup"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workload.name, "full-workload");
        assert!(workload.has_parents());
        assert_eq!(workload.package_count(), 2);
        assert_eq!(workload.file_count(), 1);
    }

    #[test]
    fn test_winget_package_override() {
        let yaml = r#"
id: Git.Git
override:
  - --override
  - '/VERYSILENT'
"#;
        let package: WingetPackage = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(package.id, "Git.Git");
        assert!(package.override_str.is_some());
        assert_eq!(package.override_str.unwrap().len(), 2);
    }
}
