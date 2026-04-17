//! CLI module - Command line interface definitions and handlers
//!
//! This module defines the CLI structure for Anvil using clap's derive macros.

pub mod banner;
pub mod commands;
pub mod completions;
#[allow(dead_code)]
pub mod formats;
pub mod output;
pub mod progress;

use clap::{Parser, Subcommand};

/// Anvil - Declarative Workstation Configuration Management
#[derive(Parser, Debug)]
#[command(name = "anvil")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Increase output verbosity (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Use custom configuration file
    #[arg(short, long, value_name = "PATH", global = true)]
    pub config: Option<std::path::PathBuf>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Apply a workload configuration to the system
    Install(commands::InstallArgs),

    /// Check system health against a workload definition
    Health(commands::HealthArgs),

    /// List available workloads
    List(commands::ListArgs),

    /// Display detailed workload information
    Show(commands::ShowArgs),

    /// Validate workload definition syntax
    Validate(commands::ValidateArgs),

    /// Initialize a new workload template
    Init(commands::InitArgs),

    /// Show installation status and state
    Status(commands::StatusArgs),

    /// Generate shell completions
    Completions(commands::CompletionsArgs),

    /// Manage file backups
    Backup(commands::BackupArgs),

    /// Manage global configuration
    Config(commands::ConfigArgs),
}

impl Cli {
    /// Returns true if colored output should be disabled
    pub fn should_disable_color(&self) -> bool {
        self.no_color || std::env::var("NO_COLOR").is_ok()
    }

    /// Returns the effective verbosity level (0 = normal, 1+ = verbose)
    pub fn verbosity_level(&self) -> u8 {
        if self.quiet {
            0
        } else {
            self.verbose.saturating_add(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        // Verify the CLI configuration is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_verbosity_level() {
        let cli = Cli {
            verbose: 2,
            quiet: false,
            config: None,
            no_color: false,
            command: Commands::List(commands::ListArgs {
                all: false,
                long: false,
                path: None,
                all_paths: false,
                output: None,
            }),
        };
        assert_eq!(cli.verbosity_level(), 3);
    }

    #[test]
    fn test_quiet_overrides_verbose() {
        let cli = Cli {
            verbose: 0,
            quiet: true,
            config: None,
            no_color: false,
            command: Commands::List(commands::ListArgs {
                all: false,
                long: false,
                path: None,
                all_paths: false,
                output: None,
            }),
        };
        assert_eq!(cli.verbosity_level(), 0);
    }
}
