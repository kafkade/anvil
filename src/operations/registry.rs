//! Registry operation module
//!
//! This module implements the `anvil registry` command which provides
//! discovery and installation of community workloads from a curated registry.

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use crate::cli::commands::{OutputFormat, RegistryArgs, RegistryCommand};
use crate::cli::output::{print_error, print_info, print_warning};
use crate::cli::Cli;
use crate::config::registry::{
    load_cache, save_cache, search_entries, version_satisfies, RegistryEntry, RegistryIndex,
};
use crate::config::sources::SourcesConfig;
use crate::config::GlobalConfig;
use crate::providers::http::HttpProvider;

/// Execute the registry command
pub fn execute(args: &RegistryArgs, cli: &Cli) -> Result<()> {
    match &args.command {
        RegistryCommand::List { output, refresh } => list_registry(*output, *refresh, cli),
        RegistryCommand::Search {
            query,
            output,
            refresh,
        } => search_registry(query, *output, *refresh, cli),
        RegistryCommand::Add {
            name,
            source_name,
            git_ref,
            refresh,
        } => add_from_registry(
            name,
            source_name.as_deref(),
            git_ref.as_deref(),
            *refresh,
            cli,
        ),
    }
}

/// Fetch the registry index, using cache when possible
fn fetch_registry(force_refresh: bool) -> Result<RegistryIndex> {
    // Try cache first
    if !force_refresh {
        if let Ok(Some(cached)) = load_cache() {
            if cached.is_fresh() {
                return Ok(cached.index);
            }
        }
    }

    // Fetch from remote
    if !HttpProvider::is_available() {
        // Fall back to stale cache if available
        if let Ok(Some(cached)) = load_cache() {
            print_warning(
                "curl is not available; using stale cache. Install curl for fresh registry data.",
            );
            return Ok(cached.index);
        }
        anyhow::bail!(
            "curl is not installed or not in PATH. \
             Install curl to access the workload registry."
        );
    }

    let config = GlobalConfig::load().unwrap_or_default();
    let url = &config.registry.url;

    print_info(&format!("Fetching registry from {}...", url));

    let body = HttpProvider::fetch_string(url).with_context(|| {
        format!(
            "Failed to fetch registry index from {}. \
             The registry may not be available yet.",
            url
        )
    })?;

    let index: RegistryIndex = serde_json::from_str(&body).with_context(|| {
        "Failed to parse registry index. The registry format may be incompatible."
    })?;

    // Validate schema version
    if index.version != "1" {
        anyhow::bail!(
            "Unsupported registry schema version '{}'. \
             Please update Anvil to the latest version.",
            index.version
        );
    }

    // Cache the result
    if let Err(e) = save_cache(&index) {
        tracing::warn!("Failed to cache registry: {}", e);
    }

    Ok(index)
}

/// List all workloads in the registry
fn list_registry(output: OutputFormat, refresh: bool, cli: &Cli) -> Result<()> {
    let index = fetch_registry(refresh)?;
    let use_color = !cli.should_disable_color();

    if index.entries.is_empty() {
        print_info("The registry is empty.");
        return Ok(());
    }

    match output {
        OutputFormat::Table => print_registry_table(&index.entries, use_color),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&index.entries)
                .context("Failed to serialize registry entries")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&index.entries)
                .context("Failed to serialize registry entries")?;
            print!("{}", yaml);
        }
        OutputFormat::Html => {
            print_registry_table(&index.entries, false);
        }
    }

    if !cli.quiet {
        println!(
            "\n{} workload(s) available in the registry",
            index.entries.len()
        );
    }

    Ok(())
}

/// Search for workloads in the registry
fn search_registry(query: &str, output: OutputFormat, refresh: bool, cli: &Cli) -> Result<()> {
    let index = fetch_registry(refresh)?;
    let use_color = !cli.should_disable_color();

    let results = search_entries(&index.entries, query);

    if results.is_empty() {
        if !cli.quiet {
            print_info(&format!("No workloads matching '{}'", query));
        }
        return Ok(());
    }

    match output {
        OutputFormat::Table => print_registry_table(&results, use_color),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&results)
                .context("Failed to serialize search results")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml =
                serde_yaml::to_string(&results).context("Failed to serialize search results")?;
            print!("{}", yaml);
        }
        OutputFormat::Html => {
            print_registry_table(&results, false);
        }
    }

    if !cli.quiet {
        println!("\n{} result(s) for '{}'", results.len(), query);
    }

    Ok(())
}

/// Add a workload from the registry as a source
fn add_from_registry(
    name: &str,
    source_name: Option<&str>,
    git_ref_override: Option<&str>,
    refresh: bool,
    _cli: &Cli,
) -> Result<()> {
    let index = fetch_registry(refresh)?;

    // Find the entry
    let entry = index
        .entries
        .iter()
        .find(|e| e.name == name)
        .with_context(|| {
            format!(
                "Workload '{}' not found in the registry. \
                 Use 'anvil registry search' to find available workloads.",
                name
            )
        })?;

    // Check version compatibility
    if let Some(ref min_ver) = entry.min_anvil_version {
        if !version_satisfies(min_ver) {
            print_error(&format!(
                "Workload '{}' requires Anvil >= {} (current: {})",
                name,
                min_ver,
                env!("CARGO_PKG_VERSION")
            ));
            anyhow::bail!(
                "Incompatible Anvil version. Update Anvil to {} or later.",
                min_ver
            );
        }
    }

    // Determine effective values
    let effective_name = source_name.unwrap_or(&entry.name);
    let effective_ref = git_ref_override.or(entry.git_ref.as_deref());
    let effective_subdir = entry.workload_subdir.as_deref();

    // Delegate to the source add machinery
    let mut sources_config = SourcesConfig::load().unwrap_or_default();

    super::source::add_remote_source(
        &mut sources_config,
        &entry.url,
        Some(effective_name),
        effective_subdir,
        effective_ref,
    )?;

    sources_config
        .save()
        .context("Failed to save sources configuration")?;

    Ok(())
}

/// Print registry entries as a formatted table
fn print_registry_table(entries: &[RegistryEntry], use_color: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let headers = if use_color {
        vec![
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
            Cell::new("Author").fg(Color::Cyan),
            Cell::new("Tags").fg(Color::Cyan),
        ]
    } else {
        vec![
            Cell::new("Name"),
            Cell::new("Description"),
            Cell::new("Author"),
            Cell::new("Tags"),
        ]
    };
    table.set_header(headers);

    for entry in entries {
        let tags_str = entry.tags.join(", ");

        let compat = entry
            .min_anvil_version
            .as_ref()
            .map(|v| !version_satisfies(v))
            .unwrap_or(false);

        let name_cell = if use_color {
            if compat {
                Cell::new(&entry.name).fg(Color::DarkGrey)
            } else {
                Cell::new(&entry.name).fg(Color::Green)
            }
        } else {
            Cell::new(&entry.name)
        };

        let row = if use_color {
            vec![
                name_cell,
                Cell::new(truncate(&entry.description, 45)),
                Cell::new(&entry.author),
                Cell::new(truncate(&tags_str, 25)).fg(Color::Yellow),
            ]
        } else {
            vec![
                name_cell,
                Cell::new(truncate(&entry.description, 45)),
                Cell::new(&entry.author),
                Cell::new(truncate(&tags_str, 25)),
            ]
        };
        table.add_row(row);
    }

    println!("{}", table);
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world!", 8), "hello...");
    }
}
