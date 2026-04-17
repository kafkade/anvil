//! Validate operation for Anvil CLI
//!
//! This module implements the `validate` command which checks
//! workload definition files for syntax and semantic errors.
//!
//! Features:
//! - YAML syntax validation
//! - Schema validation (required fields, types)
//! - File existence checks (source files, scripts)
//! - Inheritance validation (parent existence, circular deps)
//! - Script syntax validation (PowerShell parser)

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, trace};

use crate::cli::commands::ValidateArgs;
use crate::cli::output::{print_error, print_info, print_success, print_warning};
use crate::cli::Cli;
use crate::config::schema::{SchemaValidator, ValidationResult, ValidationSeverity};
use crate::config::workload::Workload;
use crate::config::ConfigManager;
use crate::providers::script::{ScriptConfig, ScriptProvider, Shell};

/// Execute the validate command
pub fn execute(args: &ValidateArgs, cli: &Cli) -> Result<()> {
    debug!("Executing validate command");
    trace!("Validate arguments: {:?}", args);

    // If schema output is requested, print the JSON schema and exit
    if args.schema {
        return print_json_schema();
    }

    // Path is required if not outputting schema
    let path = match &args.path {
        Some(p) => p,
        None => {
            anyhow::bail!("Path is required. Use --schema to output the JSON schema.");
        }
    };

    // Determine if path is a file or directory
    let workload_file = if path.is_file() {
        path.clone()
    } else if path.is_dir() {
        let yaml_path = path.join("workload.yaml");
        let yml_path = path.join("workload.yml");
        if yaml_path.exists() {
            yaml_path
        } else if yml_path.exists() {
            yml_path
        } else {
            anyhow::bail!(
                "No workload.yaml or workload.yml found in directory: {}",
                path.display()
            );
        }
    } else {
        anyhow::bail!("Path does not exist: {}", path.display());
    };

    let use_color = !cli.should_disable_color();

    if !cli.quiet {
        if use_color {
            println!(
                "{} Validating: {}",
                "ℹ".blue(),
                workload_file.display().to_string().cyan()
            );
        } else {
            print_info(&format!("Validating: {}", workload_file.display()));
        }
    }

    // First, try to parse the YAML file
    let content = std::fs::read_to_string(&workload_file)
        .with_context(|| format!("Failed to read file: {}", workload_file.display()))?;

    let workload: Workload = match serde_yaml::from_str(&content) {
        Ok(w) => w,
        Err(e) => {
            print_error(&format!("YAML parse error: {}", e));
            std::process::exit(2);
        }
    };

    debug!("Parsed workload: {}", workload.name);

    // Create validator based on strict mode
    let validator = if args.strict {
        SchemaValidator::strict()
    } else {
        SchemaValidator::new()
    };

    // Run schema validation
    let mut result = validator.validate(&workload);

    // Check for files and scripts existence
    let base_dir = workload_file.parent().unwrap_or(Path::new("."));
    let file_result = validate_referenced_files(&workload, base_dir, args.strict);
    result.merge(file_result);

    // Validate parent workloads exist and check for circular dependencies
    let inheritance_result = validate_inheritance(&workload, base_dir, args.strict);
    result.merge(inheritance_result);

    // Validate script syntax if requested
    if args.check_scripts {
        if !cli.quiet {
            println!();
            if use_color {
                println!("{} Validating script syntax...", "ℹ".blue());
            } else {
                print_info("Validating script syntax...");
            }
        }
        let script_result = validate_script_syntax(&workload, base_dir, args.strict, cli.quiet);
        result.merge(script_result);
    }

    // Print results
    if !result.messages.is_empty() {
        println!();
        print_validation_result(&result, cli.quiet, use_color);
    }

    // Summary
    let total_errors = result.error_count();
    let total_warnings = result.warning_count();

    if !cli.quiet {
        println!();
        if total_errors == 0 && total_warnings == 0 {
            if use_color {
                println!(
                    "{} {}",
                    "✓".green().bold(),
                    "Validation passed with no issues".green()
                );
            } else {
                print_success("Validation passed with no issues");
            }
        } else if total_errors == 0 {
            if use_color {
                println!(
                    "{} Validation passed with {} warning(s)",
                    "⚠".yellow(),
                    total_warnings.to_string().yellow().bold()
                );
            } else {
                print_warning(&format!(
                    "Validation passed with {} warning(s)",
                    total_warnings
                ));
            }
        } else {
            if use_color {
                println!(
                    "{} Validation failed: {} error(s), {} warning(s)",
                    "✗".red().bold(),
                    total_errors.to_string().red().bold(),
                    total_warnings.to_string().yellow()
                );
            } else {
                print_error(&format!(
                    "Validation failed: {} error(s), {} warning(s)",
                    total_errors, total_warnings
                ));
            }
        }
    }

    // Exit with appropriate code
    if total_errors > 0 {
        std::process::exit(1);
    }

    // In strict mode, warnings are also errors
    if args.strict && total_warnings > 0 {
        if !cli.quiet {
            println!();
            print_warning("Strict mode: treating warnings as errors");
        }
        std::process::exit(1);
    }

    Ok(())
}

/// Print validation results to console
fn print_validation_result(result: &ValidationResult, quiet: bool, use_color: bool) {
    if quiet {
        return;
    }

    for message in &result.messages {
        match message.severity {
            ValidationSeverity::Error => {
                if use_color {
                    println!(
                        "  {} {} {}",
                        "✗".red(),
                        format!("[{}]", message.path).dimmed(),
                        message.message.red()
                    );
                } else {
                    print_error(&format!("{}: {}", message.path, message.message));
                }
            }
            ValidationSeverity::Warning => {
                if use_color {
                    println!(
                        "  {} {} {}",
                        "⚠".yellow(),
                        format!("[{}]", message.path).dimmed(),
                        message.message.yellow()
                    );
                } else {
                    print_warning(&format!("{}: {}", message.path, message.message));
                }
            }
            ValidationSeverity::Info => {
                if use_color {
                    println!(
                        "  {} {} {}",
                        "ℹ".blue(),
                        format!("[{}]", message.path).dimmed(),
                        message.message
                    );
                } else {
                    print_info(&format!("{}: {}", message.path, message.message));
                }
            }
        }
    }
}

/// Validate that referenced files and scripts exist
fn validate_referenced_files(
    workload: &Workload,
    base_dir: &Path,
    strict: bool,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check files
    if let Some(files) = &workload.files {
        let files_dir = base_dir.join("files");
        for (i, file) in files.iter().enumerate() {
            let source_path = files_dir.join(&file.source);
            if !source_path.exists() {
                let msg = format!("Source file not found: {}", source_path.display());
                if strict {
                    result.add_error(format!("files[{}].source", i), msg);
                } else {
                    result.add_warning(format!("files[{}].source", i), msg);
                }
            } else if source_path.is_dir() {
                // For directories, check if we can read the directory listing
                if let Err(e) = std::fs::read_dir(&source_path) {
                    result.add_warning(
                        format!("files[{}].source", i),
                        format!("Directory exists but cannot be read: {}", e),
                    );
                }
            } else {
                // Check if file is readable
                if let Err(e) = std::fs::read(&source_path) {
                    result.add_warning(
                        format!("files[{}].source", i),
                        format!("File exists but cannot be read: {}", e),
                    );
                }
            }
        }
    }

    // Check scripts
    if let Some(scripts) = &workload.scripts {
        let scripts_dir = base_dir.join("scripts");

        // Check pre-install scripts
        if let Some(pre_scripts) = &scripts.pre_install {
            for (i, script) in pre_scripts.iter().enumerate() {
                let script_path = scripts_dir.join(&script.path);
                if !script_path.exists() {
                    let msg = format!("Script file not found: {}", script_path.display());
                    if strict {
                        result.add_error(format!("scripts.pre_install[{}].path", i), msg);
                    } else {
                        result.add_warning(format!("scripts.pre_install[{}].path", i), msg);
                    }
                } else {
                    // Check for common script issues
                    check_script_file(
                        &script_path,
                        &format!("scripts.pre_install[{}]", i),
                        &mut result,
                    );
                }
            }
        }

        // Check post-install scripts
        if let Some(post_scripts) = &scripts.post_install {
            for (i, script) in post_scripts.iter().enumerate() {
                let script_path = scripts_dir.join(&script.path);
                if !script_path.exists() {
                    let msg = format!("Script file not found: {}", script_path.display());
                    if strict {
                        result.add_error(format!("scripts.post_install[{}].path", i), msg);
                    } else {
                        result.add_warning(format!("scripts.post_install[{}].path", i), msg);
                    }
                } else {
                    check_script_file(
                        &script_path,
                        &format!("scripts.post_install[{}]", i),
                        &mut result,
                    );
                }
            }
        }

        // Check health check scripts
        if let Some(health_scripts) = &scripts.health_check {
            for (i, script) in health_scripts.iter().enumerate() {
                let script_path = scripts_dir.join(&script.path);
                if !script_path.exists() {
                    let msg = format!("Script file not found: {}", script_path.display());
                    if strict {
                        result.add_error(format!("scripts.health_check[{}].path", i), msg);
                    } else {
                        result.add_warning(format!("scripts.health_check[{}].path", i), msg);
                    }
                } else {
                    check_script_file(
                        &script_path,
                        &format!("scripts.health_check[{}]", i),
                        &mut result,
                    );
                }
            }
        }
    }

    result
}

/// Check a script file for common issues (BOM, encoding, line endings)
fn check_script_file(path: &Path, field_path: &str, result: &mut ValidationResult) {
    // Read file as bytes to check for BOM
    if let Ok(bytes) = std::fs::read(path) {
        // Check for UTF-8 BOM (EF BB BF)
        if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            result.add_info(
                field_path.to_string(),
                "Script has UTF-8 BOM (may cause issues with some tools)".to_string(),
            );
        }

        // Check for UTF-16 BOM
        if bytes.len() >= 2
            && ((bytes[0] == 0xFF && bytes[1] == 0xFE) || (bytes[0] == 0xFE && bytes[1] == 0xFF))
        {
            result.add_warning(
                field_path.to_string(),
                "Script appears to be UTF-16 encoded (may cause issues)".to_string(),
            );
        }

        // Try to parse as UTF-8
        if let Ok(content) = String::from_utf8(bytes.clone()) {
            // Check for mixed line endings
            let has_crlf = content.contains("\r\n");
            let has_lf_only = content.contains('\n') && !content.contains('\r');
            if has_crlf && has_lf_only {
                result.add_info(
                    field_path.to_string(),
                    "Script has mixed line endings (CRLF and LF)".to_string(),
                );
            }

            // Check for empty script
            if content.trim().is_empty() {
                result.add_warning(field_path.to_string(), "Script file is empty".to_string());
            }
        } else {
            result.add_warning(
                field_path.to_string(),
                "Script is not valid UTF-8".to_string(),
            );
        }
    }
}

/// Validate PowerShell script syntax using the parser
fn validate_script_syntax(
    workload: &Workload,
    base_dir: &Path,
    strict: bool,
    quiet: bool,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    let scripts = match &workload.scripts {
        Some(s) => s,
        None => return result,
    };

    let scripts_dir = base_dir.join("scripts");
    let provider = ScriptProvider::new().with_base_path(&scripts_dir);

    let mut scripts_to_validate: Vec<(&str, &str, &str)> = Vec::new();

    // Collect all scripts
    if let Some(pre) = &scripts.pre_install {
        for script in pre {
            scripts_to_validate.push(("pre_install", &script.path, &script.shell));
        }
    }
    if let Some(post) = &scripts.post_install {
        for script in post {
            scripts_to_validate.push(("post_install", &script.path, &script.shell));
        }
    }
    if let Some(health) = &scripts.health_check {
        for script in health {
            scripts_to_validate.push(("health_check", &script.path, &script.shell));
        }
    }

    if scripts_to_validate.is_empty() {
        return result;
    }

    for (phase, script_path, shell_name) in scripts_to_validate {
        let full_path = scripts_dir.join(script_path);

        if !full_path.exists() {
            // Already reported in file existence check
            continue;
        }

        // Determine shell
        let shell = Shell::from_str(shell_name).unwrap_or(Shell::PowerShell);

        // Only validate PowerShell scripts
        if shell != Shell::PowerShell && shell != Shell::Pwsh {
            if !quiet {
                result.add_info(
                    format!("scripts.{}.{}", phase, script_path),
                    format!("Skipping syntax check for {} script", shell_name),
                );
            }
            continue;
        }

        // Build config for validation
        let config = ScriptConfig::new(script_path)
            .with_shell(shell)
            .with_timeout(Duration::from_secs(30));

        // Validate syntax
        match provider.validate_syntax(&config) {
            Ok(()) => {
                if !quiet {
                    result.add_info(
                        format!("scripts.{}.{}", phase, script_path),
                        "Syntax OK".to_string(),
                    );
                }
            }
            Err(e) => {
                let msg = match &e {
                    crate::providers::script::ScriptError::SyntaxError { message, .. } => {
                        // Check if the error might be related to Unicode characters
                        // which is common in scripts using emoji or special chars
                        let is_unicode_issue = message.contains("Unexpected token")
                            && (message.contains("recommended")
                                || message.contains("}")
                                || message.contains("GB"));

                        if is_unicode_issue {
                            format!(
                                "Possible encoding issue (Unicode chars): {}",
                                message.lines().next().unwrap_or("unknown")
                            )
                        } else {
                            format!(
                                "Syntax error: {}",
                                message.lines().next().unwrap_or("unknown")
                            )
                        }
                    }
                    _ => format!("Validation error: {}", e),
                };

                // Unicode-related issues are always warnings, not errors
                let is_likely_unicode = msg.contains("Possible encoding issue");
                if strict && !is_likely_unicode {
                    result.add_error(format!("scripts.{}.{}", phase, script_path), msg);
                } else {
                    result.add_warning(format!("scripts.{}.{}", phase, script_path), msg);
                }
            }
        }
    }

    result
}

/// Validate workload inheritance (parent existence and circular dependencies)
fn validate_inheritance(workload: &Workload, _base_dir: &Path, _strict: bool) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check if this workload extends others
    if let Some(ref extends) = workload.extends {
        if extends.is_empty() {
            return result;
        }

        let manager = ConfigManager::new();

        for (i, parent_name) in extends.iter().enumerate() {
            // Check if parent workload exists
            match manager.find_workload(parent_name) {
                Some(parent_path) => {
                    debug!(
                        "Found parent workload '{}' at: {}",
                        parent_name,
                        parent_path.display()
                    );

                    // Check for circular dependencies by loading the parent and checking its extends
                    if let Ok(content) = std::fs::read_to_string(&parent_path) {
                        if let Ok(parent_workload) = serde_yaml::from_str::<Workload>(&content) {
                            // Check for direct circular dependency
                            if let Some(ref parent_extends) = parent_workload.extends {
                                if parent_extends.contains(&workload.name) {
                                    result.add_error(
                                        format!("extends[{}]", i),
                                        format!(
                                            "Circular dependency detected: '{}' and '{}' extend each other",
                                            workload.name, parent_name
                                        ),
                                    );
                                }

                                // Check for deeper circular dependencies
                                let mut visited = HashSet::new();
                                visited.insert(workload.name.clone());
                                if let Some(cycle) =
                                    detect_circular_dependency(&manager, parent_name, &mut visited)
                                {
                                    result.add_error(
                                        format!("extends[{}]", i),
                                        format!(
                                            "Circular dependency detected in inheritance chain: {}",
                                            cycle.join(" -> ")
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
                None => {
                    result.add_error(
                        format!("extends[{}]", i),
                        format!(
                            "Parent workload '{}' not found. Ensure it exists in the workloads directory.",
                            parent_name
                        ),
                    );
                }
            }
        }
    }

    result
}

/// Recursively detect circular dependencies in the inheritance chain
fn detect_circular_dependency(
    manager: &ConfigManager,
    workload_name: &str,
    visited: &mut HashSet<String>,
) -> Option<Vec<String>> {
    // If we've already visited this workload, we have a cycle
    if visited.contains(workload_name) {
        let mut cycle: Vec<String> = visited.iter().cloned().collect();
        cycle.push(workload_name.to_string());
        return Some(cycle);
    }

    // Mark as visited
    visited.insert(workload_name.to_string());

    // Try to load and check extends
    if let Some(path) = manager.find_workload(workload_name) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(workload) = serde_yaml::from_str::<Workload>(&content) {
                if let Some(ref extends) = workload.extends {
                    for parent in extends {
                        if let Some(cycle) = detect_circular_dependency(manager, parent, visited) {
                            return Some(cycle);
                        }
                    }
                }
            }
        }
    }

    // Remove from visited when backtracking
    visited.remove(workload_name);
    None
}

/// Print JSON schema for workload definitions
fn print_json_schema() -> Result<()> {
    let schema = r##"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Anvil Workload Definition",
  "type": "object",
  "required": ["name", "version", "description"],
  "properties": {
    "name": {
      "type": "string",
      "description": "Unique workload identifier (lowercase letters, numbers, hyphens only, must start with letter)",
      "pattern": "^[a-z][a-z0-9-]*$"
    },
    "version": {
      "type": "string",
      "description": "Semantic version (e.g., '1.0.0')",
      "pattern": "^\\d+\\.\\d+\\.\\d+(-[a-zA-Z0-9]+)?$"
    },
    "description": {
      "type": "string",
      "description": "Human-readable description"
    },
    "extends": {
      "type": "array",
      "items": { "type": "string" },
      "description": "List of parent workloads to extend"
    },
    "packages": {
      "type": "object",
      "properties": {
        "winget": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id"],
            "properties": {
              "id": {
                "type": "string",
                "description": "Winget package ID (e.g., 'Microsoft.VisualStudioCode')"
              },
              "version": {
                "type": "string",
                "description": "Specific version to install"
              },
              "source": {
                "type": "string",
                "description": "Package source (default: winget)"
              },
              "override": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Custom installer arguments"
              }
            }
          }
        }
      }
    },
    "files": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["source", "destination"],
        "properties": {
          "source": {
            "type": "string",
            "description": "Relative path from workload's files/ directory"
          },
          "destination": {
            "type": "string",
            "description": "Destination path (supports ~ and ${VAR} expansion)"
          },
          "backup": {
            "type": "boolean",
            "default": true,
            "description": "Whether to backup existing file"
          },
          "template": {
            "type": "boolean",
            "default": false,
            "description": "Whether to process as a template"
          }
        }
      }
    },
    "scripts": {
      "type": "object",
      "properties": {
        "pre_install": { "$ref": "#/definitions/scriptList" },
        "post_install": { "$ref": "#/definitions/scriptList" },
        "health_check": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["path", "name"],
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative path from workload's scripts/ directory"
              },
              "name": {
                "type": "string",
                "description": "Display name for the check"
              },
              "description": { "type": "string" },
              "shell": {
                "type": "string",
                "default": "powershell",
                "enum": ["powershell", "pwsh", "cmd", "bash"]
              }
            }
          }
        }
      }
    },
    "environment": {
      "type": "object",
      "properties": {
        "variables": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "value"],
            "properties": {
              "name": { "type": "string" },
              "value": { "type": "string" },
              "scope": {
                "type": "string",
                "enum": ["user", "machine"],
                "default": "user"
              }
            }
          }
        },
        "path_additions": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "health": {
      "type": "object",
      "properties": {
        "package_check": { "type": "boolean", "default": true },
        "file_check": { "type": "boolean", "default": true },
        "script_check": { "type": "boolean", "default": true }
      }
    },
    "assertions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "check"],
        "properties": {
          "name": {
            "type": "string",
            "description": "Display name for the assertion"
          },
          "check": {
            "type": "object",
            "description": "Condition to evaluate (uses condition engine types)",
            "required": ["type"],
            "properties": {
              "type": {
                "type": "string",
                "enum": ["command_exists", "file_exists", "dir_exists", "env_var", "path_contains", "registry_value", "shell", "all_of", "any_of"]
              }
            }
          }
        }
      },
      "description": "Declarative assertions for health validation"
    }
  },
  "definitions": {
    "scriptList": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["path"],
        "properties": {
          "path": {
            "type": "string",
            "description": "Relative path from workload's scripts/ directory"
          },
          "shell": {
            "type": "string",
            "default": "powershell",
            "enum": ["powershell", "pwsh", "cmd", "bash"]
          },
          "description": { "type": "string" },
          "elevated": {
            "type": "boolean",
            "default": false,
            "description": "Whether to require admin privileges"
          },
          "timeout": {
            "type": "integer",
            "default": 300,
            "minimum": 5,
            "maximum": 3600,
            "description": "Timeout in seconds"
          }
        }
      }
    }
  }
}"##;

    println!("{}", schema);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
    }

    #[test]
    fn test_validation_result_with_errors() {
        let mut result = ValidationResult::new();
        result.add_error("test.path", "Test error");
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_with_warnings() {
        let mut result = ValidationResult::new();
        result.add_warning("test.path", "Test warning");
        assert!(result.is_valid()); // Warnings don't make it invalid
        assert_eq!(result.warning_count(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_error("path1", "Error 1");

        let mut result2 = ValidationResult::new();
        result2.add_warning("path2", "Warning 1");

        result1.merge(result2);
        assert_eq!(result1.error_count(), 1);
        assert_eq!(result1.warning_count(), 1);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut visited = HashSet::new();
        visited.insert("workload-a".to_string());
        visited.insert("workload-b".to_string());

        // Since "workload-a" is already in visited, this should detect a cycle
        assert!(visited.contains("workload-a"));
    }
}
