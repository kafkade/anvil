//! Anvil - Declarative Workstation Configuration Management
//!
//! A declarative configuration management tool for developer workstations that automates
//! the setup and validation of development environments through composable workload definitions.

mod assertions;
mod cli;
mod conditions;
mod config;
mod operations;
mod providers;
mod state;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    // Initialize logging
    init_logging();

    // Parse command line arguments
    let cli = Cli::parse();

    // Set verbosity level
    if cli.verbose > 0 {
        tracing::debug!("Verbosity level: {}", cli.verbose);
    }

    // Execute the requested command
    match &cli.command {
        Commands::Install(args) => {
            operations::install::execute(args, &cli)?;
        }
        Commands::Health(args) => {
            operations::health::execute(args, &cli)?;
        }
        Commands::List(args) => {
            operations::list::execute(args, &cli)?;
        }
        Commands::Show(args) => {
            operations::show::execute(args, &cli)?;
        }
        Commands::Validate(args) => {
            operations::validate::execute(args, &cli)?;
        }
        Commands::Init(args) => {
            operations::init::execute(args, &cli)?;
        }
        Commands::Status(args) => {
            operations::status::execute(args, &cli)?;
        }
        Commands::Completions(args) => {
            cli::completions::generate_completions(args)?;
        }
        Commands::Backup(args) => {
            operations::backup::execute(args, &cli)?;
        }
        Commands::Config(args) => {
            operations::config::execute(args, &cli)?;
        }
    }

    Ok(())
}

/// Initialize the logging/tracing subsystem
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("anvil=info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).without_time())
        .with(filter)
        .init();
}
