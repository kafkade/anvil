//! Table output formatter for human-readable CLI output
//!
//! This module provides formatted table output for terminal display
//! with optional color support.

use std::io::Write;

use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use super::{InstallSummary, OutputFormatter, ValidationResults};
use crate::cli::output::{CheckStatus, HealthReport};
use crate::config::WorkloadInfo;

/// Table formatter for human-readable output
pub struct TableFormatter {
    /// Whether to use colored output
    use_color: bool,
}

impl TableFormatter {
    /// Create a new table formatter
    pub fn new(use_color: bool) -> Self {
        Self { use_color }
    }

    /// Create a styled table
    fn create_table(&self) -> Table {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);
        table
    }

    /// Get the color for a status
    fn status_color(&self, status: &CheckStatus) -> Color {
        match status {
            CheckStatus::Ok => Color::Green,
            CheckStatus::Fail => Color::Red,
            CheckStatus::Warn => Color::Yellow,
            CheckStatus::Skip => Color::Grey,
        }
    }
}

impl OutputFormatter for TableFormatter {
    fn format_list(&self, workloads: &[WorkloadInfo], writer: &mut dyn Write) -> Result<()> {
        if workloads.is_empty() {
            if self.use_color {
                writeln!(writer, "{} No workloads found.", "ℹ".blue())?;
            } else {
                writeln!(writer, "No workloads found.")?;
            }
            return Ok(());
        }

        let mut table = self.create_table();

        // Set header with optional color
        let headers = if self.use_color {
            vec![
                Cell::new("Name").fg(Color::Cyan),
                Cell::new("Version").fg(Color::Cyan),
                Cell::new("Description").fg(Color::Cyan),
                Cell::new("Extends").fg(Color::Cyan),
                Cell::new("Packages").fg(Color::Cyan),
                Cell::new("Files").fg(Color::Cyan),
            ]
        } else {
            vec![
                Cell::new("Name"),
                Cell::new("Version"),
                Cell::new("Description"),
                Cell::new("Extends"),
                Cell::new("Packages"),
                Cell::new("Files"),
            ]
        };
        table.set_header(headers);

        for workload in workloads {
            let extends_str = if workload.extends.is_empty() {
                "-".to_string()
            } else {
                workload.extends.join(", ")
            };

            let description = truncate(&workload.description, 35);

            let row = if self.use_color {
                vec![
                    Cell::new(&workload.name).fg(Color::Green),
                    Cell::new(&workload.version),
                    Cell::new(&description),
                    Cell::new(&extends_str).fg(Color::Yellow),
                    Cell::new(workload.package_count.to_string()),
                    Cell::new(workload.file_count.to_string()),
                ]
            } else {
                vec![
                    Cell::new(&workload.name),
                    Cell::new(&workload.version),
                    Cell::new(&description),
                    Cell::new(&extends_str),
                    Cell::new(workload.package_count.to_string()),
                    Cell::new(workload.file_count.to_string()),
                ]
            };
            table.add_row(row);
        }

        writeln!(writer, "{}", table)?;
        writeln!(writer)?;

        // Print summary
        if self.use_color {
            writeln!(
                writer,
                "{} {} workload(s) available",
                "✓".green(),
                workloads.len().to_string().bold()
            )?;
        } else {
            writeln!(writer, "{} workload(s) available", workloads.len())?;
        }

        Ok(())
    }

    fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()> {
        writeln!(writer)?;
        writeln!(
            writer,
            "╭─────────────────────────────────────────────────────────────────────╮"
        )?;
        writeln!(
            writer,
            "│                    Anvil Health Check Report                     │"
        )?;
        writeln!(
            writer,
            "│                    Workload: {:<40} │",
            truncate(&report.workload, 40)
        )?;
        writeln!(
            writer,
            "╰─────────────────────────────────────────────────────────────────────╯"
        )?;
        writeln!(writer)?;

        let mut table = self.create_table();
        table.set_header(vec!["Component", "Status", "Details"]);

        let mut current_category = String::new();
        for check in &report.checks {
            // Add category header if changed
            if check.category != current_category {
                current_category = check.category.clone();
                if self.use_color {
                    table.add_row(vec![
                        Cell::new(&current_category).fg(Color::White),
                        Cell::new(""),
                        Cell::new(""),
                    ]);
                } else {
                    table.add_row(vec![
                        Cell::new(&current_category),
                        Cell::new(""),
                        Cell::new(""),
                    ]);
                }
            }

            let status_cell = if self.use_color {
                Cell::new(format!("{} {}", check.status.symbol(), check.status))
                    .fg(self.status_color(&check.status))
            } else {
                Cell::new(check.status.to_string())
            };

            table.add_row(vec![
                Cell::new(&check.name),
                status_cell,
                Cell::new(check.message.as_deref().unwrap_or("")),
            ]);
        }

        writeln!(writer, "{}", table)?;
        writeln!(writer)?;

        // Summary line
        let overall = match report.overall_status {
            CheckStatus::Ok => {
                if self.use_color {
                    format!("{} All checks passed!", "✓".green())
                } else {
                    "All checks passed!".to_string()
                }
            }
            CheckStatus::Warn => {
                if self.use_color {
                    format!("{} Some warnings detected", "⚠".yellow())
                } else {
                    "Some warnings detected".to_string()
                }
            }
            CheckStatus::Fail => {
                if self.use_color {
                    format!("{} Health check failed", "✗".red())
                } else {
                    "Health check failed".to_string()
                }
            }
            CheckStatus::Skip => "Checks skipped".to_string(),
        };

        writeln!(writer, "{}", overall)?;
        writeln!(
            writer,
            "Summary: {} passed, {} failed, {} warning(s), {} skipped",
            report.summary.passed,
            report.summary.failed,
            report.summary.warnings,
            report.summary.skipped
        )?;

        Ok(())
    }

    fn format_install(&self, summary: &InstallSummary, writer: &mut dyn Write) -> Result<()> {
        writeln!(writer)?;
        writeln!(
            writer,
            "╭─────────────────────────────────────────────────────────────────────╮"
        )?;
        writeln!(
            writer,
            "│                    Installation Summary                              │"
        )?;
        writeln!(
            writer,
            "╰─────────────────────────────────────────────────────────────────────╯"
        )?;
        writeln!(writer)?;

        // Status indicator
        let status = if summary.is_successful() {
            if self.use_color {
                format!("{} Installation completed successfully!", "✓".green())
            } else {
                "Installation completed successfully!".to_string()
            }
        } else {
            if self.use_color {
                format!("{} Installation completed with errors", "✗".red())
            } else {
                "Installation completed with errors".to_string()
            }
        };
        writeln!(writer, "{}", status)?;
        writeln!(writer)?;

        // Packages section
        if self.use_color {
            writeln!(writer, "{}", "📦 Packages:".bold())?;
        } else {
            writeln!(writer, "Packages:")?;
        }
        writeln!(writer, "   Installed: {}", summary.packages_installed)?;
        writeln!(writer, "   Skipped:   {}", summary.packages_skipped)?;
        writeln!(writer, "   Failed:    {}", summary.packages_failed)?;
        writeln!(writer)?;

        // Files section
        if self.use_color {
            writeln!(writer, "{}", "📄 Files:".bold())?;
        } else {
            writeln!(writer, "Files:")?;
        }
        writeln!(writer, "   Copied:    {}", summary.files_copied)?;
        writeln!(writer, "   Skipped:   {}", summary.files_skipped)?;
        writeln!(writer, "   Failed:    {}", summary.files_failed)?;
        writeln!(writer)?;

        // Scripts section
        if self.use_color {
            writeln!(writer, "{}", "📜 Scripts:".bold())?;
        } else {
            writeln!(writer, "Scripts:")?;
        }
        writeln!(writer, "   Executed:  {}", summary.scripts_executed)?;
        writeln!(writer, "   Failed:    {}", summary.scripts_failed)?;
        writeln!(writer)?;

        // Duration
        writeln!(writer, "Duration: {:.2}s", summary.duration_secs)?;

        // Reboot warning
        if summary.reboot_required {
            writeln!(writer)?;
            if self.use_color {
                writeln!(
                    writer,
                    "{} A system reboot is required to complete the installation.",
                    "⚠".yellow()
                )?;
            } else {
                writeln!(
                    writer,
                    "WARNING: A system reboot is required to complete the installation."
                )?;
            }
        }

        // Failed items
        if !summary.failed_items.is_empty() {
            writeln!(writer)?;
            if self.use_color {
                writeln!(writer, "{}", "Failed Items:".red().bold())?;
            } else {
                writeln!(writer, "Failed Items:")?;
            }
            for item in &summary.failed_items {
                writeln!(writer, "   - {}", item)?;
            }
        }

        Ok(())
    }

    fn format_validate(&self, results: &ValidationResults, writer: &mut dyn Write) -> Result<()> {
        writeln!(writer)?;

        let status = if results.valid {
            if self.use_color {
                format!("{} Validation passed!", "✓".green())
            } else {
                "Validation passed!".to_string()
            }
        } else {
            if self.use_color {
                format!("{} Validation failed", "✗".red())
            } else {
                "Validation failed".to_string()
            }
        };

        writeln!(writer, "{}", status)?;
        writeln!(writer, "Workload: {}", results.workload)?;
        writeln!(writer)?;

        // Errors
        if !results.errors.is_empty() {
            if self.use_color {
                writeln!(writer, "{}", "Errors:".red().bold())?;
            } else {
                writeln!(writer, "Errors:")?;
            }
            for error in &results.errors {
                if self.use_color {
                    writeln!(
                        writer,
                        "  {} [{}] {}",
                        "✗".red(),
                        error.location,
                        error.message
                    )?;
                } else {
                    writeln!(writer, "  [{}] {}", error.location, error.message)?;
                }
            }
            writeln!(writer)?;
        }

        // Warnings
        if !results.warnings.is_empty() {
            if self.use_color {
                writeln!(writer, "{}", "Warnings:".yellow().bold())?;
            } else {
                writeln!(writer, "Warnings:")?;
            }
            for warning in &results.warnings {
                if self.use_color {
                    writeln!(
                        writer,
                        "  {} [{}] {}",
                        "⚠".yellow(),
                        warning.location,
                        warning.message
                    )?;
                } else {
                    writeln!(writer, "  [{}] {}", warning.location, warning.message)?;
                }
            }
            writeln!(writer)?;
        }

        // Summary
        writeln!(
            writer,
            "Summary: {} error(s), {} warning(s)",
            results.errors.len(),
            results.warnings.len()
        )?;

        Ok(())
    }

    fn extension(&self) -> &'static str {
        "txt"
    }

    fn mime_type(&self) -> &'static str {
        "text/plain"
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_table_formatter_creation() {
        let formatter = TableFormatter::new(true);
        assert_eq!(formatter.extension(), "txt");
        assert_eq!(formatter.mime_type(), "text/plain");
    }
}
