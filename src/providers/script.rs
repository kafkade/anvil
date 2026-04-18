//! Script execution provider for Anvil
//!
//! This module handles execution of PowerShell scripts for:
//! - Pre-installation scripts
//! - Post-installation scripts
//! - Health check validation scripts

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use thiserror::Error;

/// Errors that can occur during script execution
#[derive(Error, Debug)]
#[allow(dead_code)] // Variants cover all expected error conditions
pub enum ScriptError {
    /// Script file not found
    #[error("Script not found: {0}")]
    NotFound(PathBuf),

    /// Script execution failed
    #[error("Script execution failed: {path} (exit code: {exit_code})")]
    ExecutionFailed {
        path: PathBuf,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },

    /// Script timed out
    #[error("Script timed out after {timeout_seconds} seconds: {path}")]
    Timeout { path: PathBuf, timeout_seconds: u64 },

    /// IO error during script execution
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to spawn script process
    #[error("Failed to spawn script process: {0}")]
    SpawnFailed(String),

    /// Elevated execution required but not available
    #[error("Elevated execution required for script: {path}\n\nThis script needs to run as Administrator. Options:\n  1. Run Anvil as Administrator (right-click Terminal → Run as Administrator)\n  2. Skip elevated scripts with --skip-scripts\n  3. Mark script as non-elevated in workload.yaml if safe")]
    ElevationRequired { path: PathBuf },

    /// Invalid shell specified
    #[error("Invalid shell: {0}")]
    InvalidShell(String),

    /// PowerShell Core not available
    #[error("PowerShell Core (pwsh) requested but not installed. Install with: winget install Microsoft.PowerShell")]
    PwshNotAvailable,

    /// Script syntax error
    #[error("Script syntax error in {path}: {message}")]
    SyntaxError { path: PathBuf, message: String },

    /// Thread communication error
    #[error("Internal error: thread communication failed")]
    ThreadError,
}

/// Supported shell types
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
#[derive(Default)]
pub enum Shell {
    /// Windows PowerShell (powershell.exe)
    #[default]
    PowerShell,
    /// PowerShell Core (pwsh.exe)
    Pwsh,
    /// Windows Command Prompt (cmd.exe)
    Cmd,
    /// Bash (typically via WSL or Git Bash)
    Bash,
}

#[allow(dead_code)] // Test-only: used in module tests
impl Shell {
    /// Get the executable name for this shell
    pub fn executable(&self) -> &'static str {
        match self {
            Shell::PowerShell => "powershell.exe",
            Shell::Pwsh => "pwsh.exe",
            Shell::Cmd => "cmd.exe",
            Shell::Bash => "bash.exe",
        }
    }

    /// Get the argument to execute a script file
    pub fn script_arg(&self) -> &'static str {
        match self {
            Shell::PowerShell | Shell::Pwsh => "-File",
            Shell::Cmd => "/c",
            Shell::Bash => "-c",
        }
    }

    /// Parse a shell name from a string
    pub fn from_str(s: &str) -> Result<Self, ScriptError> {
        match s.to_lowercase().as_str() {
            "powershell" => Ok(Shell::PowerShell),
            "pwsh" => Ok(Shell::Pwsh),
            "cmd" | "command" => Ok(Shell::Cmd),
            "bash" | "sh" => Ok(Shell::Bash),
            _ => Err(ScriptError::InvalidShell(s.to_string())),
        }
    }

    /// Check if this shell is available on the system
    pub fn is_available(&self) -> bool {
        Command::new(self.executable())
            .args(match self {
                Shell::PowerShell | Shell::Pwsh => vec!["-NoProfile", "-Command", "exit 0"],
                Shell::Cmd => vec!["/c", "exit 0"],
                Shell::Bash => vec!["-c", "exit 0"],
            })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Output mode for script execution
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Capture output silently, return in result
    #[default]
    Capture,
    /// Stream output to console in real-time
    Stream,
    /// Both capture and stream
    Both,
}

/// Script execution phase
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPhase {
    PreInstall,
    PostInstall,
    Validation,
}

#[allow(dead_code)] // Test-only: used in module tests
impl std::fmt::Display for ScriptPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptPhase::PreInstall => write!(f, "pre_install"),
            ScriptPhase::PostInstall => write!(f, "post_install"),
            ScriptPhase::Validation => write!(f, "validation"),
        }
    }
}

/// Configuration for script execution
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Path to the script file
    pub path: PathBuf,

    /// Shell to use for execution
    pub shell: Shell,

    /// Whether to run with elevated privileges
    pub elevated: bool,

    /// Timeout duration
    pub timeout: Duration,

    /// Working directory for script execution
    pub working_dir: Option<PathBuf>,

    /// Additional environment variables
    pub environment: HashMap<String, String>,

    /// Additional arguments to pass to the script
    pub arguments: Vec<String>,

    /// Output mode
    pub output_mode: OutputMode,

    /// Optional prefix for streamed output
    pub output_prefix: Option<String>,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ScriptConfig {
    /// Create a new script configuration with defaults
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            shell: Shell::default(),
            elevated: false,
            timeout: Duration::from_secs(300),
            working_dir: None,
            environment: HashMap::new(),
            arguments: Vec::new(),
            output_mode: OutputMode::Capture,
            output_prefix: None,
        }
    }

    /// Set the shell
    pub fn with_shell(mut self, shell: Shell) -> Self {
        self.shell = shell;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set elevated execution
    pub fn with_elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    /// Set output mode
    pub fn with_output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    /// Set output prefix for streaming
    pub fn with_output_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.output_prefix = Some(prefix.into());
        self
    }

    /// Add an argument to pass to the script
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.arguments.push(arg.into());
        self
    }
}

/// Type of output line from script
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    Info,
    Success,
    Warning,
    Error,
    Progress,
    Debug,
    Unknown,
}

/// A parsed line of script output
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: String,
    pub line_type: LineType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Summary extracted from script output
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Default)]
pub struct ScriptSummary {
    pub passed: u32,
    pub failed: u32,
    pub warnings: u32,
}

/// Parsed script output with structured data
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct ParsedScriptOutput {
    pub lines: Vec<OutputLine>,
    pub summary: ScriptSummary,
    pub structured_data: Option<serde_json::Value>,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ParsedScriptOutput {
    /// Parse raw output into structured format
    pub fn parse(stdout: &str, stderr: &str) -> Self {
        let mut lines = Vec::new();
        let mut summary = ScriptSummary::default();
        let mut structured_data = None;
        let now = chrono::Utc::now();

        // Parse stdout lines
        for line in stdout.lines() {
            let (line_type, content) = Self::classify_line(line);

            match line_type {
                LineType::Success => summary.passed += 1,
                LineType::Error => summary.failed += 1,
                LineType::Warning => summary.warnings += 1,
                _ => {}
            }

            lines.push(OutputLine {
                content: content.to_string(),
                line_type,
                timestamp: now,
            });
        }

        // Parse stderr lines as errors
        for line in stderr.lines() {
            if !line.trim().is_empty() {
                summary.failed += 1;
                lines.push(OutputLine {
                    content: line.to_string(),
                    line_type: LineType::Error,
                    timestamp: now,
                });
            }
        }

        // Try to parse JSON from last line
        if let Some(last_line) = stdout.lines().last() {
            if last_line.trim().starts_with('{') || last_line.trim().starts_with('[') {
                if let Ok(json) = serde_json::from_str(last_line.trim()) {
                    structured_data = Some(json);
                }
            }
        }

        Self {
            lines,
            summary,
            structured_data,
        }
    }

    /// Classify a line based on markers
    fn classify_line(line: &str) -> (LineType, &str) {
        let trimmed = line.trim();

        if trimmed.starts_with("[PASS]") || trimmed.starts_with("[OK]") || trimmed.contains("✓") {
            (LineType::Success, trimmed)
        } else if trimmed.starts_with("[FAIL]")
            || trimmed.starts_with("[ERROR]")
            || trimmed.contains("✗")
        {
            (LineType::Error, trimmed)
        } else if trimmed.starts_with("[WARN]")
            || trimmed.starts_with("[WARNING]")
            || trimmed.contains("⚠")
        {
            (LineType::Warning, trimmed)
        } else if trimmed.starts_with("[DEBUG]") {
            (LineType::Debug, trimmed)
        } else if trimmed.contains('%') || trimmed.starts_with(">>") {
            (LineType::Progress, trimmed)
        } else if trimmed.starts_with("[INFO]") {
            (LineType::Info, trimmed)
        } else {
            (LineType::Unknown, trimmed)
        }
    }
}

/// Result of a script execution
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// Exit code from the script
    pub exit_code: i32,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Duration of execution
    pub duration: Duration,

    /// Whether the script succeeded (exit code 0)
    pub success: bool,

    /// Whether script indicated reboot is required (exit code 3010)
    pub requires_reboot: bool,

    /// Parsed output with structured data
    pub parsed: Option<ParsedScriptOutput>,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ScriptResult {
    /// Create a successful result
    pub fn success(stdout: String, duration: Duration) -> Self {
        let parsed = ParsedScriptOutput::parse(&stdout, "");
        Self {
            exit_code: 0,
            stdout,
            stderr: String::new(),
            duration,
            success: true,
            requires_reboot: false,
            parsed: Some(parsed),
        }
    }

    /// Create a failed result
    pub fn failure(exit_code: i32, stdout: String, stderr: String, duration: Duration) -> Self {
        let parsed = ParsedScriptOutput::parse(&stdout, &stderr);
        let requires_reboot = exit_code == 3010;
        Self {
            exit_code,
            stdout,
            stderr,
            duration,
            success: exit_code == 0 || exit_code == 3010,
            requires_reboot,
            parsed: Some(parsed),
        }
    }
}

/// Detailed result for script execution tracking
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct ScriptExecutionResult {
    pub script_name: String,
    pub script_path: PathBuf,
    pub phase: ScriptPhase,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub success: bool,
    pub requires_reboot: bool,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ScriptExecutionResult {
    pub fn from_result(
        result: &ScriptResult,
        script_name: String,
        script_path: PathBuf,
        phase: ScriptPhase,
    ) -> Self {
        Self {
            script_name,
            script_path,
            phase,
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            duration: result.duration,
            success: result.success,
            requires_reboot: result.requires_reboot,
        }
    }
}

/// Summary of script execution batch
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone, Default)]
pub struct ScriptExecutionSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration: Duration,
    pub requires_reboot: bool,
    pub results: Vec<ScriptExecutionResult>,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ScriptExecutionSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_result(&mut self, result: ScriptExecutionResult) {
        self.total += 1;
        self.total_duration += result.duration;

        if result.success {
            self.succeeded += 1;
        } else {
            self.failed += 1;
        }

        if result.requires_reboot {
            self.requires_reboot = true;
        }

        self.results.push(result);
    }

    pub fn add_skipped(&mut self) {
        self.total += 1;
        self.skipped += 1;
    }

    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Context for script execution
#[allow(dead_code)] // Test-only: used in module tests
#[derive(Debug, Clone)]
pub struct ScriptContext {
    pub workload_name: String,
    pub workload_path: PathBuf,
    pub dry_run: bool,
    pub verbose: bool,
    pub phase: ScriptPhase,
}

#[allow(dead_code)] // Test-only: used in module tests
impl ScriptContext {
    pub fn new(workload_name: impl Into<String>, workload_path: impl Into<PathBuf>) -> Self {
        Self {
            workload_name: workload_name.into(),
            workload_path: workload_path.into(),
            dry_run: false,
            verbose: false,
            phase: ScriptPhase::Validation,
        }
    }

    pub fn with_phase(mut self, phase: ScriptPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_parsing() {
        assert_eq!(Shell::from_str("powershell").unwrap(), Shell::PowerShell);
        assert_eq!(Shell::from_str("pwsh").unwrap(), Shell::Pwsh);
        assert_eq!(Shell::from_str("cmd").unwrap(), Shell::Cmd);
        assert_eq!(Shell::from_str("bash").unwrap(), Shell::Bash);
        assert!(Shell::from_str("invalid").is_err());
    }

    #[test]
    fn test_script_config_builder() {
        let config = ScriptConfig::new("test.ps1")
            .with_shell(Shell::PowerShell)
            .with_timeout(Duration::from_secs(60))
            .with_elevated(true)
            .with_env("TEST_VAR", "value")
            .with_output_mode(OutputMode::Stream)
            .with_output_prefix("[test]");

        assert_eq!(config.path, PathBuf::from("test.ps1"));
        assert_eq!(config.shell, Shell::PowerShell);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert!(config.elevated);
        assert_eq!(
            config.environment.get("TEST_VAR"),
            Some(&"value".to_string())
        );
        assert_eq!(config.output_mode, OutputMode::Stream);
        assert_eq!(config.output_prefix, Some("[test]".to_string()));
    }

    #[test]
    fn test_script_result() {
        let success = ScriptResult::success("output".to_string(), Duration::from_secs(1));
        assert!(success.success);
        assert_eq!(success.exit_code, 0);
        assert!(!success.requires_reboot);

        let failure = ScriptResult::failure(
            1,
            "out".to_string(),
            "err".to_string(),
            Duration::from_secs(1),
        );
        assert!(!failure.success);
        assert_eq!(failure.exit_code, 1);

        // Test reboot required
        let reboot = ScriptResult::failure(
            3010,
            "out".to_string(),
            "".to_string(),
            Duration::from_secs(1),
        );
        assert!(reboot.success); // 3010 is still success
        assert!(reboot.requires_reboot);
    }

    #[test]
    fn test_output_parsing() {
        let stdout = r#"
[PASS] Test 1 passed
[FAIL] Test 2 failed
[WARN] Warning message
Regular output
>> Progress message
"#;
        let parsed = ParsedScriptOutput::parse(stdout, "");

        assert_eq!(parsed.summary.passed, 1);
        assert_eq!(parsed.summary.failed, 1);
        assert_eq!(parsed.summary.warnings, 1);
    }

    #[test]
    fn test_line_classification() {
        assert_eq!(
            ParsedScriptOutput::classify_line("[PASS] Test").0,
            LineType::Success
        );
        assert_eq!(
            ParsedScriptOutput::classify_line("[FAIL] Test").0,
            LineType::Error
        );
        assert_eq!(
            ParsedScriptOutput::classify_line("[WARN] Test").0,
            LineType::Warning
        );
        assert_eq!(
            ParsedScriptOutput::classify_line(">> Progress").0,
            LineType::Progress
        );
        assert_eq!(
            ParsedScriptOutput::classify_line("Regular line").0,
            LineType::Unknown
        );
    }

    #[test]
    fn test_execution_summary() {
        let mut summary = ScriptExecutionSummary::new();

        summary.add_result(ScriptExecutionResult {
            script_name: "test1".to_string(),
            script_path: PathBuf::from("test1.ps1"),
            phase: ScriptPhase::PreInstall,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_secs(1),
            success: true,
            requires_reboot: false,
        });

        summary.add_result(ScriptExecutionResult {
            script_name: "test2".to_string(),
            script_path: PathBuf::from("test2.ps1"),
            phase: ScriptPhase::PostInstall,
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration: Duration::from_secs(2),
            success: false,
            requires_reboot: false,
        });

        summary.add_skipped();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.is_successful());
        assert_eq!(summary.total_duration, Duration::from_secs(3));
    }

    #[test]
    fn test_script_context() {
        let context = ScriptContext::new("test-workload", "/path/to/workload")
            .with_phase(ScriptPhase::PreInstall)
            .with_dry_run(true)
            .with_verbose(true);

        assert_eq!(context.workload_name, "test-workload");
        assert_eq!(context.phase, ScriptPhase::PreInstall);
        assert!(context.dry_run);
        assert!(context.verbose);
    }

    #[test]
    fn test_shell_availability() {
        // PowerShell should be available on Windows
        #[cfg(windows)]
        {
            assert!(Shell::PowerShell.is_available());
        }
    }
}
