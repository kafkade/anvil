//! Interactive health report viewer
//!
//! This view renders a full-screen TUI showing the results of
//! `anvil health` with collapsible sections and expandable item details.

use std::collections::HashSet;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::theme::Theme;
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::Tui;

/// Status of an individual health check item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HealthStatus {
    Pass,
    Fail,
    Skip,
    Warn,
}

/// A single health check item
#[allow(dead_code)]
pub struct HealthItem {
    pub name: String,
    pub status: HealthStatus,
    pub detail: Option<String>,
}

/// A section grouping related health check items
#[allow(dead_code)]
pub struct HealthSection {
    pub name: String,
    pub items: Vec<HealthItem>,
}

/// Complete health report for a workload
#[allow(dead_code)]
pub struct HealthReport {
    pub workload_name: String,
    pub sections: Vec<HealthSection>,
    pub duration: Duration,
}

/// Interactive health report viewer state
#[allow(dead_code)]
pub struct HealthViewer {
    report: HealthReport,
    selected: usize,
    expanded: HashSet<usize>,
    section_collapsed: HashSet<usize>,
    filter_failures: bool,
    scroll_offset: usize,
    quit: bool,
    theme: Theme,
}

impl HealthViewer {
    /// Create a new health viewer for the given report
    pub fn new(report: HealthReport) -> Self {
        Self {
            report,
            selected: 0,
            expanded: HashSet::new(),
            section_collapsed: HashSet::new(),
            filter_failures: false,
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
                let max = self.total_items().saturating_sub(1);
                if self.selected < max {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.expanded.contains(&self.selected) {
                    self.expanded.remove(&self.selected);
                } else {
                    self.expanded.insert(self.selected);
                }
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::NONE => {
                self.filter_failures = !self.filter_failures;
                self.selected = 0;
            }
            _ => {}
        }
    }

    /// Should the TUI exit?
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Count of visible items for navigation
    pub fn total_items(&self) -> usize {
        let mut count = 0;
        for (si, section) in self.report.sections.iter().enumerate() {
            count += 1; // section header
            if !self.section_collapsed.contains(&si) {
                for item in &section.items {
                    if self.filter_failures && item.status != HealthStatus::Fail {
                        continue;
                    }
                    count += 1;
                }
            }
        }
        count
    }

    /// Render the health report view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(3), // summary bar
            Constraint::Min(3),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_summary(frame, chunks[0]);
        self.render_main(frame, chunks[1]);
        self.render_keyhints(frame, chunks[2]);
    }

    /// Render the summary bar
    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let (total, pass, fail, skip, warn) = self.count_statuses();
        let pass_rate = if total > 0 {
            (pass as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let title = format!(" Anvil — Health Report: {} ", self.report.workload_name);

        let block = Block::default()
            .title(Span::styled(title, self.theme.title_style()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let summary_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{total} checks"), self.theme.normal()),
            Span::raw("  "),
            Span::styled(format!("{pass} pass"), self.theme.success_style()),
            Span::raw("  "),
            Span::styled(format!("{fail} fail"), self.theme.error_style()),
            Span::raw("  "),
            Span::styled(format!("{skip} skip"), self.theme.dimmed()),
            Span::raw("  "),
            Span::styled(format!("{warn} warn"), self.theme.warning_style()),
            Span::raw("  "),
            Span::styled(
                format!("{pass_rate:.0}% pass rate"),
                if pass_rate >= 100.0 {
                    self.theme.success_style()
                } else if pass_rate >= 50.0 {
                    self.theme.warning_style()
                } else {
                    self.theme.error_style()
                },
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.1}s", self.report.duration.as_secs_f64()),
                self.theme.dimmed(),
            ),
        ]);

        let paragraph = Paragraph::new(summary_line);
        frame.render_widget(paragraph, inner);
    }

    /// Render the main content area with sections and items
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        let mut flat_index: usize = 0;
        let highlight_style = Style::default().bg(self.theme.accent).fg(Color::Black);

        for (si, section) in self.report.sections.iter().enumerate() {
            let is_collapsed = self.section_collapsed.contains(&si);
            let (section_pass, section_total) = self.section_counts(section);
            let is_selected = flat_index == self.selected;

            // Section header
            let arrow = if is_collapsed { "▸" } else { "▾" };
            let header_text = format!(
                "  {} {} ({}/{})",
                arrow, section.name, section_pass, section_total
            );
            let header_style = if is_selected {
                highlight_style.add_modifier(Modifier::BOLD)
            } else {
                self.theme.normal().add_modifier(Modifier::BOLD)
            };
            lines.push(Line::styled(header_text, header_style));
            flat_index += 1;

            // Items (if section not collapsed)
            if !is_collapsed {
                for item in &section.items {
                    if self.filter_failures && item.status != HealthStatus::Fail {
                        continue;
                    }

                    let is_item_selected = flat_index == self.selected;
                    let is_expanded = self.expanded.contains(&flat_index);

                    let (icon, icon_style) = match item.status {
                        HealthStatus::Pass => ("✓", self.theme.success_style()),
                        HealthStatus::Fail => ("✗", self.theme.error_style()),
                        HealthStatus::Skip => ("○", self.theme.dimmed()),
                        HealthStatus::Warn => ("⚠", self.theme.warning_style()),
                    };

                    let item_style = if is_item_selected {
                        highlight_style
                    } else {
                        Style::default()
                    };

                    let item_icon_style = if is_item_selected {
                        highlight_style
                    } else {
                        icon_style
                    };

                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(icon, item_icon_style),
                        Span::styled(format!(" {}", item.name), item_style),
                    ]));

                    // Show detail if expanded
                    if is_expanded {
                        if let Some(detail) = &item.detail {
                            lines.push(Line::from(vec![
                                Span::raw("      "),
                                Span::styled(detail, self.theme.dimmed()),
                            ]));
                        }
                    }

                    flat_index += 1;
                }
            }
        }

        // Apply scroll offset
        let visible_height = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible_height);
        let scroll = self.scroll_offset.min(max_scroll);
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(scroll)
            .take(visible_height)
            .collect();

        let paragraph = Paragraph::new(visible_lines);
        frame.render_widget(paragraph, inner);

        // Scrollbar
        if max_scroll > 0 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    /// Render the key hints bar
    fn render_keyhints(&self, frame: &mut Frame, area: Rect) {
        let filter_desc = if self.filter_failures {
            "show all"
        } else {
            "failures only"
        };

        let hints = vec![
            KeyHint {
                key: "↑/↓",
                desc: "navigate",
            },
            KeyHint {
                key: "Enter",
                desc: "expand",
            },
            KeyHint {
                key: "f",
                desc: filter_desc,
            },
            KeyHint {
                key: "q",
                desc: "quit",
            },
        ];

        render_keyhints(frame, area, &hints, &self.theme);
    }

    /// Count statuses across all sections
    fn count_statuses(&self) -> (usize, usize, usize, usize, usize) {
        let mut total = 0;
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;
        let mut warn = 0;

        for section in &self.report.sections {
            for item in &section.items {
                total += 1;
                match item.status {
                    HealthStatus::Pass => pass += 1,
                    HealthStatus::Fail => fail += 1,
                    HealthStatus::Skip => skip += 1,
                    HealthStatus::Warn => warn += 1,
                }
            }
        }

        (total, pass, fail, skip, warn)
    }

    /// Count pass/total for a single section
    fn section_counts(&self, section: &HealthSection) -> (usize, usize) {
        let total = section.items.len();
        let pass = section
            .items
            .iter()
            .filter(|i| i.status == HealthStatus::Pass)
            .count();
        (pass, total)
    }
}

/// Run the interactive health report viewer
#[allow(dead_code)]
pub fn run_health_viewer(report: HealthReport) -> anyhow::Result<()> {
    let mut tui = Tui::new()?;
    let mut viewer = HealthViewer::new(report);

    let tick_rate = Duration::from_millis(100);

    loop {
        tui.draw(|f| viewer.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            viewer.handle_key(key);

            if viewer.should_quit() {
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

    fn sample_report() -> HealthReport {
        HealthReport {
            workload_name: "test-workload".to_string(),
            sections: vec![
                HealthSection {
                    name: "Packages".to_string(),
                    items: vec![
                        HealthItem {
                            name: "Git".to_string(),
                            status: HealthStatus::Pass,
                            detail: None,
                        },
                        HealthItem {
                            name: "Node".to_string(),
                            status: HealthStatus::Fail,
                            detail: Some("Not installed".to_string()),
                        },
                        HealthItem {
                            name: "Python".to_string(),
                            status: HealthStatus::Skip,
                            detail: None,
                        },
                    ],
                },
                HealthSection {
                    name: "Files".to_string(),
                    items: vec![HealthItem {
                        name: ".gitconfig".to_string(),
                        status: HealthStatus::Pass,
                        detail: None,
                    }],
                },
            ],
            duration: Duration::from_secs(5),
        }
    }

    #[test]
    fn test_health_viewer_initial_state() {
        let viewer = HealthViewer::new(sample_report());
        assert!(!viewer.should_quit());
        assert_eq!(viewer.selected, 0);
        assert!(viewer.expanded.is_empty());
        assert!(viewer.section_collapsed.is_empty());
        assert!(!viewer.filter_failures);
        assert_eq!(viewer.scroll_offset, 0);
        // 2 section headers + 3 items + 1 item = 6 total navigable items
        assert_eq!(viewer.total_items(), 6);
    }

    #[test]
    fn test_health_viewer_navigation() {
        let mut viewer = HealthViewer::new(sample_report());

        assert_eq!(viewer.selected, 0);

        // Move down
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);

        // Move down with j
        viewer.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(viewer.selected, 2);

        // Move up
        viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);

        // Move up with k
        viewer.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(viewer.selected, 0);

        // Can't go above 0
        viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 0);

        // Navigate to the last item
        for _ in 0..10 {
            viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        assert_eq!(viewer.selected, viewer.total_items() - 1);

        // Quit
        viewer.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(viewer.should_quit());
    }

    #[test]
    fn test_health_viewer_toggle_expand() {
        let mut viewer = HealthViewer::new(sample_report());

        // Move to first item (index 1, since 0 is section header)
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(viewer.selected, 1);

        // Expand
        assert!(!viewer.expanded.contains(&1));
        viewer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(viewer.expanded.contains(&1));

        // Collapse
        viewer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!viewer.expanded.contains(&1));
    }

    #[test]
    fn test_health_viewer_filter_failures() {
        let mut viewer = HealthViewer::new(sample_report());

        // Without filter: 2 section headers + 4 items = 6
        assert_eq!(viewer.total_items(), 6);
        assert!(!viewer.filter_failures);

        // Toggle filter
        viewer.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(viewer.filter_failures);
        assert_eq!(viewer.selected, 0); // reset on filter toggle

        // With filter: 2 section headers + 1 fail item (Node) = 3
        assert_eq!(viewer.total_items(), 3);

        // Toggle back
        viewer.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(!viewer.filter_failures);
        assert_eq!(viewer.total_items(), 6);
    }

    #[test]
    fn test_health_viewer_render_in_test_backend() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewer = HealthViewer::new(sample_report());

        // Verify rendering doesn't panic
        terminal.draw(|f| viewer.render(f)).unwrap();
    }

    #[test]
    fn test_health_viewer_ctrl_c_quits() {
        let mut viewer = HealthViewer::new(sample_report());
        assert!(!viewer.should_quit());
        viewer.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(viewer.should_quit());
    }

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Pass, HealthStatus::Pass);
        assert_ne!(HealthStatus::Pass, HealthStatus::Fail);
        assert_ne!(HealthStatus::Skip, HealthStatus::Warn);
    }
}
