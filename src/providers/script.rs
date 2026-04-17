//! Script execution provider for Anvil
//!
//! This module handles execution of PowerShell scripts for:
//! - Pre-installation scripts
//! - Post-installation scripts
//! - Health check validation scripts
//!
//! Features:
//! - Proper timeout handling with process termination
//! - Elevation detection and requirements
#![allow(dead_code)]
//! - Output streaming for real-time feedback
//! - Environment variable injection
//! - PowerShell Core (pwsh) support
//! - Structured output parsing

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use thiserror::Error;

/// Errors that can occur during script execution
#[derive(Error, Debug)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPhase {
    PreInstall,
    PostInstall,
    HealthCheck,
    Validation,
}

impl std::fmt::Display for ScriptPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptPhase::PreInstall => write!(f, "pre_install"),
            ScriptPhase::PostInstall => write!(f, "post_install"),
            ScriptPhase::HealthCheck => write!(f, "health_check"),
            ScriptPhase::Validation => write!(f, "validation"),
        }
    }
}

/// Configuration for script execution
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
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: String,
    pub line_type: LineType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Summary extracted from script output
#[derive(Debug, Clone, Default)]
pub struct ScriptSummary {
    pub passed: u32,
    pub failed: u32,
    pub warnings: u32,
}

/// Parsed script output with structured data
#[derive(Debug, Clone)]
pub struct ParsedScriptOutput {
    pub lines: Vec<OutputLine>,
    pub summary: ScriptSummary,
    pub structured_data: Option<serde_json::Value>,
}

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

/// Output stream identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Trait for handling script events
pub trait ScriptEventHandler: Send + Sync {
    /// Called when script starts executing
    fn on_start(&self, _script: &str, _phase: ScriptPhase) {}

    /// Called when a line of output is received
    fn on_output(&self, _line: &str, _stream: OutputStream) {}

    /// Called when script completes
    fn on_complete(&self, _result: &ScriptResult) {}

    /// Called when an error occurs
    fn on_error(&self, _error: &ScriptError) {}
}

/// Default event handler that does nothing
struct NoOpEventHandler;
impl ScriptEventHandler for NoOpEventHandler {}

/// Context for script execution
#[derive(Debug, Clone)]
pub struct ScriptContext {
    pub workload_name: String,
    pub workload_path: PathBuf,
    pub dry_run: bool,
    pub verbose: bool,
    pub phase: ScriptPhase,
}

impl ScriptContext {
    pub fn new(workload_name: impl Into<String>, workload_path: impl Into<PathBuf>) -> Self {
        Self {
            workload_name: workload_name.into(),
            workload_path: workload_path.into(),
            dry_run: false,
            verbose: false,
            phase: ScriptPhase::HealthCheck,
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

/// Script execution provider
pub struct ScriptProvider {
    /// Whether to run in dry-run mode
    dry_run: bool,

    /// Verbose output
    verbose: bool,

    /// Base path for resolving relative script paths
    base_path: Option<PathBuf>,

    /// Event handler for script events
    event_handler: Arc<dyn ScriptEventHandler>,

    /// Cached elevation status
    is_elevated: Option<bool>,
}

impl std::fmt::Debug for ScriptProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptProvider")
            .field("dry_run", &self.dry_run)
            .field("verbose", &self.verbose)
            .field("base_path", &self.base_path)
            .field("is_elevated", &self.is_elevated)
            .finish()
    }
}

impl ScriptProvider {
    /// Create a new script provider
    pub fn new() -> Self {
        Self {
            dry_run: false,
            verbose: false,
            base_path: None,
            event_handler: Arc::new(NoOpEventHandler),
            is_elevated: None,
        }
    }

    /// Enable dry-run mode
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the base path for resolving relative script paths
    pub fn with_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_path = Some(path.into());
        self
    }

    /// Set event handler for script events
    pub fn with_event_handler(mut self, handler: Arc<dyn ScriptEventHandler>) -> Self {
        self.event_handler = handler;
        self
    }

    /// Resolve a script path, making it absolute if necessary
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(base) = &self.base_path {
            base.join(path)
        } else {
            path.to_path_buf()
        }
    }

    /// Check if the current process is running with elevated privileges
    pub fn is_elevated(&mut self) -> bool {
        if let Some(elevated) = self.is_elevated {
            return elevated;
        }

        let elevated = Self::check_elevation();
        self.is_elevated = Some(elevated);
        elevated
    }

    /// Check elevation status (static helper)
    fn check_elevation() -> bool {
        // Use PowerShell to check if running as admin
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "True")
            .unwrap_or(false)
    }

    /// Inject environment variables into script config
    pub fn inject_environment_variables(&self, config: &mut ScriptConfig, context: &ScriptContext) {
        // Anvil-specific variables
        config
            .environment
            .insert("ANVIL_WORKLOAD".to_string(), context.workload_name.clone());
        config.environment.insert(
            "ANVIL_WORKLOAD_PATH".to_string(),
            context.workload_path.display().to_string(),
        );
        config
            .environment
            .insert("ANVIL_DRY_RUN".to_string(), context.dry_run.to_string());
        config
            .environment
            .insert("ANVIL_VERBOSE".to_string(), context.verbose.to_string());
        config
            .environment
            .insert("ANVIL_PHASE".to_string(), context.phase.to_string());
        config.environment.insert(
            "ANVIL_VERSION".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
    }

    /// Execute a script with the given configuration
    pub fn execute(&mut self, config: &ScriptConfig) -> Result<ScriptResult, ScriptError> {
        let script_path = self.resolve_path(&config.path);

        // Check if script exists
        if !script_path.exists() {
            return Err(ScriptError::NotFound(script_path));
        }

        // Check shell availability
        if config.shell == Shell::Pwsh && !Shell::Pwsh.is_available() {
            return Err(ScriptError::PwshNotAvailable);
        }

        // Check elevation requirements
        if config.elevated && !self.is_elevated() {
            return Err(ScriptError::ElevationRequired {
                path: script_path.clone(),
            });
        }

        // In dry-run mode, just report what would be done
        if self.dry_run {
            tracing::info!("Would execute script: {}", script_path.display());
            return Ok(ScriptResult::success(
                format!("[DRY RUN] Would execute: {}", script_path.display()),
                Duration::from_secs(0),
            ));
        }

        // Execute based on output mode
        match config.output_mode {
            OutputMode::Capture => self.execute_captured(config, &script_path),
            OutputMode::Stream | OutputMode::Both => self.execute_streaming(config, &script_path),
        }
    }

    /// Execute script with captured output (no streaming)
    fn execute_captured(
        &self,
        config: &ScriptConfig,
        script_path: &Path,
    ) -> Result<ScriptResult, ScriptError> {
        let mut cmd = self.build_command(config, script_path)?;

        let start = Instant::now();
        let output = self.execute_with_timeout(&mut cmd, config.timeout, script_path)?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if output.status.success() || exit_code == 3010 {
            Ok(ScriptResult::success(stdout, duration))
        } else {
            Ok(ScriptResult::failure(exit_code, stdout, stderr, duration))
        }
    }

    /// Execute script with streaming output
    fn execute_streaming(
        &self,
        config: &ScriptConfig,
        script_path: &Path,
    ) -> Result<ScriptResult, ScriptError> {
        use colored::Colorize;

        let mut cmd = self.build_command_for_streaming(config, script_path)?;

        let start = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|e| ScriptError::SpawnFailed(format!("Failed to spawn process: {}", e)))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let captured_stdout = Arc::new(Mutex::new(String::new()));
        let captured_stderr = Arc::new(Mutex::new(String::new()));

        let prefix = config.output_prefix.clone().unwrap_or_default();
        let capture_output = config.output_mode == OutputMode::Both;
        let event_handler = self.event_handler.clone();

        // Spawn thread for stdout
        let stdout_captured = captured_stdout.clone();
        let prefix_stdout = prefix.clone();
        let stdout_thread = if let Some(stdout) = stdout {
            let handler = event_handler.clone();
            Some(thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    // Stream to console with colored prefix
                    if !prefix_stdout.is_empty() {
                        print!("{} ", prefix_stdout.dimmed());
                    }
                    println!("{}", line);

                    // Capture if needed
                    if capture_output {
                        let mut captured = stdout_captured.lock().unwrap();
                        captured.push_str(&line);
                        captured.push('\n');
                    }

                    // Notify handler
                    handler.on_output(&line, OutputStream::Stdout);
                }
            }))
        } else {
            None
        };

        // Spawn thread for stderr
        let stderr_captured = captured_stderr.clone();
        let prefix_stderr = prefix;
        let stderr_thread = if let Some(stderr) = stderr {
            let handler = event_handler;
            Some(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    // Stream to console with colored prefix
                    if !prefix_stderr.is_empty() {
                        eprint!("{} ", prefix_stderr.yellow());
                    }
                    eprintln!("{}", line.yellow());

                    // Capture if needed
                    if capture_output {
                        let mut captured = stderr_captured.lock().unwrap();
                        captured.push_str(&line);
                        captured.push('\n');
                    }

                    // Notify handler
                    handler.on_output(&line, OutputStream::Stderr);
                }
            }))
        } else {
            None
        };

        // Wait for process with timeout
        let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();
        let child_arc = Arc::new(Mutex::new(child));
        let child_for_thread = child_arc.clone();

        let wait_thread = thread::spawn(move || {
            let mut child = child_for_thread.lock().unwrap();
            match child.wait() {
                Ok(status) => {
                    let _ = tx.send(status.code().unwrap_or(-1));
                }
                Err(_) => {
                    let _ = tx.send(-1);
                }
            }
        });

        let exit_code = match rx.recv_timeout(config.timeout) {
            Ok(code) => code,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Kill the process
                if let Ok(mut child) = child_arc.lock() {
                    let _ = child.kill();
                }
                return Err(ScriptError::Timeout {
                    path: script_path.to_path_buf(),
                    timeout_seconds: config.timeout.as_secs(),
                });
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => -1,
        };

        // Wait for threads to finish
        let _ = wait_thread.join();
        if let Some(t) = stdout_thread {
            let _ = t.join();
        }
        if let Some(t) = stderr_thread {
            let _ = t.join();
        }

        let duration = start.elapsed();
        let stdout = captured_stdout.lock().unwrap().clone();
        let stderr = captured_stderr.lock().unwrap().clone();

        if exit_code == 0 || exit_code == 3010 {
            Ok(ScriptResult::success(stdout, duration))
        } else {
            Ok(ScriptResult::failure(exit_code, stdout, stderr, duration))
        }
    }

    /// Build the command for script execution (captured mode)
    fn build_command(
        &self,
        config: &ScriptConfig,
        script_path: &Path,
    ) -> Result<Command, ScriptError> {
        let mut cmd = match config.shell {
            Shell::PowerShell => {
                let mut c = Command::new("powershell.exe");
                c.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ]);
                c.arg(script_path);
                c
            }
            Shell::Pwsh => {
                let mut c = Command::new("pwsh.exe");
                c.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ]);
                c.arg(script_path);
                c
            }
            Shell::Cmd => {
                let mut c = Command::new("cmd.exe");
                c.args(["/c"]);
                c.arg(script_path);
                c
            }
            Shell::Bash => {
                let mut c = Command::new("bash");
                c.arg(script_path);
                c
            }
        };

        // Add script arguments
        for arg in &config.arguments {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        } else if let Some(base_path) = &self.base_path {
            // Default to base path (workload scripts directory)
            cmd.current_dir(base_path);
        }

        // Add environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        Ok(cmd)
    }

    /// Build command for streaming mode
    fn build_command_for_streaming(
        &self,
        config: &ScriptConfig,
        script_path: &Path,
    ) -> Result<Command, ScriptError> {
        // Same as regular command but with piped stdout/stderr for reading
        self.build_command(config, script_path)
    }

    /// Execute a command with timeout
    fn execute_with_timeout(
        &self,
        cmd: &mut Command,
        timeout: Duration,
        script_path: &Path,
    ) -> Result<Output, ScriptError> {
        let child = cmd
            .spawn()
            .map_err(|e| ScriptError::SpawnFailed(format!("Failed to spawn process: {}", e)))?;

        let (tx, rx) = mpsc::channel::<Result<Output, std::io::Error>>();

        // We need to move the child into a thread to wait on it
        let handle = thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        // Wait with timeout
        match rx.recv_timeout(timeout) {
            Ok(result) => {
                let _ = handle.join();
                result.map_err(ScriptError::from)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Process timed out - we can't kill it from here since child was moved
                // The thread will eventually complete and be cleaned up
                // For now, return timeout error
                Err(ScriptError::Timeout {
                    path: script_path.to_path_buf(),
                    timeout_seconds: timeout.as_secs(),
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(ScriptError::ThreadError),
        }
    }

    /// Execute with proper timeout and process killing
    pub fn execute_with_kill_on_timeout(
        &self,
        config: &ScriptConfig,
        script_path: &Path,
    ) -> Result<ScriptResult, ScriptError> {
        let mut cmd = self.build_command(config, script_path)?;

        let start = Instant::now();

        let child = cmd
            .spawn()
            .map_err(|e| ScriptError::SpawnFailed(format!("Failed to spawn process: {}", e)))?;

        // Wrap child in Arc<Mutex> so we can kill it from timeout handler
        let child_arc = Arc::new(Mutex::new(None::<Child>));
        let child_for_kill = child_arc.clone();

        // Channel for result
        let (tx, rx) = mpsc::channel();

        // Spawn thread to wait for child
        let wait_thread = thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        // Wait with timeout
        match rx.recv_timeout(config.timeout) {
            Ok(result) => {
                let _ = wait_thread.join();
                let output = result?;
                let duration = start.elapsed();

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                if output.status.success() || exit_code == 3010 {
                    Ok(ScriptResult::success(stdout, duration))
                } else {
                    Ok(ScriptResult::failure(exit_code, stdout, stderr, duration))
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Try to kill the process
                if let Ok(mut guard) = child_for_kill.lock() {
                    if let Some(ref mut child) = *guard {
                        let _ = child.kill();
                    }
                }

                Err(ScriptError::Timeout {
                    path: script_path.to_path_buf(),
                    timeout_seconds: config.timeout.as_secs(),
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(ScriptError::ThreadError),
        }
    }

    /// Validate a script's syntax without executing it
    pub fn validate_syntax(&self, config: &ScriptConfig) -> Result<(), ScriptError> {
        let script_path = self.resolve_path(&config.path);

        if !script_path.exists() {
            return Err(ScriptError::NotFound(script_path));
        }

        // For PowerShell scripts, use parser to check syntax
        if config.shell == Shell::PowerShell || config.shell == Shell::Pwsh {
            let shell_exe = config.shell.executable();

            // Use the script path directly - PowerShell will handle it
            let path_str = script_path.display().to_string();

            // Build command with proper argument passing (not through shell)
            let mut cmd = Command::new(shell_exe);
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
            ]);

            // Build the PowerShell command as a single argument
            // Using & { } to create a script block avoids variable expansion issues
            let ps_script = format!(
                r#"& {{ $script = '{}'; $errors = $null; $null = [System.Management.Automation.Language.Parser]::ParseFile($script, [ref]$null, [ref]$errors); if ($errors) {{ foreach ($err in $errors) {{ Write-Host "Line $($err.Extent.StartLineNumber): $($err.Message)" }}; exit 1 }}; exit 0 }}"#,
                path_str.replace('\'', "''")
            );
            cmd.arg(&ps_script);

            let output = cmd.output()?;
            if !output.status.success() {
                // Get error message from stdout (where we Write-Host) or stderr
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Prefer stdout (our Write-Host output) over stderr
                let message = if !stdout.trim().is_empty() {
                    stdout.trim().to_string()
                } else if !stderr.trim().is_empty() {
                    // Extract meaningful part from stderr
                    stderr
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .take(3)
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    "Unknown syntax error".to_string()
                };

                return Err(ScriptError::SyntaxError {
                    path: script_path,
                    message,
                });
            }
        }

        // For other shells, just check if file is readable
        std::fs::read_to_string(&script_path)?;

        Ok(())
    }

    /// Check if PowerShell is available on the system
    pub fn is_powershell_available() -> bool {
        Shell::PowerShell.is_available()
    }

    /// Check if PowerShell Core is available
    pub fn is_pwsh_available() -> bool {
        Shell::Pwsh.is_available()
    }

    /// Get the preferred shell (pwsh if available, otherwise powershell)
    pub fn preferred_powershell() -> Shell {
        if Shell::Pwsh.is_available() {
            Shell::Pwsh
        } else {
            Shell::PowerShell
        }
    }
}

impl Default for ScriptProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl super::ProviderStatus for ScriptProvider {
    fn is_available(&self) -> bool {
        Self::is_powershell_available()
    }

    fn name(&self) -> &'static str {
        "script"
    }

    fn version(&self) -> Option<String> {
        // Get PowerShell version
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "$PSVersionTable.PSVersion.ToString()",
            ])
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
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
