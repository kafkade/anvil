//! YAML output formatter for human-readable structured output
//!
//! This module provides YAML output that is both human-readable
//! and machine-parseable.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use super::{InstallSummary, OutputFormatter, OutputMetadata, ValidationResults};
use crate::cli::output::HealthReport;
use crate::config::WorkloadInfo;

/// YAML formatter for structured output
pub struct YamlFormatter;

impl YamlFormatter {
    /// Create a new YAML formatter
    pub fn new() -> Self {
        Self
    }

    /// Serialize data to YAML
    fn serialize<T: Serialize>(&self, data: &T) -> Result<String> {
        let yaml = serde_yaml::to_string(data)?;
        Ok(yaml)
    }
}

impl Default for YamlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for YamlFormatter {
    fn format_list(&self, workloads: &[WorkloadInfo], writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct ListOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            workloads: &'a [WorkloadInfo],
        }

        let output = ListOutput {
            metadata: OutputMetadata::default(),
            workloads,
        };

        let yaml = self.serialize(&output)?;
        write!(writer, "{}", yaml)?;
        Ok(())
    }

    fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct HealthOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            workload: &'a str,
            report_timestamp: &'a str,
            overall_status: String,
            summary: SummaryOutput,
            checks: Vec<CheckOutput<'a>>,
        }

        #[derive(Serialize)]
        struct SummaryOutput {
            total: usize,
            passed: usize,
            failed: usize,
            warnings: usize,
            skipped: usize,
        }

        #[derive(Serialize)]
        struct CheckOutput<'a> {
            name: &'a str,
            category: &'a str,
            status: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<&'a str>,
        }

        let output = HealthOutput {
            metadata: OutputMetadata::default(),
            workload: &report.workload,
            report_timestamp: &report.timestamp,
            overall_status: report.overall_status.to_string(),
            summary: SummaryOutput {
                total: report.summary.total,
                passed: report.summary.passed,
                failed: report.summary.failed,
                warnings: report.summary.warnings,
                skipped: report.summary.skipped,
            },
            checks: report
                .checks
                .iter()
                .map(|c| CheckOutput {
                    name: &c.name,
                    category: &c.category,
                    status: c.status.to_string(),
                    message: c.message.as_deref(),
                })
                .collect(),
        };

        let yaml = self.serialize(&output)?;
        write!(writer, "{}", yaml)?;
        Ok(())
    }

    fn format_install(&self, summary: &InstallSummary, writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct InstallOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            workload: &'a str,
            success: bool,
            packages: PackageStats,
            files: FileStats,
            scripts: ScriptStats,
            duration_secs: f64,
            reboot_required: bool,
            #[serde(skip_serializing_if = "slice_is_empty")]
            failed_items: &'a [String],
        }

        fn slice_is_empty(s: &&[String]) -> bool {
            s.is_empty()
        }

        #[derive(Serialize)]
        struct PackageStats {
            installed: usize,
            skipped: usize,
            failed: usize,
        }

        #[derive(Serialize)]
        struct FileStats {
            copied: usize,
            skipped: usize,
            failed: usize,
        }

        #[derive(Serialize)]
        struct ScriptStats {
            executed: usize,
            failed: usize,
        }

        let output = InstallOutput {
            metadata: OutputMetadata::default(),
            workload: &summary.workload,
            success: summary.is_successful(),
            packages: PackageStats {
                installed: summary.packages_installed,
                skipped: summary.packages_skipped,
                failed: summary.packages_failed,
            },
            files: FileStats {
                copied: summary.files_copied,
                skipped: summary.files_skipped,
                failed: summary.files_failed,
            },
            scripts: ScriptStats {
                executed: summary.scripts_executed,
                failed: summary.scripts_failed,
            },
            duration_secs: summary.duration_secs,
            reboot_required: summary.reboot_required,
            failed_items: &summary.failed_items,
        };

        let yaml = self.serialize(&output)?;
        write!(writer, "{}", yaml)?;
        Ok(())
    }

    fn format_validate(&self, results: &ValidationResults, writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct ValidateOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            workload: &'a str,
            valid: bool,
            error_count: usize,
            warning_count: usize,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            errors: Vec<IssueOutput<'a>>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            warnings: Vec<IssueOutput<'a>>,
        }

        #[derive(Serialize)]
        struct IssueOutput<'a> {
            location: &'a str,
            message: &'a str,
        }

        let output = ValidateOutput {
            metadata: OutputMetadata::default(),
            workload: &results.workload,
            valid: results.valid,
            error_count: results.errors.len(),
            warning_count: results.warnings.len(),
            errors: results
                .errors
                .iter()
                .map(|e| IssueOutput {
                    location: &e.location,
                    message: &e.message,
                })
                .collect(),
            warnings: results
                .warnings
                .iter()
                .map(|w| IssueOutput {
                    location: &w.location,
                    message: &w.message,
                })
                .collect(),
        };

        let yaml = self.serialize(&output)?;
        write!(writer, "{}", yaml)?;
        Ok(())
    }

    fn extension(&self) -> &'static str {
        "yaml"
    }

    fn mime_type(&self) -> &'static str {
        "application/x-yaml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_formatter_creation() {
        let formatter = YamlFormatter::new();
        assert_eq!(formatter.extension(), "yaml");
        assert_eq!(formatter.mime_type(), "application/x-yaml");
    }

    #[test]
    fn test_yaml_formatter_default() {
        let formatter = YamlFormatter::default();
        assert_eq!(formatter.extension(), "yaml");
    }

    #[test]
    fn test_yaml_format_list() {
        let formatter = YamlFormatter::new();
        let mut output = Vec::new();

        let workloads = vec![WorkloadInfo {
            name: "test-workload".to_string(),
            version: "1.0.0".to_string(),
            description: "A test workload".to_string(),
            extends: vec!["base".to_string()],
            package_count: 5,
            file_count: 3,
            path: std::path::PathBuf::from("workloads/test-workload"),
        }];

        formatter.format_list(&workloads, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        assert!(content.contains("test-workload"));
        assert!(content.contains("1.0.0"));
        assert!(content.contains("winforge_version"));
    }

    #[test]
    fn test_yaml_format_install_summary() {
        let formatter = YamlFormatter::new();
        let mut output = Vec::new();

        let summary = InstallSummary {
            workload: "test".to_string(),
            packages_installed: 5,
            packages_skipped: 2,
            packages_failed: 1,
            files_copied: 3,
            files_skipped: 0,
            files_failed: 0,
            scripts_executed: 2,
            scripts_failed: 0,
            duration_secs: 45.5,
            reboot_required: false,
            failed_items: vec!["Package.Failed".to_string()],
            timestamp: chrono::Utc::now(),
        };

        formatter.format_install(&summary, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        assert!(content.contains("packages:"));
        assert!(content.contains("installed: 5"));
        assert!(content.contains("failed_items:"));
        assert!(content.contains("Package.Failed"));
    }

    #[test]
    fn test_yaml_format_validation() {
        let formatter = YamlFormatter::new();
        let mut output = Vec::new();

        let results = ValidationResults {
            workload: "test".to_string(),
            valid: false,
            errors: vec![super::super::ValidationError {
                location: "packages.winget[0]".to_string(),
                message: "Missing package ID".to_string(),
            }],
            warnings: vec![],
            timestamp: chrono::Utc::now(),
        };

        formatter.format_validate(&results, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        assert!(content.contains("valid: false"));
        assert!(content.contains("error_count: 1"));
        assert!(content.contains("Missing package ID"));
    }
}
