//! Interactive status dashboard view
//!
//! This view renders a full-screen TUI showing an overview of
//! Anvil's current state: system info, source summary, and
//! installed workloads.

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::theme::Theme;
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::Tui;

/// Information about a single installed workload
#[allow(dead_code)]
pub struct InstalledWorkload {
    pub name: String,
    pub version: String,
    pub installed_at: String,
}

/// Summary of configured workload sources
#[allow(dead_code)]
pub struct SourceSummary {
    pub local_count: usize,
    pub remote_count: usize,
    pub total_workloads: usize,
}

/// System-level information
#[allow(dead_code)]
pub struct SystemInfo {
    pub anvil_version: String,
    pub os: String,
    pub winget_available: bool,
}

/// All data needed for the status dashboard
#[allow(dead_code)]
pub struct StatusInfo {
    pub installed_workloads: Vec<InstalledWorkload>,
    pub source_summary: SourceSummary,
    pub system_info: SystemInfo,
}

/// The status dashboard state
#[allow(dead_code)]
pub struct StatusDashboard {
    info: StatusInfo,
    selected: usize,
    scroll_offset: usize,
    quit: bool,
    theme: Theme,
}

#[allow(dead_code)]
impl StatusDashboard {
    /// Create a new status dashboard from the given info
    pub fn new(info: StatusInfo) -> Self {
        Self {
            info,
            selected: 0,
            scroll_offset: 0,
            quit: false,
            theme: Theme::dark(),
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                self.quit = true;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.info.installed_workloads.len().saturating_sub(1);
                if self.selected < max {
                    self.selected += 1;
                }
            }
            _ => {}
        }
    }

    /// Should the TUI exit?
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Render the dashboard
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(5), // system info
            Constraint::Length(3), // source summary
            Constraint::Min(3),    // workload list
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_system_info(frame, chunks[0]);
        self.render_source_summary(frame, chunks[1]);
        self.render_workload_list(frame, chunks[2]);
        self.render_keyhints(frame, chunks[3]);
    }

    /// Render the system info block
    fn render_system_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(
                " Anvil \u{2014} Status Dashboard ",
                self.theme.title_style(),
            ))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let winget_span = if self.info.system_info.winget_available {
            Span::styled("available", self.theme.success_style())
        } else {
            Span::styled("not available", self.theme.error_style())
        };

        let lines = vec![
            Line::from(vec![Span::raw(format!(
                "  Anvil Version: {}",
                self.info.system_info.anvil_version
            ))]),
            Line::from(vec![Span::raw(format!(
                "  OS: {}",
                self.info.system_info.os
            ))]),
            Line::from(vec![Span::raw("  Winget: "), winget_span]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render the source summary block
    fn render_source_summary(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(" Sources ", self.theme.title_style()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let summary = format!(
            "  Local: {}  Remote: {}  Total: {}",
            self.info.source_summary.local_count,
            self.info.source_summary.remote_count,
            self.info.source_summary.total_workloads,
        );

        let paragraph = Paragraph::new(Line::raw(summary)).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render the installed workloads list
    fn render_workload_list(&self, frame: &mut Frame, area: Rect) {
        let count = self.info.installed_workloads.len();
        let title = format!(" Installed Workloads ({}) ", count);
        let block = Block::default()
            .title(Span::styled(title, self.theme.title_style()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.info.installed_workloads.is_empty() {
            let paragraph = Paragraph::new(Line::styled(
                "  No workloads installed",
                self.theme.dimmed(),
            ));
            frame.render_widget(paragraph, inner);
            return;
        }

        let visible_height = inner.height as usize;
        let lines: Vec<Line> = self
            .info
            .installed_workloads
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|(i, w)| {
                let text = format!("  {}  v{}  installed {}", w.name, w.version, w.installed_at);
                if i == self.selected {
                    Line::styled(
                        text,
                        Style::default()
                            .bg(self.theme.accent)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::raw(text)
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    /// Render the key hints bar
    fn render_keyhints(&self, frame: &mut Frame, area: Rect) {
        let hints = vec![
            KeyHint {
                key: "↑/↓",
                desc: "navigate",
            },
            KeyHint {
                key: "q",
                desc: "quit",
            },
        ];

        render_keyhints(frame, area, &hints, &self.theme);
    }
}

/// Run the interactive status dashboard
#[allow(dead_code)]
pub fn run_status_dashboard(info: StatusInfo) -> anyhow::Result<()> {
    let mut tui = Tui::new()?;
    let mut dashboard = StatusDashboard::new(info);

    let tick_rate = Duration::from_millis(100);

    loop {
        tui.draw(|f| dashboard.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            dashboard.handle_key(key);

            if dashboard.should_quit() {
                break;
            }
        }
    }

    tui.restore()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info() -> StatusInfo {
        StatusInfo {
            installed_workloads: vec![
                InstalledWorkload {
                    name: "essentials".to_string(),
                    version: "1.0.0".to_string(),
                    installed_at: "2024-01-15 10:30:00".to_string(),
                },
                InstalledWorkload {
                    name: "rust-developer".to_string(),
                    version: "1.2.0".to_string(),
                    installed_at: "2024-01-16 14:00:00".to_string(),
                },
            ],
            source_summary: SourceSummary {
                local_count: 3,
                remote_count: 1,
                total_workloads: 4,
            },
            system_info: SystemInfo {
                anvil_version: "0.4.0".to_string(),
                os: "Windows 11".to_string(),
                winget_available: true,
            },
        }
    }

    #[test]
    fn test_status_initial_state() {
        let dashboard = StatusDashboard::new(sample_info());
        assert_eq!(dashboard.selected, 0);
        assert_eq!(dashboard.scroll_offset, 0);
        assert!(!dashboard.should_quit());
        assert_eq!(dashboard.info.installed_workloads.len(), 2);
    }

    #[test]
    fn test_status_navigation() {
        let mut dashboard = StatusDashboard::new(sample_info());

        // Move down
        dashboard.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 1);

        // Clamp at max
        dashboard.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 1);

        // Move up
        dashboard.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 0);

        // Clamp at 0
        dashboard.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 0);

        // j/k bindings
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 1);
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 0);
    }

    #[test]
    fn test_status_quit() {
        let mut dashboard = StatusDashboard::new(sample_info());

        // q quits
        assert!(!dashboard.should_quit());
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(dashboard.should_quit());

        // Ctrl+C also quits
        let mut dashboard2 = StatusDashboard::new(sample_info());
        dashboard2.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(dashboard2.should_quit());
    }

    #[test]
    fn test_status_empty_workloads() {
        let info = StatusInfo {
            installed_workloads: vec![],
            source_summary: SourceSummary {
                local_count: 0,
                remote_count: 0,
                total_workloads: 0,
            },
            system_info: SystemInfo {
                anvil_version: "0.4.0".to_string(),
                os: "Windows 11".to_string(),
                winget_available: false,
            },
        };

        let mut dashboard = StatusDashboard::new(info);
        assert_eq!(dashboard.selected, 0);

        // Navigation should not panic with empty list
        dashboard.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 0);
        dashboard.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(dashboard.selected, 0);
    }

    #[test]
    fn test_status_render_in_test_backend() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let dashboard = StatusDashboard::new(sample_info());

        // Should render without panicking
        terminal.draw(|f| dashboard.render(f)).unwrap();
    }

    #[test]
    fn test_status_render_empty_workloads() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let info = StatusInfo {
            installed_workloads: vec![],
            source_summary: SourceSummary {
                local_count: 0,
                remote_count: 0,
                total_workloads: 0,
            },
            system_info: SystemInfo {
                anvil_version: "0.4.0".to_string(),
                os: "Linux".to_string(),
                winget_available: false,
            },
        };

        let dashboard = StatusDashboard::new(info);
        terminal.draw(|f| dashboard.render(f)).unwrap();
    }
}
