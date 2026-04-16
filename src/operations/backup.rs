//! Backup operation module
//!
//! This module implements the `anvil backup` command, which provides
//! backup management functionality including list, show, restore, clean, and verify.

use std::io::Write;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use crate::cli::commands::{BackupArgs, BackupCommand, OutputFormat};
use crate::cli::output::{print_error, print_info, print_success, print_warning};
use crate::cli::Cli;
use crate::providers::backup::{format_size, BackupEntry, BackupManager};

/// Execute the backup command
pub fn execute(args: &BackupArgs, cli: &Cli) -> Result<()> {
    let use_color = !cli.should_disable_color();
    let _verbosity = cli.verbosity_level();

    match &args.command {
        BackupCommand::Create {
            name,
            workload,
            include_packages,
            compress,
        } => create_backup(
            name.as_deref(),
            workload.as_deref(),
            *include_packages,
            *compress,
            use_color,
        ),

        BackupCommand::List {
            workload,
            output,
            long,
        } => list_backups(workload.as_deref(), *output, *long, use_color),

        BackupCommand::Show { id } => show_backup(id, use_color),

        BackupCommand::Restore {
            id,
            workload,
            dry_run,
            force,
        } => restore_backup(
            id.as_deref(),
            workload.as_deref(),
            *dry_run,
            *force,
            use_color,
        ),

        BackupCommand::Clean {
            older_than,
            dry_run,
            force,
        } => clean_backups(*older_than, *dry_run, *force, use_color),

        BackupCommand::Verify { workload, fix } => {
            verify_backups(workload.as_deref(), *fix, use_color)
        }
    }
}

/// Create a new backup
fn create_backup(
    name: Option<&str>,
    workload: Option<&str>,
    include_packages: bool,
    compress: bool,
    use_color: bool,
) -> Result<()> {
    let manager = BackupManager::new().context("Failed to initialize backup manager")?;

    print_info("Creating backup...");

    // Generate backup ID
    let backup_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let backup_name = name.unwrap_or(&backup_id);

    // Get backup directory
    let backup_dir = manager.backup_dir();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_folder = backup_dir.join(format!("{}_{}", timestamp, backup_name));

    // Create backup directory
    std::fs::create_dir_all(&backup_folder)
        .with_context(|| format!("Failed to create backup directory: {}", backup_folder.display()))?;

    // Create manifest
    let manifest = BackupManifest {
        id: backup_id.clone(),
        name: name.map(|s| s.to_string()),
        created_at: chrono::Utc::now(),
        workload: workload.map(|s| s.to_string()),
        anvil_version: env!("CARGO_PKG_VERSION").to_string(),
        system_info: get_system_info(),
        files: Vec::new(),
        packages: if include_packages {
            Some(export_packages(&backup_folder)?)
        } else {
            None
        },
        environment: Some(snapshot_environment()),
        total_size: 0,
        compressed: compress,
    };

    // Save manifest
    let manifest_path = backup_folder.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .context("Failed to serialize backup manifest")?;
    std::fs::write(&manifest_path, manifest_json)
        .with_context(|| format!("Failed to write manifest: {}", manifest_path.display()))?;

    if use_color {
        print_success(&format!(
            "Backup created: {} ({})",
            backup_name,
            backup_folder.display()
        ));
    } else {
        println!("Backup created: {} ({})", backup_name, backup_folder.display());
    }

    if include_packages {
        print_info("Package list exported to backup");
    }

    if compress {
        print_warning("Compression not yet implemented - backup saved uncompressed");
    }

    Ok(())
}

/// Backup manifest structure
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupManifest {
    id: String,
    name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    workload: Option<String>,
    anvil_version: String,
    system_info: SystemInfo,
    files: Vec<BackupFileEntry>,
    packages: Option<PackageSnapshot>,
    environment: Option<EnvironmentSnapshot>,
    total_size: u64,
    compressed: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupFileEntry {
    original_path: std::path::PathBuf,
    backup_path: std::path::PathBuf,
    hash: String,
    size: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PackageSnapshot {
    packages: Vec<String>,
    export_file: Option<std::path::PathBuf>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EnvironmentSnapshot {
    user_variables: std::collections::HashMap<String, String>,
    path_additions: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SystemInfo {
    os_version: String,
    hostname: String,
    username: String,
}

/// Get system information
fn get_system_info() -> SystemInfo {
    SystemInfo {
        os_version: std::env::var("OS").unwrap_or_else(|_| "Windows".to_string()),
        hostname: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string()),
        username: std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string()),
    }
}

/// Export installed packages using winget
fn export_packages(backup_folder: &std::path::Path) -> Result<PackageSnapshot> {
    let export_file = backup_folder.join("packages.json");

    // Try to run winget export
    let output = std::process::Command::new("winget")
        .args(["export", "-o", &export_file.to_string_lossy(), "--accept-source-agreements"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            Ok(PackageSnapshot {
                packages: Vec::new(), // Will be populated from export file
                export_file: Some(export_file),
            })
        }
        _ => {
            // Winget export failed, try to list packages instead
            let list_output = std::process::Command::new("winget")
                .args(["list", "--disable-interactivity"])
                .output();

            let packages = if let Ok(result) = list_output {
                String::from_utf8_lossy(&result.stdout)
                    .lines()
                    .skip(2) // Skip header
                    .filter_map(|line| line.split_whitespace().next())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            };

            Ok(PackageSnapshot {
                packages,
                export_file: None,
            })
        }
    }
}

/// Snapshot current environment variables
fn snapshot_environment() -> EnvironmentSnapshot {
    let user_variables: std::collections::HashMap<String, String> = std::env::vars()
        .filter(|(k, _)| {
            // Only include common user-modifiable variables
            matches!(
                k.to_uppercase().as_str(),
                "PATH" | "HOME" | "USERPROFILE" | "CARGO_HOME" | "RUSTUP_HOME" | "GOPATH" | "JAVA_HOME"
            )
        })
        .collect();

    let path_additions = std::env::var("PATH")
        .unwrap_or_default()
        .split(';')
        .filter(|p| !p.is_empty())
        .map(|s| s.to_string())
        .collect();

    EnvironmentSnapshot {
        user_variables,
        path_additions,
    }
}

/// List all backups
fn list_backups(
    workload: Option<&str>,
    output: OutputFormat,
    long: bool,
    use_color: bool,
) -> Result<()> {
    let manager = BackupManager::new().context("Failed to initialize backup manager")?;

    let backups = if let Some(workload) = workload {
        manager
            .list_for_workload(workload)
            .context("Failed to list backups")?
    } else {
        manager.list().context("Failed to list backups")?
    };

    if backups.is_empty() {
        if let Some(w) = workload {
            print_info(&format!("No backups found for workload: {}", w));
        } else {
            print_info("No backups found.");
        }
        return Ok(());
    }

    match output {
        OutputFormat::Table => print_backup_table(&backups, long, use_color),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&backups)?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&backups)?;
            println!("{}", yaml);
        }
        OutputFormat::Html => print_backup_html(&backups),
    }

    // Print summary
    let total_size: u64 = backups.iter().map(|b| b.size).sum();
    println!();
    print_info(&format!(
        "Total: {} backup(s) ({})",
        backups.len(),
        format_size(total_size)
    ));

    Ok(())
}

/// Print backups as a formatted table
fn print_backup_table(backups: &[BackupEntry], long: bool, use_color: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    if long {
        table.set_header(vec![
            "ID",
            "Date",
            "Workload",
            "Original Path",
            "Backup Path",
            "Size",
            "Hash",
        ]);

        for backup in backups {
            let date_str = backup.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let size_str = backup.formatted_size();
            let hash_short = backup.hash.chars().skip(7).take(12).collect::<String>();

            table.add_row(vec![
                Cell::new(&backup.id),
                Cell::new(&date_str),
                Cell::new(&backup.workload),
                Cell::new(backup.original_path.display()),
                Cell::new(backup.backup_path.display()),
                Cell::new(&size_str),
                Cell::new(&hash_short),
            ]);
        }
    } else {
        table.set_header(vec!["ID", "Date", "Workload", "File", "Size"]);

        for backup in backups {
            let date_str = backup.timestamp.format("%Y-%m-%d %H:%M").to_string();
            let size_str = backup.formatted_size();
            let file_name = backup
                .original_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| backup.original_path.display().to_string());

            let id_cell = if use_color {
                Cell::new(&backup.id).fg(Color::Cyan)
            } else {
                Cell::new(&backup.id)
            };

            table.add_row(vec![
                id_cell,
                Cell::new(&date_str),
                Cell::new(&backup.workload),
                Cell::new(&file_name),
                Cell::new(&size_str),
            ]);
        }
    }

    println!("{table}");
}

/// Print backups as HTML
fn print_backup_html(backups: &[BackupEntry]) {
    println!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Anvil Backups</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 20px; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        tr:hover {{ background-color: #ddd; }}
        .id {{ font-family: monospace; color: #0066cc; }}
        .size {{ text-align: right; }}
    </style>
</head>
<body>
    <h1>Anvil Backups</h1>
    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Date</th>
                <th>Workload</th>
                <th>Original Path</th>
                <th>Size</th>
            </tr>
        </thead>
        <tbody>"#
    );

    for backup in backups {
        let date_str = backup.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        println!(
            r#"            <tr>
                <td class="id">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td class="size">{}</td>
            </tr>"#,
            backup.id,
            date_str,
            backup.workload,
            backup.original_path.display(),
            backup.formatted_size()
        );
    }

    println!(
        r#"        </tbody>
    </table>
    <p>Total: {} backup(s) ({}) </p>
</body>
</html>"#,
        backups.len(),
        format_size(backups.iter().map(|b| b.size).sum())
    );
}

/// Show details for a specific backup
fn show_backup(id: &str, _use_color: bool) -> Result<()> {
    let manager = BackupManager::new().context("Failed to initialize backup manager")?;

    let backup = manager
        .get(id)
        .context(format!("Backup not found: {}", id))?;

    println!("Backup Details");
    println!("==============");
    println!();
    println!("ID:            {}", backup.id);
    println!(
        "Timestamp:     {}",
        backup.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Workload:      {}", backup.workload);
    println!("Original Path: {}", backup.original_path.display());
    println!("Backup Path:   {}", backup.backup_path.display());
    println!("Size:          {}", backup.formatted_size());
    println!("Hash:          {}", backup.hash);

    // Check if backup file exists
    if backup.backup_path.exists() {
        println!("Status:        Available");
    } else {
        print_warning("Status:        MISSING - backup file does not exist");
    }

    if let Some(desc) = &backup.description {
        println!("Description:   {}", desc);
    }

    Ok(())
}

/// Restore a backup
fn restore_backup(
    id: Option<&str>,
    workload: Option<&str>,
    dry_run: bool,
    force: bool,
    _use_color: bool,
) -> Result<()> {
    let manager = if dry_run {
        BackupManager::new()?.with_dry_run(true)
    } else {
        BackupManager::new()?
    };

    if let Some(id) = id {
        // Restore single backup by ID
        let backup = manager
            .get(id)
            .context(format!("Backup not found: {}", id))?;

        if !force {
            println!(
                "Restore backup {} to {}?",
                id,
                backup.original_path.display()
            );
            print!("Continue? [y/N] ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                print_info("Restore cancelled.");
                return Ok(());
            }
        }

        if dry_run {
            print_info(&format!(
                "Would restore {} -> {}",
                backup.backup_path.display(),
                backup.original_path.display()
            ));
        } else {
            manager
                .restore_by_id(id)
                .context("Failed to restore backup")?;
            print_success(&format!(
                "Restored {} to {}",
                id,
                backup.original_path.display()
            ));
        }
    } else if let Some(workload) = workload {
        // Restore all backups for a workload
        let backups = manager.list_for_workload(workload)?;

        if backups.is_empty() {
            print_warning(&format!("No backups found for workload: {}", workload));
            return Ok(());
        }

        if !force {
            println!(
                "Restore {} backup(s) for workload '{}'?",
                backups.len(),
                workload
            );
            for backup in &backups {
                println!("  - {} -> {}", backup.id, backup.original_path.display());
            }
            print!("Continue? [y/N] ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                print_info("Restore cancelled.");
                return Ok(());
            }
        }

        let restored = manager
            .restore_workload(workload)
            .context("Failed to restore workload backups")?;

        if dry_run {
            print_info(&format!("Would restore {} file(s)", restored.len()));
        } else {
            print_success(&format!("Restored {} file(s)", restored.len()));
        }
    } else {
        print_error("Either --id or --workload must be specified");
        return Ok(());
    }

    Ok(())
}

/// Clean old backups
fn clean_backups(older_than: u32, dry_run: bool, force: bool, _use_color: bool) -> Result<()> {
    let manager = if dry_run {
        BackupManager::new()?.with_dry_run(true)
    } else {
        BackupManager::new()?
    };

    // Preview what will be cleaned
    let all_backups = manager.list()?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than as i64);
    let to_clean: Vec<_> = all_backups
        .iter()
        .filter(|b| b.timestamp < cutoff)
        .collect();

    if to_clean.is_empty() {
        print_info(&format!("No backups older than {} days found.", older_than));
        return Ok(());
    }

    let total_size: u64 = to_clean.iter().map(|b| b.size).sum();

    if !force {
        println!(
            "Clean {} backup(s) older than {} days?",
            to_clean.len(),
            older_than
        );
        println!("This will free {} of space.", format_size(total_size));
        println!();
        for backup in &to_clean {
            let age = chrono::Utc::now()
                .signed_duration_since(backup.timestamp)
                .num_days();
            println!(
                "  - {} ({} days old): {}",
                backup.id,
                age,
                backup.original_path.display()
            );
        }
        println!();
        print!("Continue? [y/N] ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            print_info("Clean cancelled.");
            return Ok(());
        }
    }

    let result = manager
        .clean_older_than(older_than)
        .context("Failed to clean backups")?;

    if dry_run {
        print_info(&format!(
            "Would remove {} backup(s), freeing {}",
            result.removed_count,
            result.formatted_bytes_freed()
        ));
    } else {
        print_success(&format!(
            "Removed {} backup(s), freed {}",
            result.removed_count,
            result.formatted_bytes_freed()
        ));
    }

    Ok(())
}

/// Verify backup integrity
fn verify_backups(workload: Option<&str>, fix: bool, _use_color: bool) -> Result<()> {
    let manager = BackupManager::new().context("Failed to initialize backup manager")?;

    print_info("Verifying backup integrity...");
    println!();

    let result = manager.verify_all().context("Failed to verify backups")?;

    // Filter by workload if specified
    let (missing, corrupted, errors) = if let Some(workload) = workload {
        let workload_backups: std::collections::HashSet<_> = manager
            .list_for_workload(workload)?
            .iter()
            .map(|b| b.id.clone())
            .collect();

        let missing: Vec<_> = result
            .missing
            .iter()
            .filter(|id| workload_backups.contains(*id))
            .cloned()
            .collect();
        let corrupted: Vec<_> = result
            .corrupted
            .iter()
            .filter(|(id, _)| workload_backups.contains(id))
            .cloned()
            .collect();
        let errors: Vec<_> = result
            .errors
            .iter()
            .filter(|id| workload_backups.contains(*id))
            .cloned()
            .collect();

        (missing, corrupted, errors)
    } else {
        (
            result.missing.clone(),
            result.corrupted.clone(),
            result.errors.clone(),
        )
    };

    let total = if workload.is_some() {
        manager.list_for_workload(workload.unwrap())?.len()
    } else {
        result.total
    };

    let valid = total - missing.len() - corrupted.len() - errors.len();

    println!("Verification Results:");
    println!("  Total:     {}", total);
    println!("  Valid:     {}", valid);
    println!("  Missing:   {}", missing.len());
    println!("  Corrupted: {}", corrupted.len());
    println!("  Errors:    {}", errors.len());
    println!();

    if !missing.is_empty() {
        print_warning("Missing backup files:");
        for id in &missing {
            println!("  - {}", id);
        }
        println!();
    }

    if !corrupted.is_empty() {
        print_error("Corrupted backups (hash mismatch):");
        for (id, _actual_hash) in &corrupted {
            println!("  - {}", id);
        }
        println!();
    }

    if !errors.is_empty() {
        print_error("Backup verification errors:");
        for id in &errors {
            println!("  - {}", id);
        }
        println!();
    }

    if missing.is_empty() && corrupted.is_empty() && errors.is_empty() {
        print_success("All backups verified successfully!");
    } else if fix {
        print_info("Fixing backup index by removing invalid entries...");

        let mut fix_count = 0;
        for id in missing.iter().chain(corrupted.iter().map(|(id, _)| id)) {
            if let Err(e) = manager.delete_by_id(id) {
                print_warning(&format!("Failed to remove entry {}: {}", id, e));
            } else {
                fix_count += 1;
            }
        }

        print_success(&format!(
            "Removed {} invalid entries from backup index",
            fix_count
        ));
    } else {
        print_info("Run with --fix to remove invalid entries from the backup index.");
    }

    Ok(())
}
