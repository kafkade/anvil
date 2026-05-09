//! Workload configuration structures
//!
//! This module defines the data structures for parsing and representing
//! workload definition files (workload.yaml).
use serde::{Deserialize, Serialize};

/// A complete workload definition
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

    /// Inline command definitions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<CommandBlock>,

    /// Environment configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,

    /// Health check configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthConfig>,

    /// Declarative assertions for health validation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assertions: Option<Vec<Assertion>>,

    /// Font definitions for installation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonts: Option<Vec<FontEntry>>,

    /// Windows Terminal configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalConfig>,

    /// Windows feature toggles
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<FeatureEntry>>,
}
impl Workload {
    /// Create an empty workload (used as a base for merging)
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: String::new(),
            extends: None,
            packages: None,
            files: None,
            commands: None,
            environment: None,
            health: None,
            assertions: None,
            fonts: None,
            terminal: None,
            features: None,
        }
    }

    /// Get the total number of packages across all managers
    pub fn package_count(&self) -> usize {
        self.packages
            .as_ref()
            .map(|p| {
                let winget = p.winget.as_ref().map(|w| w.len()).unwrap_or(0);
                let brew = p.brew.as_ref().map(|b| b.len()).unwrap_or(0);
                let apt = p.apt.as_ref().map(|a| a.len()).unwrap_or(0);
                winget + brew + apt
            })
            .unwrap_or(0)
    }

    /// Get the total number of files
    pub fn file_count(&self) -> usize {
        self.files.as_ref().map(|f| f.len()).unwrap_or(0)
    }

    /// Get the total number of fonts
    pub fn font_count(&self) -> usize {
        self.fonts.as_ref().map(|f| f.len()).unwrap_or(0)
    }

    /// Get the total number of feature toggles
    pub fn feature_count(&self) -> usize {
        self.features.as_ref().map(|f| f.len()).unwrap_or(0)
    }

    /// Get the total number of commands (pre_install + post_install)
    pub fn command_count(&self) -> usize {
        self.commands
            .as_ref()
            .map(|c| {
                let pre = c.pre_install.as_ref().map(|p| p.len()).unwrap_or(0);
                let post = c.post_install.as_ref().map(|p| p.len()).unwrap_or(0);
                pre + post
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
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
            commands: None,
            environment: None,
            health: None,
            assertions: None,
            fonts: None,
            terminal: None,
            features: None,
        }
    }

    /// Check if this workload extends other workloads
    pub fn has_parents(&self) -> bool {
        self.extends
            .as_ref()
            .map(|e| !e.is_empty())
            .unwrap_or(false)
    }

    /// Get the total number of assertions
    pub fn assertion_count(&self) -> usize {
        self.assertions.as_ref().map(|a| a.len()).unwrap_or(0)
    }
}

/// A declarative assertion using the condition engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// Display name for the assertion
    pub name: String,

    /// The condition to evaluate
    pub check: crate::conditions::Condition,
}

/// Package manager definitions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Packages {
    /// Winget package definitions (Windows)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winget: Option<Vec<WingetPackage>>,

    /// Homebrew package definitions (macOS/Linux)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brew: Option<Vec<BrewPackage>>,

    /// APT package definitions (Debian/Ubuntu)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apt: Option<Vec<AptPackage>>,
}

/// A single winget package definition
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

#[cfg(test)]
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

/// A Homebrew package definition (schema-only, not yet implemented)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrewPackage {
    /// Package name (e.g., "git", "node")
    pub name: String,
    /// Whether this is a cask (GUI app) vs formula (CLI tool)
    #[serde(default)]
    pub cask: bool,
    /// Tap source (e.g., "homebrew/cask-fonts")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap: Option<String>,
}

/// An APT package definition (schema-only, not yet implemented)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptPackage {
    /// Package name (e.g., "git", "build-essential")
    pub name: String,
    /// Specific version constraint
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A file to be copied to the target system
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

/// A font to download and install
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontEntry {
    /// Display name of the font
    pub name: String,

    /// URL to download the font archive (zip)
    pub url: String,

    /// Font version string
    pub version: String,

    /// Font file format to look for inside the archive
    #[serde(default = "default_font_format")]
    pub format: String,

    /// Subdirectory within the archive containing font files
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subfolder: Option<String>,

    /// Specific font file variants to install (default: all matching format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
}

fn default_font_format() -> String {
    "ttf".to_string()
}

/// Windows Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Color schemes to add/update
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schemes: Option<Vec<ColorScheme>>,

    /// Profile defaults to set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_defaults: Option<serde_json::Value>,
}

/// A Windows Terminal color scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub black: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yellow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purple: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cyan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white: Option<String>,
    #[serde(
        rename = "brightBlack",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_black: Option<String>,
    #[serde(rename = "brightRed", default, skip_serializing_if = "Option::is_none")]
    pub bright_red: Option<String>,
    #[serde(
        rename = "brightGreen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_green: Option<String>,
    #[serde(
        rename = "brightYellow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_yellow: Option<String>,
    #[serde(
        rename = "brightBlue",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_blue: Option<String>,
    #[serde(
        rename = "brightPurple",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_purple: Option<String>,
    #[serde(
        rename = "brightCyan",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_cyan: Option<String>,
    #[serde(
        rename = "brightWhite",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bright_white: Option<String>,
    #[serde(
        rename = "cursorColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cursor_color: Option<String>,
    #[serde(
        rename = "selectionBackground",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selection_background: Option<String>,
}

/// A Windows feature toggle (registry-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEntry {
    /// Display name of the feature
    pub name: String,

    /// Feature type (currently only "registry_toggle")
    #[serde(rename = "type", default = "default_feature_type")]
    pub feature_type: String,

    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Minimum Windows build number required
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_build: Option<u32>,

    /// Registry configuration
    pub registry: RegistryConfig,

    /// Whether elevation is required
    #[serde(default)]
    pub elevated: bool,
}

fn default_feature_type() -> String {
    "registry_toggle".to_string()
}

/// Registry configuration for a feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry key path (without hive prefix)
    pub path: String,

    /// Registry hive (HKLM or HKCU)
    #[serde(default = "default_hive")]
    pub hive: String,

    /// Values to set
    pub values: Vec<RegistryValueEntry>,
}

fn default_hive() -> String {
    "HKLM".to_string()
}

/// A single registry value to set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryValueEntry {
    /// Value name
    pub name: String,

    /// Value type (dword, string, expand_string)
    #[serde(rename = "type", default = "default_reg_type")]
    pub value_type: String,

    /// Value to set (as string — converted to appropriate type)
    pub value: serde_json::Value,
}

fn default_reg_type() -> String {
    "dword".to_string()
}

#[cfg(test)]
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

/// Commands block for inline command execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandBlock {
    /// Commands to run before package installation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<Vec<CommandEntry>>,

    /// Commands to run after package installation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_install: Option<Vec<CommandEntry>>,
}

fn default_timeout() -> u64 {
    300
}

/// A single command to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    /// Shell command string to execute (required)
    pub run: String,

    /// Human-readable description (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Timeout in seconds (default: 300)
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Whether the command requires admin privileges (default: false)
    #[serde(default)]
    pub elevated: bool,

    /// Condition that must be true for this command to run (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<crate::conditions::Condition>,

    /// Whether to continue if this command fails (default: false)
    #[serde(default)]
    pub continue_on_error: bool,
}

/// Environment configuration
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

#[cfg(test)]
impl EnvVariable {
    /// Create a new user-scoped environment variable
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            scope: default_scope(),
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

    /// Whether to evaluate declarative assertions (default: true)
    #[serde(default = "default_true")]
    pub assertion_check: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            package_check: true,
            file_check: true,
            assertion_check: true,
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

    #[test]
    fn test_deserialize_workload_with_assertions() {
        let yaml = r#"
name: test-workload
version: "1.0.0"
description: "Workload with assertions"
assertions:
  - name: "Git is installed"
    check:
      type: command_exists
      command: git
  - name: "Config dir exists"
    check:
      type: dir_exists
      path: "~/.config/app"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workload.assertion_count(), 2);
        let assertions = workload.assertions.unwrap();
        assert_eq!(assertions[0].name, "Git is installed");
        assert_eq!(assertions[1].name, "Config dir exists");
    }

    #[test]
    fn test_deserialize_assertion_condition_types() {
        let yaml_command = r#"
name: "command check"
check:
  type: command_exists
  command: rustc
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_command).unwrap();
        assert_eq!(assertion.name, "command check");
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::CommandExists { .. }
        ));

        let yaml_file = r#"
name: "file check"
check:
  type: file_exists
  path: "~/.bashrc"
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_file).unwrap();
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::FileExists { .. }
        ));

        let yaml_env = r#"
name: "env check"
check:
  type: env_var
  name: HOME
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_env).unwrap();
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::EnvVar { .. }
        ));

        let yaml_shell = r#"
name: "shell check"
check:
  type: shell
  command: "echo hello"
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_shell).unwrap();
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::Shell { .. }
        ));

        let yaml_path = r#"
name: "path check"
check:
  type: path_contains
  substring: ".cargo/bin"
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_path).unwrap();
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::PathContains { .. }
        ));

        let yaml_registry = r#"
name: "registry check"
check:
  type: registry_value
  hive: HKCU
  key: "SOFTWARE\\Test"
  name: TestValue
"#;
        let assertion: Assertion = serde_yaml::from_str(yaml_registry).unwrap();
        assert!(matches!(
            assertion.check,
            crate::conditions::Condition::RegistryValue { .. }
        ));
    }

    #[test]
    fn test_assertion_count() {
        let mut workload = Workload::new("test", "1.0.0", "test");
        assert_eq!(workload.assertion_count(), 0);

        workload.assertions = Some(vec![
            Assertion {
                name: "check1".to_string(),
                check: crate::conditions::Condition::CommandExists {
                    command: "git".to_string(),
                },
            },
            Assertion {
                name: "check2".to_string(),
                check: crate::conditions::Condition::FileExists {
                    path: "~/.bashrc".to_string(),
                },
            },
        ]);
        assert_eq!(workload.assertion_count(), 2);
    }

    #[test]
    fn test_workload_without_assertions_backward_compat() {
        let yaml = r#"
name: legacy-workload
version: "1.0.0"
description: "No assertions"
packages:
  winget:
    - id: Git.Git
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert!(workload.assertions.is_none());
        assert_eq!(workload.assertion_count(), 0);
        assert_eq!(workload.package_count(), 1);
    }

    #[test]
    fn test_deserialize_brew_packages() {
        let yaml = r#"
name: brew-workload
version: "1.0.0"
description: "Workload with brew packages"
packages:
  brew:
    - name: git
    - name: visual-studio-code
      cask: true
    - name: font-cascadia-code
      cask: true
      tap: "homebrew/cask-fonts"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        let brew = workload.packages.as_ref().unwrap().brew.as_ref().unwrap();
        assert_eq!(brew.len(), 3);
        assert_eq!(brew[0].name, "git");
        assert!(!brew[0].cask);
        assert!(brew[1].cask);
        assert_eq!(brew[2].tap.as_deref(), Some("homebrew/cask-fonts"));
        assert_eq!(workload.package_count(), 3);
    }

    #[test]
    fn test_deserialize_apt_packages() {
        let yaml = r#"
name: apt-workload
version: "1.0.0"
description: "Workload with apt packages"
packages:
  apt:
    - name: git
    - name: build-essential
      version: "12.9"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        let apt = workload.packages.as_ref().unwrap().apt.as_ref().unwrap();
        assert_eq!(apt.len(), 2);
        assert_eq!(apt[0].name, "git");
        assert!(apt[0].version.is_none());
        assert_eq!(apt[1].name, "build-essential");
        assert_eq!(apt[1].version.as_deref(), Some("12.9"));
        assert_eq!(workload.package_count(), 2);
    }

    #[test]
    fn test_deserialize_all_three_managers() {
        let yaml = r#"
name: multi-manager
version: "1.0.0"
description: "Workload with all managers"
packages:
  winget:
    - id: Git.Git
  brew:
    - name: git
  apt:
    - name: git
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        let pkgs = workload.packages.as_ref().unwrap();
        assert_eq!(pkgs.winget.as_ref().unwrap().len(), 1);
        assert_eq!(pkgs.brew.as_ref().unwrap().len(), 1);
        assert_eq!(pkgs.apt.as_ref().unwrap().len(), 1);
        assert_eq!(workload.package_count(), 3);
    }

    #[test]
    fn test_package_count_all_managers() {
        let yaml = r#"
name: count-test
version: "1.0.0"
description: "Count test"
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
  brew:
    - name: git
    - name: node
    - name: visual-studio-code
      cask: true
  apt:
    - name: git
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workload.package_count(), 6); // 2 + 3 + 1
    }

    #[test]
    fn test_backward_compat_winget_only() {
        let yaml = r#"
name: winget-only
version: "1.0.0"
description: "Only winget"
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
      version: "1.85.0"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workload.package_count(), 2);
        let pkgs = workload.packages.as_ref().unwrap();
        assert!(pkgs.brew.is_none());
        assert!(pkgs.apt.is_none());
        assert!(pkgs.winget.is_some());
    }

    #[test]
    fn test_deserialize_workload_with_commands() {
        let yaml = r#"
name: cmd-workload
version: "1.0.0"
description: "Workload with commands"
commands:
  pre_install:
    - run: "echo pre-step"
      description: "Pre-install echo"
  post_install:
    - run: "echo post-step"
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert!(workload.commands.is_some());
        let cmds = workload.commands.as_ref().unwrap();
        assert_eq!(cmds.pre_install.as_ref().unwrap().len(), 1);
        assert_eq!(cmds.post_install.as_ref().unwrap().len(), 1);
        assert_eq!(cmds.pre_install.as_ref().unwrap()[0].run, "echo pre-step");
        assert_eq!(
            cmds.pre_install.as_ref().unwrap()[0].description.as_deref(),
            Some("Pre-install echo")
        );
    }

    #[test]
    fn test_deserialize_command_with_when_condition() {
        let yaml = r#"
name: cond-cmd
version: "1.0.0"
description: "Command with when"
commands:
  post_install:
    - run: "cargo install sccache"
      when:
        type: command_exists
        command: cargo
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        let cmds = workload.commands.as_ref().unwrap();
        let post = cmds.post_install.as_ref().unwrap();
        assert_eq!(post.len(), 1);
        assert!(post[0].when.is_some());
        assert!(matches!(
            post[0].when.as_ref().unwrap(),
            crate::conditions::Condition::CommandExists { .. }
        ));
    }

    #[test]
    fn test_command_count() {
        let mut workload = Workload::new("test", "1.0.0", "test");
        assert_eq!(workload.command_count(), 0);

        workload.commands = Some(CommandBlock {
            pre_install: Some(vec![CommandEntry {
                run: "echo a".to_string(),
                description: None,
                timeout: 300,
                elevated: false,
                when: None,
                continue_on_error: false,
            }]),
            post_install: Some(vec![
                CommandEntry {
                    run: "echo b".to_string(),
                    description: None,
                    timeout: 300,
                    elevated: false,
                    when: None,
                    continue_on_error: false,
                },
                CommandEntry {
                    run: "echo c".to_string(),
                    description: None,
                    timeout: 300,
                    elevated: false,
                    when: None,
                    continue_on_error: false,
                },
            ]),
        });
        assert_eq!(workload.command_count(), 3);
    }

    #[test]
    fn test_workload_without_commands_backward_compat() {
        let yaml = r#"
name: legacy
version: "1.0.0"
description: "No commands"
packages:
  winget:
    - id: Git.Git
"#;
        let workload: Workload = serde_yaml::from_str(yaml).unwrap();
        assert!(workload.commands.is_none());
        assert_eq!(workload.command_count(), 0);
        assert_eq!(workload.package_count(), 1);
    }

    #[test]
    fn test_command_entry_all_fields() {
        let yaml = r#"
run: "npm install -g typescript"
description: "Install TypeScript globally"
timeout: 120
elevated: true
continue_on_error: true
when:
  type: command_exists
  command: npm
"#;
        let cmd: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.run, "npm install -g typescript");
        assert_eq!(
            cmd.description.as_deref(),
            Some("Install TypeScript globally")
        );
        assert_eq!(cmd.timeout, 120);
        assert!(cmd.elevated);
        assert!(cmd.continue_on_error);
        assert!(cmd.when.is_some());
    }
}
