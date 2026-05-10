//! Source management operation module
//!
//! This module implements the `anvil source` command which provides
//! management of workload sources — both local directories and remote
//! git repositories.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use crate::cli::commands::{OutputFormat, SourceArgs, SourceCommand};
use crate::cli::output::{print_error, print_info, print_success, print_warning};
use crate::cli::Cli;
use crate::config::default_workload_paths;
use crate::config::sources::{
    is_git_url, repo_name_from_url, sources_dir, validate_source_name, Source, SourceType,
    SourcesConfig,
};
use crate::providers::git::GitProvider;

/// Execute the source command
pub fn execute(args: &SourceArgs, cli: &Cli) -> Result<()> {
    match &args.command {
        SourceCommand::List { output } => list_sources(*output, cli),
        SourceCommand::Add {
            location,
            name,
            path,
            git_ref,
        } => add_source(
            location,
            name.as_deref(),
            path.as_deref(),
            git_ref.as_deref(),
            cli,
        ),
        SourceCommand::Remove { name, delete } => remove_source(name, *delete, cli),
        SourceCommand::Status { output } => show_status(*output, cli),
        SourceCommand::Sync { name } => sync_sources(name.as_deref(), cli),
    }
}

/// List all configured sources including defaults, user-configured, and remote
fn list_sources(output: OutputFormat, cli: &Cli) -> Result<()> {
    let sources_config = SourcesConfig::load().unwrap_or_default();
    let use_color = !cli.should_disable_color();

    // Build the complete source list with origin labels
    let mut entries: Vec<SourceListEntry> = Vec::new();

    // 1. Managed sources from sources.json
    for source in &sources_config.sources {
        let workload_path = source.workload_path();
        let workload_count = count_workloads(&workload_path);

        entries.push(SourceListEntry {
            name: source.name.clone(),
            origin: source.source_type.to_string(),
            path: source.workload_path().to_string_lossy().to_string(),
            workloads: workload_count,
            url: source.url.clone(),
        });
    }

    // 2. Default paths
    for path in default_workload_paths() {
        // Skip if already covered by a managed source
        let path_str = path.to_string_lossy().to_string();
        if entries.iter().any(|e| e.path == path_str) {
            continue;
        }
        let workload_count = count_workloads(&path);
        entries.push(SourceListEntry {
            name: "-".to_string(),
            origin: "default".to_string(),
            path: path_str,
            workloads: workload_count,
            url: None,
        });
    }

    if entries.is_empty() && !cli.quiet {
        print_info("No sources configured. Add one with 'anvil source add <path-or-url>'");
        return Ok(());
    }

    match output {
        OutputFormat::Table => print_source_table(&entries, use_color),
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&entries).context("Failed to serialize sources")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&entries).context("Failed to serialize sources")?;
            print!("{}", yaml);
        }
        OutputFormat::Html => {
            print_source_table(&entries, false);
        }
    }

    Ok(())
}

/// Add a new workload source (local path or git URL)
fn add_source(
    location: &str,
    name: Option<&str>,
    subdir: Option<&str>,
    git_ref: Option<&str>,
    _cli: &Cli,
) -> Result<()> {
    let mut sources_config = SourcesConfig::load().unwrap_or_default();

    if is_git_url(location) {
        add_remote_source(&mut sources_config, location, name, subdir, git_ref)?;
    } else {
        add_local_source(&mut sources_config, location, name, subdir)?;
    }

    sources_config
        .save()
        .context("Failed to save sources configuration")?;

    Ok(())
}

/// Add a local directory as a source
fn add_local_source(
    config: &mut SourcesConfig,
    path_str: &str,
    name: Option<&str>,
    subdir: Option<&str>,
) -> Result<()> {
    let path = PathBuf::from(path_str);
    let canonical = std::fs::canonicalize(&path)
        .with_context(|| format!("Path does not exist: {}", path.display()))?;

    // Strip Windows UNC extended-length prefix (\\?\) for cleaner display and storage
    #[cfg(windows)]
    let canonical = {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            canonical
        }
    };

    if !canonical.is_dir() {
        anyhow::bail!("Path is not a directory: {}", canonical.display());
    }

    let source_name = match name {
        Some(n) => n.to_string(),
        None => canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string()),
    };

    validate_source_name(&source_name)?;

    let mut source = Source::new_local(source_name.clone(), canonical);
    source.workload_subdir = subdir.map(|s| s.to_string());

    config
        .add(source)
        .with_context(|| format!("Failed to add source '{}'", source_name))?;

    print_success(&format!("Added local source '{}'", source_name));

    Ok(())
}

/// Add a remote git repository as a source
///
/// This is a public helper so that `operations::registry` can reuse it.
pub fn add_remote_source(
    config: &mut SourcesConfig,
    url: &str,
    name: Option<&str>,
    subdir: Option<&str>,
    git_ref: Option<&str>,
) -> Result<()> {
    if !GitProvider::is_available() {
        anyhow::bail!(
            "git is not installed or not in PATH. \
             Install git to use remote sources."
        );
    }

    let source_name = match name {
        Some(n) => n.to_string(),
        None => repo_name_from_url(url)
            .context("Cannot determine source name from URL. Use --name to specify one.")?,
    };

    validate_source_name(&source_name)?;

    // Determine clone destination
    let sources_root = sources_dir()?;
    let clone_path = sources_root.join(&source_name);

    if clone_path.exists() {
        anyhow::bail!(
            "Destination already exists: {}. \
             Use a different --name or remove it first.",
            clone_path.display()
        );
    }

    // Create parent directory
    std::fs::create_dir_all(&sources_root).with_context(|| {
        format!(
            "Failed to create sources directory: {}",
            sources_root.display()
        )
    })?;

    print_info(&format!("Cloning {} into {}...", url, clone_path.display()));

    GitProvider::clone(url, &clone_path, git_ref)
        .with_context(|| format!("Failed to clone repository: {}", url))?;

    let mut source = Source::new_remote(
        source_name.clone(),
        url.to_string(),
        clone_path,
        git_ref.map(|s| s.to_string()),
        subdir.map(|s| s.to_string()),
    );
    source.last_synced = Some(Utc::now());

    config
        .add(source)
        .with_context(|| format!("Failed to add source '{}'", source_name))?;

    print_success(&format!("Added remote source '{}'", source_name));

    Ok(())
}

/// Remove a workload source
fn remove_source(name: &str, delete_files: bool, _cli: &Cli) -> Result<()> {
    let mut sources_config = SourcesConfig::load().unwrap_or_default();

    let removed = sources_config.remove(name);

    match removed {
        Some(source) => {
            // Delete cloned files if requested and this is a remote source
            if delete_files && source.source_type == SourceType::Remote {
                if source.local_path.exists() {
                    std::fs::remove_dir_all(&source.local_path).with_context(|| {
                        format!(
                            "Failed to delete source directory: {}",
                            source.local_path.display()
                        )
                    })?;
                    print_info(&format!(
                        "Deleted cloned files at {}",
                        source.local_path.display()
                    ));
                }
            } else if source.source_type == SourceType::Remote && source.local_path.exists() {
                print_info(&format!(
                    "Cloned files remain at {}. Use --delete to remove them.",
                    source.local_path.display()
                ));
            }

            sources_config
                .save()
                .context("Failed to save sources configuration")?;
            print_success(&format!("Removed source '{}'", name));
        }
        None => {
            print_error(&format!("Source '{}' not found", name));
            if !sources_config.sources.is_empty() {
                println!("\nAvailable sources:");
                for s in &sources_config.sources {
                    println!("  - {} ({})", s.name, s.source_type);
                }
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Show sync status for all sources
fn show_status(output: OutputFormat, cli: &Cli) -> Result<()> {
    let sources_config = SourcesConfig::load().unwrap_or_default();
    let use_color = !cli.should_disable_color();

    if sources_config.sources.is_empty() {
        print_info("No sources configured. Add one with 'anvil source add <path-or-url>'");
        return Ok(());
    }

    let mut statuses: Vec<SourceStatusEntry> = Vec::new();

    for source in &sources_config.sources {
        let exists = source.local_path.exists() || source.workload_path().exists();
        let workload_count = count_workloads(&source.workload_path());

        let (current_ref, dirty) = if source.source_type == SourceType::Remote && exists {
            let ref_str = GitProvider::current_ref(&source.local_path)
                .unwrap_or_else(|_| "unknown".to_string());
            let is_dirty = GitProvider::is_dirty(&source.local_path).unwrap_or(false);
            (Some(ref_str), Some(is_dirty))
        } else {
            (None, None)
        };

        statuses.push(SourceStatusEntry {
            name: source.name.clone(),
            source_type: source.source_type.to_string(),
            path: source.workload_path().to_string_lossy().to_string(),
            exists,
            workloads: workload_count,
            current_ref,
            dirty,
            last_synced: source.last_synced.map(|t| t.to_rfc3339()),
        });
    }

    match output {
        OutputFormat::Table => print_status_table(&statuses, use_color),
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&statuses).context("Failed to serialize status")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&statuses).context("Failed to serialize status")?;
            print!("{}", yaml);
        }
        OutputFormat::Html => {
            print_status_table(&statuses, false);
        }
    }

    Ok(())
}

/// Sync remote sources
fn sync_sources(name: Option<&str>, _cli: &Cli) -> Result<()> {
    let mut sources_config = SourcesConfig::load().unwrap_or_default();

    if !GitProvider::is_available() {
        anyhow::bail!("git is not installed or not in PATH");
    }

    let sources_to_sync: Vec<usize> = if let Some(name) = name {
        // Sync a specific source
        let idx = sources_config
            .sources
            .iter()
            .position(|s| s.name == name)
            .with_context(|| format!("Source '{}' not found", name))?;

        if sources_config.sources[idx].source_type != SourceType::Remote {
            anyhow::bail!("Source '{}' is not a remote source", name);
        }

        vec![idx]
    } else {
        // Sync all remote sources
        sources_config
            .remote_sources()
            .iter()
            .filter_map(|s| {
                sources_config
                    .sources
                    .iter()
                    .position(|src| src.name == s.name)
            })
            .collect()
    };

    if sources_to_sync.is_empty() {
        print_info("No remote sources to sync");
        return Ok(());
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for idx in &sources_to_sync {
        let source = &sources_config.sources[*idx];
        let source_name = source.name.clone();
        let repo_path = source.local_path.clone();
        let git_ref = source.git_ref.clone();

        print_info(&format!("Syncing '{}'...", source_name));

        if !repo_path.exists() {
            print_warning(&format!(
                "Source '{}' directory not found: {}",
                source_name,
                repo_path.display()
            ));
            fail_count += 1;
            continue;
        }

        match GitProvider::sync(&repo_path, git_ref.as_deref()) {
            Ok(()) => {
                sources_config.sources[*idx].last_synced = Some(Utc::now());
                print_success(&format!("Synced '{}'", source_name));
                success_count += 1;
            }
            Err(crate::providers::git::GitError::DirtyWorkingTree) => {
                print_warning(&format!(
                    "Skipping '{}': working tree has local modifications",
                    source_name
                ));
                fail_count += 1;
            }
            Err(e) => {
                print_error(&format!("Failed to sync '{}': {}", source_name, e));
                fail_count += 1;
            }
        }
    }

    // Save updated timestamps
    sources_config
        .save()
        .context("Failed to save sources configuration")?;

    if fail_count > 0 {
        println!("\n{} synced, {} failed", success_count, fail_count);
    } else {
        print_success(&format!(
            "All {} source(s) synced successfully",
            success_count
        ));
    }

    Ok(())
}

/// Count workloads in a directory
fn count_workloads(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }

    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_dir()
                        && (e.path().join("workload.yaml").exists()
                            || e.path().join("workload.yml").exists())
                })
                .count()
        })
        .unwrap_or(0)
}

/// Entry for the source list table
#[derive(Debug, serde::Serialize)]
struct SourceListEntry {
    name: String,
    origin: String,
    path: String,
    workloads: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

/// Entry for the source status table
#[derive(Debug, serde::Serialize)]
struct SourceStatusEntry {
    name: String,
    #[serde(rename = "type")]
    source_type: String,
    path: String,
    exists: bool,
    workloads: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_synced: Option<String>,
}

/// Print the source list as a formatted table
fn print_source_table(entries: &[SourceListEntry], use_color: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let headers = if use_color {
        vec![
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Type").fg(Color::Cyan),
            Cell::new("Path").fg(Color::Cyan),
            Cell::new("Workloads").fg(Color::Cyan),
        ]
    } else {
        vec![
            Cell::new("Name"),
            Cell::new("Type"),
            Cell::new("Path"),
            Cell::new("Workloads"),
        ]
    };
    table.set_header(headers);

    for entry in entries {
        let type_cell = if use_color {
            match entry.origin.as_str() {
                "local" => Cell::new(&entry.origin).fg(Color::Green),
                "remote" => Cell::new(&entry.origin).fg(Color::Blue),
                "default" => Cell::new(&entry.origin).fg(Color::DarkGrey),
                _ => Cell::new(&entry.origin),
            }
        } else {
            Cell::new(&entry.origin)
        };

        let row = if use_color {
            vec![
                Cell::new(&entry.name).fg(Color::Green),
                type_cell,
                Cell::new(&entry.path),
                Cell::new(entry.workloads.to_string()),
            ]
        } else {
            vec![
                Cell::new(&entry.name),
                type_cell,
                Cell::new(&entry.path),
                Cell::new(entry.workloads.to_string()),
            ]
        };
        table.add_row(row);
    }

    println!("{}", table);
}

/// Print the source status as a formatted table
fn print_status_table(entries: &[SourceStatusEntry], use_color: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let headers = if use_color {
        vec![
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Type").fg(Color::Cyan),
            Cell::new("Status").fg(Color::Cyan),
            Cell::new("Ref").fg(Color::Cyan),
            Cell::new("Workloads").fg(Color::Cyan),
            Cell::new("Last Synced").fg(Color::Cyan),
        ]
    } else {
        vec![
            Cell::new("Name"),
            Cell::new("Type"),
            Cell::new("Status"),
            Cell::new("Ref"),
            Cell::new("Workloads"),
            Cell::new("Last Synced"),
        ]
    };
    table.set_header(headers);

    for entry in entries {
        let status = if !entry.exists {
            "missing"
        } else if entry.dirty == Some(true) {
            "modified"
        } else {
            "ok"
        };

        let status_cell = if use_color {
            match status {
                "ok" => Cell::new("✓ ok").fg(Color::Green),
                "modified" => Cell::new("⚠ modified").fg(Color::Yellow),
                "missing" => Cell::new("✗ missing").fg(Color::Red),
                _ => Cell::new(status),
            }
        } else {
            Cell::new(status)
        };

        let ref_str = entry.current_ref.as_deref().unwrap_or("-");
        let synced_str = entry.last_synced.as_deref().unwrap_or("-");

        let row = if use_color {
            vec![
                Cell::new(&entry.name).fg(Color::Green),
                Cell::new(&entry.source_type),
                status_cell,
                Cell::new(ref_str),
                Cell::new(entry.workloads.to_string()),
                Cell::new(synced_str),
            ]
        } else {
            vec![
                Cell::new(&entry.name),
                Cell::new(&entry.source_type),
                status_cell,
                Cell::new(ref_str),
                Cell::new(entry.workloads.to_string()),
                Cell::new(synced_str),
            ]
        };
        table.add_row(row);
    }

    println!("{}", table);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_workloads_empty_dir() {
        let temp = tempfile::TempDir::new().unwrap();
        assert_eq!(count_workloads(temp.path()), 0);
    }

    #[test]
    fn test_count_workloads_with_workloads() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create a valid workload directory
        let workload_dir = temp.path().join("test-workload");
        std::fs::create_dir(&workload_dir).unwrap();
        std::fs::write(
            workload_dir.join("workload.yaml"),
            "name: test\nversion: \"1.0.0\"\ndescription: test",
        )
        .unwrap();

        // Create a non-workload directory
        let other_dir = temp.path().join("not-a-workload");
        std::fs::create_dir(&other_dir).unwrap();

        assert_eq!(count_workloads(temp.path()), 1);
    }

    #[test]
    fn test_count_workloads_nonexistent_dir() {
        assert_eq!(
            count_workloads(std::path::Path::new("/nonexistent/path")),
            0
        );
    }
}
