//! Schema validation module for Anvil workload definitions
//!
//! This module provides validation logic for workload YAML files,
//! ensuring they conform to the expected schema and contain valid values.

use anyhow::{Context, Result};
use std::path::Path;

use super::workload::Workload;

/// Validation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Error - validation failure that prevents operation
    Error,
    /// Warning - potential issue but not blocking
    Warning,
    /// Info - informational message
    Info,
}

/// A single validation message
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Severity of the validation issue
    pub severity: ValidationSeverity,
    /// Path within the document where the issue occurred (e.g., "packages.winget[0].id")
    pub path: String,
    /// Description of the issue
    pub message: String,
}

impl ValidationMessage {
    /// Create a new error message
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new warning message
    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new info message
    pub fn info(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Info,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            ValidationSeverity::Error => "ERROR",
            ValidationSeverity::Warning => "WARN",
            ValidationSeverity::Info => "INFO",
        };
        write!(f, "[{}] {}: {}", severity, self.path, self.message)
    }
}

/// Result of validating a workload
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// List of validation messages
    pub messages: Vec<ValidationMessage>,
}

#[allow(dead_code)]
impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a message to the validation result
    pub fn add(&mut self, message: ValidationMessage) {
        self.messages.push(message);
    }

    /// Add an error message
    pub fn add_error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.add(ValidationMessage::error(path, message));
    }

    /// Add a warning message
    pub fn add_warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.add(ValidationMessage::warning(path, message));
    }

    /// Add an info message
    pub fn add_info(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.add(ValidationMessage::info(path, message));
    }

    /// Check if the validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self
            .messages
            .iter()
            .any(|m| m.severity == ValidationSeverity::Error)
    }

    /// Check if the validation passed with no warnings
    pub fn is_perfect(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the count of errors
    pub fn error_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Error)
            .count()
    }

    /// Get the count of warnings
    pub fn warning_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Warning)
            .count()
    }

    /// Get all error messages
    pub fn errors(&self) -> impl Iterator<Item = &ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Error)
    }

    /// Get all warning messages
    pub fn warnings(&self) -> impl Iterator<Item = &ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Warning)
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.messages.extend(other.messages);
    }
}

/// Schema validator for workload definitions
#[allow(dead_code)]
pub struct SchemaValidator {
    /// Enable strict validation mode
    strict: bool,
}

#[allow(dead_code)]
impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Self {
        Self { strict: false }
    }

    /// Create a new schema validator with strict mode enabled
    pub fn strict() -> Self {
        Self { strict: true }
    }

    /// Validate a workload definition
    pub fn validate(&self, workload: &Workload) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate required fields
        self.validate_metadata(workload, &mut result);

        // Validate packages
        self.validate_packages(workload, &mut result);

        // Validate files
        self.validate_files(workload, &mut result);

        // Validate scripts
        self.validate_scripts(workload, &mut result);

        // Validate environment
        self.validate_environment(workload, &mut result);

        result
    }

    /// Validate a workload file from path
    pub fn validate_file(&self, path: &Path) -> Result<ValidationResult> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let workload: Workload = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML: {}", path.display()))?;

        Ok(self.validate(&workload))
    }

    /// Validate workload metadata
    fn validate_metadata(&self, workload: &Workload, result: &mut ValidationResult) {
        // Name is required
        if workload.name.is_empty() {
            result.add_error("name", "Workload name is required");
        } else if !is_valid_workload_name(&workload.name) {
            result.add_error(
                "name",
                format!(
                    "Workload name '{}' is invalid. Must start with a lowercase letter and contain only lowercase letters, numbers, and hyphens",
                    workload.name
                ),
            );
        }

        // Version validation
        if workload.version.is_empty() {
            if self.strict {
                result.add_error("version", "Version is required in strict mode");
            } else {
                result.add_warning("version", "Version is recommended");
            }
        } else if !is_valid_version(&workload.version) {
            result.add_warning(
                "version",
                format!(
                    "Version '{}' does not follow semantic versioning (e.g., '1.0.0' or '1.0.0-beta')",
                    workload.version
                ),
            );
        }

        // Description should be present
        if workload.description.is_empty() {
            if self.strict {
                result.add_warning("description", "Description is recommended");
            } else {
                result.add_info("description", "Consider adding a description");
            }
        }

        // Validate extends field
        if let Some(ref extends) = workload.extends {
            for (i, parent) in extends.iter().enumerate() {
                if parent.is_empty() {
                    result.add_error(
                        format!("extends[{}]", i),
                        "Parent workload name cannot be empty",
                    );
                } else if !is_valid_workload_name(parent) {
                    result.add_warning(
                        format!("extends[{}]", i),
                        format!(
                            "Parent workload name '{}' should follow naming conventions",
                            parent
                        ),
                    );
                }
            }
        }
    }

    /// Validate package definitions
    fn validate_packages(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(packages) = &workload.packages {
            if let Some(winget_packages) = &packages.winget {
                for (i, package) in winget_packages.iter().enumerate() {
                    let path = format!("packages.winget[{}]", i);

                    // Package ID is required
                    if package.id.is_empty() {
                        result.add_error(format!("{}.id", path), "Package ID is required");
                    } else {
                        // Check if this is an msstore package
                        let is_msstore = package
                            .source
                            .as_ref()
                            .map(|s| s.to_lowercase() == "msstore")
                            .unwrap_or(false);

                        if is_msstore {
                            // Validate Microsoft Store package ID format
                            if !is_valid_msstore_id(&package.id) {
                                result.add_warning(
                                    format!("{}.id", path),
                                    format!(
                                        "Package ID '{}' may not be a valid Microsoft Store ID. Expected alphanumeric format (e.g., '9NBLGGH4NNS1')",
                                        package.id
                                    ),
                                );
                            }
                        } else if !is_valid_winget_id(&package.id) {
                            result.add_warning(
                                format!("{}.id", path),
                                format!(
                                    "Package ID '{}' may not be a valid winget ID. Expected format: 'Publisher.PackageName'",
                                    package.id
                                ),
                            );
                        }
                    }

                    // Warn about version pinning in strict mode
                    if self.strict && package.version.is_none() {
                        result.add_warning(
                            format!("{}.version", path),
                            "Consider pinning package version for reproducibility",
                        );
                    }

                    // Validate version format if provided
                    if let Some(ref version) = package.version {
                        if version.is_empty() {
                            result.add_warning(
                                format!("{}.version", path),
                                "Version string is empty, will install latest",
                            );
                        }
                    }

                    // Validate source if provided
                    if let Some(ref source) = package.source {
                        let valid_sources = ["winget", "msstore"];
                        if !valid_sources.contains(&source.to_lowercase().as_str()) {
                            result.add_warning(
                                format!("{}.source", path),
                                format!(
                                    "Unknown source '{}'. Expected one of: {:?}",
                                    source, valid_sources
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate file definitions
    fn validate_files(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(files) = &workload.files {
            for (i, file) in files.iter().enumerate() {
                let path = format!("files[{}]", i);

                // Source is required
                if file.source.is_empty() {
                    result.add_error(format!("{}.source", path), "Source path is required");
                }

                // Destination is required
                if file.destination.is_empty() {
                    result.add_error(
                        format!("{}.destination", path),
                        "Destination path is required",
                    );
                }

                // Check for suspicious destination paths
                if file.destination.contains("..") {
                    result.add_warning(
                        format!("{}.destination", path),
                        "Destination path contains '..', which may be a security risk",
                    );
                }

                // Warn about absolute paths without variable expansion
                if (file.destination.starts_with('/') || file.destination.starts_with('\\'))
                    && !file.destination.contains('~')
                    && !file.destination.contains("${")
                {
                    result.add_info(
                        format!("{}.destination", path),
                        "Consider using ~ or ${HOME} for cross-user compatibility",
                    );
                }
            }
        }
    }

    /// Validate script definitions
    fn validate_scripts(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(scripts) = &workload.scripts {
            // Validate pre-install scripts
            if let Some(pre_scripts) = &scripts.pre_install {
                for (i, script) in pre_scripts.iter().enumerate() {
                    let path = format!("scripts.pre_install[{}]", i);
                    self.validate_script_entry(&path, script, result);
                }
            }

            // Validate post-install scripts
            if let Some(post_scripts) = &scripts.post_install {
                for (i, script) in post_scripts.iter().enumerate() {
                    let path = format!("scripts.post_install[{}]", i);
                    self.validate_script_entry(&path, script, result);
                }
            }

            // Validate health check scripts
            if let Some(health_scripts) = &scripts.health_check {
                for (i, script) in health_scripts.iter().enumerate() {
                    let path = format!("scripts.health_check[{}]", i);

                    // Path is required
                    if script.path.is_empty() {
                        result.add_error(format!("{}.path", path), "Script path is required");
                    }

                    // Health check scripts should have a name
                    if script.name.is_empty() {
                        result.add_warning(
                            format!("{}.name", path),
                            "Health check scripts should have a display name",
                        );
                    }

                    // Validate shell value
                    let valid_shells = ["powershell", "pwsh", "cmd", "bash"];
                    if !valid_shells.contains(&script.shell.to_lowercase().as_str()) {
                        result.add_warning(
                            format!("{}.shell", path),
                            format!(
                                "Unknown shell '{}'. Expected one of: {:?}",
                                script.shell, valid_shells
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Validate a single script entry
    fn validate_script_entry(
        &self,
        path: &str,
        script: &super::workload::ScriptEntry,
        result: &mut ValidationResult,
    ) {
        // Path is required
        if script.path.is_empty() {
            result.add_error(format!("{}.path", path), "Script path is required");
        }

        // Validate shell value
        let valid_shells = ["powershell", "pwsh", "cmd", "bash"];
        if !valid_shells.contains(&script.shell.to_lowercase().as_str()) {
            result.add_warning(
                format!("{}.shell", path),
                format!(
                    "Unknown shell '{}'. Expected one of: {:?}",
                    script.shell, valid_shells
                ),
            );
        }

        // Validate timeout values
        if script.timeout < 5 {
            result.add_warning(
                format!("{}.timeout", path),
                format!(
                    "Timeout of {} seconds may be too short. Consider at least 5 seconds.",
                    script.timeout
                ),
            );
        } else if script.timeout > 3600 {
            result.add_warning(
                format!("{}.timeout", path),
                format!(
                    "Timeout of {} seconds ({:.1} hours) is very long. Consider if this is intentional.",
                    script.timeout,
                    script.timeout as f64 / 3600.0
                ),
            );
        }

        // Warn about elevated scripts
        if script.elevated && self.strict {
            result.add_info(
                format!("{}.elevated", path),
                "Script requires elevated privileges. Ensure this is necessary.",
            );
        }
    }

    /// Validate environment definitions
    fn validate_environment(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(environment) = &workload.environment {
            if let Some(variables) = &environment.variables {
                for (i, var) in variables.iter().enumerate() {
                    let path = format!("environment.variables[{}]", i);

                    // Name is required
                    if var.name.is_empty() {
                        result.add_error(format!("{}.name", path), "Variable name is required");
                    } else if !is_valid_env_var_name(&var.name) {
                        result.add_warning(
                            format!("{}.name", path),
                            format!(
                                "Variable name '{}' contains characters that may cause issues",
                                var.name
                            ),
                        );
                    }

                    // Value can be empty but warn about it
                    if var.value.is_empty() {
                        result.add_info(
                            format!("{}.value", path),
                            format!("Variable '{}' has an empty value", var.name),
                        );
                    }

                    // Validate scope
                    let valid_scopes = ["user", "machine", "process"];
                    if !valid_scopes.contains(&var.scope.to_lowercase().as_str()) {
                        result.add_error(
                            format!("{}.scope", path),
                            format!(
                                "Invalid scope '{}'. Expected one of: {:?}",
                                var.scope, valid_scopes
                            ),
                        );
                    }

                    // Warn about machine scope (requires elevation)
                    if var.scope.to_lowercase() == "machine" {
                        result.add_info(
                            format!("{}.scope", path),
                            "Machine-scope variables require administrator privileges",
                        );
                    }
                }
            }

            // Validate path additions
            if let Some(paths) = &environment.path_additions {
                for (i, path_entry) in paths.iter().enumerate() {
                    let path = format!("environment.path_additions[{}]", i);

                    if path_entry.is_empty() {
                        result.add_error(path, "PATH addition cannot be empty");
                    }
                }
            }
        }
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a workload name is valid
///
/// Valid names:
/// - Start with a lowercase letter
/// - Contain only lowercase letters, numbers, and hyphens
/// - Not empty
pub fn is_valid_workload_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must start with a letter
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() {
        return false;
    }

    // Can only contain lowercase letters, numbers, and hyphens
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Check if a version string follows semantic versioning
///
/// Accepts formats like:
/// - "1.0.0"
/// - "1.0.0-beta"
/// - "1.0.0-alpha.1"
/// - "0.1.0-rc1"
fn is_valid_version(version: &str) -> bool {
    if version.is_empty() {
        return false;
    }

    // Simple semver pattern: MAJOR.MINOR.PATCH[-PRERELEASE]
    let parts: Vec<&str> = version.split('-').collect();
    if parts.is_empty() || parts.len() > 2 {
        return false;
    }

    // Check the version numbers part
    let version_nums: Vec<&str> = parts[0].split('.').collect();
    if version_nums.len() != 3 {
        return false;
    }

    // Each part should be a number
    for num in &version_nums {
        if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }

    // If there's a prerelease part, it should be alphanumeric with dots
    if parts.len() == 2 {
        let prerelease = parts[1];
        if prerelease.is_empty() {
            return false;
        }
        // Allow alphanumeric and dots in prerelease
        if !prerelease
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
        {
            return false;
        }
    }

    true
}

/// Check if a winget package ID is valid
///
/// Winget IDs typically follow the format: Publisher.PackageName
/// Examples:
/// - Microsoft.VisualStudioCode
/// - Git.Git
/// - Python.Python.3.12
///   Check if a Microsoft Store package ID is valid
///   Microsoft Store IDs are alphanumeric, typically 12 characters (e.g., 9NBLGGH4NNS1)
fn is_valid_msstore_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }

    // Microsoft Store IDs are alphanumeric and typically 12 characters
    // They usually start with '9' but we'll be lenient and just check alphanumeric
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }

    // Typical MS Store IDs are 12 characters, but allow some flexibility (8-14 chars)
    let len = id.len();
    if !(8..=14).contains(&len) {
        return false;
    }

    true
}

fn is_valid_winget_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }

    // Should contain at least one dot
    if !id.contains('.') {
        return false;
    }

    // Should not start or end with a dot
    if id.starts_with('.') || id.ends_with('.') {
        return false;
    }

    // Should not have consecutive dots
    if id.contains("..") {
        return false;
    }

    // Each segment should be non-empty and contain valid characters
    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2 {
        return false;
    }

    for segment in segments {
        if segment.is_empty() {
            return false;
        }
        // Allow alphanumeric, underscores, and hyphens in segments
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return false;
        }
    }

    true
}

/// Check if an environment variable name is valid
fn is_valid_env_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // First character should be a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Rest should be alphanumeric or underscore
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    // Workload name tests
    #[test]
    fn test_valid_workload_names() {
        assert!(is_valid_workload_name("rust-developer"));
        assert!(is_valid_workload_name("essentials"));
        assert!(is_valid_workload_name("python3"));
        assert!(is_valid_workload_name("a"));
        assert!(is_valid_workload_name("my-workload-123"));
    }

    #[test]
    fn test_invalid_workload_names() {
        assert!(!is_valid_workload_name(""));
        assert!(!is_valid_workload_name("123"));
        assert!(!is_valid_workload_name("-invalid"));
        assert!(!is_valid_workload_name("Invalid"));
        assert!(!is_valid_workload_name("has_underscore"));
        assert!(!is_valid_workload_name("has space"));
        assert!(!is_valid_workload_name("UPPERCASE"));
        assert!(!is_valid_workload_name("CamelCase"));
    }

    // Version tests
    #[test]
    fn test_valid_versions() {
        assert!(is_valid_version("1.0.0"));
        assert!(is_valid_version("0.1.0"));
        assert!(is_valid_version("10.20.30"));
        assert!(is_valid_version("1.0.0-beta"));
        assert!(is_valid_version("1.0.0-alpha.1"));
        assert!(is_valid_version("2.0.0-rc1"));
    }

    #[test]
    fn test_invalid_versions() {
        assert!(!is_valid_version(""));
        assert!(!is_valid_version("1.0"));
        assert!(!is_valid_version("1"));
        assert!(!is_valid_version("1.0.0.0"));
        assert!(!is_valid_version("v1.0.0"));
        assert!(!is_valid_version("1.0.0-"));
        assert!(!is_valid_version("a.b.c"));
    }

    // Winget ID tests
    #[test]
    fn test_valid_winget_ids() {
        assert!(is_valid_winget_id("Microsoft.VisualStudioCode"));
        assert!(is_valid_winget_id("Git.Git"));
        assert!(is_valid_winget_id("Python.Python.3"));
        assert!(is_valid_winget_id("Publisher.Package-Name"));
        assert!(is_valid_winget_id("Rustlang.Rustup"));
        assert!(is_valid_winget_id("JanDeDobbeleer.OhMyPosh"));
    }

    #[test]
    fn test_invalid_winget_ids() {
        assert!(!is_valid_winget_id(""));
        assert!(!is_valid_winget_id("NoDotsHere"));
        assert!(!is_valid_winget_id(".StartsWithDot"));
        assert!(!is_valid_winget_id("EndsWithDot."));
        assert!(!is_valid_winget_id("Has..DoubleDots"));
        assert!(!is_valid_winget_id("Has Spaces.Package"));
    }

    // Microsoft Store ID tests
    #[test]
    fn test_valid_msstore_ids() {
        assert!(is_valid_msstore_id("9PFXXSHC64H3")); // Raycast
        assert!(is_valid_msstore_id("9NBLGGH4NNS1")); // Typical MS Store ID
        assert!(is_valid_msstore_id("9N0DX20HK701")); // Windows Terminal
        assert!(is_valid_msstore_id("XPFFZHVGQWWLHB")); // Some MS Store apps have longer IDs
    }

    #[test]
    fn test_invalid_msstore_ids() {
        assert!(!is_valid_msstore_id(""));
        assert!(!is_valid_msstore_id("short")); // Too short (< 8 chars)
        assert!(!is_valid_msstore_id("9NBLGGH4NNS1TOOLONG")); // Too long (> 14 chars)
        assert!(!is_valid_msstore_id("Microsoft.App")); // This is a winget ID, not msstore
        assert!(!is_valid_msstore_id("9NBLGGH-NNS1")); // Contains hyphen
        assert!(!is_valid_msstore_id("9NBLGGH NNS1")); // Contains space
    }

    // Environment variable name tests
    #[test]
    fn test_valid_env_var_names() {
        assert!(is_valid_env_var_name("PATH"));
        assert!(is_valid_env_var_name("MY_VAR"));
        assert!(is_valid_env_var_name("_PRIVATE"));
        assert!(is_valid_env_var_name("VAR123"));
        assert!(is_valid_env_var_name("RUST_BACKTRACE"));
    }

    #[test]
    fn test_invalid_env_var_names() {
        assert!(!is_valid_env_var_name(""));
        assert!(!is_valid_env_var_name("123VAR"));
        assert!(!is_valid_env_var_name("VAR-NAME"));
        assert!(!is_valid_env_var_name("VAR NAME"));
    }

    // ValidationResult tests
    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(result.is_perfect());

        result.add_warning("test", "warning message");
        assert!(result.is_valid());
        assert!(!result.is_perfect());
        assert_eq!(result.warning_count(), 1);

        result.add_error("test", "error message");
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_error("path1", "error 1");

        let mut result2 = ValidationResult::new();
        result2.add_warning("path2", "warning 1");
        result2.add_error("path3", "error 2");

        result1.merge(result2);
        assert_eq!(result1.error_count(), 2);
        assert_eq!(result1.warning_count(), 1);
        assert_eq!(result1.messages.len(), 3);
    }

    // SchemaValidator tests
    #[test]
    fn test_validate_minimal_workload() {
        let workload = Workload::new("test-workload", "1.0.0", "A test workload");
        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_empty_name() {
        let workload = Workload::new("", "1.0.0", "A test workload");
        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(!result.is_valid());
        assert!(result.errors().any(|e| e.path == "name"));
    }

    #[test]
    fn test_validate_invalid_name() {
        let workload = Workload::new("Invalid-Name", "1.0.0", "A test workload");
        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_missing_version_strict() {
        let workload = Workload::new("test-workload", "", "A test workload");
        let validator = SchemaValidator::strict();
        let result = validator.validate(&workload);
        assert!(!result.is_valid()); // Strict mode makes missing version an error
    }

    #[test]
    fn test_validate_missing_version_normal() {
        let workload = Workload::new("test-workload", "", "A test workload");
        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(result.is_valid()); // Normal mode only warns
        assert!(result.warning_count() > 0);
    }

    #[test]
    fn test_validate_invalid_version_format() {
        let workload = Workload::new("test-workload", "1.0", "A test workload");
        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(result.is_valid()); // Invalid format is just a warning
        assert!(result.warning_count() > 0);
    }
}
