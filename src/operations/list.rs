//! List operation for Anvil CLI
//!
//! This module implements the `list` command which displays
//! available workloads from all configured search paths.

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::{debug, info, trace};

use crate::cli::{commands::ListArgs, Cli};
use crate::config::ConfigManager;

/// Execute the list command
pub fn execute(args: &ListArgs, cli: &Cli) -> Result<()> {
    debug!("Executing list command");
    trace!("List arguments: {:?}", args);

    let mut manager = ConfigManager::new();

    // Add custom search path if provided
    if let Some(ref path) = args.path {
        debug!("Adding custom search path: {}", path.display());
        manager.add_search_path(path.clone());
    }

    // Choose between all-paths mode and normal mode
    let workloads = if args.all_paths {
        manager.list_all_workloads().with_context(|| {
            "Failed to list workloads. Ensure the workloads directory exists and is accessible."
        })?
    } else {
        manager.list_workloads().with_context(|| {
            "Failed to list workloads. Ensure the workloads directory exists and is accessible."
        })?
    };

    if workloads.is_empty() {
        if !cli.quiet {
            let use_color = !cli.should_disable_color();
            if use_color {
                println!("{} No workloads found.", "ℹ".blue());
                println!(
                    "  {} Create a workload with '{}'",
                    "→".dimmed(),
                    "anvil init <name>".cyan()
                );
            } else {
                println!("No workloads found.");
                println!("Hint: Create a workload with 'anvil init <name>'");
            }
        }
        return Ok(());
    }

    info!("Found {} workload(s)", workloads.len());

    // TUI mode: interactive browser when no explicit format and TTY
    if !args.no_tui && args.output.is_none() && crate::tui::should_use_tui() {
        let entries: Vec<_> = workloads
            .iter()
            .map(|w| crate::tui::views::browser::WorkloadEntry {
                name: w.name.clone(),
                version: w.version.clone(),
                description: w.description.clone(),
                extends: w.extends.clone(),
                package_count: w.package_count,
                file_count: w.file_count,
                source: "local".to_string(),
                path: w
                    .path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            })
            .collect();
        if let Some(selected) = crate::tui::views::browser::run_browser(entries)? {
            // Open the detail view for the selected workload
            let mut detail_manager = ConfigManager::new();
            if let Some(wpath) = detail_manager.find_workload(&selected) {
                let workload = detail_manager.load_workload(&wpath)?;
                let dir = wpath
                    .parent()
                    .unwrap_or(&wpath)
                    .to_string_lossy()
                    .to_string();
                let detail = crate::tui::views::detail::WorkloadDetail {
                    name: workload.name.clone(),
                    version: workload.version.clone(),
                    description: workload.description.clone(),
                    path: dir,
                    extends: workload.extends.unwrap_or_default(),
                    packages: workload
                        .packages
                        .as_ref()
                        .and_then(|p| p.winget.as_ref())
                        .map(|pkgs| pkgs.iter().map(|p| p.id.clone()).collect())
                        .unwrap_or_default(),
                    files: workload
                        .files
                        .as_ref()
                        .map(|f| {
                            f.iter()
                                .map(|fe| crate::tui::views::detail::FileEntry {
                                    source: fe.source.clone(),
                                    destination: fe.destination.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    commands: workload
                        .commands
                        .as_ref()
                        .map(|cb| {
                            let mut cmds = Vec::new();
                            if let Some(ref pre) = cb.pre_install {
                                for cmd in pre {
                                    cmds.push(crate::tui::views::detail::CommandEntry {
                                        name: cmd
                                            .description
                                            .clone()
                                            .unwrap_or_else(|| cmd.run.clone()),
                                        phase: "pre_install".to_string(),
                                    });
                                }
                            }
                            if let Some(ref post) = cb.post_install {
                                for cmd in post {
                                    cmds.push(crate::tui::views::detail::CommandEntry {
                                        name: cmd
                                            .description
                                            .clone()
                                            .unwrap_or_else(|| cmd.run.clone()),
                                        phase: "post_install".to_string(),
                                    });
                                }
                            }
                            cmds
                        })
                        .unwrap_or_default(),
                    assertions: workload
                        .assertions
                        .as_ref()
                        .map(|a| a.iter().map(|ass| ass.name.clone()).collect())
                        .unwrap_or_default(),
                };
                crate::tui::views::detail::run_detail_view(detail)?;
            }
        }
        return Ok(());
    }

    // Determine output format
    let format = args.output.unwrap_or_default();

    match format {
        crate::cli::commands::OutputFormat::Table => {
            print_table(
                &workloads,
                args.long,
                args.all_paths,
                !cli.should_disable_color(),
            );
        }
        crate::cli::commands::OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&workloads)
                .context("Failed to serialize workloads to JSON")?;
            println!("{}", json);
        }
        crate::cli::commands::OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&workloads)
                .context("Failed to serialize workloads to YAML")?;
            print!("{}", yaml);
        }
        crate::cli::commands::OutputFormat::Html => {
            // Fall back to table for now
            print_table(&workloads, args.long, args.all_paths, false);
        }
    }

    Ok(())
}

/// Print workloads as a formatted table
fn print_table(
    workloads: &[crate::config::WorkloadInfo],
    long_format: bool,
    show_all_paths: bool,
    use_color: bool,
) {
    use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    if long_format || show_all_paths {
        // Long format header — includes Path column
        let mut headers = if use_color {
            vec![
                Cell::new("Name").fg(Color::Cyan),
                Cell::new("Version").fg(Color::Cyan),
                Cell::new("Description").fg(Color::Cyan),
                Cell::new("Extends").fg(Color::Cyan),
                Cell::new("Packages").fg(Color::Cyan),
                Cell::new("Files").fg(Color::Cyan),
                Cell::new("Path").fg(Color::Cyan),
            ]
        } else {
            vec![
                Cell::new("Name"),
                Cell::new("Version"),
                Cell::new("Description"),
                Cell::new("Extends"),
                Cell::new("Packages"),
                Cell::new("Files"),
                Cell::new("Path"),
            ]
        };

        if show_all_paths {
            if use_color {
                headers.push(Cell::new("Status").fg(Color::Cyan));
            } else {
                headers.push(Cell::new("Status"));
            }
        }

        table.set_header(headers);

        for workload in workloads {
            let extends_str = if workload.extends.is_empty() {
                "-".to_string()
            } else {
                workload.extends.join(", ")
            };

            let is_shadowed = show_all_paths && !workload.shadowed_paths.is_empty();
            let path_display = workload
                .path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut row = if use_color {
                let name_color = if is_shadowed {
                    Color::DarkGrey
                } else {
                    Color::Green
                };
                vec![
                    Cell::new(&workload.name).fg(name_color),
                    Cell::new(&workload.version),
                    Cell::new(truncate(&workload.description, 35)),
                    Cell::new(&extends_str).fg(Color::Yellow),
                    Cell::new(workload.package_count.to_string()),
                    Cell::new(workload.file_count.to_string()),
                    Cell::new(&path_display),
                ]
            } else {
                vec![
                    Cell::new(&workload.name),
                    Cell::new(&workload.version),
                    Cell::new(truncate(&workload.description, 35)),
                    Cell::new(&extends_str),
                    Cell::new(workload.package_count.to_string()),
                    Cell::new(workload.file_count.to_string()),
                    Cell::new(&path_display),
                ]
            };

            if show_all_paths {
                if is_shadowed {
                    let cell = if use_color {
                        Cell::new("(shadowed)").fg(Color::DarkGrey)
                    } else {
                        Cell::new("(shadowed)")
                    };
                    row.push(cell);
                } else {
                    row.push(Cell::new(""));
                }
            }

            table.add_row(row);
        }
    } else {
        // Short format header
        let headers = if use_color {
            vec![
                Cell::new("Name").fg(Color::Cyan),
                Cell::new("Version").fg(Color::Cyan),
                Cell::new("Description").fg(Color::Cyan),
            ]
        } else {
            vec![
                Cell::new("Name"),
                Cell::new("Version"),
                Cell::new("Description"),
            ]
        };
        table.set_header(headers);

        for workload in workloads {
            let row = if use_color {
                vec![
                    Cell::new(&workload.name).fg(Color::Green),
                    Cell::new(&workload.version),
                    Cell::new(truncate(&workload.description, 50)),
                ]
            } else {
                vec![
                    Cell::new(&workload.name),
                    Cell::new(&workload.version),
                    Cell::new(truncate(&workload.description, 50)),
                ]
            };
            table.add_row(row);
        }
    }

    println!("{}", table);

    // Print summary with color
    let count = workloads.len();
    if use_color {
        println!(
            "\n{} {} workload(s) available",
            "✓".green(),
            count.to_string().bold()
        );
    } else {
        println!("\n{} workload(s) available", count);
    }
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
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("this is a very long string", 10);
        assert_eq!(result, "this is...");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
    }
}
