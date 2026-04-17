//! Command execution module for Anvil
//!
//! This module handles execution of inline commands defined in workload
//! `commands:` blocks, with conditional execution, timeout enforcement,
//! and structured result reporting.

use std::time::Duration;

use serde::Serialize;
use thiserror::Error;

/// Errors that can occur during command execution
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum CommandError {
    /// Command executable was not found on the system
    #[error("Command not found: {0}")]
    NotFound(String),

    /// Command exceeded its timeout duration
    #[error("Command timed out after {timeout_seconds} seconds: {command}")]
    Timeout {
        command: String,
        timeout_seconds: u64,
    },

    /// Command requires elevated privileges that are not available
    #[error("Command requires elevated privileges: {0}")]
    ElevationRequired(String),

    /// Command execution failed for a general reason
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    /// IO error during command execution
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Status of a command execution
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CommandStatus {
    /// Command executed successfully (exit code 0)
    Success,
    /// Command failed (non-zero exit code)
    Failed,
    /// Command was skipped (when condition not met)
    Skipped,
    /// Command timed out
    TimedOut,
}

/// Result of executing a single command
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    /// Display name or the command string
    pub name: String,
    /// Execution status
    pub status: CommandStatus,
    /// Exit code (None if skipped or timed out)
    pub exit_code: Option<i32>,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr
    pub stderr: String,
    /// Human-readable message
    pub message: String,
    /// How long the command took
    pub duration: Duration,
}

/// Summary of a batch of command executions
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize)]
pub struct CommandSummary {
    /// Total commands in the batch
    pub total: usize,
    /// Commands that succeeded
    pub succeeded: usize,
    /// Commands that failed
    pub failed: usize,
    /// Commands that were skipped
    pub skipped: usize,
    /// Commands that timed out
    pub timed_out: usize,
    /// Total execution duration
    pub total_duration: Duration,
    /// Whether any command requires reboot
    pub requires_reboot: bool,
    /// Individual results
    pub results: Vec<CommandResult>,
}

#[allow(dead_code)]
impl CommandSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_result(&mut self, result: CommandResult) {
        self.total += 1;
        self.total_duration += result.duration;
        match result.status {
            CommandStatus::Success => self.succeeded += 1,
            CommandStatus::Failed => self.failed += 1,
            CommandStatus::Skipped => self.skipped += 1,
            CommandStatus::TimedOut => {
                self.timed_out += 1;
                self.failed += 1; // timed out counts as failure
            }
        }
        self.results.push(result);
    }

    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Execute a list of commands, evaluating `when` conditions and enforcing timeouts.
pub fn execute_commands(
    commands: &[crate::config::workload::CommandEntry],
    _phase: &str,
    verbose: bool,
    dry_run: bool,
) -> CommandSummary {
    let mut summary = CommandSummary::new();

    for entry in commands {
        let display_name = entry.description.as_deref().unwrap_or(&entry.run);

        // Evaluate `when` condition if present
        if let Some(condition) = &entry.when {
            let result = crate::conditions::evaluate(condition);
            if !result.passed {
                summary.add_result(CommandResult {
                    name: display_name.to_string(),
                    status: CommandStatus::Skipped,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    message: format!("Condition not met: {}", result.message),
                    duration: Duration::ZERO,
                });
                continue;
            }
        }

        if dry_run {
            summary.add_result(CommandResult {
                name: display_name.to_string(),
                status: CommandStatus::Skipped,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                message: "Dry run — would execute".to_string(),
                duration: Duration::ZERO,
            });
            continue;
        }

        // Execute the command
        let result = execute_single_command(entry, display_name, verbose);
        let should_stop = result.status == CommandStatus::Failed && !entry.continue_on_error;
        summary.add_result(result);

        if should_stop {
            break;
        }
    }

    summary
}

fn execute_single_command(
    entry: &crate::config::workload::CommandEntry,
    display_name: &str,
    _verbose: bool,
) -> CommandResult {
    use std::process::Command;
    use std::time::Instant;

    let start = Instant::now();

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive", "-Command", &entry.run]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &entry.run]);
        c
    };

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    match cmd.spawn() {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                let duration = start.elapsed();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                if duration.as_secs() > entry.timeout {
                    CommandResult {
                        name: display_name.to_string(),
                        status: CommandStatus::TimedOut,
                        exit_code: Some(exit_code),
                        stdout,
                        stderr,
                        message: format!("Timed out after {}s", entry.timeout),
                        duration,
                    }
                } else if exit_code == 0 {
                    CommandResult {
                        name: display_name.to_string(),
                        status: CommandStatus::Success,
                        exit_code: Some(0),
                        stdout,
                        stderr,
                        message: "Completed successfully".to_string(),
                        duration,
                    }
                } else {
                    CommandResult {
                        name: display_name.to_string(),
                        status: CommandStatus::Failed,
                        exit_code: Some(exit_code),
                        stdout,
                        stderr,
                        message: format!("Exited with code {}", exit_code),
                        duration,
                    }
                }
            }
            Err(e) => CommandResult {
                name: display_name.to_string(),
                status: CommandStatus::Failed,
                exit_code: None,
                stdout: String::new(),
                stderr: e.to_string(),
                message: format!("Failed to wait for process: {}", e),
                duration: start.elapsed(),
            },
        },
        Err(e) => CommandResult {
            name: display_name.to_string(),
            status: CommandStatus::Failed,
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            message: format!("Failed to spawn command: {}", e),
            duration: start.elapsed(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_result_construction() {
        let result = CommandResult {
            name: "echo hello".to_string(),
            status: CommandStatus::Success,
            exit_code: Some(0),
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            message: "Command succeeded".to_string(),
            duration: Duration::from_millis(50),
        };

        assert_eq!(result.name, "echo hello");
        assert_eq!(result.status, CommandStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "hello\n");
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_command_status_variants() {
        assert_eq!(CommandStatus::Success, CommandStatus::Success);
        assert_ne!(CommandStatus::Success, CommandStatus::Failed);
        assert_ne!(CommandStatus::Skipped, CommandStatus::TimedOut);
        assert_ne!(CommandStatus::Failed, CommandStatus::Skipped);
    }

    #[test]
    fn test_command_summary_empty() {
        let summary = CommandSummary::new();
        assert_eq!(summary.total, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.timed_out, 0);
        assert!(summary.is_successful());
        assert!(summary.results.is_empty());
    }

    #[test]
    fn test_command_summary_tracks_success() {
        let mut summary = CommandSummary::new();
        summary.add_result(CommandResult {
            name: "cmd1".to_string(),
            status: CommandStatus::Success,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            message: "ok".to_string(),
            duration: Duration::from_millis(100),
        });

        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 0);
        assert!(summary.is_successful());
        assert_eq!(summary.total_duration, Duration::from_millis(100));
    }

    #[test]
    fn test_command_summary_tracks_failure() {
        let mut summary = CommandSummary::new();
        summary.add_result(CommandResult {
            name: "bad-cmd".to_string(),
            status: CommandStatus::Failed,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error\n".to_string(),
            message: "failed".to_string(),
            duration: Duration::from_millis(200),
        });

        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 1);
        assert!(!summary.is_successful());
    }

    #[test]
    fn test_command_summary_timeout_counts_as_failure() {
        let mut summary = CommandSummary::new();
        summary.add_result(CommandResult {
            name: "slow-cmd".to_string(),
            status: CommandStatus::TimedOut,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            message: "timed out".to_string(),
            duration: Duration::from_secs(300),
        });

        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.failed, 1);
        assert!(!summary.is_successful());
    }

    #[test]
    fn test_command_summary_mixed_results() {
        let mut summary = CommandSummary::new();

        summary.add_result(CommandResult {
            name: "success-cmd".to_string(),
            status: CommandStatus::Success,
            exit_code: Some(0),
            stdout: "done".to_string(),
            stderr: String::new(),
            message: "ok".to_string(),
            duration: Duration::from_millis(50),
        });

        summary.add_result(CommandResult {
            name: "skipped-cmd".to_string(),
            status: CommandStatus::Skipped,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            message: "condition not met".to_string(),
            duration: Duration::ZERO,
        });

        summary.add_result(CommandResult {
            name: "failed-cmd".to_string(),
            status: CommandStatus::Failed,
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "err".to_string(),
            message: "failed".to_string(),
            duration: Duration::from_millis(100),
        });

        summary.add_result(CommandResult {
            name: "timeout-cmd".to_string(),
            status: CommandStatus::TimedOut,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            message: "timed out".to_string(),
            duration: Duration::from_secs(300),
        });

        assert_eq!(summary.total, 4);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 2); // 1 failed + 1 timed out
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.timed_out, 1);
        assert!(!summary.is_successful());
        assert_eq!(summary.results.len(), 4);
        assert_eq!(
            summary.total_duration,
            Duration::from_millis(50 + 100) + Duration::from_secs(300)
        );
    }

    #[test]
    fn test_command_summary_all_skipped_is_successful() {
        let mut summary = CommandSummary::new();
        summary.add_result(CommandResult {
            name: "skipped".to_string(),
            status: CommandStatus::Skipped,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            message: "condition not met".to_string(),
            duration: Duration::ZERO,
        });

        assert_eq!(summary.total, 1);
        assert_eq!(summary.skipped, 1);
        assert!(summary.is_successful());
    }

    fn make_entry(run: &str) -> crate::config::workload::CommandEntry {
        crate::config::workload::CommandEntry {
            run: run.to_string(),
            description: None,
            timeout: 300,
            elevated: false,
            when: None,
            continue_on_error: false,
        }
    }

    #[test]
    fn test_execute_commands_empty_list() {
        let commands: Vec<crate::config::workload::CommandEntry> = vec![];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        assert_eq!(summary.total, 0);
        assert!(summary.is_successful());
    }

    #[test]
    fn test_execute_commands_simple_success() {
        let cmd = if cfg!(target_os = "windows") {
            "Write-Output 'hello'"
        } else {
            "echo hello"
        };
        let commands = vec![make_entry(cmd)];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);
        assert!(summary.is_successful());
        assert!(summary.results[0].stdout.contains("hello"));
    }

    #[test]
    fn test_execute_commands_failure() {
        let commands = vec![make_entry("exit 1")];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.failed, 1);
        assert!(!summary.is_successful());
        assert_eq!(summary.results[0].status, CommandStatus::Failed);
    }

    #[test]
    fn test_execute_commands_stops_on_failure() {
        let ok_cmd = if cfg!(target_os = "windows") {
            "Write-Output 'after'"
        } else {
            "echo after"
        };
        let commands = vec![make_entry("exit 1"), make_entry(ok_cmd)];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        // Should stop after first failure (continue_on_error is false)
        assert_eq!(summary.total, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_execute_commands_continue_on_error() {
        let ok_cmd = if cfg!(target_os = "windows") {
            "Write-Output 'after'"
        } else {
            "echo after"
        };
        let mut entry1 = make_entry("exit 1");
        entry1.continue_on_error = true;
        let entry2 = make_entry(ok_cmd);
        let commands = vec![entry1, entry2];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.succeeded, 1);
    }

    #[test]
    fn test_execute_commands_dry_run() {
        let cmd = if cfg!(target_os = "windows") {
            "Write-Output 'should not run'"
        } else {
            "echo 'should not run'"
        };
        let commands = vec![make_entry(cmd)];
        let summary = super::execute_commands(&commands, "pre_install", false, true);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.skipped, 1);
        assert!(summary.results[0].message.contains("Dry run"));
    }

    #[test]
    fn test_execute_commands_when_condition_skips() {
        // Use a condition that will never be true
        let mut entry = make_entry("echo should-not-run");
        entry.when = Some(crate::conditions::Condition::FileExists {
            path: "__anvil_nonexistent_file_for_test_12345__".to_string(),
        });
        let commands = vec![entry];
        let summary = super::execute_commands(&commands, "pre_install", false, false);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.skipped, 1);
        assert!(summary.results[0].message.contains("Condition not met"));
    }
}
