//! Output formatting module for Winforge CLI
//!
//! This module provides utilities for formatting output in various formats:
//! - Table: Human-readable formatted tables
//! - JSON: Machine-readable JSON output
//! - YAML: Human-readable YAML output
//! - HTML: Report-style HTML output

use std::io::Write;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use serde::Serialize;

use crate::cli::commands::OutputFormat;

/// Status indicator for check results
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckStatus {
    /// Check passed successfully
    Ok,
    /// Check failed
    Fail,
    /// Check passed with warnings
    Warn,
    /// Check was skipped
    Skip,
}

#[allow(dead_code)]
impl CheckStatus {
    /// Returns the display symbol for the status
    pub fn symbol(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Warn => "⚠",
            CheckStatus::Skip => "○",
        }
    }

    /// Returns the colored display string
    pub fn colored_symbol(&self) -> String {
        use colored::Colorize;
        match self {
            CheckStatus::Ok => "✓".green().to_string(),
            CheckStatus::Fail => "✗".red().to_string(),
            CheckStatus::Warn => "⚠".yellow().to_string(),
            CheckStatus::Skip => "○".dimmed().to_string(),
        }
    }

    /// Returns the color for table cells
    pub fn table_color(&self) -> Color {
        match self {
            CheckStatus::Ok => Color::Green,
            CheckStatus::Fail => Color::Red,
            CheckStatus::Warn => Color::Yellow,
            CheckStatus::Skip => Color::Grey,
        }
    }
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Ok => write!(f, "OK"),
            CheckStatus::Fail => write!(f, "FAIL"),
            CheckStatus::Warn => write!(f, "WARN"),
            CheckStatus::Skip => write!(f, "SKIP"),
        }
    }
}

/// A single check result for reporting
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Name of the check
    pub name: String,
    /// Status of the check
    pub status: CheckStatus,
    /// Optional message with details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Category of the check (e.g., "Packages", "Files", "Scripts")
    pub category: String,
    /// Optional detailed output (e.g., script output for failed health checks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
}

#[allow(dead_code)]
impl CheckResult {
    /// Create a new passing check result
    pub fn ok(name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Ok,
            message: None,
            category: category.into(),
            details: None,
        }
    }

    /// Create a new passing check result with a message
    pub fn ok_with_message(
        name: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Ok,
            message: Some(message.into()),
            category: category.into(),
            details: None,
        }
    }

    /// Create a new failing check result
    pub fn fail(
        name: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: Some(message.into()),
            category: category.into(),
            details: None,
        }
    }

    /// Create a new failing check result with details
    pub fn fail_with_details(
        name: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: Some(message.into()),
            category: category.into(),
            details: if details.is_empty() {
                None
            } else {
                Some(details)
            },
        }
    }

    /// Create a new warning check result
    pub fn warn(
        name: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            message: Some(message.into()),
            category: category.into(),
            details: None,
        }
    }

    /// Create a skipped check result
    pub fn skip(
        name: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Skip,
            message: Some(message.into()),
            category: category.into(),
            details: None,
        }
    }
}

/// Summary of health check results
#[derive(Debug, Clone, Serialize)]
pub struct HealthSummary {
    /// Total number of checks
    pub total: usize,
    /// Number of passed checks
    pub passed: usize,
    /// Number of failed checks
    pub failed: usize,
    /// Number of warnings
    pub warnings: usize,
    /// Number of skipped checks
    pub skipped: usize,
}

#[allow(dead_code)]
impl HealthSummary {
    /// Calculate summary from a list of check results
    pub fn from_results(results: &[CheckResult]) -> Self {
        let mut summary = Self {
            total: results.len(),
            passed: 0,
            failed: 0,
            warnings: 0,
            skipped: 0,
        };

        for result in results {
            match result.status {
                CheckStatus::Ok => summary.passed += 1,
                CheckStatus::Fail => summary.failed += 1,
                CheckStatus::Warn => summary.warnings += 1,
                CheckStatus::Skip => summary.skipped += 1,
            }
        }

        summary
    }

    /// Returns true if all checks passed (no failures)
    pub fn is_healthy(&self) -> bool {
        self.failed == 0
    }

    /// Returns true if all checks passed with no warnings
    pub fn is_perfect(&self) -> bool {
        self.failed == 0 && self.warnings == 0
    }
}

/// Health check report structure
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    /// Name of the workload being checked
    pub workload: String,
    /// Timestamp of the check
    pub timestamp: String,
    /// Overall status
    pub overall_status: CheckStatus,
    /// Summary statistics
    pub summary: HealthSummary,
    /// Individual check results
    pub checks: Vec<CheckResult>,
}

impl HealthReport {
    /// Create a new health report
    pub fn new(workload: impl Into<String>, checks: Vec<CheckResult>) -> Self {
        let summary = HealthSummary::from_results(&checks);
        let overall_status = if summary.failed > 0 {
            CheckStatus::Fail
        } else if summary.warnings > 0 {
            CheckStatus::Warn
        } else {
            CheckStatus::Ok
        };

        Self {
            workload: workload.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            overall_status,
            summary,
            checks,
        }
    }
}

/// Output formatter enum for different output formats
/// Using enum dispatch instead of trait objects for dyn-compatibility
pub enum Formatter {
    Table(TableFormatter),
    Json(JsonFormatter),
    Yaml(YamlFormatter),
}

#[allow(dead_code)]
impl Formatter {
    /// Format and write a health report
    pub fn format_health_report<W: Write>(
        &self,
        report: &HealthReport,
        writer: &mut W,
    ) -> Result<()> {
        match self {
            Formatter::Table(f) => f.format_health_report(report, writer),
            Formatter::Json(f) => f.format_health_report(report, writer),
            Formatter::Yaml(f) => f.format_health_report(report, writer),
        }
    }

    /// Format and write a workload list
    pub fn format_workload_list<W: Write>(
        &self,
        workloads: &[WorkloadInfo],
        writer: &mut W,
    ) -> Result<()> {
        match self {
            Formatter::Table(f) => f.format_workload_list(workloads, writer),
            Formatter::Json(f) => f.format_workload_list(workloads, writer),
            Formatter::Yaml(f) => f.format_workload_list(workloads, writer),
        }
    }
}

/// Basic workload information for listing
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct WorkloadInfo {
    /// Workload name
    pub name: String,
    /// Workload version
    pub version: String,
    /// Short description
    pub description: String,
    /// Parent workloads (if any)
    pub extends: Vec<String>,
    /// Number of packages
    pub package_count: usize,
    /// Number of files
    pub file_count: usize,
}

/// Table formatter for human-readable output
pub struct TableFormatter {
    /// Whether to use colored output
    pub use_color: bool,
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
}

#[allow(dead_code)]
impl TableFormatter {
    pub fn format_health_report<W: Write>(
        &self,
        report: &HealthReport,
        writer: &mut W,
    ) -> Result<()> {
        writeln!(writer)?;

        // Print title
        if self.use_color {
            use colored::Colorize;
            writeln!(writer, "{}", "Health Check Report".bold())?;
            writeln!(writer, "Workload: {}", report.workload.cyan())?;
        } else {
            writeln!(writer, "Health Check Report")?;
            writeln!(writer, "Workload: {}", report.workload)?;
        }
        writeln!(writer)?;

        // Build the main table
        let mut table = self.create_table();
        table.set_header(vec![
            Cell::new("Component").fg(Color::White),
            Cell::new("Status").fg(Color::White),
            Cell::new("Details").fg(Color::White),
        ]);

        let mut current_category = String::new();
        for check in &report.checks {
            if check.category != current_category {
                current_category = check.category.clone();
                table.add_row(vec![
                    Cell::new(&current_category).fg(Color::Cyan),
                    Cell::new(""),
                    Cell::new(""),
                ]);
            }

            let status_cell = if self.use_color {
                Cell::new(format!("{} {}", check.status.symbol(), check.status))
                    .fg(check.status.table_color())
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

        // Print summary
        let summary = if self.use_color {
            use colored::Colorize;
            format!(
                "Summary: {} passed, {} failed, {} warning(s)",
                report.summary.passed.to_string().green(),
                if report.summary.failed > 0 {
                    report.summary.failed.to_string().red()
                } else {
                    report.summary.failed.to_string().normal()
                },
                if report.summary.warnings > 0 {
                    report.summary.warnings.to_string().yellow()
                } else {
                    report.summary.warnings.to_string().normal()
                }
            )
        } else {
            format!(
                "Summary: {} passed, {} failed, {} warning(s)",
                report.summary.passed, report.summary.failed, report.summary.warnings
            )
        };
        writeln!(writer, "{}", summary)?;

        Ok(())
    }

    pub fn format_workload_list<W: Write>(
        &self,
        workloads: &[WorkloadInfo],
        writer: &mut W,
    ) -> Result<()> {
        let mut table = self.create_table();
        table.set_header(vec!["Name", "Version", "Description", "Extends"]);

        for workload in workloads {
            table.add_row(vec![
                Cell::new(&workload.name),
                Cell::new(&workload.version),
                Cell::new(&workload.description),
                Cell::new(workload.extends.join(", ")),
            ]);
        }

        writeln!(writer, "{}", table)?;
        Ok(())
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter {
    /// Whether to pretty-print the output
    pub pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }
}

#[allow(dead_code)]
impl JsonFormatter {
    pub fn format_health_report<W: Write>(
        &self,
        report: &HealthReport,
        writer: &mut W,
    ) -> Result<()> {
        let json = if self.pretty {
            serde_json::to_string_pretty(report)?
        } else {
            serde_json::to_string(report)?
        };
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    pub fn format_workload_list<W: Write>(
        &self,
        workloads: &[WorkloadInfo],
        writer: &mut W,
    ) -> Result<()> {
        let json = if self.pretty {
            serde_json::to_string_pretty(workloads)?
        } else {
            serde_json::to_string(workloads)?
        };
        writeln!(writer, "{}", json)?;
        Ok(())
    }
}

/// YAML formatter for human-readable structured output
pub struct YamlFormatter;

#[allow(dead_code)]
impl YamlFormatter {
    pub fn format_health_report<W: Write>(
        &self,
        report: &HealthReport,
        writer: &mut W,
    ) -> Result<()> {
        let yaml = serde_yaml::to_string(report)?;
        writeln!(writer, "{}", yaml)?;
        Ok(())
    }

    pub fn format_workload_list<W: Write>(
        &self,
        workloads: &[WorkloadInfo],
        writer: &mut W,
    ) -> Result<()> {
        let yaml = serde_yaml::to_string(workloads)?;
        writeln!(writer, "{}", yaml)?;
        Ok(())
    }
}

/// Get the appropriate formatter for the given output format
pub fn get_formatter(format: OutputFormat, use_color: bool) -> Formatter {
    match format {
        OutputFormat::Table => Formatter::Table(TableFormatter::new(use_color)),
        OutputFormat::Json => Formatter::Json(JsonFormatter::new(true)),
        OutputFormat::Yaml => Formatter::Yaml(YamlFormatter),
        OutputFormat::Html => {
            // For now, fall back to table format
            // HTML formatter can be implemented later
            Formatter::Table(TableFormatter::new(false))
        }
    }
}

/// Print a success message to stdout
pub fn print_success(message: &str) {
    use colored::Colorize;
    println!("{} {}", "✓".green(), message);
}

/// Print an error message to stderr
pub fn print_error(message: &str) {
    use colored::Colorize;
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message to stderr
pub fn print_warning(message: &str) {
    use colored::Colorize;
    eprintln!("{} {}", "⚠".yellow(), message);
}

/// Print an info message to stdout
pub fn print_info(message: &str) {
    use colored::Colorize;
    println!("{} {}", "ℹ".blue(), message);
}
