//! Output format module for Anvil CLI
//!
//! This module provides multiple output formats for CLI commands:
//! - Table: Human-readable formatted tables (default)
//! - JSON: Machine-readable JSON output
//! - YAML: Human-readable YAML output
//! - HTML: Report-style HTML output

pub mod html;
pub mod json;
pub mod table;
pub mod yaml;

use std::io::Write;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::cli::output::HealthReport;
use crate::config::WorkloadInfo;

/// Metadata included in output
#[derive(Debug, Clone, Serialize)]
pub struct OutputMetadata {
    /// Timestamp of the output
    pub timestamp: DateTime<Utc>,
    /// Anvil version
    pub anvil_version: String,
    /// Optional workload version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload_version: Option<String>,
}

impl Default for OutputMetadata {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            anvil_version: env!("CARGO_PKG_VERSION").to_string(),
            workload_version: None,
        }
    }
}

impl OutputMetadata {
    /// Create metadata with workload version
    pub fn with_workload_version(version: impl Into<String>) -> Self {
        Self {
            workload_version: Some(version.into()),
            ..Default::default()
        }
    }
}

/// Installation summary for reporting
#[derive(Debug, Clone, Serialize)]
pub struct InstallSummary {
    /// Workload name
    pub workload: String,
    /// Number of packages installed
    pub packages_installed: usize,
    /// Number of packages skipped
    pub packages_skipped: usize,
    /// Number of packages failed
    pub packages_failed: usize,
    /// Number of files copied
    pub files_copied: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Number of files failed
    pub files_failed: usize,
    /// Scripts executed
    pub scripts_executed: usize,
    /// Scripts failed
    pub scripts_failed: usize,
    /// Total duration in seconds
    pub duration_secs: f64,
    /// Whether reboot is required
    pub reboot_required: bool,
    /// List of failed items
    pub failed_items: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl Default for InstallSummary {
    fn default() -> Self {
        Self {
            workload: String::new(),
            packages_installed: 0,
            packages_skipped: 0,
            packages_failed: 0,
            files_copied: 0,
            files_skipped: 0,
            files_failed: 0,
            scripts_executed: 0,
            scripts_failed: 0,
            duration_secs: 0.0,
            reboot_required: false,
            failed_items: Vec::new(),
            timestamp: Utc::now(),
        }
    }
}

impl InstallSummary {
    /// Create a new install summary
    pub fn new(workload: impl Into<String>) -> Self {
        Self {
            workload: workload.into(),
            ..Default::default()
        }
    }

    /// Check if installation was successful
    pub fn is_successful(&self) -> bool {
        self.packages_failed == 0 && self.files_failed == 0 && self.scripts_failed == 0
    }
}

/// Validation results for output
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResults {
    /// Workload path or name
    pub workload: String,
    /// Whether validation passed
    pub valid: bool,
    /// List of errors
    pub errors: Vec<ValidationError>,
    /// List of warnings
    pub warnings: Vec<ValidationWarning>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// A validation error
#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    /// Error location (field path)
    pub location: String,
    /// Error message
    pub message: String,
}

/// A validation warning
#[derive(Debug, Clone, Serialize)]
pub struct ValidationWarning {
    /// Warning location (field path)
    pub location: String,
    /// Warning message
    pub message: String,
}

/// Wrapper for workload list output with metadata
#[derive(Debug, Clone, Serialize)]
pub struct WorkloadListOutput {
    /// Output metadata
    #[serde(flatten)]
    pub metadata: OutputMetadata,
    /// List of workloads
    pub workloads: Vec<WorkloadInfo>,
}

impl WorkloadListOutput {
    /// Create a new workload list output
    pub fn new(workloads: Vec<WorkloadInfo>) -> Self {
        Self {
            metadata: OutputMetadata::default(),
            workloads,
        }
    }
}

/// Wrapper for health report output with metadata
#[derive(Debug, Clone, Serialize)]
pub struct HealthReportOutput {
    /// Output metadata
    #[serde(flatten)]
    pub metadata: OutputMetadata,
    /// Health report data
    #[serde(flatten)]
    pub report: HealthReport,
}

impl HealthReportOutput {
    /// Create a new health report output
    pub fn new(report: HealthReport) -> Self {
        Self {
            metadata: OutputMetadata::default(),
            report,
        }
    }
}

/// Output format trait for formatting different types of output
pub trait OutputFormatter {
    /// Format a workload list
    fn format_list(&self, workloads: &[WorkloadInfo], writer: &mut dyn Write) -> Result<()>;

    /// Format a health report
    fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()>;

    /// Format an installation summary
    fn format_install(&self, summary: &InstallSummary, writer: &mut dyn Write) -> Result<()>;

    /// Format validation results
    fn format_validate(&self, results: &ValidationResults, writer: &mut dyn Write) -> Result<()>;

    /// Get the file extension for this format
    fn extension(&self) -> &'static str;

    /// Get the MIME type for this format
    fn mime_type(&self) -> &'static str;
}

/// Factory for creating formatters
pub fn create_formatter(
    format: crate::cli::commands::OutputFormat,
    use_color: bool,
    compact: bool,
) -> Box<dyn OutputFormatter> {
    use crate::cli::commands::OutputFormat;

    match format {
        OutputFormat::Table => Box::new(table::TableFormatter::new(use_color)),
        OutputFormat::Json => Box::new(json::JsonFormatter::new(!compact)),
        OutputFormat::Yaml => Box::new(yaml::YamlFormatter::new()),
        OutputFormat::Html => Box::new(html::HtmlFormatter::new()),
    }
}

/// Format output and optionally write to file
pub fn format_to_file_or_stdout<T: Serialize>(
    formatter: &dyn OutputFormatter,
    data: &T,
    file_path: Option<&std::path::Path>,
) -> Result<()> {
    let mut output: Box<dyn Write> = if let Some(path) = file_path {
        Box::new(std::fs::File::create(path)?)
    } else {
        Box::new(std::io::stdout())
    };

    // Generic serialization for simple cases
    let json = serde_json::to_string_pretty(data)?;
    writeln!(output, "{}", json)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_metadata_default() {
        let meta = OutputMetadata::default();
        assert_eq!(meta.anvil_version, env!("CARGO_PKG_VERSION"));
        assert!(meta.workload_version.is_none());
    }

    #[test]
    fn test_output_metadata_with_version() {
        let meta = OutputMetadata::with_workload_version("1.0.0");
        assert_eq!(meta.workload_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_install_summary_success() {
        let summary = InstallSummary::new("test");
        assert!(summary.is_successful());
    }

    #[test]
    fn test_install_summary_failure() {
        let mut summary = InstallSummary::new("test");
        summary.packages_failed = 1;
        assert!(!summary.is_successful());
    }
}
