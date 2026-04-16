//! HTML output formatter for standalone report generation
//!
//! This module provides HTML output with embedded CSS for standalone
//! reports that can be opened in any web browser.

use std::io::Write;

use anyhow::Result;
use chrono::Utc;

use super::{InstallSummary, OutputFormatter, ValidationResults};
use crate::cli::output::{CheckStatus, HealthReport};
use crate::config::WorkloadInfo;

/// HTML formatter for report generation
pub struct HtmlFormatter {
    /// Custom title for the report
    title: Option<String>,
}

impl HtmlFormatter {
    /// Create a new HTML formatter
    pub fn new() -> Self {
        Self { title: None }
    }

    /// Create a new HTML formatter with custom title
    #[allow(dead_code)]
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
        }
    }

    /// Get embedded CSS styles
    fn css() -> &'static str {
        r#"
        :root {
            --color-bg: #ffffff;
            --color-bg-secondary: #f8f9fa;
            --color-text: #212529;
            --color-text-secondary: #6c757d;
            --color-border: #dee2e6;
            --color-success: #28a745;
            --color-danger: #dc3545;
            --color-warning: #ffc107;
            --color-info: #17a2b8;
            --color-primary: #007bff;
        }

        @media (prefers-color-scheme: dark) {
            :root {
                --color-bg: #1a1a2e;
                --color-bg-secondary: #16213e;
                --color-text: #eaeaea;
                --color-text-secondary: #a0a0a0;
                --color-border: #3a3a5c;
            }
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            font-size: 14px;
            line-height: 1.6;
            color: var(--color-text);
            background-color: var(--color-bg);
            padding: 20px;
            max-width: 1200px;
            margin: 0 auto;
        }

        header {
            border-bottom: 2px solid var(--color-primary);
            padding-bottom: 20px;
            margin-bottom: 30px;
        }

        h1 {
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 10px;
        }

        h2 {
            font-size: 18px;
            font-weight: 600;
            margin: 20px 0 15px 0;
            padding-bottom: 8px;
            border-bottom: 1px solid var(--color-border);
        }

        .meta {
            color: var(--color-text-secondary);
            font-size: 12px;
        }

        .summary-cards {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin: 20px 0;
        }

        .card {
            background: var(--color-bg-secondary);
            border-radius: 8px;
            padding: 15px;
            border: 1px solid var(--color-border);
        }

        .card-title {
            font-size: 12px;
            text-transform: uppercase;
            color: var(--color-text-secondary);
            margin-bottom: 5px;
        }

        .card-value {
            font-size: 28px;
            font-weight: 600;
        }

        .card-value.success { color: var(--color-success); }
        .card-value.danger { color: var(--color-danger); }
        .card-value.warning { color: var(--color-warning); }
        .card-value.info { color: var(--color-info); }

        table {
            width: 100%;
            border-collapse: collapse;
            margin: 15px 0;
            background: var(--color-bg);
        }

        th, td {
            text-align: left;
            padding: 10px 12px;
            border: 1px solid var(--color-border);
        }

        th {
            background: var(--color-bg-secondary);
            font-weight: 600;
            font-size: 12px;
            text-transform: uppercase;
        }

        tr:nth-child(even) {
            background: var(--color-bg-secondary);
        }

        tr:hover {
            background: rgba(0, 123, 255, 0.05);
        }

        .status {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 4px 10px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 500;
        }

        .status-ok { background: rgba(40, 167, 69, 0.15); color: var(--color-success); }
        .status-fail { background: rgba(220, 53, 69, 0.15); color: var(--color-danger); }
        .status-warn { background: rgba(255, 193, 7, 0.15); color: #856404; }
        .status-skip { background: rgba(108, 117, 125, 0.15); color: var(--color-text-secondary); }

        .status-icon::before {
            font-size: 14px;
        }

        .status-ok .status-icon::before { content: '✓'; }
        .status-fail .status-icon::before { content: '✗'; }
        .status-warn .status-icon::before { content: '⚠'; }
        .status-skip .status-icon::before { content: '○'; }

        .badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 500;
        }

        .badge-primary { background: var(--color-primary); color: white; }
        .badge-secondary { background: var(--color-text-secondary); color: white; }

        footer {
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid var(--color-border);
            color: var(--color-text-secondary);
            font-size: 12px;
            text-align: center;
        }

        .empty-state {
            text-align: center;
            padding: 40px;
            color: var(--color-text-secondary);
        }

        .category-header {
            background: var(--color-bg-secondary);
            font-weight: 600;
        }

        .category-header td {
            padding-top: 15px;
            padding-bottom: 10px;
            border-bottom: 2px solid var(--color-border);
        }

        code {
            font-family: 'SF Mono', Monaco, 'Cascadia Code', Consolas, monospace;
            background: var(--color-bg-secondary);
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 13px;
        }

        .alert {
            padding: 15px;
            border-radius: 6px;
            margin: 15px 0;
        }

        .alert-success {
            background: rgba(40, 167, 69, 0.1);
            border: 1px solid var(--color-success);
            color: var(--color-success);
        }

        .alert-danger {
            background: rgba(220, 53, 69, 0.1);
            border: 1px solid var(--color-danger);
            color: var(--color-danger);
        }

        .alert-warning {
            background: rgba(255, 193, 7, 0.1);
            border: 1px solid var(--color-warning);
            color: #856404;
        }
        "#
    }

    /// Write HTML header
    fn write_header(&self, writer: &mut dyn Write, title: &str, subtitle: &str) -> Result<()> {
        writeln!(writer, "<!DOCTYPE html>")?;
        writeln!(writer, "<html lang=\"en\">")?;
        writeln!(writer, "<head>")?;
        writeln!(writer, "    <meta charset=\"UTF-8\">")?;
        writeln!(
            writer,
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )?;
        writeln!(writer, "    <meta name=\"generator\" content=\"Anvil {}\">" , env!("CARGO_PKG_VERSION"))?;
        writeln!(writer, "    <title>{}</title>", html_escape(title))?;
        writeln!(writer, "    <style>{}</style>", Self::css())?;
        writeln!(writer, "</head>")?;
        writeln!(writer, "<body>")?;
        writeln!(writer, "<header>")?;
        writeln!(writer, "    <h1>{}</h1>", html_escape(title))?;
        writeln!(
            writer,
            "    <p class=\"meta\">Generated by Anvil {} on {}</p>",
            env!("CARGO_PKG_VERSION"),
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        if !subtitle.is_empty() {
            writeln!(writer, "    <p class=\"meta\">{}</p>", html_escape(subtitle))?;
        }
        writeln!(writer, "</header>")?;
        Ok(())
    }

    /// Write HTML footer
    fn write_footer(&self, writer: &mut dyn Write) -> Result<()> {
        writeln!(writer, "<footer>")?;
        writeln!(
            writer,
            "    <p>Generated by <strong>Anvil</strong> v{} &middot; {}</p>",
            env!("CARGO_PKG_VERSION"),
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(writer, "</footer>")?;
        writeln!(writer, "</body>")?;
        writeln!(writer, "</html>")?;
        Ok(())
    }

    /// Format status as HTML
    fn format_status(status: &CheckStatus) -> String {
        let (class, label) = match status {
            CheckStatus::Ok => ("status-ok", "OK"),
            CheckStatus::Fail => ("status-fail", "FAIL"),
            CheckStatus::Warn => ("status-warn", "WARN"),
            CheckStatus::Skip => ("status-skip", "SKIP"),
        };
        format!(
            "<span class=\"status {}\"><span class=\"status-icon\"></span>{}</span>",
            class, label
        )
    }
}

impl Default for HtmlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for HtmlFormatter {
    fn format_list(&self, workloads: &[WorkloadInfo], writer: &mut dyn Write) -> Result<()> {
        let title = self.title.as_deref().unwrap_or("Anvil Workloads");
        self.write_header(writer, title, "")?;

        writeln!(writer, "<main>")?;

        // Summary cards
        writeln!(writer, "<div class=\"summary-cards\">")?;
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Total Workloads</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value info\">{}</div>",
            workloads.len()
        )?;
        writeln!(writer, "    </div>")?;

        let total_packages: usize = workloads.iter().map(|w| w.package_count).sum();
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Total Packages</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value\">{}</div>",
            total_packages
        )?;
        writeln!(writer, "    </div>")?;

        let total_files: usize = workloads.iter().map(|w| w.file_count).sum();
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Total Files</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value\">{}</div>",
            total_files
        )?;
        writeln!(writer, "    </div>")?;
        writeln!(writer, "</div>")?;

        // Workloads table
        writeln!(writer, "<h2>Available Workloads</h2>")?;

        if workloads.is_empty() {
            writeln!(writer, "<div class=\"empty-state\">")?;
            writeln!(writer, "    <p>No workloads found.</p>")?;
            writeln!(writer, "</div>")?;
        } else {
            writeln!(writer, "<table>")?;
            writeln!(writer, "<thead>")?;
            writeln!(writer, "    <tr>")?;
            writeln!(writer, "        <th>Name</th>")?;
            writeln!(writer, "        <th>Version</th>")?;
            writeln!(writer, "        <th>Description</th>")?;
            writeln!(writer, "        <th>Extends</th>")?;
            writeln!(writer, "        <th>Packages</th>")?;
            writeln!(writer, "        <th>Files</th>")?;
            writeln!(writer, "    </tr>")?;
            writeln!(writer, "</thead>")?;
            writeln!(writer, "<tbody>")?;

            for workload in workloads {
                let extends_str = if workload.extends.is_empty() {
                    "—".to_string()
                } else {
                    workload
                        .extends
                        .iter()
                        .map(|e| format!("<span class=\"badge badge-secondary\">{}</span>", html_escape(e)))
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                writeln!(writer, "    <tr>")?;
                writeln!(
                    writer,
                    "        <td><strong>{}</strong></td>",
                    html_escape(&workload.name)
                )?;
                writeln!(
                    writer,
                    "        <td><code>{}</code></td>",
                    html_escape(&workload.version)
                )?;
                writeln!(
                    writer,
                    "        <td>{}</td>",
                    html_escape(&workload.description)
                )?;
                writeln!(writer, "        <td>{}</td>", extends_str)?;
                writeln!(writer, "        <td>{}</td>", workload.package_count)?;
                writeln!(writer, "        <td>{}</td>", workload.file_count)?;
                writeln!(writer, "    </tr>")?;
            }

            writeln!(writer, "</tbody>")?;
            writeln!(writer, "</table>")?;
        }

        writeln!(writer, "</main>")?;

        self.write_footer(writer)
    }

    fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()> {
        let title = self
            .title
            .as_deref()
            .unwrap_or("Anvil Health Check Report");
        let subtitle = format!("Workload: {}", report.workload);
        self.write_header(writer, title, &subtitle)?;

        writeln!(writer, "<main>")?;

        // Overall status alert
        let (alert_class, alert_msg) = match report.overall_status {
            CheckStatus::Ok => ("alert-success", "✓ All health checks passed!"),
            CheckStatus::Warn => ("alert-warning", "⚠ Some checks have warnings"),
            CheckStatus::Fail => ("alert-danger", "✗ Health check failed"),
            CheckStatus::Skip => ("alert-warning", "○ Checks were skipped"),
        };
        writeln!(
            writer,
            "<div class=\"alert {}\">{}</div>",
            alert_class, alert_msg
        )?;

        // Summary cards
        writeln!(writer, "<div class=\"summary-cards\">")?;
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Total Checks</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value\">{}</div>",
            report.summary.total
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Passed</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value success\">{}</div>",
            report.summary.passed
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Failed</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value danger\">{}</div>",
            report.summary.failed
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Warnings</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value warning\">{}</div>",
            report.summary.warnings
        )?;
        writeln!(writer, "    </div>")?;
        writeln!(writer, "</div>")?;

        // Checks table
        writeln!(writer, "<h2>Health Check Results</h2>")?;
        writeln!(writer, "<table>")?;
        writeln!(writer, "<thead>")?;
        writeln!(writer, "    <tr>")?;
        writeln!(writer, "        <th>Component</th>")?;
        writeln!(writer, "        <th>Status</th>")?;
        writeln!(writer, "        <th>Details</th>")?;
        writeln!(writer, "    </tr>")?;
        writeln!(writer, "</thead>")?;
        writeln!(writer, "<tbody>")?;

        let mut current_category = String::new();
        for check in &report.checks {
            // Category header
            if check.category != current_category {
                current_category = check.category.clone();
                writeln!(writer, "    <tr class=\"category-header\">")?;
                writeln!(
                    writer,
                    "        <td colspan=\"3\">{}</td>",
                    html_escape(&current_category)
                )?;
                writeln!(writer, "    </tr>")?;
            }

            writeln!(writer, "    <tr>")?;
            writeln!(
                writer,
                "        <td>{}</td>",
                html_escape(&check.name)
            )?;
            writeln!(
                writer,
                "        <td>{}</td>",
                Self::format_status(&check.status)
            )?;
            writeln!(
                writer,
                "        <td>{}</td>",
                html_escape(check.message.as_deref().unwrap_or(""))
            )?;
            writeln!(writer, "    </tr>")?;
        }

        writeln!(writer, "</tbody>")?;
        writeln!(writer, "</table>")?;

        writeln!(writer, "</main>")?;

        self.write_footer(writer)
    }

    fn format_install(&self, summary: &InstallSummary, writer: &mut dyn Write) -> Result<()> {
        let title = self.title.as_deref().unwrap_or("Anvil Installation Report");
        let subtitle = format!("Workload: {}", summary.workload);
        self.write_header(writer, title, &subtitle)?;

        writeln!(writer, "<main>")?;

        // Overall status
        let (alert_class, alert_msg) = if summary.is_successful() {
            ("alert-success", "✓ Installation completed successfully!")
        } else {
            ("alert-danger", "✗ Installation completed with errors")
        };
        writeln!(
            writer,
            "<div class=\"alert {}\">{}</div>",
            alert_class, alert_msg
        )?;

        // Summary cards
        writeln!(writer, "<div class=\"summary-cards\">")?;

        // Packages
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Packages Installed</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value success\">{}</div>",
            summary.packages_installed
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Files Copied</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value success\">{}</div>",
            summary.files_copied
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Scripts Executed</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value success\">{}</div>",
            summary.scripts_executed
        )?;
        writeln!(writer, "    </div>")?;

        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Duration</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value\">{:.1}s</div>",
            summary.duration_secs
        )?;
        writeln!(writer, "    </div>")?;
        writeln!(writer, "</div>")?;

        // Detailed stats
        writeln!(writer, "<h2>Installation Details</h2>")?;
        writeln!(writer, "<table>")?;
        writeln!(writer, "<thead>")?;
        writeln!(writer, "    <tr>")?;
        writeln!(writer, "        <th>Category</th>")?;
        writeln!(writer, "        <th>Successful</th>")?;
        writeln!(writer, "        <th>Skipped</th>")?;
        writeln!(writer, "        <th>Failed</th>")?;
        writeln!(writer, "    </tr>")?;
        writeln!(writer, "</thead>")?;
        writeln!(writer, "<tbody>")?;
        writeln!(writer, "    <tr>")?;
        writeln!(writer, "        <td><strong>Packages</strong></td>")?;
        writeln!(writer, "        <td>{}</td>", summary.packages_installed)?;
        writeln!(writer, "        <td>{}</td>", summary.packages_skipped)?;
        writeln!(writer, "        <td>{}</td>", summary.packages_failed)?;
        writeln!(writer, "    </tr>")?;
        writeln!(writer, "    <tr>")?;
        writeln!(writer, "        <td><strong>Files</strong></td>")?;
        writeln!(writer, "        <td>{}</td>", summary.files_copied)?;
        writeln!(writer, "        <td>{}</td>", summary.files_skipped)?;
        writeln!(writer, "        <td>{}</td>", summary.files_failed)?;
        writeln!(writer, "    </tr>")?;
        writeln!(writer, "    <tr>")?;
        writeln!(writer, "        <td><strong>Scripts</strong></td>")?;
        writeln!(writer, "        <td>{}</td>", summary.scripts_executed)?;
        writeln!(writer, "        <td>—</td>")?;
        writeln!(writer, "        <td>{}</td>", summary.scripts_failed)?;
        writeln!(writer, "    </tr>")?;
        writeln!(writer, "</tbody>")?;
        writeln!(writer, "</table>")?;

        // Reboot warning
        if summary.reboot_required {
            writeln!(
                writer,
                "<div class=\"alert alert-warning\">⚠ A system reboot is required to complete the installation.</div>"
            )?;
        }

        // Failed items
        if !summary.failed_items.is_empty() {
            writeln!(writer, "<h2>Failed Items</h2>")?;
            writeln!(writer, "<table>")?;
            writeln!(writer, "<thead>")?;
            writeln!(writer, "    <tr><th>Item</th></tr>")?;
            writeln!(writer, "</thead>")?;
            writeln!(writer, "<tbody>")?;
            for item in &summary.failed_items {
                writeln!(
                    writer,
                    "    <tr><td><code>{}</code></td></tr>",
                    html_escape(item)
                )?;
            }
            writeln!(writer, "</tbody>")?;
            writeln!(writer, "</table>")?;
        }

        writeln!(writer, "</main>")?;

        self.write_footer(writer)
    }

    fn format_validate(&self, results: &ValidationResults, writer: &mut dyn Write) -> Result<()> {
        let title = self.title.as_deref().unwrap_or("Anvil Validation Report");
        let subtitle = format!("Workload: {}", results.workload);
        self.write_header(writer, title, &subtitle)?;

        writeln!(writer, "<main>")?;

        // Overall status
        let (alert_class, alert_msg) = if results.valid {
            ("alert-success", "✓ Validation passed!")
        } else {
            ("alert-danger", "✗ Validation failed")
        };
        writeln!(
            writer,
            "<div class=\"alert {}\">{}</div>",
            alert_class, alert_msg
        )?;

        // Summary cards
        writeln!(writer, "<div class=\"summary-cards\">")?;
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Errors</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value danger\">{}</div>",
            results.errors.len()
        )?;
        writeln!(writer, "    </div>")?;
        writeln!(writer, "    <div class=\"card\">")?;
        writeln!(writer, "        <div class=\"card-title\">Warnings</div>")?;
        writeln!(
            writer,
            "        <div class=\"card-value warning\">{}</div>",
            results.warnings.len()
        )?;
        writeln!(writer, "    </div>")?;
        writeln!(writer, "</div>")?;

        // Errors
        if !results.errors.is_empty() {
            writeln!(writer, "<h2>Errors</h2>")?;
            writeln!(writer, "<table>")?;
            writeln!(writer, "<thead>")?;
            writeln!(writer, "    <tr><th>Location</th><th>Message</th></tr>")?;
            writeln!(writer, "</thead>")?;
            writeln!(writer, "<tbody>")?;
            for error in &results.errors {
                writeln!(writer, "    <tr>")?;
                writeln!(
                    writer,
                    "        <td><code>{}</code></td>",
                    html_escape(&error.location)
                )?;
                writeln!(
                    writer,
                    "        <td>{}</td>",
                    html_escape(&error.message)
                )?;
                writeln!(writer, "    </tr>")?;
            }
            writeln!(writer, "</tbody>")?;
            writeln!(writer, "</table>")?;
        }

        // Warnings
        if !results.warnings.is_empty() {
            writeln!(writer, "<h2>Warnings</h2>")?;
            writeln!(writer, "<table>")?;
            writeln!(writer, "<thead>")?;
            writeln!(writer, "    <tr><th>Location</th><th>Message</th></tr>")?;
            writeln!(writer, "</thead>")?;
            writeln!(writer, "<tbody>")?;
            for warning in &results.warnings {
                writeln!(writer, "    <tr>")?;
                writeln!(
                    writer,
                    "        <td><code>{}</code></td>",
                    html_escape(&warning.location)
                )?;
                writeln!(
                    writer,
                    "        <td>{}</td>",
                    html_escape(&warning.message)
                )?;
                writeln!(writer, "    </tr>")?;
            }
            writeln!(writer, "</tbody>")?;
            writeln!(writer, "</table>")?;
        }

        writeln!(writer, "</main>")?;

        self.write_footer(writer)
    }

    fn extension(&self) -> &'static str {
        "html"
    }

    fn mime_type(&self) -> &'static str {
        "text/html"
    }
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_formatter_creation() {
        let formatter = HtmlFormatter::new();
        assert_eq!(formatter.extension(), "html");
        assert_eq!(formatter.mime_type(), "text/html");
    }

    #[test]
    fn test_html_formatter_with_title() {
        let formatter = HtmlFormatter::with_title("Custom Report");
        assert!(formatter.title.is_some());
        assert_eq!(formatter.title.unwrap(), "Custom Report");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_format_status() {
        let ok = HtmlFormatter::format_status(&CheckStatus::Ok);
        assert!(ok.contains("status-ok"));
        assert!(ok.contains("OK"));

        let fail = HtmlFormatter::format_status(&CheckStatus::Fail);
        assert!(fail.contains("status-fail"));
        assert!(fail.contains("FAIL"));
    }

    #[test]
    fn test_format_empty_list() {
        let formatter = HtmlFormatter::new();
        let mut output = Vec::new();

        formatter.format_list(&[], &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("No workloads found"));
    }

    #[test]
    fn test_format_list_with_workloads() {
        let formatter = HtmlFormatter::new();
        let mut output = Vec::new();

        let workloads = vec![WorkloadInfo {
            name: "test-workload".to_string(),
            version: "1.0.0".to_string(),
            description: "A <test> workload".to_string(),
            extends: vec!["base".to_string()],
            package_count: 5,
            file_count: 3,
            path: std::path::PathBuf::from("workloads/test"),
        }];

        formatter.format_list(&workloads, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();

        assert!(content.contains("test-workload"));
        assert!(content.contains("1.0.0"));
        // Should be escaped
        assert!(content.contains("A &lt;test&gt; workload"));
        assert!(content.contains("badge-secondary"));
    }
}
