//! Anvil - Declarative Workstation Configuration Management
//!
//! A declarative configuration management tool for developer workstations that automates
//! the setup and validation of development environments through composable workload definitions.

mod assertions;
mod cli;
mod commands;
mod conditions;
mod config;
mod operations;
mod providers;
mod state;
mod tui;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    // Show animated banner for --version/-V before clap processes it
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        cli::banner::show_version();
        return Ok(());
    }

    // Initialize logging
    init_logging();

    // Parse command line arguments
    let cli = Cli::parse();

    // Set verbosity level
    if cli.verbose > 0 {
        tracing::debug!("Verbosity level: {}", cli.verbose);
    }

    // When no subcommand is given, always launch the TUI workload browser
    let command = match cli.command {
        Some(ref cmd) => cmd,
        None => {
            launch_default_tui()?;
            return Ok(());
        }
    };

    // Execute the requested command
    match command {
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
        Commands::Source(args) => {
            operations::source::execute(args, &cli)?;
        }
        Commands::Registry(args) => {
            operations::registry::execute(args, &cli)?;
        }
    }

    Ok(())
}

/// Launch the TUI workload browser as the default experience
///
/// Called when `anvil` is run without any subcommand. Launches the
/// ratatui-based interactive browser directly, falling back to help
/// text only when stdin is not a terminal (e.g. piped or in CI).
///
/// When the user selects a workload, transitions to a detail view
/// with collapsible sections. Pressing Esc returns to the browser.
/// Pressing 'i' or 'd' exits the TUI and runs install or dry-run.
fn launch_default_tui() -> Result<()> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        use clap::CommandFactory;
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }

    use colored::Colorize;
    use tui::views::browser::BrowserOutcome;
    use tui::views::detail::DetailOutcome;

    let mut manager = config::ConfigManager::new();
    let workloads = manager
        .list_workloads()
        .context("Failed to discover workloads")?;

    if workloads.is_empty() {
        println!("{} No workloads found.", "ℹ".blue());
        println!(
            "  {} Create a workload with '{}'",
            "→".dimmed(),
            "anvil init <name>".cyan()
        );
        return Ok(());
    }

    let entries: Vec<tui::views::browser::WorkloadEntry> = workloads
        .iter()
        .map(|w| tui::views::browser::WorkloadEntry {
            name: w.name.clone(),
            version: w.version.clone(),
            description: w.description.clone(),
            extends: w.extends.clone(),
            package_count: w.package_count,
            file_count: w.file_count,
            command_count: w.command_count,
            font_count: w.font_count,
            feature_count: w.feature_count,
            assertion_count: w.assertion_count,
            source: "local".to_string(),
            path: w.path.to_string_lossy().to_string(),
        })
        .collect();

    let mut browser = tui::views::browser::WorkloadBrowser::new(entries);

    loop {
        // Run the TUI phase (browser, and optionally detail) inside a
        // scoped block so the Tui is dropped before any install runs.
        let action = (|| -> Result<TuiAction> {
            let mut tui_instance = tui::Tui::new()?;
            browser.reset();

            loop {
                match browser.run(&mut tui_instance)? {
                    BrowserOutcome::Quit => return Ok(TuiAction::Quit),
                    BrowserOutcome::Install(name, path) => {
                        return Ok(TuiAction::Install {
                            name,
                            path,
                            dry_run: false,
                        });
                    }
                    BrowserOutcome::DryRun(name, path) => {
                        return Ok(TuiAction::Install {
                            name,
                            path,
                            dry_run: true,
                        });
                    }
                    BrowserOutcome::Health(name, path) => {
                        let health_outcome = operations::health::execute_health_inline(
                            &mut tui_instance,
                            &path,
                            &name,
                        )?;
                        match health_outcome {
                            tui::views::health::HealthOutcome::Back => {
                                browser.reset();
                                continue;
                            }
                            tui::views::health::HealthOutcome::Quit => {
                                return Ok(TuiAction::Quit);
                            }
                        }
                    }
                    BrowserOutcome::Select(selected_name) => {
                        let detail = load_workload_detail(&mut manager, &selected_name);
                        let mut detail_view = tui::views::detail::DetailView::new(detail);
                        match detail_view.run(&mut tui_instance)? {
                            DetailOutcome::Back => {
                                // Return to browser
                                browser.reset();
                                continue;
                            }
                            DetailOutcome::Quit => return Ok(TuiAction::Quit),
                            DetailOutcome::Install(name, path) => {
                                return Ok(TuiAction::Install {
                                    name,
                                    path,
                                    dry_run: false,
                                });
                            }
                            DetailOutcome::DryRun(name, path) => {
                                return Ok(TuiAction::Install {
                                    name,
                                    path,
                                    dry_run: true,
                                });
                            }
                        }
                    }
                }
            }
        })();

        match action {
            Ok(TuiAction::Quit) => break,
            Ok(TuiAction::Install {
                name,
                path,
                dry_run,
            }) => {
                run_install_from_tui(&name, &path, dry_run)?;
                // After install completes, prompt before re-entering the TUI
                println!();
                println!("Press Enter to return to the workload browser...");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
            Err(_) => {
                // TUI failed to start — fall back to help text
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                break;
            }
        }
    }

    Ok(())
}

/// Internal action returned by the TUI phase
enum TuiAction {
    Quit,
    Install {
        name: String,
        path: String,
        dry_run: bool,
    },
}

/// Run `operations::install::execute` from the TUI using the workload's
/// resolved filesystem path (not just the name) so it works regardless of
/// which search path the workload lives in.
fn run_install_from_tui(workload_name: &str, workload_path: &str, dry_run: bool) -> Result<()> {
    let args = cli::commands::InstallArgs {
        workload: workload_path.to_string(),
        dry_run,
        force: false,
        packages_only: false,
        skip_packages: false,
        skip_files: false,
        no_backup: false,
        upgrade: false,
        retry_failed: false,
        parallel: false,
        jobs: 4,
        timeout: 3600,
        files_only: false,
        force_files: false,
        no_tui: false,
    };

    let cli_instance = Cli {
        command: None,
        verbose: 0,
        quiet: false,
        no_color: false,
        config: None,
    };

    operations::install::execute(&args, &cli_instance)
        .with_context(|| format!("Failed to install workload '{}'", workload_name))
}

/// Build a `WorkloadDetail` from the full workload definition.
///
/// Tries to load the workload YAML via `ConfigManager`. If loading fails
/// (e.g. the workload was discovered but can't be parsed), returns a
/// minimal detail populated from the name alone.
fn load_workload_detail(
    manager: &mut config::ConfigManager,
    name: &str,
) -> tui::views::detail::WorkloadDetail {
    let found_path = manager.find_workload(name);
    let path_str = found_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let result = match found_path {
        Some(p) => manager.load_workload(&p),
        None => {
            return tui::views::detail::WorkloadDetail {
                name: name.to_string(),
                version: "?".to_string(),
                description: "Workload not found".to_string(),
                extends: Vec::new(),
                packages: Vec::new(),
                files: Vec::new(),
                commands: Vec::new(),
                assertions: Vec::new(),
                fonts: Vec::new(),
                features: Vec::new(),
                environment: Vec::new(),
                path: String::new(),
            };
        }
    };

    match result {
        Ok(w) => {
            let packages: Vec<String> = w
                .packages
                .as_ref()
                .and_then(|p| p.winget.as_ref())
                .map(|pkgs| pkgs.iter().map(|p| p.id.clone()).collect())
                .unwrap_or_default();

            let files: Vec<tui::views::detail::FileEntry> = w
                .files
                .as_ref()
                .map(|fs| {
                    fs.iter()
                        .map(|f| tui::views::detail::FileEntry {
                            source: f.source.clone(),
                            destination: f.destination.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let commands: Vec<tui::views::detail::CommandEntry> = {
                let mut cmds = Vec::new();
                if let Some(ref cb) = w.commands {
                    if let Some(ref pre) = cb.pre_install {
                        for c in pre {
                            cmds.push(tui::views::detail::CommandEntry {
                                name: c.description.clone().unwrap_or_else(|| c.run.clone()),
                                phase: "pre_install".to_string(),
                            });
                        }
                    }
                    if let Some(ref post) = cb.post_install {
                        for c in post {
                            cmds.push(tui::views::detail::CommandEntry {
                                name: c.description.clone().unwrap_or_else(|| c.run.clone()),
                                phase: "post_install".to_string(),
                            });
                        }
                    }
                }
                cmds
            };

            let assertions: Vec<String> = w
                .assertions
                .as_ref()
                .map(|a| a.iter().map(|a| a.name.clone()).collect())
                .unwrap_or_default();

            let fonts: Vec<String> = w
                .fonts
                .as_ref()
                .map(|fs| {
                    fs.iter()
                        .map(|f| format!("{} v{}", f.name, f.version))
                        .collect()
                })
                .unwrap_or_default();

            let features: Vec<String> = w
                .features
                .as_ref()
                .map(|fs| {
                    fs.iter()
                        .map(|f| {
                            format!(
                                "{} ({})",
                                f.name,
                                f.description.as_deref().unwrap_or(&f.feature_type)
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            let environment: Vec<String> = {
                let mut env = Vec::new();
                if let Some(ref e) = w.environment {
                    if let Some(ref vars) = e.variables {
                        for v in vars {
                            env.push(format!("{}={}", v.name, v.value));
                        }
                    }
                    if let Some(ref paths) = e.path_additions {
                        for p in paths {
                            env.push(format!("PATH += {}", p));
                        }
                    }
                }
                env
            };

            tui::views::detail::WorkloadDetail {
                name: w.name,
                version: w.version,
                description: w.description,
                extends: w.extends.unwrap_or_default(),
                packages,
                files,
                commands,
                assertions,
                fonts,
                features,
                environment,
                path: path_str.clone(),
            }
        }
        Err(_) => tui::views::detail::WorkloadDetail {
            name: name.to_string(),
            version: "?".to_string(),
            description: "Failed to load workload details".to_string(),
            extends: Vec::new(),
            packages: Vec::new(),
            files: Vec::new(),
            commands: Vec::new(),
            assertions: Vec::new(),
            fonts: Vec::new(),
            features: Vec::new(),
            environment: Vec::new(),
            path: path_str,
        },
    }
}

/// Initialize the logging/tracing subsystem
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("anvil=info"));

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .without_time()
                .with_writer(std::io::stderr),
        )
        .with(filter)
        .init();
}
