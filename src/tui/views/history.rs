//! Install history view
//!
//! This view renders a full-screen TUI showing past installation
//! runs with a detail preview pane for the selected entry.

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::theme::Theme;
use crate::tui::widgets::chrome::{badge, render_header};
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::Tui;

/// Result status of an install run
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum InstallResult {
    Ok,
    Partial,
    Fail,
}

/// A single install history entry
#[allow(dead_code)]
pub struct HistoryEntry {
    pub workload: String,
    pub version: String,
    pub date: String,
    pub duration: Duration,
    pub result: InstallResult,
    pub summary: String,
    pub path: String,
    pub installed: usize,
    pub skipped: usize,
    pub failed: usize,
    /// Previous version if this was an upgrade
    pub previous_version: Option<String>,
    /// How many times this workload has been installed total
    pub install_count: usize,
}

/// The result of running the history view
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HistoryOutcome {
    /// User quit (q / Ctrl+C)
    Quit,
    /// User went back (Esc)
    Back,
    /// User wants to re-install a workload (name, path)
    Reinstall(String, String),
    /// User wants to view the log for this entry
    ViewLog(String),
}

/// Interactive install history viewer state
#[allow(dead_code)]
pub struct HistoryViewer {
    entries: Vec<HistoryEntry>,
    selected: usize,
    scroll_offset: usize,
    quit: bool,
    outcome: Option<HistoryOutcome>,
    theme: Theme,
}

#[allow(dead_code)]
impl HistoryViewer {
    /// Create a new history viewer
    pub fn new(entries: Vec<HistoryEntry>) -> Self {
        Self {
            entries,
            selected: 0,
            scroll_offset: 0,
            quit: false,
            outcome: None,
            theme: Theme::dark(),
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                self.outcome = Some(HistoryOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.outcome = Some(HistoryOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Esc | KeyCode::Backspace => {
                self.outcome = Some(HistoryOutcome::Back);
                self.quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + 1).min(self.entries.len() - 1);
                }
            }
            // Re-install the selected workload
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                if let Some(entry) = self.entries.get(self.selected) {
                    self.outcome = Some(HistoryOutcome::Reinstall(
                        entry.workload.clone(),
                        entry.path.clone(),
                    ));
                    self.quit = true;
                }
            }
            // View log for selected entry
            KeyCode::Enter => {
                if let Some(entry) = self.entries.get(self.selected) {
                    self.outcome = Some(HistoryOutcome::ViewLog(entry.workload.clone()));
                    self.quit = true;
                }
            }
            _ => {}
        }
    }

    /// Whether the view should exit
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Returns the outcome after the view exits
    pub fn outcome(&self) -> HistoryOutcome {
        self.outcome.clone().unwrap_or(HistoryOutcome::Quit)
    }

    /// Render the full view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // branded header
            Constraint::Length(1), // hairline divider
            Constraint::Length(1), // column headers
            Constraint::Length(1), // hairline divider
            Constraint::Min(6),    // split: table (top) + detail (bottom)
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_header(frame, chunks[0]);
        self.render_hairline(frame, chunks[1]);
        self.render_column_headers(frame, chunks[2]);
        self.render_hairline(frame, chunks[3]);
        self.render_body(frame, chunks[4]);
        self.render_keyhints(frame, chunks[5]);
    }

    /// Render the branded header
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let unique_workloads: std::collections::HashSet<&str> =
            self.entries.iter().map(|e| e.workload.as_str()).collect();
        let right_text = format!(
            "{} runs · {} workloads",
            self.entries.len(),
            unique_workloads.len()
        );

        render_header(
            frame,
            area,
            &self.theme,
            &["Install History"],
            Some(Line::from(vec![Span::styled(
                right_text,
                self.theme.dimmed(),
            )])),
        );
    }

    /// Render a hairline divider
    fn render_hairline(&self, frame: &mut Frame, area: Rect) {
        let line = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                line,
                Style::default().fg(self.theme.hairline),
            ))),
            area,
        );
    }

    /// Render the column header row
    fn render_column_headers(&self, frame: &mut Frame, area: Rect) {
        let style = self.theme.label_style();
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<18}", "WORKLOAD"), style),
            Span::styled(format!("{:<12}", "VERSION"), style),
            Span::styled(format!("{:<18}", "DATE"), style),
            Span::styled(format!("{:<12}", "DURATION"), style),
            Span::styled(format!("{:<10}", "RESULT"), style),
            Span::styled("SUMMARY", style),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    /// Render the body: table rows (top) + detail pane (bottom)
    fn render_body(&self, frame: &mut Frame, area: Rect) {
        // Split: table rows take ~60%, detail pane takes ~40%
        let table_height = (area.height as usize * 55) / 100;
        let table_height = table_height.max(3);
        let parts = Layout::vertical([
            Constraint::Length(table_height as u16),
            Constraint::Length(1), // hairline
            Constraint::Min(3),    // detail
        ])
        .split(area);

        self.render_table(frame, parts[0]);
        self.render_hairline(frame, parts[1]);
        self.render_detail(frame, parts[2]);
    }

    /// Render the scrollable table rows
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            let empty = Paragraph::new(Line::styled(
                "  No install history found",
                self.theme.dimmed(),
            ));
            frame.render_widget(empty, area);
            return;
        }

        let visible_height = area.height as usize;

        // Auto-scroll to keep selected visible
        let scroll = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(i, entry)| {
                let is_selected = i == self.selected;

                let duration = format_duration_short(entry.duration);

                let (result_text, result_style) = match entry.result {
                    InstallResult::Ok => ("OK", self.theme.success_style()),
                    InstallResult::Partial => ("PARTIAL", self.theme.warning_style()),
                    InstallResult::Fail => ("FAIL", self.theme.error_style()),
                };

                let workload_style = if is_selected {
                    self.theme
                        .selection()
                        .add_modifier(Modifier::BOLD)
                        .bg(self.theme.bg_inset)
                } else {
                    self.theme.running_style()
                };

                let text_style = if is_selected {
                    self.theme.normal().bg(self.theme.bg_inset)
                } else {
                    self.theme.normal()
                };

                let faint_style = if is_selected {
                    self.theme.faint_style().bg(self.theme.bg_inset)
                } else {
                    self.theme.faint_style()
                };

                let prefix = if is_selected {
                    Span::styled("▌ ", Style::default().fg(self.theme.accent))
                } else {
                    Span::raw("  ")
                };

                Line::from(vec![
                    prefix,
                    Span::styled(format!("{:<18}", entry.workload), workload_style),
                    Span::styled(format!("{:<12}", entry.version), faint_style),
                    Span::styled(format!("{:<18}", entry.date), text_style),
                    Span::styled(format!("{:<12}", duration), faint_style),
                    Span::styled(format!("{:<10}", result_text), result_style),
                    Span::styled(&entry.summary, faint_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);

        // Scrollbar
        let total = self.entries.len();
        if total > visible_height {
            let max_scroll = total.saturating_sub(visible_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    /// Render the detail pane for the selected entry
    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            return;
        }

        let entry = &self.entries[self.selected];
        let mut lines: Vec<Line> = Vec::new();

        // Blank line for top padding
        lines.push(Line::raw(""));

        // Name + version badge + result badge + date + duration
        let version_text = format!("v{}", entry.version);
        let (result_text, result_color) = match entry.result {
            InstallResult::Ok => ("OK", self.theme.success),
            InstallResult::Partial => ("PARTIAL", self.theme.warning),
            InstallResult::Fail => ("FAIL", self.theme.error),
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &entry.workload,
                self.theme.normal().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(&version_text, Style::default().fg(self.theme.faint)),
            Span::raw("  "),
            badge(result_text, result_color),
            Span::styled(
                format!(
                    "  · {}  · {}",
                    entry.date,
                    format_duration_short(entry.duration)
                ),
                self.theme.dimmed(),
            ),
        ]));

        // Blank line
        lines.push(Line::raw(""));

        // Path
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Path: ", self.theme.faint_style()),
            Span::styled(&entry.path, self.theme.running_style()),
        ]));

        // Blank line
        lines.push(Line::raw(""));

        // Stats: installed / skipped / failed
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("✓ ", self.theme.success_style()),
            Span::styled(
                format!("{} installed", entry.installed),
                self.theme.normal(),
            ),
            Span::raw("   "),
            Span::styled("○ ", self.theme.dimmed()),
            Span::styled(format!("{} skipped", entry.skipped), self.theme.normal()),
            Span::raw("   "),
            Span::styled("✗ ", self.theme.error_style()),
            Span::styled(format!("{} failed", entry.failed), self.theme.normal()),
        ]));

        // Upgrade info
        if let Some(prev) = &entry.previous_version {
            lines.push(Line::raw(""));
            let ordinal = match entry.install_count {
                1 => "1st".to_string(),
                2 => "2nd".to_string(),
                3 => "3rd".to_string(),
                n => format!("{}th", n),
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "↑ upgraded from v{} · {} install of this workload",
                        prev, ordinal
                    ),
                    self.theme.dimmed(),
                ),
            ]));
        } else if entry.install_count > 1 {
            lines.push(Line::raw(""));
            let ordinal = match entry.install_count {
                2 => "2nd".to_string(),
                3 => "3rd".to_string(),
                n => format!("{}th", n),
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{} install of this workload", ordinal),
                    self.theme.dimmed(),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }

    /// Render the key hints bar
    fn render_keyhints(&self, frame: &mut Frame, area: Rect) {
        let hints = vec![
            KeyHint {
                key: "↑↓",
                desc: "navigate",
            },
            KeyHint {
                key: "↵",
                desc: "view log",
            },
            KeyHint {
                key: "r",
                desc: "re-install",
            },
            KeyHint {
                key: "Esc",
                desc: "back",
            },
            KeyHint {
                key: "q",
                desc: "quit",
            },
        ];

        render_keyhints(frame, area, &hints, &self.theme);
    }

    /// Run the history viewer event loop
    pub fn run(&mut self, tui: &mut Tui) -> anyhow::Result<HistoryOutcome> {
        let tick_rate = Duration::from_millis(100);

        loop {
            tui.draw(|f| self.render(f))?;

            if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
                self.handle_key(key);

                if self.should_quit() {
                    break;
                }
            }
        }

        Ok(self.outcome())
    }
}

/// Format a duration as "Xm Ys"
fn format_duration_short(dur: Duration) -> String {
    let secs = dur.as_secs();
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{}m {:02}s", mins, rem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<HistoryEntry> {
        vec![
            HistoryEntry {
                workload: "essentials".to_string(),
                version: "2.0.0".to_string(),
                date: "2025-06-02 14:23".to_string(),
                duration: Duration::from_secs(312),
                result: InstallResult::Ok,
                summary: "26 installed".to_string(),
                path: "C:\\src\\winforge\\workloads\\essentials\\workload.yaml".to_string(),
                installed: 26,
                skipped: 0,
                failed: 0,
                previous_version: Some("1.9.0".to_string()),
                install_count: 3,
            },
            HistoryEntry {
                workload: "essentials".to_string(),
                version: "2.0.0".to_string(),
                date: "2025-05-28 09:15".to_string(),
                duration: Duration::from_secs(161),
                result: InstallResult::Partial,
                summary: "22 installed, 4 skipped".to_string(),
                path: "C:\\src\\winforge\\workloads\\essentials\\workload.yaml".to_string(),
                installed: 22,
                skipped: 4,
                failed: 0,
                previous_version: None,
                install_count: 2,
            },
            HistoryEntry {
                workload: "rust-developer".to_string(),
                version: "1.3.0".to_string(),
                date: "2025-05-28 09:20".to_string(),
                duration: Duration::from_secs(63),
                result: InstallResult::Ok,
                summary: "3 installed".to_string(),
                path: "C:\\workloads\\rust-developer\\workload.yaml".to_string(),
                installed: 3,
                skipped: 0,
                failed: 0,
                previous_version: None,
                install_count: 1,
            },
        ]
    }

    #[test]
    fn test_history_initial_state() {
        let viewer = HistoryViewer::new(sample_entries());
        assert!(!viewer.should_quit());
        assert_eq!(viewer.selected, 0);
        assert_eq!(viewer.entries.len(), 3);
    }

    #[test]
    fn test_history_navigation() {
        let mut viewer = HistoryViewer::new(sample_entries());
        assert_eq!(viewer.selected, 0);

        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);

        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 2);

        // Can't go past end
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 2);

        viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);

        // j/k navigation
        viewer.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(viewer.selected, 0);

        viewer.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);
    }

    #[test]
    fn test_history_quit() {
        let mut viewer = HistoryViewer::new(sample_entries());
        viewer.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(viewer.should_quit());
        assert_eq!(viewer.outcome(), HistoryOutcome::Quit);
    }

    #[test]
    fn test_history_back() {
        let mut viewer = HistoryViewer::new(sample_entries());
        viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(viewer.should_quit());
        assert_eq!(viewer.outcome(), HistoryOutcome::Back);
    }

    #[test]
    fn test_history_reinstall() {
        let mut viewer = HistoryViewer::new(sample_entries());
        viewer.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert!(viewer.should_quit());
        assert_eq!(
            viewer.outcome(),
            HistoryOutcome::Reinstall(
                "essentials".to_string(),
                "C:\\src\\winforge\\workloads\\essentials\\workload.yaml".to_string(),
            )
        );
    }

    #[test]
    fn test_history_view_log() {
        let mut viewer = HistoryViewer::new(sample_entries());
        viewer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(viewer.should_quit());
        assert_eq!(
            viewer.outcome(),
            HistoryOutcome::ViewLog("essentials".to_string())
        );
    }

    #[test]
    fn test_history_empty() {
        let viewer = HistoryViewer::new(vec![]);
        assert_eq!(viewer.entries.len(), 0);
    }

    #[test]
    fn test_history_render_in_test_backend() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewer = HistoryViewer::new(sample_entries());

        terminal.draw(|f| viewer.render(f)).unwrap();
    }

    #[test]
    fn test_format_duration_short() {
        assert_eq!(format_duration_short(Duration::from_secs(312)), "5m 12s");
        assert_eq!(format_duration_short(Duration::from_secs(63)), "1m 03s");
        assert_eq!(format_duration_short(Duration::from_secs(48)), "0m 48s");
    }
}
