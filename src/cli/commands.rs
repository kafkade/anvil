//! Command argument definitions for Anvil CLI
//!
//! This module defines the argument structures for each CLI command.

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Arguments for the `config` command
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Config subcommand
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., "defaults.shell")
        #[arg(value_name = "KEY")]
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., "defaults.shell")
        #[arg(value_name = "KEY")]
        key: String,

        /// Value to set
        #[arg(value_name = "VALUE")]
        value: String,
    },

    /// List all configuration values
    List {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
    },

    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Open configuration file in default editor
    Edit,

    /// Show configuration file path
    Path,
}

/// Arguments for the `status` command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Name of the workload to show status for (optional, shows all if not specified)
    #[arg(value_name = "WORKLOAD")]
    pub workload: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub output: OutputFormat,

    /// Show detailed status including timestamps
    #[arg(short, long)]
    pub long: bool,

    /// Clear stored state for the specified workload
    #[arg(long)]
    pub clear: bool,
}

/// Arguments for the `install` command
#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Name of the workload to install (or path to workload.yaml)
    #[arg(value_name = "WORKLOAD")]
    pub workload: String,

    /// Show what would be done without making changes
    #[arg(short, long)]
    pub dry_run: bool,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub force: bool,

    /// Only install packages, skip files and scripts
    #[arg(short, long)]
    pub packages_only: bool,

    /// Skip package installation
    #[arg(long)]
    pub skip_packages: bool,

    /// Skip file operations
    #[arg(long)]
    pub skip_files: bool,

    /// Don't backup existing files before overwriting
    #[arg(long)]
    pub no_backup: bool,

    /// Upgrade existing packages to specified versions
    #[arg(long)]
    pub upgrade: bool,

    /// Retry only failed packages from previous run
    #[arg(long)]
    pub retry_failed: bool,

    /// Run installations in parallel where safe
    #[arg(long)]
    pub parallel: bool,

    /// Number of parallel package installations
    #[arg(short, long, value_name = "N", default_value = "4")]
    pub jobs: usize,

    /// Global timeout for operations in seconds
    #[arg(long, value_name = "SECONDS", default_value = "3600")]
    pub timeout: u64,

    /// Only process files, skip packages and scripts
    #[arg(long)]
    pub files_only: bool,

    /// Force overwrite files without checking hash
    #[arg(long)]
    pub force_files: bool,

    /// Disable interactive TUI dashboard (use plain progress output)
    #[arg(long)]
    pub no_tui: bool,
}

/// Arguments for the `health` command
#[derive(Args, Debug)]
pub struct HealthArgs {
    /// Name of the workload to check against
    #[arg(value_name = "WORKLOAD")]
    pub workload: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub output: OutputFormat,

    /// Write report to file instead of stdout
    #[arg(short, long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Stop on first failure
    #[arg(long)]
    pub fail_fast: bool,

    /// Only check packages
    #[arg(long)]
    pub packages_only: bool,

    /// Only check files
    #[arg(long)]
    pub files_only: bool,

    /// Only evaluate declarative assertions
    #[arg(long)]
    pub assertions_only: bool,

    /// Treat warnings as errors
    #[arg(short, long)]
    pub strict: bool,

    /// Attempt to install missing packages
    #[arg(long)]
    pub fix: bool,

    /// Update packages with available updates
    #[arg(long)]
    pub update: bool,

    /// Skip cache and query winget directly
    #[arg(long)]
    pub no_cache: bool,

    /// Show file differences for modified files
    #[arg(long)]
    pub show_diff: bool,
}

/// Arguments for the `list` command
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Include built-in and custom workloads
    #[arg(short, long)]
    pub all: bool,

    /// Show detailed information
    #[arg(short, long)]
    pub long: bool,

    /// Search for workloads in additional path
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Show all discovered paths including shadowed duplicates
    #[arg(long)]
    pub all_paths: bool,

    /// Output format
    #[arg(short, long, value_enum)]
    pub output: Option<OutputFormat>,
}

/// Arguments for the `show` command
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Name of the workload to display
    #[arg(value_name = "WORKLOAD")]
    pub workload: String,

    /// Show resolved configuration (with inheritance applied)
    #[arg(short, long)]
    pub resolved: bool,

    /// Show inheritance tree visualization
    #[arg(long)]
    pub show_inheritance: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "yaml")]
    pub output: ConfigOutputFormat,
}

/// Arguments for the `validate` command
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to workload.yaml file or workload directory
    #[arg(value_name = "PATH", required_unless_present = "schema")]
    pub path: Option<PathBuf>,

    /// Enable strict validation mode
    #[arg(long)]
    pub strict: bool,

    /// Output JSON schema for workload definitions
    #[arg(long)]
    pub schema: bool,

    /// Validate script syntax using PowerShell parser
    #[arg(long)]
    pub check_scripts: bool,

    /// Only validate scripts (skip other validation)
    #[arg(long)]
    pub scripts_only: bool,
}

/// Arguments for the `init` command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Name for the new workload
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Base template to use
    #[arg(short, long, value_enum, default_value = "standard")]
    pub template: WorkloadTemplate,

    /// Parent workload to extend
    #[arg(short, long, value_name = "PARENT")]
    pub extends: Option<String>,

    /// Output directory
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

/// Arguments for the `completions` command
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    pub shell: ShellType,
}

/// Output format for health checks and listings
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// Formatted table output
    #[default]
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// HTML report
    Html,
}

/// Output format for configuration display
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ConfigOutputFormat {
    /// YAML format
    #[default]
    Yaml,
    /// JSON format
    Json,
}

/// Workload template types for initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum WorkloadTemplate {
    /// Minimal workload with just metadata
    Minimal,
    /// Standard workload with common sections
    #[default]
    Standard,
    /// Full workload with all sections and examples
    Full,
}

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellType {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    Powershell,
    /// Elvish shell
    Elvish,
}

/// Arguments for the `backup` command
#[derive(Args, Debug)]
pub struct BackupArgs {
    /// Backup subcommand
    #[command(subcommand)]
    pub command: BackupCommand,
}

/// Arguments for the `report` command
#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Workload to generate report for
    #[arg(value_name = "WORKLOAD")]
    pub workload: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "html")]
    pub format: OutputFormat,

    /// Output file path
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Include system information in report
    #[arg(long)]
    pub include_system: bool,

    /// Include installation history
    #[arg(long)]
    pub include_history: bool,
}

/// Backup subcommands
#[derive(Subcommand, Debug)]
pub enum BackupCommand {
    /// Create a new backup
    Create {
        /// Name for the backup
        #[arg(short, long)]
        name: Option<String>,

        /// Only backup files related to workload
        #[arg(short, long)]
        workload: Option<String>,

        /// Include winget package list export
        #[arg(long)]
        include_packages: bool,

        /// Create compressed archive
        #[arg(long)]
        compress: bool,
    },

    /// List all backups
    List {
        /// Filter by workload name
        #[arg(short, long)]
        workload: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Show detailed information
        #[arg(short, long)]
        long: bool,
    },

    /// Show details for a specific backup
    Show {
        /// Backup ID
        #[arg(value_name = "ID")]
        id: String,
    },

    /// Restore a backup
    Restore {
        /// Backup ID to restore (or use --workload)
        #[arg(value_name = "ID", required_unless_present = "workload")]
        id: Option<String>,

        /// Restore all backups for a workload
        #[arg(short, long)]
        workload: Option<String>,

        /// Show what would be done without making changes
        #[arg(short, long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(short, long)]
        force: bool,
    },

    /// Clean old backups
    Clean {
        /// Remove backups older than N days
        #[arg(long, value_name = "DAYS", default_value = "30")]
        older_than: u32,

        /// Show what would be done without making changes
        #[arg(short, long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(short, long)]
        force: bool,
    },

    /// Verify backup integrity
    Verify {
        /// Only verify backups for a specific workload
        #[arg(short, long)]
        workload: Option<String>,

        /// Fix issues by removing corrupted/missing entries
        #[arg(long)]
        fix: bool,
    },
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Html => write!(f, "html"),
        }
    }
}

impl std::fmt::Display for ConfigOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigOutputFormat::Yaml => write!(f, "yaml"),
            ConfigOutputFormat::Json => write!(f, "json"),
        }
    }
}

impl std::fmt::Display for WorkloadTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadTemplate::Minimal => write!(f, "minimal"),
            WorkloadTemplate::Standard => write!(f, "standard"),
            WorkloadTemplate::Full => write!(f, "full"),
        }
    }
}

/// Arguments for the `source` command
#[derive(Args, Debug)]
pub struct SourceArgs {
    /// Source subcommand
    #[command(subcommand)]
    pub command: SourceCommand,
}

/// Source subcommands
#[derive(Subcommand, Debug)]
pub enum SourceCommand {
    /// List all configured sources
    List {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
    },

    /// Add a workload source (local path or git URL)
    Add {
        /// Local path or git repository URL
        #[arg(value_name = "PATH_OR_URL")]
        location: String,

        /// Name for the source (auto-detected from URL or path if not specified)
        #[arg(short, long)]
        name: Option<String>,

        /// Subdirectory within the source to search for workloads
        #[arg(short, long, value_name = "DIR")]
        path: Option<String>,

        /// Git branch or tag to track (remote sources only)
        #[arg(short = 'r', long = "ref", value_name = "REF")]
        git_ref: Option<String>,
    },

    /// Remove a workload source
    Remove {
        /// Name of the source to remove
        #[arg(value_name = "NAME")]
        name: String,

        /// Also delete cloned files for remote sources
        #[arg(long)]
        delete: bool,
    },

    /// Show sync status for all sources
    Status {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
    },

    /// Sync remote sources (git pull)
    Sync {
        /// Name of a specific source to sync (syncs all if omitted)
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
}

/// Arguments for the `registry` command
#[derive(Args, Debug)]
pub struct RegistryArgs {
    /// Registry subcommand
    #[command(subcommand)]
    pub command: RegistryCommand,
}

/// Registry subcommands
#[derive(Subcommand, Debug)]
pub enum RegistryCommand {
    /// List all workloads in the registry
    List {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Force refresh from remote registry
        #[arg(long)]
        refresh: bool,
    },

    /// Search for workloads in the registry
    Search {
        /// Search query (matches name, description, tags, author)
        #[arg(value_name = "QUERY")]
        query: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Force refresh from remote registry
        #[arg(long)]
        refresh: bool,
    },

    /// Add a workload from the registry as a source
    Add {
        /// Name of the registry workload to add
        #[arg(value_name = "NAME")]
        name: String,

        /// Override the source name
        #[arg(long, value_name = "NAME")]
        source_name: Option<String>,

        /// Override the git ref from the registry entry
        #[arg(short = 'r', long = "ref", value_name = "REF")]
        git_ref: Option<String>,

        /// Force refresh from remote registry
        #[arg(long)]
        refresh: bool,
    },
}
