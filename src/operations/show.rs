//! Show operation - Display detailed workload information
//!
//! This operation displays comprehensive information about a specific workload,
//! including its metadata, packages, files, scripts, and configuration.

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::{debug, trace};

use crate::cli::commands::{ConfigOutputFormat, ShowArgs};
use crate::cli::Cli;
use crate::config::{ConfigManager, InheritanceGraph, Workload};

/// Execute the show command
pub fn execute(args: &ShowArgs, cli: &Cli) -> Result<()> {
    debug!("Executing show command for workload: {}", args.workload);
    trace!("Show arguments: {:?}", args);

    let mut manager = ConfigManager::new();

    let use_color = !cli.should_disable_color();

    // If --show-inheritance is specified, display inheritance tree
    if args.show_inheritance {
        return show_inheritance_tree(&args.workload, &mut manager, use_color);
    }

    // Load the workload (optionally resolved with inheritance)
    let workload = if args.resolved {
        debug!("Loading resolved workload with inheritance");
        manager
            .load_resolved(&args.workload)
            .with_context(|| format!("Failed to load resolved workload: {}", args.workload))?
    } else {
        let path = manager
            .find_workload(&args.workload)
            .with_context(|| format!("Workload not found: {}", args.workload))?;
        debug!("Loading workload from: {}", path.display());
        manager.load_workload(&path)?
    };

    // If outputting to terminal with YAML format and not quiet, show human-readable summary first
    let show_summary = !cli.quiet && atty::is(atty::Stream::Stdout);

    if show_summary {
        display_workload_summary(&workload, use_color, args.resolved);
        println!();

        if use_color {
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", "Raw Configuration:".bold());
        } else {
            println!("{}", "-".repeat(60));
            println!("Raw Configuration:");
        }
        println!();
    }

    // Format and display the workload
    match args.output {
        ConfigOutputFormat::Yaml => {
            let yaml =
                serde_yaml::to_string(&workload).context("Failed to serialize workload to YAML")?;
            println!("{}", yaml);
        }
        ConfigOutputFormat::Json => {
            let json = serde_json::to_string_pretty(&workload)
                .context("Failed to serialize workload to JSON")?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Display workload information in a human-readable format
fn display_workload_summary(workload: &Workload, use_color: bool, is_resolved: bool) {
    println!();

    // Header
    if use_color {
        let resolved_indicator = if is_resolved {
            " (resolved)".yellow().to_string()
        } else {
            String::new()
        };
        println!(
            "{} {}{}",
            "Workload:".bold(),
            workload.name.green().bold(),
            resolved_indicator
        );
    } else {
        let resolved_indicator = if is_resolved { " (resolved)" } else { "" };
        println!("Workload: {}{}", workload.name, resolved_indicator);
    }

    // Basic info
    print_field("Version", &workload.version, use_color);
    print_field("Description", &workload.description, use_color);

    // Extends (only show if not resolved)
    if !is_resolved {
        if let Some(ref extends) = workload.extends {
            if !extends.is_empty() {
                let extends_str = extends.join(", ");
                if use_color {
                    println!("  {} {}", "Extends:".dimmed(), extends_str.yellow());
                } else {
                    println!("  Extends: {}", extends_str);
                }
            }
        }
    }

    println!();

    // Summary counts
    let package_count = workload.package_count();
    let file_count = workload.file_count();
    let script_count = workload.script_count();

    if use_color {
        println!("{}", "Summary:".bold());
        println!("  📦 {} package(s)", package_count.to_string().cyan());
        println!("  📄 {} file(s)", file_count.to_string().cyan());
        println!("  📜 {} script(s)", script_count.to_string().cyan());
    } else {
        println!("Summary:");
        println!("  Packages: {}", package_count);
        println!("  Files: {}", file_count);
        println!("  Scripts: {}", script_count);
    }

    // Packages summary
    if let Some(ref packages) = workload.packages {
        if let Some(ref winget) = packages.winget {
            if !winget.is_empty() {
                println!();
                if use_color {
                    println!("{}", "Packages (winget):".bold());
                } else {
                    println!("Packages (winget):");
                }
                for pkg in winget {
                    let version_str = pkg
                        .version
                        .as_ref()
                        .map(|v| format!(" @ {}", v))
                        .unwrap_or_default();
                    if use_color {
                        println!(
                            "  {} {}{}",
                            "•".dimmed(),
                            pkg.id.green(),
                            version_str.dimmed()
                        );
                    } else {
                        println!("  - {}{}", pkg.id, version_str);
                    }
                }
            }
        }
    }

    // Files summary
    if let Some(ref files) = workload.files {
        if !files.is_empty() {
            println!();
            if use_color {
                println!("{}", "Files:".bold());
            } else {
                println!("Files:");
            }
            for file in files {
                if use_color {
                    println!(
                        "  {} {} {} {}",
                        "•".dimmed(),
                        file.source.cyan(),
                        "→".dimmed(),
                        file.destination.yellow()
                    );
                } else {
                    println!("  - {} -> {}", file.source, file.destination);
                }
            }
        }
    }

    // Scripts summary
    if let Some(ref scripts) = workload.scripts {
        let mut _has_scripts = false;

        if let Some(ref pre) = scripts.pre_install {
            if !pre.is_empty() {
                if !_has_scripts {
                    println!();
                    _has_scripts = true;
                }
                if use_color {
                    println!("{}", "Pre-install Scripts:".bold());
                } else {
                    println!("Pre-install Scripts:");
                }
                for script in pre {
                    print_script_entry(&script.path, script.description.as_deref(), use_color);
                }
            }
        }

        if let Some(ref post) = scripts.post_install {
            if !post.is_empty() {
                if !_has_scripts {
                    println!();
                    _has_scripts = true;
                }
                if use_color {
                    println!("{}", "Post-install Scripts:".bold());
                } else {
                    println!("Post-install Scripts:");
                }
                for script in post {
                    print_script_entry(&script.path, script.description.as_deref(), use_color);
                }
            }
        }
    }

    // Environment summary
    if let Some(ref env) = workload.environment {
        let mut has_env = false;

        if let Some(ref vars) = env.variables {
            if !vars.is_empty() {
                println!();
                has_env = true;
                if use_color {
                    println!("{}", "Environment Variables:".bold());
                } else {
                    println!("Environment Variables:");
                }
                for var in vars {
                    if use_color {
                        println!(
                            "  {} {}={} ({})",
                            "•".dimmed(),
                            var.name.cyan(),
                            var.value.green(),
                            var.scope.dimmed()
                        );
                    } else {
                        println!("  - {}={} ({})", var.name, var.value, var.scope);
                    }
                }
            }
        }

        if let Some(ref paths) = env.path_additions {
            if !paths.is_empty() {
                if !has_env {
                    println!();
                }
                if use_color {
                    println!("{}", "PATH Additions:".bold());
                } else {
                    println!("PATH Additions:");
                }
                for path in paths {
                    if use_color {
                        println!("  {} {}", "•".dimmed(), path.yellow());
                    } else {
                        println!("  - {}", path);
                    }
                }
            }
        }
    }
}

/// Print a single field with label and value
fn print_field(label: &str, value: &str, use_color: bool) {
    if value.is_empty() {
        return;
    }
    if use_color {
        println!("  {} {}", format!("{}:", label).dimmed(), value);
    } else {
        println!("  {}: {}", label, value);
    }
}

/// Print a script entry
fn print_script_entry(path: &str, description: Option<&str>, use_color: bool) {
    if use_color {
        let desc = description
            .map(|d| format!(" - {}", d.dimmed()))
            .unwrap_or_default();
        println!("  {} {}{}", "•".dimmed(), path.cyan(), desc);
    } else {
        let desc = description.map(|d| format!(" - {}", d)).unwrap_or_default();
        println!("  - {}{}", path, desc);
    }
}

/// Count total number of scripts in a workload
/// Display inheritance tree for a workload
fn show_inheritance_tree(
    workload_name: &str,
    manager: &mut ConfigManager,
    use_color: bool,
) -> Result<()> {
    // Build the inheritance graph
    let graph = InheritanceGraph::build(workload_name, manager).map_err(|e| {
        anyhow::anyhow!(
            "Failed to build inheritance graph: {}\n\nSuggestion: {}",
            e,
            e.suggestion()
        )
    })?;

    // Load the workload to get version info
    let workload_path = manager
        .find_workload(workload_name)
        .with_context(|| format!("Workload not found: {}", workload_name))?;
    let workload = manager.load_workload(&workload_path)?;

    println!();

    // Header
    if use_color {
        println!(
            "{} {} (v{})",
            "Workload:".bold(),
            workload.name.green().bold(),
            workload.version
        );
    } else {
        println!("Workload: {} (v{})", workload.name, workload.version);
    }

    if !workload.description.is_empty() {
        println!("Description: {}", workload.description);
    }

    println!();

    // Inheritance tree
    if use_color {
        println!("{}", "Inheritance Tree:".bold());
    } else {
        println!("Inheritance Tree:");
    }

    let tree = graph.format_tree(workload_name);
    for line in tree.lines() {
        if use_color {
            // Color the workload names in the tree
            if line.contains("└──") || line.contains("├──") {
                let parts: Vec<&str> = line.splitn(2, "── ").collect();
                if parts.len() == 2 {
                    println!("{}── {}", parts[0], parts[1].cyan());
                } else {
                    println!("{}", line);
                }
            } else if !line.trim().is_empty() {
                println!("{}", line.cyan());
            } else {
                println!("{}", line);
            }
        } else {
            println!("{}", line);
        }
    }

    println!();

    // Stats
    let stats = graph.stats();
    if use_color {
        println!("{}", "Resolved Configuration:".bold());
    } else {
        println!("Resolved Configuration:");
    }

    // Load resolved workload to get accurate counts
    let resolved = manager
        .load_resolved(workload_name)
        .with_context(|| format!("Failed to resolve workload: {}", workload_name))?;

    let package_count = resolved.package_count();
    let file_count = resolved.file_count();
    let script_count = resolved.script_count();

    if use_color {
        println!(
            "  Packages: {} (from {} workload(s))",
            package_count.to_string().cyan(),
            stats.total_workloads
        );
        println!("  Files: {}", file_count.to_string().cyan());
        println!("  Scripts: {}", script_count.to_string().cyan());
        println!(
            "  Inheritance depth: {}",
            stats.max_depth.to_string().cyan()
        );
    } else {
        println!(
            "  Packages: {} (from {} workload(s))",
            package_count, stats.total_workloads
        );
        println!("  Files: {}", file_count);
        println!("  Scripts: {}", script_count);
        println!("  Inheritance depth: {}", stats.max_depth);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::workload::{Packages, WingetPackage};

    #[test]
    fn test_script_count_empty() {
        let workload = Workload::new("test", "1.0.0", "Test workload");
        assert_eq!(workload.script_count(), 0);
    }

    #[test]
    fn test_script_count_with_scripts() {
        use crate::config::workload::{ScriptEntry, Scripts};

        let mut workload = Workload::new("test", "1.0.0", "Test workload");
        workload.scripts = Some(Scripts {
            pre_install: Some(vec![ScriptEntry::new("pre.ps1")]),
            post_install: Some(vec![
                ScriptEntry::new("post1.ps1"),
                ScriptEntry::new("post2.ps1"),
            ]),
        });

        assert_eq!(workload.script_count(), 3);
    }

    #[test]
    fn test_workload_package_count() {
        let mut workload = Workload::new("test", "1.0.0", "Test workload");
        assert_eq!(workload.package_count(), 0);

        workload.packages = Some(Packages {
            winget: Some(vec![
                WingetPackage::new("Package.One"),
                WingetPackage::new("Package.Two"),
            ]),
            ..Default::default()
        });
        assert_eq!(workload.package_count(), 2);
    }
}
