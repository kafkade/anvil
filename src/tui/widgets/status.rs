//! Status badge widget
//!
//! Renders colored status indicators like `✓ installed`, `⠋ installing`, `✗ failed`.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::tui::theme::Theme;

/// Item status for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ItemStatus {
    Pending,
    Running,
    Success,
    Skipped,
    Failed,
}

/// Spinner frames for the running animation
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

impl ItemStatus {
    /// Get the status symbol
    pub fn symbol(&self, tick: usize) -> &str {
        match self {
            ItemStatus::Pending => "○",
            ItemStatus::Running => {
                let idx = tick % SPINNER_FRAMES.len();
                // Return the spinner char as a str — we use a static lookup
                match idx {
                    0 => "⠋",
                    1 => "⠙",
                    2 => "⠹",
                    3 => "⠸",
                    4 => "⠼",
                    5 => "⠴",
                    6 => "⠦",
                    7 => "⠧",
                    8 => "⠇",
                    _ => "⠏",
                }
            }
            ItemStatus::Success => "✓",
            ItemStatus::Skipped => "○",
            ItemStatus::Failed => "✗",
        }
    }

    /// Get the label text
    pub fn label(&self) -> &str {
        match self {
            ItemStatus::Pending => "pending",
            ItemStatus::Running => "installing",
            ItemStatus::Success => "installed",
            ItemStatus::Skipped => "skipped",
            ItemStatus::Failed => "failed",
        }
    }

    /// Get the style for this status
    pub fn style(&self, theme: &Theme) -> Style {
        match self {
            ItemStatus::Pending => theme.dimmed(),
            ItemStatus::Running => theme.running_style(),
            ItemStatus::Success => theme.success_style(),
            ItemStatus::Skipped => theme.dimmed(),
            ItemStatus::Failed => theme.error_style(),
        }
    }
}

/// Render a status badge as a line of spans
pub fn status_line<'a>(
    name: &'a str,
    status: ItemStatus,
    message: Option<&'a str>,
    tick: usize,
    theme: &Theme,
    width: u16,
) -> Line<'a> {
    let symbol = status.symbol(tick);
    let label = message.unwrap_or(status.label());
    let style = status.style(theme);

    let name_width = width.saturating_sub(4 + label.len() as u16) as usize;
    let padded_name = if name.len() > name_width {
        format!("{}...", &name[..name_width.saturating_sub(3)])
    } else {
        format!("{:<width$}", name, width = name_width)
    };

    Line::from(vec![
        Span::styled(format!("  {} ", symbol), style),
        Span::raw(padded_name),
        Span::styled(label.to_string(), style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_symbols() {
        assert_eq!(ItemStatus::Pending.symbol(0), "○");
        assert_eq!(ItemStatus::Success.symbol(0), "✓");
        assert_eq!(ItemStatus::Failed.symbol(0), "✗");
        // Running should show spinner
        assert_eq!(ItemStatus::Running.symbol(0), "⠋");
        assert_eq!(ItemStatus::Running.symbol(1), "⠙");
    }

    #[test]
    fn test_status_labels() {
        assert_eq!(ItemStatus::Pending.label(), "pending");
        assert_eq!(ItemStatus::Running.label(), "installing");
        assert_eq!(ItemStatus::Success.label(), "installed");
    }

    #[test]
    fn test_status_line_renders() {
        let theme = Theme::dark();
        let line = status_line("Test.Package", ItemStatus::Success, None, 0, &theme, 60);
        assert!(!line.spans.is_empty());
    }
}
