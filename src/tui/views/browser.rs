//! Workload browser — interactive list for `anvil list`
//!
//! This view renders a full-screen TUI showing available workloads
//! with search/filter, navigation, and a detail preview pane.

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

/// A single workload entry for the browser
#[allow(dead_code)]
pub struct WorkloadEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub extends: Vec<String>,
    pub package_count: usize,
    pub file_count: usize,
    pub source: String,
}

/// Interactive workload browser state
pub struct WorkloadBrowser {
    workloads: Vec<WorkloadEntry>,
    filtered: Vec<usize>,
    selected: usize,
    search_query: String,
    searching: bool,
    quit: bool,
    confirmed: bool,
    theme: Theme,
}

impl WorkloadBrowser {
    /// Create a new browser pre-populated with all workloads visible
    pub fn new(workloads: Vec<WorkloadEntry>) -> Self {
        let filtered: Vec<usize> = (0..workloads.len()).collect();
        Self {
            workloads,
            filtered,
            selected: 0,
            search_query: String::new(),
            searching: false,
            quit: false,
            confirmed: false,
            theme: Theme::dark(),
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            // Ctrl+C always quits
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.quit = true;
            }

            // q quits only when not searching
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                self.quit = true;
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k')
                if key.modifiers == KeyModifiers::NONE && !self.searching =>
            {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j')
                if key.modifiers == KeyModifiers::NONE
                    && !self.searching
                    && !self.filtered.is_empty() =>
            {
                self.selected = (self.selected + 1).min(self.filtered.len() - 1);
            }

            // Enter search mode
            KeyCode::Char('/') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                self.searching = true;
            }

            // Escape clears search
            KeyCode::Esc if self.searching => {
                self.searching = false;
                self.search_query.clear();
                self.apply_filter();
                self.selected = 0;
            }

            // Enter: confirm search or select workload
            KeyCode::Enter if key.modifiers == KeyModifiers::NONE => {
                if self.searching {
                    self.searching = false;
                } else if !self.filtered.is_empty() {
                    self.confirmed = true;
                    self.quit = true;
                }
            }

            // Search input
            KeyCode::Char(c) if self.searching => {
                self.search_query.push(c);
                self.apply_filter();
                self.selected = 0;
            }
            KeyCode::Backspace if self.searching => {
                self.search_query.pop();
                self.apply_filter();
                self.selected = 0;
            }

            _ => {}
        }
    }

    /// Rebuild `filtered` indices from the current search query
    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered = self
            .workloads
            .iter()
            .enumerate()
            .filter(|(_, w)| query.is_empty() || w.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();

        // Clamp selection
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    /// Whether the browser should exit
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Returns the name of the confirmed workload, if any
    pub fn selected_name(&self) -> Option<String> {
        if self.confirmed && !self.filtered.is_empty() {
            let idx = self.filtered[self.selected];
            Some(self.workloads[idx].name.clone())
        } else {
            None
        }
    }

    /// Render the browser view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let outer = Layout::vertical([
            Constraint::Length(3), // title
            Constraint::Min(1),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_title(frame, outer[0]);
        self.render_main(frame, outer[1]);
        self.render_keyhints(frame, outer[2]);
    }

    /// Render the title bar
    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(
                " Anvil — Workload Browser ",
                self.theme.title_style(),
            ))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        frame.render_widget(block, area);
    }

    /// Render the main two-pane area
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let panes = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_list(frame, panes[0]);
        self.render_preview(frame, panes[1]);
    }

    /// Render the workload list (left pane)
    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let title = if self.searching {
            format!(
                "Workloads ({}) \u{1F50D} {}",
                self.filtered.len(),
                self.search_query
            )
        } else {
            format!("Workloads ({})", self.filtered.len())
        };

        let block = Block::default()
            .title(Span::styled(title, self.theme.normal()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible_height = inner.height as usize;

        // Compute scroll so the selected item is always visible
        let scroll_offset = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        let lines: Vec<Line> = self
            .filtered
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, &idx)| {
                let entry = &self.workloads[idx];
                let style = if i == self.selected {
                    Style::default()
                        .bg(self.theme.accent)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    self.theme.normal()
                };
                Line::styled(format!("  {}", entry.name), style)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);

        // Scrollbar
        let total = self.filtered.len();
        if total > visible_height {
            let max_scroll = total.saturating_sub(visible_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    /// Render the detail preview (right pane)
    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Details", self.theme.normal()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.filtered.is_empty() {
            let empty = Paragraph::new(Line::styled("  No workloads found", self.theme.dimmed()));
            frame.render_widget(empty, inner);
            return;
        }

        let idx = self.filtered[self.selected];
        let entry = &self.workloads[idx];

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", entry.name),
            self.theme.title_style(),
        )]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Version: ", self.theme.dimmed()),
            Span::styled(&entry.version, self.theme.normal()),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Description: ", self.theme.dimmed()),
            Span::styled(&entry.description, self.theme.normal()),
        ]));
        lines.push(Line::raw(""));

        if !entry.extends.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Extends: ", self.theme.dimmed()),
                Span::styled(entry.extends.join(", "), self.theme.normal()),
            ]));
            lines.push(Line::raw(""));
        }

        lines.push(Line::from(vec![
            Span::styled("  Packages: ", self.theme.dimmed()),
            Span::styled(entry.package_count.to_string(), self.theme.normal()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Files: ", self.theme.dimmed()),
            Span::styled(entry.file_count.to_string(), self.theme.normal()),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Source: ", self.theme.dimmed()),
            Span::styled(&entry.source, self.theme.normal()),
        ]));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    /// Render the bottom key hints bar
    fn render_keyhints(&self, frame: &mut Frame, area: Rect) {
        let hints = if self.searching {
            vec![
                KeyHint {
                    key: "type",
                    desc: "filter",
                },
                KeyHint {
                    key: "Enter",
                    desc: "confirm",
                },
                KeyHint {
                    key: "Esc",
                    desc: "clear",
                },
            ]
        } else {
            vec![
                KeyHint {
                    key: "↑/↓",
                    desc: "navigate",
                },
                KeyHint {
                    key: "/",
                    desc: "search",
                },
                KeyHint {
                    key: "Enter",
                    desc: "select",
                },
                KeyHint {
                    key: "q",
                    desc: "quit",
                },
            ]
        };

        render_keyhints(frame, area, &hints, &self.theme);
    }
}

/// Run the interactive workload browser
///
/// Returns `Some(name)` if the user selected a workload, `None` if they quit.
pub fn run_browser(workloads: Vec<WorkloadEntry>) -> anyhow::Result<Option<String>> {
    let mut tui = Tui::new()?;
    let mut browser = WorkloadBrowser::new(workloads);

    let tick_rate = Duration::from_millis(100);

    loop {
        tui.draw(|f| browser.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            browser.handle_key(key);

            if browser.should_quit() {
                break;
            }
        }
    }

    tui.restore()?;
    Ok(browser.selected_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a small set of test workloads
    fn sample_workloads() -> Vec<WorkloadEntry> {
        vec![
            WorkloadEntry {
                name: "essentials".to_string(),
                version: "1.0.0".to_string(),
                description: "Core developer tools".to_string(),
                extends: vec![],
                package_count: 12,
                file_count: 3,
                source: "default".to_string(),
            },
            WorkloadEntry {
                name: "rust-developer".to_string(),
                version: "1.2.0".to_string(),
                description: "Rust toolchain and utilities".to_string(),
                extends: vec!["essentials".to_string()],
                package_count: 5,
                file_count: 1,
                source: "local".to_string(),
            },
            WorkloadEntry {
                name: "node-developer".to_string(),
                version: "0.9.0".to_string(),
                description: "Node.js ecosystem".to_string(),
                extends: vec!["essentials".to_string()],
                package_count: 8,
                file_count: 2,
                source: "remote".to_string(),
            },
        ]
    }

    #[test]
    fn test_browser_initial_state() {
        let browser = WorkloadBrowser::new(sample_workloads());
        assert_eq!(browser.selected, 0);
        assert!(!browser.searching);
        assert!(!browser.should_quit());
        assert!(!browser.confirmed);
        assert_eq!(browser.filtered.len(), 3);
        assert!(browser.search_query.is_empty());
    }

    #[test]
    fn test_browser_navigation() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Down increments
        browser.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(browser.selected, 1);
        browser.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(browser.selected, 2);

        // Clamps at end
        browser.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(browser.selected, 2);

        // Up decrements
        browser.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(browser.selected, 1);

        // Clamps at start
        browser.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(browser.selected, 0);
        browser.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(browser.selected, 0);

        // k/j also work
        browser.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(browser.selected, 1);
        browser.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn test_browser_search() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Enter search mode
        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(browser.searching);

        // Type "rust"
        browser.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));

        assert_eq!(browser.search_query, "rust");
        assert_eq!(browser.filtered.len(), 1);
        assert_eq!(
            browser.workloads[browser.filtered[0]].name,
            "rust-developer"
        );

        // Backspace removes a char
        browser.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(browser.search_query, "rus");

        // Enter confirms search (exits search mode but keeps filter)
        browser.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!browser.searching);
        assert_eq!(browser.search_query, "rus");

        // Esc clears search entirely
        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!browser.searching);
        assert!(browser.search_query.is_empty());
        assert_eq!(browser.filtered.len(), 3);
    }

    #[test]
    fn test_browser_select() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Move to second item
        browser.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(browser.selected, 1);

        // Press Enter to confirm
        browser.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(browser.confirmed);
        assert!(browser.should_quit());
        assert_eq!(browser.selected_name(), Some("rust-developer".to_string()));
    }

    #[test]
    fn test_browser_quit_without_selection() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        browser.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(browser.should_quit());
        assert!(!browser.confirmed);
        assert_eq!(browser.selected_name(), None);
    }

    #[test]
    fn test_browser_ctrl_c_quits_during_search() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Enter search mode, then Ctrl+C should still quit
        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(browser.searching);
        browser.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(browser.should_quit());
    }

    #[test]
    fn test_browser_render_in_test_backend() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let browser = WorkloadBrowser::new(sample_workloads());

        // Should render without panicking
        terminal.draw(|f| browser.render(f)).unwrap();
    }

    #[test]
    fn test_browser_render_empty_list() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let browser = WorkloadBrowser::new(vec![]);

        terminal.draw(|f| browser.render(f)).unwrap();
        assert_eq!(browser.selected_name(), None);
    }

    #[test]
    fn test_browser_search_no_results() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

        assert_eq!(browser.search_query, "zz");
        assert!(browser.filtered.is_empty());
        assert_eq!(browser.selected, 0);
    }
}
