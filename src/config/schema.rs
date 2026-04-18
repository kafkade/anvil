//! Schema validation module for Anvil workload definitions
//!
//! This module provides validation logic for workload YAML files,
//! ensuring they conform to the expected schema and contain valid values.
use super::workload::{Packages, Workload};

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

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.messages.extend(other.messages);
    }
}

#[cfg(test)]
impl ValidationResult {
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
}

/// Schema validator for workload definitions
pub struct SchemaValidator {
    /// Enable strict validation mode
    strict: bool,
}
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

        // Validate commands
        self.validate_commands(workload, &mut result);

        // Validate environment
        self.validate_environment(workload, &mut result);

        // Validate assertions
        self.validate_assertions(workload, &mut result);

        result
    }

    /// Check raw YAML content for removed scripts fields
    pub fn check_removed_scripts_fields(content: &str, result: &mut ValidationResult) {
        // Parse as serde_yaml::Value to detect removed fields
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
            if let Some(scripts) = value.get("scripts") {
                if scripts.get("health_check").is_some() {
                    result.add_error(
                        "scripts.health_check",
                        "scripts.health_check has been removed in v1.0. Use declarative assertions instead. See https://anvil.kafkade.com/docs/workload-authoring.html",
                    );
                }
                if scripts.get("pre_install").is_some() || scripts.get("post_install").is_some() {
                    result.add_error(
                        "scripts",
                        "scripts.pre_install and scripts.post_install have been removed in v1.0. Use the commands block instead. See https://anvil.kafkade.com/docs/workload-authoring.html",
                    );
                }
            }
        }
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

            // Validate brew packages
            if let Some(brew_packages) = &packages.brew {
                for (i, package) in brew_packages.iter().enumerate() {
                    let path = format!("packages.brew[{}]", i);

                    if package.name.is_empty() {
                        result.add_error(format!("{}.name", path), "Package name is required");
                    }

                    // Validate tap format if provided (must be "owner/repo")
                    if let Some(ref tap) = package.tap {
                        if !is_valid_brew_tap(tap) {
                            result.add_warning(
                                format!("{}.tap", path),
                                format!(
                                    "Tap '{}' may not be valid. Expected format: 'owner/repo'",
                                    tap
                                ),
                            );
                        }
                    }
                }
            }

            // Validate apt packages
            if let Some(apt_packages) = &packages.apt {
                for (i, package) in apt_packages.iter().enumerate() {
                    let path = format!("packages.apt[{}]", i);

                    if package.name.is_empty() {
                        result.add_error(format!("{}.name", path), "Package name is required");
                    }

                    // Validate version string if provided
                    if let Some(ref version) = package.version {
                        if version.is_empty() {
                            result.add_warning(
                                format!("{}.version", path),
                                "Version string is empty, will install latest",
                            );
                        }
                    }
                }
            }

            // Check platform availability
            self.validate_manager_availability(packages, result);
        }
    }

    /// Warn if workload references package managers not available on the current platform
    fn validate_manager_availability(&self, packages: &Packages, result: &mut ValidationResult) {
        if cfg!(target_os = "windows") {
            if packages
                .brew
                .as_ref()
                .map(|b| !b.is_empty())
                .unwrap_or(false)
            {
                result.add_warning(
                    "packages.brew",
                    "Homebrew packages defined but Homebrew is not available on Windows",
                );
            }
            if packages
                .apt
                .as_ref()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
            {
                result.add_warning(
                    "packages.apt",
                    "APT packages defined but APT is not available on Windows",
                );
            }
        }
        if !cfg!(target_os = "windows")
            && packages
                .winget
                .as_ref()
                .map(|w| !w.is_empty())
                .unwrap_or(false)
        {
            result.add_warning(
                "packages.winget",
                "Winget packages defined but winget is only available on Windows",
            );
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

    /// Validate command definitions
    fn validate_commands(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(commands) = &workload.commands {
            // Validate pre-install commands
            if let Some(pre_cmds) = &commands.pre_install {
                for (i, cmd) in pre_cmds.iter().enumerate() {
                    let path = format!("commands.pre_install[{}]", i);
                    self.validate_command_entry(&path, cmd, result);
                }
            }

            // Validate post-install commands
            if let Some(post_cmds) = &commands.post_install {
                for (i, cmd) in post_cmds.iter().enumerate() {
                    let path = format!("commands.post_install[{}]", i);
                    self.validate_command_entry(&path, cmd, result);
                }
            }
        }
    }

    /// Validate a single command entry
    fn validate_command_entry(
        &self,
        path: &str,
        cmd: &super::workload::CommandEntry,
        result: &mut ValidationResult,
    ) {
        // run is required and must be non-empty
        if cmd.run.is_empty() {
            result.add_error(format!("{}.run", path), "Command string is required");
        }

        // timeout must be > 0
        if cmd.timeout == 0 {
            result.add_error(
                format!("{}.timeout", path),
                "Timeout must be greater than 0",
            );
        } else if cmd.timeout > 3600 {
            result.add_warning(
                format!("{}.timeout", path),
                format!(
                    "Timeout of {} seconds ({:.1} hours) is very long. Consider if this is intentional.",
                    cmd.timeout,
                    cmd.timeout as f64 / 3600.0
                ),
            );
        }

        // Warn about elevated commands in strict mode
        if cmd.elevated && self.strict {
            result.add_info(
                format!("{}.elevated", path),
                "Command requires elevated privileges. Ensure this is necessary.",
            );
        }

        // Validate the when condition if present
        if let Some(condition) = &cmd.when {
            self.validate_condition(&format!("{}.when", path), condition, result);
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

    /// Validate assertion definitions
    fn validate_assertions(&self, workload: &Workload, result: &mut ValidationResult) {
        if let Some(assertions) = &workload.assertions {
            for (i, assertion) in assertions.iter().enumerate() {
                let path = format!("assertions[{}]", i);

                // Name is required
                if assertion.name.is_empty() {
                    result.add_error(format!("{}.name", path), "Assertion name is required");
                }

                // Validate the condition structure
                self.validate_condition(&format!("{}.check", path), &assertion.check, result);
            }
        }
    }

    /// Validate a condition is structurally valid
    fn validate_condition(
        &self,
        path: &str,
        condition: &crate::conditions::Condition,
        result: &mut ValidationResult,
    ) {
        use crate::conditions::Condition;

        match condition {
            Condition::CommandExists { command } => {
                if command.is_empty() {
                    result.add_error(format!("{}.command", path), "Command cannot be empty");
                }
            }
            Condition::FileExists { path: file_path } => {
                if file_path.is_empty() {
                    result.add_error(format!("{}.path", path), "File path cannot be empty");
                }
            }
            Condition::DirExists { path: dir_path } => {
                if dir_path.is_empty() {
                    result.add_error(format!("{}.path", path), "Directory path cannot be empty");
                }
            }
            Condition::EnvVar { name, .. } => {
                if name.is_empty() {
                    result.add_error(
                        format!("{}.name", path),
                        "Environment variable name cannot be empty",
                    );
                }
            }
            Condition::PathContains { substring } => {
                if substring.is_empty() {
                    result.add_error(
                        format!("{}.substring", path),
                        "PATH substring cannot be empty",
                    );
                }
            }
            Condition::RegistryValue {
                hive, key, name, ..
            } => {
                if hive.is_empty() {
                    result.add_error(format!("{}.hive", path), "Registry hive cannot be empty");
                } else {
                    let valid_hives = ["HKCU", "HKLM"];
                    if !valid_hives.contains(&hive.as_str()) {
                        result.add_warning(
                            format!("{}.hive", path),
                            format!(
                                "Unknown registry hive '{}'. Expected one of: {:?}",
                                hive, valid_hives
                            ),
                        );
                    }
                }
                if key.is_empty() {
                    result.add_error(format!("{}.key", path), "Registry key cannot be empty");
                }
                if name.is_empty() {
                    result.add_error(
                        format!("{}.name", path),
                        "Registry value name cannot be empty",
                    );
                }
            }
            Condition::Shell { command, .. } => {
                if command.is_empty() {
                    result.add_error(format!("{}.command", path), "Shell command cannot be empty");
                }
            }
            Condition::AllOf { conditions } => {
                if conditions.is_empty() {
                    result.add_warning(path.to_string(), "all_of has no conditions");
                }
                for (j, cond) in conditions.iter().enumerate() {
                    self.validate_condition(&format!("{}[{}]", path, j), cond, result);
                }
            }
            Condition::AnyOf { conditions } => {
                if conditions.is_empty() {
                    result.add_warning(path.to_string(), "any_of has no conditions");
                }
                for (j, cond) in conditions.iter().enumerate() {
                    self.validate_condition(&format!("{}[{}]", path, j), cond, result);
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

/// Check if a Homebrew tap identifier is valid (format: "owner/repo")
fn is_valid_brew_tap(tap: &str) -> bool {
    if tap.is_empty() {
        return false;
    }
    let parts: Vec<&str> = tap.split('/').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::workload::{AptPackage, BrewPackage};

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

    #[test]
    fn test_validate_brew_packages_valid() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: Some(vec![
                BrewPackage {
                    name: "git".to_string(),
                    cask: false,
                    tap: None,
                },
                BrewPackage {
                    name: "visual-studio-code".to_string(),
                    cask: true,
                    tap: Some("homebrew/cask".to_string()),
                },
            ]),
            apt: None,
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        // No errors expected (warnings about platform availability are ok)
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_brew_empty_name() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: Some(vec![BrewPackage {
                name: "".to_string(),
                cask: false,
                tap: None,
            }]),
            apt: None,
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(!result.is_valid());
        assert!(result
            .errors()
            .any(|e| e.path.contains("brew") && e.message.contains("name")));
    }

    #[test]
    fn test_validate_brew_invalid_tap() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: Some(vec![BrewPackage {
                name: "font-fira-code".to_string(),
                cask: true,
                tap: Some("invalid-tap".to_string()),
            }]),
            apt: None,
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(result.is_valid()); // Invalid tap is a warning, not error
        assert!(result
            .warnings()
            .any(|w| w.path.contains("tap") && w.message.contains("owner/repo")));
    }

    #[test]
    fn test_validate_apt_packages_valid() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: None,
            apt: Some(vec![
                AptPackage {
                    name: "git".to_string(),
                    version: None,
                },
                AptPackage {
                    name: "build-essential".to_string(),
                    version: Some("12.9".to_string()),
                },
            ]),
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_apt_empty_name() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: None,
            apt: Some(vec![AptPackage {
                name: "".to_string(),
                version: None,
            }]),
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);
        assert!(!result.is_valid());
        assert!(result
            .errors()
            .any(|e| e.path.contains("apt") && e.message.contains("name")));
    }

    #[test]
    fn test_validate_manager_availability_on_windows() {
        let mut workload = Workload::new("test-workload", "1.0.0", "A test workload");
        workload.packages = Some(Packages {
            winget: None,
            brew: Some(vec![BrewPackage {
                name: "git".to_string(),
                cask: false,
                tap: None,
            }]),
            apt: Some(vec![AptPackage {
                name: "git".to_string(),
                version: None,
            }]),
        });

        let validator = SchemaValidator::new();
        let result = validator.validate(&workload);

        if cfg!(target_os = "windows") {
            assert!(result.warnings().any(
                |w| w.path == "packages.brew" && w.message.contains("not available on Windows")
            ));
            assert!(result.warnings().any(
                |w| w.path == "packages.apt" && w.message.contains("not available on Windows")
            ));
        }
    }

    #[test]
    fn test_validate_brew_tap_format() {
        assert!(is_valid_brew_tap("homebrew/cask"));
        assert!(is_valid_brew_tap("homebrew/cask-fonts"));
        assert!(is_valid_brew_tap("user/repo"));
        assert!(!is_valid_brew_tap(""));
        assert!(!is_valid_brew_tap("invalid"));
        assert!(!is_valid_brew_tap("/empty-owner"));
        assert!(!is_valid_brew_tap("empty-repo/"));
    }
}
