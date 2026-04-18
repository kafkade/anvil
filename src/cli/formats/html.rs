//! HTML output formatter for standalone report generation
//!
//! This module provides HTML output with embedded CSS for standalone
//! reports that can be opened in any web browser.
use std::io::Write;

use anyhow::Result;
use chrono::Utc;

use crate::cli::output::{CheckStatus, HealthReport};

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
        writeln!(
            writer,
            "    <meta name=\"generator\" content=\"Anvil {}\">",
            env!("CARGO_PKG_VERSION")
        )?;
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
            writeln!(
                writer,
                "    <p class=\"meta\">{}</p>",
                html_escape(subtitle)
            )?;
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

    /// Format a health report as HTML
    pub fn format_health(&self, report: &HealthReport, writer: &mut dyn Write) -> Result<()> {
        let title = self.title.as_deref().unwrap_or("Anvil Health Check Report");
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
        writeln!(
            writer,
            "        <div class=\"card-title\">Total Checks</div>"
        )?;
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
            writeln!(writer, "        <td>{}</td>", html_escape(&check.name))?;
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
}

impl Default for HtmlFormatter {
    fn default() -> Self {
        Self::new()
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
}
