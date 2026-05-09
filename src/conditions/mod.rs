//! Condition/predicate engine for Anvil
//!
//! Provides a composable, serde-tagged condition system for evaluating
//! system state: command existence, file/dir checks, environment variables,
//! PATH contents, Windows registry values, and arbitrary shell commands.
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::expand_variables;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during condition evaluation.
#[derive(Debug, Error)]
#[allow(dead_code)] // Variants needed for exhaustive condition error coverage
pub enum ConditionError {
    #[error("Failed to execute command: {0}")]
    CommandExecution(String),

    #[error("Invalid condition configuration: {0}")]
    InvalidConfig(String),

    #[error("Registry query failed: {0}")]
    RegistryError(String),
}

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Structured result of evaluating a single [`Condition`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionResult {
    /// Which condition variant was evaluated (e.g. `"command_exists"`).
    pub condition_type: String,
    /// Whether the condition passed.
    pub passed: bool,
    /// The actual value observed on the system, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// The expected value, if the condition specifies one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Human-readable description of the outcome.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Condition enum
// ---------------------------------------------------------------------------

/// A declarative predicate that can be evaluated against the current system.
///
/// Serializes as an internally-tagged enum so that YAML/JSON representations
/// use a `type` discriminator field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Check whether a command is available on `PATH`.
    CommandExists { command: String },

    /// Check whether a file exists at the given path (`~` is expanded).
    FileExists { path: String },

    /// Check whether a directory exists at the given path (`~` is expanded).
    DirExists { path: String },

    /// Check whether an environment variable is set, optionally matching a value.
    EnvVar {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },

    /// Check whether `PATH` contains a given substring.
    PathContains { substring: String },

    /// Query a Windows registry value under `HKCU` or `HKLM`.
    RegistryValue {
        /// Registry hive – `"HKCU"` or `"HKLM"`.
        hive: String,
        /// Full key path, e.g. `"SOFTWARE\\Microsoft\\Windows\\CurrentVersion"`.
        key: String,
        /// Value name inside the key.
        name: String,
        /// Expected data; if omitted the check only asserts presence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected: Option<String>,
    },

    /// Run a shell command; the condition passes when the exit code is 0.
    Shell {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },

    /// All child conditions must pass (logical AND).
    AllOf { conditions: Vec<Condition> },

    /// At least one child condition must pass (logical OR).
    AnyOf { conditions: Vec<Condition> },

    /// Check whether a font is installed by name pattern in the Windows font registry.
    FontInstalled {
        /// Font name pattern to search for (e.g., "Lilex", "Cascadia")
        name: String,
    },
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate a [`Condition`] and return a structured [`ConditionResult`].
pub fn evaluate(condition: &Condition) -> ConditionResult {
    match condition {
        Condition::CommandExists { command } => eval_command_exists(command),
        Condition::FileExists { path } => eval_file_exists(path),
        Condition::DirExists { path } => eval_dir_exists(path),
        Condition::EnvVar { name, value } => eval_env_var(name, value.as_deref()),
        Condition::PathContains { substring } => eval_path_contains(substring),
        Condition::RegistryValue {
            hive,
            key,
            name,
            expected,
        } => eval_registry_value(hive, key, name, expected.as_deref()),
        Condition::Shell {
            command,
            description,
        } => eval_shell(command, description.as_deref()),
        Condition::AllOf { conditions } => eval_all_of(conditions),
        Condition::AnyOf { conditions } => eval_any_of(conditions),
        Condition::FontInstalled { name } => eval_font_installed(name),
    }
}

// ---------------------------------------------------------------------------
// Individual evaluators
// ---------------------------------------------------------------------------

fn eval_command_exists(command: &str) -> ConditionResult {
    let found = if cfg!(windows) {
        Command::new("where")
            .arg(command)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new("which")
            .arg(command)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    ConditionResult {
        condition_type: "command_exists".to_string(),
        passed: found,
        actual: Some(if found {
            "found".to_string()
        } else {
            "not found".to_string()
        }),
        expected: Some(command.to_string()),
        message: if found {
            format!("Command '{}' is available on PATH", command)
        } else {
            format!("Command '{}' was not found on PATH", command)
        },
    }
}

fn eval_file_exists(path: &str) -> ConditionResult {
    let expanded = expand_variables(path, None);
    let exists = Path::new(&expanded).is_file();

    ConditionResult {
        condition_type: "file_exists".to_string(),
        passed: exists,
        actual: Some(if exists {
            "exists".to_string()
        } else {
            "not found".to_string()
        }),
        expected: Some(expanded.clone()),
        message: if exists {
            format!("File '{}' exists", expanded)
        } else {
            format!("File '{}' does not exist", expanded)
        },
    }
}

fn eval_dir_exists(path: &str) -> ConditionResult {
    let expanded = expand_variables(path, None);
    let exists = Path::new(&expanded).is_dir();

    ConditionResult {
        condition_type: "dir_exists".to_string(),
        passed: exists,
        actual: Some(if exists {
            "exists".to_string()
        } else {
            "not found".to_string()
        }),
        expected: Some(expanded.clone()),
        message: if exists {
            format!("Directory '{}' exists", expanded)
        } else {
            format!("Directory '{}' does not exist", expanded)
        },
    }
}

fn eval_env_var(name: &str, expected: Option<&str>) -> ConditionResult {
    match std::env::var(name) {
        Ok(actual_value) => {
            if let Some(exp) = expected {
                let matches = actual_value == exp;
                ConditionResult {
                    condition_type: "env_var".to_string(),
                    passed: matches,
                    actual: Some(actual_value),
                    expected: Some(exp.to_string()),
                    message: if matches {
                        format!("Environment variable '{}' matches expected value", name)
                    } else {
                        format!(
                            "Environment variable '{}' exists but does not match expected value",
                            name
                        )
                    },
                }
            } else {
                ConditionResult {
                    condition_type: "env_var".to_string(),
                    passed: true,
                    actual: Some(actual_value),
                    expected: None,
                    message: format!("Environment variable '{}' is set", name),
                }
            }
        }
        Err(_) => ConditionResult {
            condition_type: "env_var".to_string(),
            passed: false,
            actual: None,
            expected: expected.map(|s| s.to_string()),
            message: format!("Environment variable '{}' is not set", name),
        },
    }
}

fn eval_path_contains(substring: &str) -> ConditionResult {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ';' } else { ':' };
    let found = path_var
        .split(separator)
        .any(|entry| entry.contains(substring));

    ConditionResult {
        condition_type: "path_contains".to_string(),
        passed: found,
        actual: None,
        expected: Some(substring.to_string()),
        message: if found {
            format!("PATH contains an entry matching '{}'", substring)
        } else {
            format!("PATH does not contain an entry matching '{}'", substring)
        },
    }
}

fn eval_registry_value(
    hive: &str,
    key: &str,
    name: &str,
    expected: Option<&str>,
) -> ConditionResult {
    let full_key = format!("{}\\{}", hive, key);

    let output = Command::new("reg")
        .args(["query", &full_key, "/v", name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // `reg query` output contains a line like:
            //     ValueName    REG_SZ    ActualData
            let actual_value = parse_reg_query_output(&stdout, name);

            if let Some(exp) = expected {
                let matches = actual_value.as_deref() == Some(exp);
                ConditionResult {
                    condition_type: "registry_value".to_string(),
                    passed: matches,
                    actual: actual_value,
                    expected: Some(exp.to_string()),
                    message: if matches {
                        format!("Registry value '{}\\{}' matches expected", full_key, name)
                    } else {
                        format!(
                            "Registry value '{}\\{}' does not match expected",
                            full_key, name
                        )
                    },
                }
            } else {
                ConditionResult {
                    condition_type: "registry_value".to_string(),
                    passed: true,
                    actual: actual_value,
                    expected: None,
                    message: format!("Registry value '{}\\{}' exists", full_key, name),
                }
            }
        }
        _ => ConditionResult {
            condition_type: "registry_value".to_string(),
            passed: false,
            actual: None,
            expected: expected.map(|s| s.to_string()),
            message: format!("Registry value '{}\\{}' was not found", full_key, name),
        },
    }
}

/// Parse `reg query` stdout to extract the value data for a given value name.
fn parse_reg_query_output(output: &str, value_name: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        // Lines look like: "    ValueName    REG_SZ    SomeValue"
        if trimmed.starts_with(value_name) {
            // Split by whitespace: [name, type, ...data]
            let parts: Vec<&str> = trimmed.splitn(3, "    ").collect();
            if parts.len() == 3 {
                return Some(parts[2].trim().to_string());
            }
        }
    }
    None
}

fn eval_shell(command: &str, description: Option<&str>) -> ConditionResult {
    let label = description.unwrap_or(command);

    let result = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", command])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    };

    match result {
        Ok(status) => {
            let passed = status.success();
            ConditionResult {
                condition_type: "shell".to_string(),
                passed,
                actual: Some(
                    status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                expected: Some("0".to_string()),
                message: if passed {
                    format!("Shell command passed: {}", label)
                } else {
                    format!(
                        "Shell command failed (exit {}): {}",
                        status.code().unwrap_or(-1),
                        label,
                    )
                },
            }
        }
        Err(e) => ConditionResult {
            condition_type: "shell".to_string(),
            passed: false,
            actual: Some(e.to_string()),
            expected: Some("0".to_string()),
            message: format!("Shell command could not be executed: {}", label),
        },
    }
}

fn eval_font_installed(name: &str) -> ConditionResult {
    let script = format!(
        "$fonts = Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts' -ErrorAction SilentlyContinue; \
         if ($fonts) {{ ($fonts.PSObject.Properties | Where-Object {{ $_.Name -like '*{}*' }}).Count -gt 0 }} else {{ $false }}",
        name.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();

    let installed = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .trim()
            .eq_ignore_ascii_case("true"),
        _ => false,
    };

    ConditionResult {
        condition_type: "font_installed".to_string(),
        passed: installed,
        actual: Some(if installed {
            "installed".to_string()
        } else {
            "not found".to_string()
        }),
        expected: Some(name.to_string()),
        message: if installed {
            format!("Font '{}' is installed", name)
        } else {
            format!("Font '{}' is not installed", name)
        },
    }
}

fn eval_all_of(conditions: &[Condition]) -> ConditionResult {
    let results: Vec<ConditionResult> = conditions.iter().map(evaluate).collect();
    let passed = results.iter().all(|r| r.passed);
    let total = results.len();
    let passed_count = results.iter().filter(|r| r.passed).count();

    ConditionResult {
        condition_type: "all_of".to_string(),
        passed,
        actual: Some(format!("{}/{} passed", passed_count, total)),
        expected: Some(format!("{}/{} passed", total, total)),
        message: if passed {
            format!("All {} conditions passed", total)
        } else {
            let failed: Vec<String> = results
                .iter()
                .filter(|r| !r.passed)
                .map(|r| r.message.clone())
                .collect();
            format!(
                "{}/{} conditions failed: {}",
                total - passed_count,
                total,
                failed.join("; ")
            )
        },
    }
}

fn eval_any_of(conditions: &[Condition]) -> ConditionResult {
    let results: Vec<ConditionResult> = conditions.iter().map(evaluate).collect();
    let passed = results.iter().any(|r| r.passed);
    let total = results.len();
    let passed_count = results.iter().filter(|r| r.passed).count();

    ConditionResult {
        condition_type: "any_of".to_string(),
        passed,
        actual: Some(format!("{}/{} passed", passed_count, total)),
        expected: Some(format!("≥1/{} passed", total)),
        message: if passed {
            format!(
                "{}/{} conditions passed (at least one required)",
                passed_count, total
            )
        } else {
            format!("None of {} conditions passed", total)
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // -- command_exists --

    #[test]
    fn test_command_exists_found() {
        // `cmd` is always available on Windows; on Unix `sh` is universal
        let cmd = if cfg!(windows) { "cmd" } else { "sh" };
        let result = evaluate(&Condition::CommandExists {
            command: cmd.to_string(),
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "command_exists");
        assert_eq!(result.actual.as_deref(), Some("found"));
    }

    #[test]
    fn test_command_exists_not_found() {
        let result = evaluate(&Condition::CommandExists {
            command: "anvil_nonexistent_command_xyz_12345".to_string(),
        });
        assert!(!result.passed);
        assert_eq!(result.condition_type, "command_exists");
        assert_eq!(result.actual.as_deref(), Some("not found"));
    }

    // -- file_exists --

    #[test]
    fn test_file_exists_pass() {
        // Cargo.toml is always present in the repo root
        let result = evaluate(&Condition::FileExists {
            path: "Cargo.toml".to_string(),
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "file_exists");
    }

    #[test]
    fn test_file_exists_fail() {
        let result = evaluate(&Condition::FileExists {
            path: "nonexistent_file_xyz_12345.txt".to_string(),
        });
        assert!(!result.passed);
    }

    // -- dir_exists --

    #[test]
    fn test_dir_exists_pass() {
        let result = evaluate(&Condition::DirExists {
            path: "src".to_string(),
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "dir_exists");
    }

    #[test]
    fn test_dir_exists_fail() {
        let result = evaluate(&Condition::DirExists {
            path: "nonexistent_dir_xyz_12345".to_string(),
        });
        assert!(!result.passed);
    }

    // -- env_var --

    #[test]
    fn test_env_var_exists() {
        // PATH is always set
        let result = evaluate(&Condition::EnvVar {
            name: "PATH".to_string(),
            value: None,
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "env_var");
        assert!(result.actual.is_some());
    }

    #[test]
    fn test_env_var_not_set() {
        let result = evaluate(&Condition::EnvVar {
            name: "ANVIL_TEST_NONEXISTENT_VAR_XYZ_12345".to_string(),
            value: None,
        });
        assert!(!result.passed);
    }

    #[test]
    fn test_env_var_value_match() {
        std::env::set_var("ANVIL_TEST_COND_VAR", "hello");
        let result = evaluate(&Condition::EnvVar {
            name: "ANVIL_TEST_COND_VAR".to_string(),
            value: Some("hello".to_string()),
        });
        assert!(result.passed);
        std::env::remove_var("ANVIL_TEST_COND_VAR");
    }

    #[test]
    fn test_env_var_value_mismatch() {
        std::env::set_var("ANVIL_TEST_COND_VAR2", "world");
        let result = evaluate(&Condition::EnvVar {
            name: "ANVIL_TEST_COND_VAR2".to_string(),
            value: Some("hello".to_string()),
        });
        assert!(!result.passed);
        assert_eq!(result.actual.as_deref(), Some("world"));
        assert_eq!(result.expected.as_deref(), Some("hello"));
        std::env::remove_var("ANVIL_TEST_COND_VAR2");
    }

    // -- path_contains --

    #[test]
    fn test_path_contains_pass() {
        // Windows always has something like "System32" on PATH
        let substring = if cfg!(windows) { "System32" } else { "/usr" };
        let result = evaluate(&Condition::PathContains {
            substring: substring.to_string(),
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "path_contains");
    }

    #[test]
    fn test_path_contains_fail() {
        let result = evaluate(&Condition::PathContains {
            substring: "anvil_nonexistent_path_xyz_12345".to_string(),
        });
        assert!(!result.passed);
    }

    // -- shell --

    #[test]
    fn test_shell_pass() {
        let cmd = if cfg!(windows) { "echo ok" } else { "true" };
        let result = evaluate(&Condition::Shell {
            command: cmd.to_string(),
            description: Some("echo test".to_string()),
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "shell");
        assert_eq!(result.actual.as_deref(), Some("0"));
    }

    #[test]
    fn test_shell_fail() {
        let cmd = if cfg!(windows) {
            "cmd /C exit 1"
        } else {
            "false"
        };
        let result = evaluate(&Condition::Shell {
            command: cmd.to_string(),
            description: None,
        });
        assert!(!result.passed);
    }

    // -- all_of --

    #[test]
    fn test_all_of_pass() {
        let result = evaluate(&Condition::AllOf {
            conditions: vec![
                Condition::EnvVar {
                    name: "PATH".to_string(),
                    value: None,
                },
                Condition::DirExists {
                    path: "src".to_string(),
                },
            ],
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "all_of");
        assert_eq!(result.actual.as_deref(), Some("2/2 passed"));
    }

    #[test]
    fn test_all_of_fail() {
        let result = evaluate(&Condition::AllOf {
            conditions: vec![
                Condition::EnvVar {
                    name: "PATH".to_string(),
                    value: None,
                },
                Condition::FileExists {
                    path: "nonexistent_xyz_12345.txt".to_string(),
                },
            ],
        });
        assert!(!result.passed);
        assert_eq!(result.actual.as_deref(), Some("1/2 passed"));
    }

    // -- any_of --

    #[test]
    fn test_any_of_pass() {
        let result = evaluate(&Condition::AnyOf {
            conditions: vec![
                Condition::FileExists {
                    path: "nonexistent_xyz_12345.txt".to_string(),
                },
                Condition::EnvVar {
                    name: "PATH".to_string(),
                    value: None,
                },
            ],
        });
        assert!(result.passed);
        assert_eq!(result.condition_type, "any_of");
    }

    #[test]
    fn test_any_of_fail() {
        let result = evaluate(&Condition::AnyOf {
            conditions: vec![
                Condition::FileExists {
                    path: "nonexistent_a_12345.txt".to_string(),
                },
                Condition::FileExists {
                    path: "nonexistent_b_12345.txt".to_string(),
                },
            ],
        });
        assert!(!result.passed);
    }

    // -- serde roundtrip --

    #[test]
    fn test_serde_command_exists_roundtrip() {
        let cond = Condition::CommandExists {
            command: "git".to_string(),
        };
        let yaml = serde_yaml::to_string(&cond).unwrap();
        assert!(yaml.contains("type: command_exists"));
        assert!(yaml.contains("command: git"));
        let deser: Condition = serde_yaml::from_str(&yaml).unwrap();
        if let Condition::CommandExists { command } = deser {
            assert_eq!(command, "git");
        } else {
            panic!("Expected CommandExists variant");
        }
    }

    #[test]
    fn test_serde_env_var_with_value_roundtrip() {
        let cond = Condition::EnvVar {
            name: "EDITOR".to_string(),
            value: Some("vim".to_string()),
        };
        let yaml = serde_yaml::to_string(&cond).unwrap();
        let deser: Condition = serde_yaml::from_str(&yaml).unwrap();
        if let Condition::EnvVar { name, value } = deser {
            assert_eq!(name, "EDITOR");
            assert_eq!(value.as_deref(), Some("vim"));
        } else {
            panic!("Expected EnvVar variant");
        }
    }

    #[test]
    fn test_serde_all_of_roundtrip() {
        let cond = Condition::AllOf {
            conditions: vec![
                Condition::CommandExists {
                    command: "git".to_string(),
                },
                Condition::FileExists {
                    path: "~/.gitconfig".to_string(),
                },
            ],
        };
        let yaml = serde_yaml::to_string(&cond).unwrap();
        assert!(yaml.contains("type: all_of"));
        let deser: Condition = serde_yaml::from_str(&yaml).unwrap();
        if let Condition::AllOf { conditions } = deser {
            assert_eq!(conditions.len(), 2);
        } else {
            panic!("Expected AllOf variant");
        }
    }

    #[test]
    fn test_serde_from_yaml_block() {
        let yaml = r#"
type: all_of
conditions:
  - type: command_exists
    command: git
  - type: file_exists
    path: ~/.gitconfig
  - type: env_var
    name: EDITOR
    value: vim
"#;
        let cond: Condition = serde_yaml::from_str(yaml).unwrap();
        if let Condition::AllOf { conditions } = cond {
            assert_eq!(conditions.len(), 3);
        } else {
            panic!("Expected AllOf variant");
        }
    }

    #[test]
    fn test_serde_registry_value_roundtrip() {
        let cond = Condition::RegistryValue {
            hive: "HKCU".to_string(),
            key: "SOFTWARE\\Microsoft".to_string(),
            name: "TestValue".to_string(),
            expected: Some("1".to_string()),
        };
        let yaml = serde_yaml::to_string(&cond).unwrap();
        assert!(yaml.contains("type: registry_value"));
        let deser: Condition = serde_yaml::from_str(&yaml).unwrap();
        if let Condition::RegistryValue {
            hive,
            key,
            name,
            expected,
        } = deser
        {
            assert_eq!(hive, "HKCU");
            assert_eq!(key, "SOFTWARE\\Microsoft");
            assert_eq!(name, "TestValue");
            assert_eq!(expected.as_deref(), Some("1"));
        } else {
            panic!("Expected RegistryValue variant");
        }
    }

    #[test]
    fn test_serde_shell_roundtrip() {
        let cond = Condition::Shell {
            command: "echo hello".to_string(),
            description: Some("Test echo".to_string()),
        };
        let yaml = serde_yaml::to_string(&cond).unwrap();
        assert!(yaml.contains("type: shell"));
        let deser: Condition = serde_yaml::from_str(&yaml).unwrap();
        if let Condition::Shell {
            command,
            description,
        } = deser
        {
            assert_eq!(command, "echo hello");
            assert_eq!(description.as_deref(), Some("Test echo"));
        } else {
            panic!("Expected Shell variant");
        }
    }

    #[test]
    fn test_condition_result_fields() {
        let result = ConditionResult {
            condition_type: "test".to_string(),
            passed: true,
            actual: Some("42".to_string()),
            expected: Some("42".to_string()),
            message: "Test passed".to_string(),
        };
        assert!(result.passed);
        assert_eq!(result.actual.as_deref(), Some("42"));
        assert_eq!(result.expected.as_deref(), Some("42"));
    }

    // -- nested composition --

    #[test]
    fn test_nested_all_of_any_of() {
        let cond = Condition::AllOf {
            conditions: vec![
                Condition::EnvVar {
                    name: "PATH".to_string(),
                    value: None,
                },
                Condition::AnyOf {
                    conditions: vec![
                        Condition::FileExists {
                            path: "Cargo.toml".to_string(),
                        },
                        Condition::FileExists {
                            path: "nonexistent_xyz_12345.txt".to_string(),
                        },
                    ],
                },
            ],
        };
        let result = evaluate(&cond);
        assert!(result.passed);
        assert_eq!(result.condition_type, "all_of");
    }

    // -- parse_reg_query_output --

    #[test]
    fn test_parse_reg_query_output() {
        let output = r#"
HKEY_CURRENT_USER\SOFTWARE\Microsoft

    TestValue    REG_SZ    SomeData
"#;
        let val = parse_reg_query_output(output, "TestValue");
        assert_eq!(val.as_deref(), Some("SomeData"));
    }

    #[test]
    fn test_parse_reg_query_output_not_found() {
        let output = "ERROR: The system was unable to find the specified registry key or value.";
        let val = parse_reg_query_output(output, "TestValue");
        assert!(val.is_none());
    }
}
