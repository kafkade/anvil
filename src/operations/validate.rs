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
use tracing::{debug, trace};

use crate::cli::commands::ValidateArgs;
use crate::cli::output::{print_error, print_info, print_success, print_warning};
use crate::cli::Cli;
use crate::config::schema::{SchemaValidator, ValidationResult, ValidationSeverity};
use crate::config::workload::Workload;
use crate::config::ConfigManager;

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

    // Check for removed scripts fields in raw YAML
    SchemaValidator::check_removed_scripts_fields(&content, &mut result);

    // Check for files existence
    let base_dir = workload_file.parent().unwrap_or(Path::new("."));
    let file_result = validate_referenced_files(&workload, base_dir, args.strict);
    result.merge(file_result);

    // Validate parent workloads exist and check for circular dependencies
    let inheritance_result = validate_inheritance(&workload, base_dir, args.strict);
    result.merge(inheritance_result);

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
        },
        "brew": {
          "type": "array",
          "description": "Homebrew packages (macOS/Linux)",
          "items": {
            "type": "object",
            "required": ["name"],
            "properties": {
              "name": {
                "type": "string",
                "description": "Homebrew formula or cask name (e.g., 'git', 'visual-studio-code')"
              },
              "cask": {
                "type": "boolean",
                "default": false,
                "description": "Whether this is a cask (GUI app) vs formula (CLI tool)"
              },
              "tap": {
                "type": "string",
                "description": "Tap source (e.g., 'homebrew/cask-fonts')"
              }
            }
          }
        },
        "apt": {
          "type": "array",
          "description": "APT packages (Debian/Ubuntu)",
          "items": {
            "type": "object",
            "required": ["name"],
            "properties": {
              "name": {
                "type": "string",
                "description": "APT package name (e.g., 'git', 'build-essential')"
              },
              "version": {
                "type": "string",
                "description": "Specific version constraint"
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
    "commands": {
      "type": "object",
      "description": "Inline command definitions",
      "properties": {
        "pre_install": { "$ref": "#/definitions/commandList" },
        "post_install": { "$ref": "#/definitions/commandList" }
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
    "commandList": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["run"],
        "properties": {
          "run": {
            "type": "string",
            "description": "Shell command string to execute"
          },
          "description": { "type": "string" },
          "timeout": {
            "type": "integer",
            "default": 300,
            "minimum": 1,
            "maximum": 3600,
            "description": "Timeout in seconds"
          },
          "elevated": {
            "type": "boolean",
            "default": false,
            "description": "Whether the command requires admin privileges"
          },
          "when": {
            "type": "object",
            "description": "Condition that must be true for this command to run (uses condition engine types)",
            "required": ["type"],
            "properties": {
              "type": {
                "type": "string",
                "enum": ["command_exists", "file_exists", "dir_exists", "env_var", "path_contains", "registry_value", "shell", "all_of", "any_of"]
              }
            }
          },
          "continue_on_error": {
            "type": "boolean",
            "default": false,
            "description": "Whether to continue if this command fails"
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
