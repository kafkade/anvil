//! Workload browser — interactive list for `anvil list`
//!
//! This view renders a full-screen TUI showing available workloads
//! with search/filter, navigation, and a detail preview pane.

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::tui::theme::{SectionKind, Theme};
use crate::tui::widgets::chrome::render_header;
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::Tui;

/// A single workload entry for the browser
pub struct WorkloadEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub extends: Vec<String>,
    pub package_count: usize,
    pub file_count: usize,
    pub command_count: usize,
    pub font_count: usize,
    pub feature_count: usize,
    pub assertion_count: usize,
    pub source: String,
    /// Filesystem path to the workload.yaml file
    pub path: String,
}

/// The result of running the browser view
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserOutcome {
    /// User quit the browser (q / Ctrl+C)
    Quit,
    /// User selected a workload to view details
    Select(String),
    /// User requested install of a workload (name, path)
    Install(String, String),
    /// User requested dry-run of a workload (name, path)
    DryRun(String, String),
    /// User requested health check of a workload (name, path)
    Health(String, String),
}

/// Browser layout mode for the list + preview panes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserLayoutMode {
    /// Auto-detect based on terminal width
    Auto,
    /// Force side-by-side (list left, details right)
    Horizontal,
    /// Force stacked (list top, details bottom)
    Vertical,
}

impl BrowserLayoutMode {
    fn next(self) -> Self {
        match self {
            Self::Auto => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Auto,
        }
    }
}

/// Interactive workload browser state
pub struct WorkloadBrowser {
    workloads: Vec<WorkloadEntry>,
    filtered: Vec<usize>,
    selected: usize,
    search_query: String,
    searching: bool,
    quit: bool,
    outcome: Option<BrowserOutcome>,
    theme: Theme,
    layout_mode: BrowserLayoutMode,
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
            outcome: None,
            theme: Theme::dark(),
            layout_mode: BrowserLayoutMode::Auto,
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            // Ctrl+C always quits
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.outcome = Some(BrowserOutcome::Quit);
                self.quit = true;
            }

            // q quits only when not searching
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                self.outcome = Some(BrowserOutcome::Quit);
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
                    let name = self.focused_name().unwrap();
                    self.outcome = Some(BrowserOutcome::Select(name));
                    self.quit = true;
                }
            }

            // Install focused workload
            KeyCode::Char('i') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                if let Some((name, path)) = self.focused_name_and_path() {
                    self.outcome = Some(BrowserOutcome::Install(name, path));
                    self.quit = true;
                }
            }

            // Dry-run focused workload
            KeyCode::Char('d') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                if let Some((name, path)) = self.focused_name_and_path() {
                    self.outcome = Some(BrowserOutcome::DryRun(name, path));
                    self.quit = true;
                }
            }

            // Health check focused workload
            KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE && !self.searching => {
                if let Some((name, path)) = self.focused_name_and_path() {
                    self.outcome = Some(BrowserOutcome::Health(name, path));
                    self.quit = true;
                }
            }

            // Toggle layout mode
            KeyCode::Tab if !self.searching => {
                self.layout_mode = self.layout_mode.next();
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

    /// Returns the name of the currently focused workload, if any
    fn focused_name(&self) -> Option<String> {
        if !self.filtered.is_empty() {
            let idx = self.filtered[self.selected];
            Some(self.workloads[idx].name.clone())
        } else {
            None
        }
    }

    /// Returns the (name, path) of the currently focused workload, if any
    fn focused_name_and_path(&self) -> Option<(String, String)> {
        if !self.filtered.is_empty() {
            let idx = self.filtered[self.selected];
            let e = &self.workloads[idx];
            Some((e.name.clone(), e.path.clone()))
        } else {
            None
        }
    }

    /// Returns the outcome after the browser exits
    pub fn outcome(&self) -> BrowserOutcome {
        self.outcome.clone().unwrap_or(BrowserOutcome::Quit)
    }

    /// Render the browser view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let outer = Layout::vertical([
            Constraint::Length(1), // branded header bar
            Constraint::Min(1),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        render_header(
            frame,
            outer[0],
            &self.theme,
            &["Workload Browser"],
            Some(Line::from(vec![Span::styled(
                format!("{} workloads", self.workloads.len()),
                self.theme.dimmed(),
            )])),
        );
        self.render_main(frame, outer[1]);
        self.render_keyhints(frame, outer[2]);
    }

    /// Minimum terminal width for side-by-side (horizontal) layout.
    /// Below this, the list and detail panes stack vertically.
    const MIN_HORIZONTAL_WIDTH: u16 = 100;

    /// Whether the effective layout is horizontal for the given width
    fn is_horizontal(&self, width: u16) -> bool {
        match self.layout_mode {
            BrowserLayoutMode::Auto => width >= Self::MIN_HORIZONTAL_WIDTH,
            BrowserLayoutMode::Horizontal => true,
            BrowserLayoutMode::Vertical => false,
        }
    }

    /// Render the main two-pane area
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        if self.is_horizontal(area.width) {
            // Wide: side-by-side with a 1-col gap between panes
            let panes = Layout::horizontal([
                Constraint::Percentage(45),
                Constraint::Length(1), // divider column
                Constraint::Percentage(55),
            ])
            .split(area);

            self.render_list(frame, panes[0]);

            // Vertical divider line
            let divider_lines: Vec<Line> = (0..panes[1].height)
                .map(|_| Line::from(Span::styled("│", self.theme.border_style())))
                .collect();
            frame.render_widget(Paragraph::new(divider_lines), panes[1]);

            self.render_preview(frame, panes[2]);
        } else {
            // Narrow: stacked with a 1-row divider
            let panes = Layout::vertical([
                Constraint::Percentage(50),
                Constraint::Length(1), // divider row
                Constraint::Percentage(50),
            ])
            .split(area);

            self.render_list(frame, panes[0]);

            // Horizontal divider
            let divider_text = "─".repeat(panes[1].width as usize);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    divider_text,
                    self.theme.border_style(),
                ))),
                panes[1],
            );

            self.render_preview(frame, panes[2]);
        }
    }

    /// Render the workload list (left pane)
    fn render_list(&self, frame: &mut Frame, area: Rect) {
        // Bordered block for the list pane
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_style());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split: search bar (1 line) + gap (1 line) + list items
        let chunks = Layout::vertical([
            Constraint::Length(1), // search bar
            Constraint::Length(1), // padding below search
            Constraint::Min(1),    // list items
        ])
        .split(inner);

        // Search bar with bg_inset
        let search_text = if self.searching {
            format!("/ {}", self.search_query)
        } else {
            "/ Search workloads...".to_string()
        };
        let search_style = if self.searching {
            self.theme.normal()
        } else {
            self.theme.faint_style()
        };
        let search_bar = Paragraph::new(Line::from(Span::styled(
            format!(" {}", search_text),
            search_style,
        )))
        .style(Style::default().bg(self.theme.bg_inset));
        frame.render_widget(search_bar, chunks[0]);

        // List items area
        let list_area = chunks[2];
        let visible_height = list_area.height as usize;
        let list_width = list_area.width as usize;

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

                // Build right-side info: "V1.0.0  ☁ N pkg"
                let version_text = format!("V{}", entry.version);
                let remote_icon = if entry.source == "remote" { " ☁" } else { "" };
                let pkg_text = format!("{} pkg", entry.package_count);
                let right_text = format!("{}{}  {}", version_text, remote_icon, pkg_text);
                let right_len = right_text.len();

                // Name gets the remaining space (minus prefix + right + gap)
                let prefix_len = 2; // "▌ " or "  "
                let gap = 2;
                let name_max = list_width.saturating_sub(prefix_len + right_len + gap);
                let name_display = if entry.name.len() > name_max && name_max > 3 {
                    format!("{}…", &entry.name[..name_max.saturating_sub(1)])
                } else {
                    entry.name.clone()
                };
                let name_pad = name_max.saturating_sub(name_display.len());

                if i == self.selected {
                    Line::from(vec![
                        Span::styled("▌", Style::default().fg(self.theme.accent)),
                        Span::styled(
                            format!(" {}{}", name_display, " ".repeat(name_pad)),
                            self.theme.selection().bg(self.theme.bg_inset),
                        ),
                        Span::styled(
                            format!("  {}", version_text),
                            Style::default()
                                .fg(self.theme.muted)
                                .bg(self.theme.bg_inset)
                                .add_modifier(Modifier::BOLD),
                        ),
                        if !remote_icon.is_empty() {
                            Span::styled(
                                " ☁",
                                Style::default().fg(self.theme.info).bg(self.theme.bg_inset),
                            )
                        } else {
                            Span::styled("", Style::default().bg(self.theme.bg_inset))
                        },
                        Span::styled(
                            format!("  {}", pkg_text),
                            Style::default()
                                .fg(self.theme.muted)
                                .bg(self.theme.bg_inset),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{}{}", name_display, " ".repeat(name_pad)),
                            self.theme.normal(),
                        ),
                        Span::styled(
                            format!("  {}", version_text),
                            Style::default()
                                .fg(self.theme.faint)
                                .add_modifier(Modifier::BOLD),
                        ),
                        if entry.source == "remote" {
                            Span::styled(" ☁", Style::default().fg(self.theme.info))
                        } else {
                            Span::raw("")
                        },
                        Span::styled(format!("  {}", pkg_text), self.theme.faint_style()),
                    ])
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, list_area);

        // Scrollbar
        let total = self.filtered.len();
        if total > visible_height {
            let max_scroll = total.saturating_sub(visible_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);
            frame.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
        }
    }

    /// Render the detail preview (right pane)
    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        // Bordered block for the preview pane
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
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

        // Top padding
        lines.push(Line::raw(""));

        // Name + version badge + source badge
        let source_label = entry.source.to_uppercase();
        let source_color = if entry.source == "remote" {
            self.theme.info
        } else {
            self.theme.success
        };
        lines.push(Line::from(vec![
            Span::styled(&entry.name, self.theme.title_style()),
            Span::styled(
                format!("  V{}", entry.version),
                Style::default()
                    .fg(self.theme.faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} ", source_label),
                Style::default()
                    .fg(source_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Description (wrapped across lines)
        if !entry.description.is_empty() {
            lines.push(Line::raw(""));
            let max_width = inner.width.saturating_sub(2) as usize;
            let desc = &entry.description;
            if max_width == 0 || desc.len() <= max_width {
                lines.push(Line::from(Span::styled(desc.as_str(), self.theme.body())));
            } else {
                let mut remaining = desc.as_str();
                while !remaining.is_empty() {
                    if remaining.len() <= max_width {
                        lines.push(Line::from(Span::styled(remaining, self.theme.body())));
                        break;
                    }
                    let split_at = remaining[..max_width].rfind(' ').unwrap_or(max_width);
                    lines.push(Line::from(Span::styled(
                        &remaining[..split_at],
                        self.theme.body(),
                    )));
                    remaining = remaining[split_at..].trim_start();
                }
            }
        }

        // Hairline divider helper
        let divider_width = inner.width.saturating_sub(2) as usize;
        let hairline = Line::from(Span::styled(
            "─".repeat(divider_width),
            Style::default().fg(self.theme.hairline),
        ));

        // ── SOURCE ──
        lines.push(Line::raw(""));
        lines.push(hairline.clone());
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("SOURCE", self.theme.label_style())));
        lines.push(Line::raw(""));
        let source_icon = if entry.source == "remote" {
            "☁"
        } else {
            "📁"
        };
        let source_style = if entry.source == "remote" {
            self.theme.running_style()
        } else {
            self.theme.success_style()
        };
        lines.push(Line::from(vec![Span::styled(
            format!("{} {}", source_icon, entry.source),
            source_style,
        )]));
        lines.push(Line::from(Span::styled(
            &entry.path,
            self.theme.running_style(),
        )));

        // ── CONTENTS ──
        lines.push(Line::raw(""));
        lines.push(hairline.clone());
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "CONTENTS",
            self.theme.label_style(),
        )));
        lines.push(Line::raw(""));

        // Row 1: Packages + Fonts
        let row1 = self.contents_row(
            SectionKind::Packages,
            entry.package_count,
            SectionKind::Fonts,
            entry.font_count,
        );
        lines.push(row1);

        // Row 2: Files + Features
        let row2 = self.contents_row(
            SectionKind::Files,
            entry.file_count,
            SectionKind::Features,
            entry.feature_count,
        );
        lines.push(row2);

        // Row 3: Commands + Assertions
        let row3 = self.contents_row(
            SectionKind::Commands,
            entry.command_count,
            SectionKind::Assertions,
            entry.assertion_count,
        );
        lines.push(row3);

        // ── META ──
        lines.push(Line::raw(""));
        lines.push(hairline);
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Last installed  ", self.theme.label_style()),
            Span::styled("Extends  ", self.theme.label_style()),
        ]));
        let extends_text = if entry.extends.is_empty() {
            "None".to_string()
        } else {
            entry.extends.join(", ")
        };
        lines.push(Line::from(vec![
            Span::styled("—               ", self.theme.faint_style()),
            Span::styled(extends_text, self.theme.normal()),
        ]));

        // Render with left padding inside the block
        let padded = Rect {
            x: inner.x + 1,
            y: inner.y,
            width: inner.width.saturating_sub(2),
            height: inner.height,
        };
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, padded);
    }

    /// Build a 2-column contents row with equal-width columns.
    /// Each column: `icon label       count` (fixed 20 chars).
    fn contents_row(
        &self,
        left_kind: SectionKind,
        left_count: usize,
        right_kind: SectionKind,
        right_count: usize,
    ) -> Line<'_> {
        // Fixed column width: 1 icon + 1 space + 11 label + 1 space + 4 count = 18
        // Then 4-char gap between columns
        let col_label_w = 11;
        let col_count_w = 4;

        let left_label = format!("{:width$}", left_kind.label(), width = col_label_w);
        let left_ct = format!("{:>width$}", left_count, width = col_count_w);
        let right_label = format!("{:width$}", right_kind.label(), width = col_label_w);
        let right_ct = format!("{:>width$}", right_count, width = col_count_w);

        Line::from(vec![
            Span::styled(
                left_kind.icon(),
                Style::default().fg(self.theme.section_color(left_kind)),
            ),
            Span::styled(format!(" {}", left_label), self.theme.normal()),
            Span::styled(left_ct, self.theme.normal()),
            Span::raw("    "),
            Span::styled(
                right_kind.icon(),
                Style::default().fg(self.theme.section_color(right_kind)),
            ),
            Span::styled(format!(" {}", right_label), self.theme.normal()),
            Span::styled(right_ct, self.theme.normal()),
        ])
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
                    key: "↕",
                    desc: "navigate",
                },
                KeyHint {
                    key: "/",
                    desc: "search",
                },
                KeyHint {
                    key: "↵",
                    desc: "open",
                },
                KeyHint {
                    key: "i",
                    desc: "install",
                },
                KeyHint {
                    key: "d",
                    desc: "dry-run",
                },
                KeyHint {
                    key: "h",
                    desc: "health",
                },
                KeyHint {
                    key: "q",
                    desc: "quit",
                },
            ]
        };

        render_keyhints(frame, area, &hints, &self.theme);
    }
    /// Reset the browser state so it can be re-entered after returning
    /// from a detail view or install action. Layout preference is preserved.
    pub fn reset(&mut self) {
        self.quit = false;
        self.outcome = None;
    }

    /// Run the browser event loop using an existing TUI session.
    ///
    /// Returns the outcome indicating what the user chose.
    pub fn run(&mut self, tui: &mut Tui) -> anyhow::Result<BrowserOutcome> {
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

/// Run the interactive workload browser
///
/// Returns the outcome indicating what the user chose.
pub fn run_browser(workloads: Vec<WorkloadEntry>) -> anyhow::Result<BrowserOutcome> {
    let mut tui = Tui::new()?;
    let mut browser = WorkloadBrowser::new(workloads);
    let result = browser.run(&mut tui)?;
    tui.restore()?;
    Ok(result)
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
                command_count: 1,
                font_count: 2,
                feature_count: 1,
                assertion_count: 8,
                source: "default".to_string(),
                path: "/workloads/essentials/workload.yaml".to_string(),
            },
            WorkloadEntry {
                name: "rust-developer".to_string(),
                version: "1.2.0".to_string(),
                description: "Rust toolchain and utilities".to_string(),
                extends: vec!["essentials".to_string()],
                package_count: 5,
                file_count: 1,
                command_count: 0,
                font_count: 0,
                feature_count: 0,
                assertion_count: 3,
                source: "local".to_string(),
                path: "/workloads/rust-developer/workload.yaml".to_string(),
            },
            WorkloadEntry {
                name: "node-developer".to_string(),
                version: "0.9.0".to_string(),
                description: "Node.js ecosystem".to_string(),
                extends: vec!["essentials".to_string()],
                package_count: 8,
                file_count: 2,
                command_count: 2,
                font_count: 0,
                feature_count: 0,
                assertion_count: 5,
                source: "remote".to_string(),
                path: "/workloads/node-developer/workload.yaml".to_string(),
            },
        ]
    }

    #[test]
    fn test_browser_initial_state() {
        let browser = WorkloadBrowser::new(sample_workloads());
        assert_eq!(browser.selected, 0);
        assert!(!browser.searching);
        assert!(!browser.should_quit());
        assert_eq!(browser.outcome(), BrowserOutcome::Quit);
        assert_eq!(browser.filtered.len(), 3);
        assert!(browser.search_query.is_empty());
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Auto);
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
        assert!(browser.should_quit());
        assert_eq!(
            browser.outcome(),
            BrowserOutcome::Select("rust-developer".to_string())
        );
    }

    #[test]
    fn test_browser_quit_without_selection() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        browser.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(browser.should_quit());
        assert_eq!(browser.outcome(), BrowserOutcome::Quit);
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

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let browser = WorkloadBrowser::new(sample_workloads());

        // Wide terminal: should render horizontal layout without panicking
        terminal.draw(|f| browser.render(f)).unwrap();
    }

    #[test]
    fn test_browser_render_narrow_terminal() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let browser = WorkloadBrowser::new(sample_workloads());

        // Narrow terminal: should render vertical (stacked) layout without panicking
        terminal.draw(|f| browser.render(f)).unwrap();
    }

    #[test]
    fn test_browser_render_empty_list() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let browser = WorkloadBrowser::new(vec![]);

        terminal.draw(|f| browser.render(f)).unwrap();
        assert_eq!(browser.outcome(), BrowserOutcome::Quit);
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

    #[test]
    fn test_browser_install_key() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Navigate to second item and press 'i'
        browser.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(browser.should_quit());
        assert_eq!(
            browser.outcome(),
            BrowserOutcome::Install(
                "rust-developer".to_string(),
                "/workloads/rust-developer/workload.yaml".to_string(),
            )
        );
    }

    #[test]
    fn test_browser_dryrun_key() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Press 'd' on first item
        browser.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(browser.should_quit());
        assert_eq!(
            browser.outcome(),
            BrowserOutcome::DryRun(
                "essentials".to_string(),
                "/workloads/essentials/workload.yaml".to_string(),
            )
        );
    }

    #[test]
    fn test_browser_install_ignored_during_search() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Enter search mode
        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(browser.searching);

        // 'i' and 'd' should be search input, not actions
        browser.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(!browser.should_quit());
        assert_eq!(browser.search_query, "i");

        browser.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(!browser.should_quit());
        assert_eq!(browser.search_query, "id");
    }

    #[test]
    fn test_browser_install_empty_list_does_nothing() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        // Filter to empty
        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        browser.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(browser.filtered.is_empty());

        // 'i' does nothing when list is empty
        browser.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(!browser.should_quit());
    }

    #[test]
    fn test_browser_layout_toggle() {
        let mut browser = WorkloadBrowser::new(sample_workloads());
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Auto);

        // Tab cycles: Auto -> Horizontal -> Vertical -> Auto
        browser.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Horizontal);

        browser.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Vertical);

        browser.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Auto);
    }

    #[test]
    fn test_browser_layout_toggle_ignored_during_search() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        browser.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(browser.searching);

        browser.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Auto);
    }

    #[test]
    fn test_browser_layout_preserved_across_reset() {
        let mut browser = WorkloadBrowser::new(sample_workloads());

        browser.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Horizontal);

        browser.reset();
        assert_eq!(browser.layout_mode, BrowserLayoutMode::Horizontal);
    }

    #[test]
    fn test_browser_is_horizontal() {
        let browser = WorkloadBrowser::new(sample_workloads());

        // Auto mode
        assert!(browser.is_horizontal(120));
        assert!(!browser.is_horizontal(80));

        // Forced modes
        let mut b2 = WorkloadBrowser::new(sample_workloads());
        b2.layout_mode = BrowserLayoutMode::Horizontal;
        assert!(b2.is_horizontal(50)); // forced horizontal even on narrow

        b2.layout_mode = BrowserLayoutMode::Vertical;
        assert!(!b2.is_horizontal(200)); // forced vertical even on wide
    }

    #[test]
    fn test_browser_render_forced_vertical_on_wide() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut browser = WorkloadBrowser::new(sample_workloads());
        browser.layout_mode = BrowserLayoutMode::Vertical;

        terminal.draw(|f| browser.render(f)).unwrap();
    }
}
