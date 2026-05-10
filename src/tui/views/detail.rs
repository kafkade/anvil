//! Interactive workload detail view
//!
//! This view renders a full-screen TUI showing workload metadata
//! with collapsible sections for packages, files, commands, etc.

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

/// A file entry in the workload
#[allow(dead_code)]
pub struct FileEntry {
    pub source: String,
    pub destination: String,
}

/// A command/script entry in the workload
#[allow(dead_code)]
pub struct CommandEntry {
    pub name: String,
    pub phase: String,
}

/// All metadata for a single workload
#[allow(dead_code)]
pub struct WorkloadDetail {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: String,
    pub extends: Vec<String>,
    pub packages: Vec<String>,
    pub files: Vec<FileEntry>,
    pub commands: Vec<CommandEntry>,
    pub assertions: Vec<String>,
}

/// Number of collapsible sections
const SECTION_COUNT: usize = 5;

/// Interactive detail view state
#[allow(dead_code)]
pub struct DetailView {
    detail: WorkloadDetail,
    selected: usize,
    collapsed: HashSet<usize>,
    scroll_offset: usize,
    quit: bool,
    theme: Theme,
}

#[allow(dead_code)]
impl DetailView {
    /// Create a new detail view for the given workload
    pub fn new(detail: WorkloadDetail) -> Self {
        Self {
            detail,
            selected: 0,
            collapsed: HashSet::new(),
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
                let max = self.total_lines().saturating_sub(1);
                if self.selected < max {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                // Determine which section header the cursor is on and toggle it
                if let Some(section_idx) = self.line_to_section(self.selected) {
                    if self.collapsed.contains(&section_idx) {
                        self.collapsed.remove(&section_idx);
                    } else {
                        self.collapsed.insert(section_idx);
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether the view should exit
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Total number of navigable lines in the content area
    pub fn total_lines(&self) -> usize {
        self.build_lines(None).len()
    }

    /// Map a line index to a section index if it's a section header
    fn line_to_section(&self, line_idx: usize) -> Option<usize> {
        let mut current = 0;
        for section in 0..SECTION_COUNT {
            if current == line_idx {
                return Some(section);
            }
            current += 1; // header line
            if !self.collapsed.contains(&section) {
                current += self.section_item_count(section);
            }
        }
        None
    }

    /// Number of child items in a section
    fn section_item_count(&self, section: usize) -> usize {
        match section {
            0 => {
                if self.detail.extends.is_empty() {
                    1 // "None"
                } else {
                    self.detail.extends.len()
                }
            }
            1 => self.detail.packages.len(),
            2 => self.detail.files.len(),
            3 => self.detail.commands.len(),
            4 => self.detail.assertions.len(),
            _ => 0,
        }
    }

    /// Build the content lines, optionally highlighting the selected line.
    /// When `highlight_style` is `None`, lines are built without selection styling
    /// (used for counting). When `Some`, the selected line gets that style.
    fn build_lines(&self, highlight_style: Option<Style>) -> Vec<Line<'_>> {
        let mut lines: Vec<Line> = Vec::new();
        let mut line_idx: usize = 0;

        for section in 0..SECTION_COUNT {
            let (label, _count) = self.section_header_info(section);
            let is_collapsed = self.collapsed.contains(&section);
            let arrow = if is_collapsed { "▸" } else { "▾" };
            let header_text = format!("  {} {}", arrow, label);

            let header_style =
                if let Some(hl) = highlight_style.filter(|_| line_idx == self.selected) {
                    hl
                } else {
                    self.theme.title_style()
                };
            lines.push(Line::from(Span::styled(header_text, header_style)));
            line_idx += 1;

            if !is_collapsed {
                let items = self.section_items(section);
                if items.is_empty() && section != 0 {
                    // No items — nothing to show
                } else {
                    for item_text in &items {
                        let item_style = if let Some(hl) =
                            highlight_style.filter(|_| line_idx == self.selected)
                        {
                            hl
                        } else {
                            self.theme.normal()
                        };
                        lines.push(Line::from(Span::styled(
                            format!("    {}", item_text),
                            item_style,
                        )));
                        line_idx += 1;
                    }
                }
            }
        }

        lines
    }

    /// Section header label and item count
    fn section_header_info(&self, section: usize) -> (String, usize) {
        match section {
            0 => ("Inheritance".to_string(), self.detail.extends.len()),
            1 => {
                let n = self.detail.packages.len();
                (format!("Packages ({})", n), n)
            }
            2 => {
                let n = self.detail.files.len();
                (format!("Files ({})", n), n)
            }
            3 => {
                let n = self.detail.commands.len();
                (format!("Commands ({})", n), n)
            }
            4 => {
                let n = self.detail.assertions.len();
                (format!("Assertions ({})", n), n)
            }
            _ => ("Unknown".to_string(), 0),
        }
    }

    /// Item strings for a section
    fn section_items(&self, section: usize) -> Vec<String> {
        match section {
            0 => {
                if self.detail.extends.is_empty() {
                    vec!["None".to_string()]
                } else {
                    self.detail.extends.clone()
                }
            }
            1 => self.detail.packages.clone(),
            2 => self
                .detail
                .files
                .iter()
                .map(|f| format!("{} → {}", f.source, f.destination))
                .collect(),
            3 => self
                .detail
                .commands
                .iter()
                .map(|c| format!("[{}] {}", c.phase, c.name))
                .collect(),
            4 => self.detail.assertions.clone(),
            _ => Vec::new(),
        }
    }

    /// Render the full view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(5), // header
            Constraint::Min(3),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_header(frame, chunks[0]);
        self.render_main(frame, chunks[1]);
        self.render_keyhints(frame, chunks[2]);
    }

    /// Render the header block with workload name, version, description
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" Anvil — Workload: {} ", self.detail.name);
        let block = Block::default()
            .title(Span::styled(title, self.theme.title_style()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(
                &self.detail.name,
                Style::default()
                    .fg(self.theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("v{}", self.detail.version),
                self.theme.dimmed(),
            )),
            Line::from(Span::styled(&self.detail.description, self.theme.normal())),
            Line::from(Span::styled(&self.detail.path, self.theme.dimmed())),
        ];

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    /// Render the main content area with collapsible sections
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let highlight = Style::default().bg(self.theme.accent).fg(Color::Black);
        let lines = self.build_lines(Some(highlight));

        let visible_height = inner.height as usize;
        let total = lines.len();
        let max_scroll = total.saturating_sub(visible_height);

        // Auto-scroll to keep selected line visible
        let scroll = if self.selected < self.scroll_offset {
            self.selected
        } else if self.selected >= self.scroll_offset + visible_height {
            self.selected.saturating_sub(visible_height) + 1
        } else {
            self.scroll_offset
        }
        .min(max_scroll);

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
        let hints = vec![
            KeyHint {
                key: "↑/↓",
                desc: "navigate",
            },
            KeyHint {
                key: "Enter",
                desc: "toggle",
            },
            KeyHint {
                key: "q",
                desc: "quit",
            },
        ];

        render_keyhints(frame, area, &hints, &self.theme);
    }
}

/// Run the interactive detail view event loop
#[allow(dead_code)]
pub fn run_detail_view(detail: WorkloadDetail) -> anyhow::Result<()> {
    let mut tui = Tui::new()?;
    let mut view = DetailView::new(detail);

    let tick_rate = Duration::from_millis(100);

    loop {
        tui.draw(|f| view.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            view.handle_key(key);

            if view.should_quit() {
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

    fn sample_detail() -> WorkloadDetail {
        WorkloadDetail {
            name: "test-workload".to_string(),
            version: "1.0.0".to_string(),
            description: "A test workload".to_string(),
            path: "C:\\test\\workloads\\test-workload".to_string(),
            extends: vec!["essentials".to_string()],
            packages: vec!["Git.Git".to_string(), "Node.js".to_string()],
            files: vec![FileEntry {
                source: "config.yaml".to_string(),
                destination: "~/.config/app/config.yaml".to_string(),
            }],
            commands: vec![CommandEntry {
                name: "setup-env".to_string(),
                phase: "post_install".to_string(),
            }],
            assertions: vec!["git is installed".to_string()],
        }
    }

    #[test]
    fn test_detail_initial_state() {
        let view = DetailView::new(sample_detail());
        assert_eq!(view.selected, 0);
        assert!(view.collapsed.is_empty());
        assert!(!view.should_quit());
        assert!(view.total_lines() > 0);
    }

    #[test]
    fn test_detail_navigation() {
        let mut view = DetailView::new(sample_detail());
        let max = view.total_lines().saturating_sub(1);

        // Move down
        view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(view.selected, 1);

        // Move down with j
        view.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(view.selected, 2);

        // Move up
        view.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(view.selected, 1);

        // Move up with k
        view.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(view.selected, 0);

        // Cannot go above 0
        view.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(view.selected, 0);

        // Navigate to end and verify clamping
        for _ in 0..max + 5 {
            view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        assert_eq!(view.selected, max);
    }

    #[test]
    fn test_detail_toggle_section() {
        let mut view = DetailView::new(sample_detail());
        let initial_lines = view.total_lines();

        // Selected line 0 is the first section header (Inheritance)
        assert_eq!(view.selected, 0);
        assert!(!view.collapsed.contains(&0));

        // Toggle collapse
        view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(view.collapsed.contains(&0));
        assert!(view.total_lines() < initial_lines);

        // Toggle expand
        view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!view.collapsed.contains(&0));
        assert_eq!(view.total_lines(), initial_lines);
    }

    #[test]
    fn test_detail_quit() {
        let mut view = DetailView::new(sample_detail());

        // q quits
        assert!(!view.should_quit());
        view.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(view.should_quit());

        // Ctrl+C also quits
        let mut view2 = DetailView::new(sample_detail());
        assert!(!view2.should_quit());
        view2.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(view2.should_quit());
    }
}
