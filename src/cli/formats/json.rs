//! JSON output formatter for machine-readable output
//!
//! This module provides JSON output with metadata including
//! timestamps and version information.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use super::{
    InstallSummary, OutputFormatter, OutputMetadata, ValidationResults, WorkloadListOutput,
};
use crate::cli::output::HealthReport;
use crate::config::WorkloadInfo;

/// JSON formatter for machine-readable output
pub struct JsonFormatter {
    /// Whether to pretty-print the output
    pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    /// Serialize data to JSON
    fn serialize<T: Serialize>(&self, data: &T) -> Result<String> {
        let json = if self.pretty {
            serde_json::to_string_pretty(data)?
        } else {
            serde_json::to_string(data)?
        };
        Ok(json)
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_list(&self, workloads: &[WorkloadInfo], writer: &mut dyn Write) -> Result<()> {
        let output = WorkloadListOutput::new(workloads.to_vec());
        let json = self.serialize(&output)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()> {
        // Create output with metadata
        #[derive(Serialize)]
        struct HealthOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            workload: &'a str,
            timestamp: &'a str,
            overall_status: &'a crate::cli::output::CheckStatus,
            summary: &'a crate::cli::output::HealthSummary,
            checks: &'a [crate::cli::output::CheckResult],
        }

        let output = HealthOutput {
            metadata: OutputMetadata::default(),
            workload: &report.workload,
            timestamp: &report.timestamp,
            overall_status: &report.overall_status,
            summary: &report.summary,
            checks: &report.checks,
        };

        let json = self.serialize(&output)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    fn format_install(&self, summary: &InstallSummary, writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct InstallOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            #[serde(flatten)]
            summary: &'a InstallSummary,
        }

        let output = InstallOutput {
            metadata: OutputMetadata::default(),
            summary,
        };

        let json = self.serialize(&output)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    fn format_validate(&self, results: &ValidationResults, writer: &mut dyn Write) -> Result<()> {
        #[derive(Serialize)]
        struct ValidateOutput<'a> {
            #[serde(flatten)]
            metadata: OutputMetadata,
            #[serde(flatten)]
            results: &'a ValidationResults,
        }

        let output = ValidateOutput {
            metadata: OutputMetadata::default(),
            results,
        };

        let json = self.serialize(&output)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    fn extension(&self) -> &'static str {
        "json"
    }

    fn mime_type(&self) -> &'static str {
        "application/json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatter_pretty() {
        let formatter = JsonFormatter::new(true);
        assert_eq!(formatter.extension(), "json");
        assert_eq!(formatter.mime_type(), "application/json");
    }

    #[test]
    fn test_json_formatter_compact() {
        let formatter = JsonFormatter::new(false);
        let mut output = Vec::new();

        let workloads = vec![WorkloadInfo {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test workload".to_string(),
            extends: vec![],
            package_count: 0,
            file_count: 0,
            path: std::path::PathBuf::from("workloads/test"),
        }];

        formatter.format_list(&workloads, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        // Should not have pretty formatting (single line for main content)
        assert!(content.contains("\"name\":\"test\""));
    }

    #[test]
    fn test_json_formatter_pretty_output() {
        let formatter = JsonFormatter::new(true);
        let mut output = Vec::new();

        let workloads = vec![WorkloadInfo {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test workload".to_string(),
            extends: vec![],
            package_count: 0,
            file_count: 0,
            path: std::path::PathBuf::from("workloads/test"),
        }];

        formatter.format_list(&workloads, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        // Should have pretty formatting (indented)
        assert!(content.contains("  "));
        assert!(content.contains("anvil_version"));
    }
}
