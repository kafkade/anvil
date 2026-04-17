//! Status operation module
//!
//! This module implements the `anvil status` command which displays
//! the current installation state and cached information for workloads.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use colored::Colorize;

use crate::cli::commands::StatusArgs;
use crate::cli::output::{print_info, print_success, print_warning};
use crate::cli::Cli;
use crate::state::{get_state_dir, InstallationState, PackageCache, PackageStatus};

/// Execute the status command
pub fn execute(args: &StatusArgs, cli: &Cli) -> Result<()> {
    let use_color = !cli.should_disable_color();

    // Handle --clear flag
    if args.clear {
        return clear_state(&args.workload, use_color);
    }

    // Show status for specific workload or all
    if let Some(ref workload_name) = args.workload {
        show_workload_status(workload_name, args, use_color)?;
    } else {
        show_all_status(args, use_color)?;
    }

    Ok(())
}

/// Clear state for a workload
fn clear_state(workload: &Option<String>, _use_color: bool) -> Result<()> {
    if let Some(ref name) = workload {
        InstallationState::delete(name)?;
        print_success(&format!("Cleared state for workload: {}", name));
    } else {
        // Clear all state files
        let state_dir = get_state_dir()?;
        let entries = std::fs::read_dir(&state_dir)
            .with_context(|| format!("Failed to read state directory: {}", state_dir.display()))?;

        let mut count = 0;
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .map(|e| e == "json")
                .unwrap_or(false)
                && std::fs::remove_file(entry.path()).is_ok() {
                    count += 1;
                }
        }

        // Also clear package cache
        PackageCache::delete()?;

        print_success(&format!(
            "Cleared {} state file(s) and package cache",
            count
        ));
    }

    Ok(())
}

/// Show status for a specific workload
fn show_workload_status(workload_name: &str, args: &StatusArgs, use_color: bool) -> Result<()> {
    match InstallationState::load(workload_name)? {
        Some(state) => {
            print_state_details(&state, args.long, use_color);
        }
        None => {
            print_info(&format!(
                "No installation state found for workload: {}",
                workload_name
            ));
            print_info("Run 'anvil install <workload>' to create installation state.");
        }
    }

    Ok(())
}

/// Show status for all workloads
fn show_all_status(_args: &StatusArgs, use_color: bool) -> Result<()> {
    let state_dir = get_state_dir()?;

    if !state_dir.exists() {
        print_info("No installation state found.");
        print_info("Run 'anvil install <workload>' to create installation state.");
        return Ok(());
    }

    let entries: Vec<_> = std::fs::read_dir(&state_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        print_info("No installation state found.");
        print_info("Run 'anvil install <workload>' to create installation state.");
        return Ok(());
    }

    println!();
    println!("{}", "=== Anvil Installation Status ===".bold());
    println!();

    for entry in entries {
        let path = entry.path();
        let workload_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Ok(Some(state)) = InstallationState::load(&workload_name) {
            print_state_summary(&state, use_color);
            println!();
        }
    }

    // Show cache info
    show_cache_info(use_color)?;

    Ok(())
}

/// Print a summary of installation state
fn print_state_summary(state: &InstallationState, _use_color: bool) {
    let summary = state.summary();
    let local_time: DateTime<Local> = state.updated_at.into();

    let status_indicator = if summary.is_complete() {
        if summary.is_successful() {
            "✓".green()
        } else {
            "⚠".yellow()
        }
    } else {
        "○".dimmed()
    };

    println!(
        "{} {} (v{})",
        status_indicator,
        state.workload_name.bold(),
        state.workload_version
    );

    println!(
        "  Status: {} | Installed: {} | Failed: {} | Skipped: {}",
        if state.completed {
            "Complete".green()
        } else {
            "In Progress".yellow()
        },
        summary.installed.to_string().green(),
        if summary.failed > 0 {
            summary.failed.to_string().red()
        } else {
            "0".dimmed()
        },
        summary.skipped.to_string().dimmed()
    );

    println!(
        "  Last updated: {}",
        local_time.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
    );
}

/// Print detailed installation state
fn print_state_details(state: &InstallationState, long_format: bool, _use_color: bool) {
    let summary = state.summary();
    let local_start: DateTime<Local> = state.started_at.into();
    let local_updated: DateTime<Local> = state.updated_at.into();

    println!();
    println!(
        "{}",
        format!("=== Installation Status: {} ===", state.workload_name).bold()
    );
    println!();

    println!("Workload:     {}", state.workload_name);
    println!("Version:      {}", state.workload_version);
    println!("Session ID:   {}", state.session_id);
    println!(
        "Status:       {}",
        if state.completed {
            "Complete".green()
        } else {
            "In Progress".yellow()
        }
    );
    println!("Started:      {}", local_start.format("%Y-%m-%d %H:%M:%S"));
    println!(
        "Last Updated: {}",
        local_updated.format("%Y-%m-%d %H:%M:%S")
    );

    println!();
    println!("{}", "Summary:".bold());
    println!("  Total packages:  {}", summary.total);
    println!(
        "  Installed:       {}",
        summary.installed.to_string().green()
    );
    println!("  Upgraded:        {}", summary.upgraded.to_string().cyan());
    println!(
        "  Skipped:         {}",
        summary.skipped.to_string().dimmed()
    );
    println!(
        "  Failed:          {}",
        if summary.failed > 0 {
            summary.failed.to_string().red()
        } else {
            "0".normal()
        }
    );

    if summary.reboot_required {
        println!();
        print_warning("A reboot is required to complete some installations");
    }

    // Show package details
    if long_format || summary.failed > 0 {
        println!();
        println!("{}", "Package Details:".bold());

        let mut packages: Vec<_> = state.packages.iter().collect();
        packages.sort_by(|a, b| a.0.cmp(b.0));

        for (id, record) in packages {
            let status_icon = match record.status {
                PackageStatus::Installed => "✓".green(),
                PackageStatus::Upgraded => "↑".cyan(),
                PackageStatus::Failed => "✗".red(),
                PackageStatus::Skipped => "○".dimmed(),
                PackageStatus::Pending => "?".yellow(),
                PackageStatus::Installing => "⟳".blue(),
            };

            let version_info = record
                .installed_version
                .as_ref()
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();

            let duration_info = record
                .duration_secs
                .map(|d| format!(" [{:.1}s]", d))
                .unwrap_or_default();

            println!(
                "  {} {}{} - {}{}",
                status_icon, id, version_info, record.status, duration_info
            );

            if long_format {
                if let Some(ref error) = record.error {
                    println!("      Error: {}", error.red());
                }

                let local_time: DateTime<Local> = record.timestamp.into();
                println!(
                    "      Updated: {}",
                    local_time.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
                );
            }
        }
    }

    // Show failed packages with errors
    let failed = state.failed_packages();
    if !failed.is_empty() && !long_format {
        println!();
        println!("{}", "Failed Packages:".bold().red());
        for record in failed {
            println!("  {} - {}", record.id.red(), record.status);
            if let Some(ref error) = record.error {
                println!("    Error: {}", error);
            }
        }

        println!();
        print_info(&format!(
            "To retry failed packages: anvil install {} --retry-failed",
            state.workload_name
        ));
    }

    // Show packages requiring reboot
    let reboot_packages = state.packages_requiring_reboot();
    if !reboot_packages.is_empty() {
        println!();
        println!("{}", "Packages Requiring Reboot:".bold().yellow());
        for record in reboot_packages {
            println!("  - {}", record.id);
        }
    }
}

/// Show package cache information
fn show_cache_info(_use_color: bool) -> Result<()> {
    let cache = PackageCache::load()?;
    let stats = cache.stats();

    if stats.total_entries > 0 {
        let local_time: DateTime<Local> = stats.last_updated.into();

        println!("{}", "Package Cache:".bold());
        println!(
            "  Entries:   {} ({} valid)",
            stats.total_entries, stats.valid_entries
        );
        println!("  Installed: {}", stats.installed_count);
        println!("  TTL:       {} minutes", stats.ttl_minutes);
        println!(
            "  Updated:   {}",
            local_time.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
        );

        if stats.with_updates_count > 0 {
            println!(
                "  {}",
                format!(
                    "{} package(s) with updates available",
                    stats.with_updates_count
                )
                .yellow()
            );
        }
    }

    Ok(())
}
