//! Interactive workload detail view
//!
//! This view renders a full-screen TUI showing workload metadata
//! with tabbed sections for packages, files, commands, etc.

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Tabs,
    },
    Frame,
};

use crate::tui::theme::{SectionKind, Theme};
use crate::tui::widgets::chrome::render_header;
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::Tui;

/// A file entry in the workload
pub struct FileEntry {
    pub source: String,
    pub destination: String,
}

/// A command/script entry in the workload
pub struct CommandEntry {
    pub name: String,
    pub phase: String,
}

/// All metadata for a single workload
pub struct WorkloadDetail {
    pub name: String,
    pub version: String,
    pub description: String,
    pub extends: Vec<String>,
    pub packages: Vec<String>,
    pub files: Vec<FileEntry>,
    pub commands: Vec<CommandEntry>,
    pub assertions: Vec<String>,
    pub fonts: Vec<String>,
    pub features: Vec<String>,
    pub environment: Vec<String>,
    /// Filesystem path to the workload.yaml file
    pub path: String,
}

/// Number of tabs in the detail view
const TAB_COUNT: usize = 6;

/// Tab indices
const TAB_OVERVIEW: usize = 0;
const TAB_PACKAGES: usize = 1;
const TAB_FILES: usize = 2;
const TAB_COMMANDS: usize = 3;
const TAB_CONFIG: usize = 4;
const TAB_ASSERTIONS: usize = 5;

/// The result of running the detail view
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailOutcome {
    /// User went back to browser (Esc / Backspace)
    Back,
    /// User quit the app (q / Ctrl+C)
    Quit,
    /// User requested install of this workload (name, path)
    Install(String, String),
    /// User requested dry-run of this workload (name, path)
    DryRun(String, String),
}

/// Interactive detail view state
pub struct DetailView {
    detail: WorkloadDetail,
    active_tab: usize,
    selected: usize,
    scroll_offset: usize,
    quit: bool,
    outcome: Option<DetailOutcome>,
    theme: Theme,
}

impl DetailView {
    /// Create a new detail view for the given workload
    pub fn new(detail: WorkloadDetail) -> Self {
        Self {
            detail,
            active_tab: TAB_OVERVIEW,
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
                self.outcome = Some(DetailOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.outcome = Some(DetailOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Esc | KeyCode::Backspace => {
                self.outcome = Some(DetailOutcome::Back);
                self.quit = true;
            }
            // Tab navigation with ←/→
            KeyCode::Left | KeyCode::Char('h') => {
                if self.active_tab > 0 {
                    self.active_tab -= 1;
                    self.selected = 0;
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.active_tab < TAB_COUNT - 1 {
                    self.active_tab += 1;
                    self.selected = 0;
                    self.scroll_offset = 0;
                }
            }
            // Content scrolling with ↑/↓
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.tab_content_lines().saturating_sub(1);
                if self.selected < max {
                    self.selected += 1;
                }
            }
            KeyCode::Char('i') if key.modifiers == KeyModifiers::NONE => {
                self.outcome = Some(DetailOutcome::Install(
                    self.detail.name.clone(),
                    self.detail.path.clone(),
                ));
                self.quit = true;
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::NONE => {
                self.outcome = Some(DetailOutcome::DryRun(
                    self.detail.name.clone(),
                    self.detail.path.clone(),
                ));
                self.quit = true;
            }
            _ => {}
        }
    }

    /// Whether the view should exit
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Returns the outcome after the view exits
    pub fn outcome(&self) -> DetailOutcome {
        self.outcome.clone().unwrap_or(DetailOutcome::Back)
    }

    /// Number of content lines in the current tab
    pub fn tab_content_lines(&self) -> usize {
        self.build_tab_lines(None).len()
    }

    /// Tab titles with counts
    fn tab_titles(&self) -> Vec<String> {
        vec![
            "Overview".to_string(),
            format!("Packages ({})", self.detail.packages.len()),
            format!("Files ({})", self.detail.files.len()),
            format!("Commands ({})", self.detail.commands.len()),
            "Config".to_string(),
            format!("Assertions ({})", self.detail.assertions.len()),
        ]
    }

    /// Build content lines for the active tab
    fn build_tab_lines(&self, highlight_style: Option<Style>) -> Vec<Line<'_>> {
        match self.active_tab {
            TAB_OVERVIEW => self.build_overview_lines(highlight_style),
            TAB_PACKAGES => self.build_list_lines(&self.detail.packages, highlight_style),
            TAB_FILES => self.build_files_lines(highlight_style),
            TAB_COMMANDS => self.build_commands_lines(highlight_style),
            TAB_CONFIG => self.build_config_lines(highlight_style),
            TAB_ASSERTIONS => self.build_list_lines(&self.detail.assertions, highlight_style),
            _ => vec![],
        }
    }

    fn build_overview_lines(&self, _hl: Option<Style>) -> Vec<Line<'_>> {
        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Description: ", self.theme.dimmed()),
                Span::styled(&self.detail.description, self.theme.body()),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Version:     ", self.theme.dimmed()),
                Span::styled(&self.detail.version, self.theme.normal()),
            ]),
            Line::raw(""),
        ];

        if !self.detail.extends.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    SectionKind::Inheritance.icon(),
                    Style::default().fg(self.theme.section_color(SectionKind::Inheritance)),
                ),
                Span::styled("  Extends: ", self.theme.dimmed()),
                Span::styled(self.detail.extends.join(", "), self.theme.normal()),
            ]));
            lines.push(Line::raw(""));
        }

        // Content breakdown
        let sections = [
            (SectionKind::Packages, self.detail.packages.len()),
            (SectionKind::Files, self.detail.files.len()),
            (SectionKind::Commands, self.detail.commands.len()),
            (SectionKind::Fonts, self.detail.fonts.len()),
            (SectionKind::Features, self.detail.features.len()),
            (SectionKind::Environment, self.detail.environment.len()),
            (SectionKind::Assertions, self.detail.assertions.len()),
        ];

        for (kind, count) in &sections {
            if *count > 0 {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        kind.icon(),
                        Style::default().fg(self.theme.section_color(*kind)),
                    ),
                    Span::styled(
                        format!("  {:12} {}", kind.label(), count),
                        self.theme.normal(),
                    ),
                ]));
            }
        }

        lines
    }

    fn build_list_lines<'a>(
        &'a self,
        items: &'a [String],
        highlight_style: Option<Style>,
    ) -> Vec<Line<'a>> {
        if items.is_empty() {
            return vec![Line::styled("  No items", self.theme.dimmed())];
        }
        items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if let Some(hl) = highlight_style.filter(|_| i == self.selected) {
                    hl
                } else {
                    self.theme.normal()
                };
                Line::from(vec![
                    if Some(i) == Some(self.selected) && highlight_style.is_some() {
                        Span::styled("▌", Style::default().fg(self.theme.accent))
                    } else {
                        Span::raw(" ")
                    },
                    Span::styled(format!(" {}", item), style),
                ])
            })
            .collect()
    }

    fn build_files_lines(&self, highlight_style: Option<Style>) -> Vec<Line<'_>> {
        if self.detail.files.is_empty() {
            return vec![Line::styled("  No files", self.theme.dimmed())];
        }
        self.detail
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let style = if let Some(hl) = highlight_style.filter(|_| i == self.selected) {
                    hl
                } else {
                    self.theme.normal()
                };
                Line::from(vec![
                    if Some(i) == Some(self.selected) && highlight_style.is_some() {
                        Span::styled("▌", Style::default().fg(self.theme.accent))
                    } else {
                        Span::raw(" ")
                    },
                    Span::styled(format!(" {} → {}", f.source, f.destination), style),
                ])
            })
            .collect()
    }

    fn build_commands_lines(&self, highlight_style: Option<Style>) -> Vec<Line<'_>> {
        if self.detail.commands.is_empty() {
            return vec![Line::styled("  No commands", self.theme.dimmed())];
        }
        self.detail
            .commands
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if let Some(hl) = highlight_style.filter(|_| i == self.selected) {
                    hl
                } else {
                    self.theme.normal()
                };
                Line::from(vec![
                    if Some(i) == Some(self.selected) && highlight_style.is_some() {
                        Span::styled("▌", Style::default().fg(self.theme.accent))
                    } else {
                        Span::raw(" ")
                    },
                    Span::styled(format!(" [{}] {}", c.phase, c.name), style),
                ])
            })
            .collect()
    }

    fn build_config_lines(&self, _hl: Option<Style>) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        // Fonts section
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                SectionKind::Fonts.icon(),
                Style::default().fg(self.theme.section_color(SectionKind::Fonts)),
            ),
            Span::styled(
                format!("  Fonts ({})", self.detail.fonts.len()),
                self.theme.title_style(),
            ),
        ]));
        for font in &self.detail.fonts {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(font.as_str(), self.theme.normal()),
            ]));
        }
        if self.detail.fonts.is_empty() {
            lines.push(Line::styled("    None", self.theme.dimmed()));
        }
        lines.push(Line::raw(""));

        // Features section
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                SectionKind::Features.icon(),
                Style::default().fg(self.theme.section_color(SectionKind::Features)),
            ),
            Span::styled(
                format!("  Features ({})", self.detail.features.len()),
                self.theme.title_style(),
            ),
        ]));
        for feat in &self.detail.features {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(feat.as_str(), self.theme.normal()),
            ]));
        }
        if self.detail.features.is_empty() {
            lines.push(Line::styled("    None", self.theme.dimmed()));
        }
        lines.push(Line::raw(""));

        // Environment section
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                SectionKind::Environment.icon(),
                Style::default().fg(self.theme.section_color(SectionKind::Environment)),
            ),
            Span::styled(
                format!("  Environment ({})", self.detail.environment.len()),
                self.theme.title_style(),
            ),
        ]));
        for env in &self.detail.environment {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(env.as_str(), self.theme.normal()),
            ]));
        }
        if self.detail.environment.is_empty() {
            lines.push(Line::styled("    None", self.theme.dimmed()));
        }

        lines
    }

    /// Render the full view
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // branded header
            Constraint::Length(1), // tabs
            Constraint::Length(2), // description + path strip
            Constraint::Min(3),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_branded_header(frame, chunks[0]);
        self.render_tabs(frame, chunks[1]);
        self.render_info_strip(frame, chunks[2]);
        self.render_main(frame, chunks[3]);
        self.render_keyhints(frame, chunks[4]);
    }

    /// Render the branded header bar with breadcrumb
    fn render_branded_header(&self, frame: &mut Frame, area: Rect) {
        render_header(
            frame,
            area,
            &self.theme,
            &["Workloads", &self.detail.name],
            Some(Line::from(vec![Span::styled(
                format!("v{}", self.detail.version),
                self.theme.dimmed(),
            )])),
        );
    }

    /// Render the tabs bar
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles = self.tab_titles();
        let tabs = Tabs::new(titles)
            .select(self.active_tab)
            .style(self.theme.dimmed())
            .highlight_style(self.theme.brand_style())
            .divider(Span::styled("·", self.theme.faint_style()));
        frame.render_widget(tabs, area);
    }

    /// Render the description + path info strip
    fn render_info_strip(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(vec![
                Span::raw(" "),
                Span::styled(&self.detail.description, self.theme.body()),
            ]),
            Line::from(vec![
                Span::raw(" "),
                Span::styled(&self.detail.path, self.theme.running_style()),
            ]),
        ];
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }

    /// Render the main content area for the active tab
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let highlight = self.theme.selection().bg(self.theme.bg_inset);
        let lines = self.build_tab_lines(Some(highlight));

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
                key: "←/→",
                desc: "tabs",
            },
            KeyHint {
                key: "↑/↓",
                desc: "scroll",
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

    /// Run the detail view event loop using an existing TUI session.
    pub fn run(&mut self, tui: &mut Tui) -> anyhow::Result<DetailOutcome> {
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

/// Run the interactive detail view event loop
pub fn run_detail_view(detail: WorkloadDetail) -> anyhow::Result<DetailOutcome> {
    let mut tui = Tui::new()?;
    let mut view = DetailView::new(detail);
    let outcome = view.run(&mut tui)?;
    tui.restore()?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_detail() -> WorkloadDetail {
        WorkloadDetail {
            name: "test-workload".to_string(),
            version: "1.0.0".to_string(),
            description: "A test workload".to_string(),
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
            fonts: vec!["Cascadia Code NF v2407.24".to_string()],
            features: vec!["Windows Sudo (registry_toggle)".to_string()],
            environment: vec!["GIT_EDITOR=code --wait".to_string()],
            path: "/workloads/test-workload/workload.yaml".to_string(),
        }
    }

    #[test]
    fn test_detail_initial_state() {
        let view = DetailView::new(sample_detail());
        assert_eq!(view.selected, 0);
        assert_eq!(view.active_tab, TAB_OVERVIEW);
        assert!(!view.should_quit());
        assert!(view.tab_content_lines() > 0);
    }

    #[test]
    fn test_detail_navigation() {
        let mut view = DetailView::new(sample_detail());
        let max = view.tab_content_lines().saturating_sub(1);

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
    fn test_detail_tab_switching() {
        let mut view = DetailView::new(sample_detail());
        assert_eq!(view.active_tab, TAB_OVERVIEW);

        // Move right to Packages tab
        view.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(view.active_tab, TAB_PACKAGES);
        assert_eq!(view.selected, 0); // reset on tab change

        // Move right to Files tab
        view.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(view.active_tab, TAB_FILES);

        // Move left back to Packages
        view.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(view.active_tab, TAB_PACKAGES);

        // Move left back to Overview
        view.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(view.active_tab, TAB_OVERVIEW);

        // Can't go below 0
        view.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(view.active_tab, TAB_OVERVIEW);

        // Navigate to last tab
        for _ in 0..TAB_COUNT + 2 {
            view.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        }
        assert_eq!(view.active_tab, TAB_COUNT - 1);
    }

    #[test]
    fn test_detail_quit() {
        let mut view = DetailView::new(sample_detail());

        // q quits
        assert!(!view.should_quit());
        view.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(view.should_quit());
        assert_eq!(view.outcome(), DetailOutcome::Quit);

        // Ctrl+C also quits
        let mut view2 = DetailView::new(sample_detail());
        assert!(!view2.should_quit());
        view2.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(view2.should_quit());
        assert_eq!(view2.outcome(), DetailOutcome::Quit);

        // Esc goes back
        let mut view3 = DetailView::new(sample_detail());
        assert!(!view3.should_quit());
        view3.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(view3.should_quit());
        assert_eq!(view3.outcome(), DetailOutcome::Back);

        // Backspace also goes back
        let mut view4 = DetailView::new(sample_detail());
        view4.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(view4.outcome(), DetailOutcome::Back);
    }

    #[test]
    fn test_detail_install_key() {
        let mut view = DetailView::new(sample_detail());

        view.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(view.should_quit());
        assert_eq!(
            view.outcome(),
            DetailOutcome::Install(
                "test-workload".to_string(),
                "/workloads/test-workload/workload.yaml".to_string(),
            )
        );
    }

    #[test]
    fn test_detail_dryrun_key() {
        let mut view = DetailView::new(sample_detail());

        view.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(view.should_quit());
        assert_eq!(
            view.outcome(),
            DetailOutcome::DryRun(
                "test-workload".to_string(),
                "/workloads/test-workload/workload.yaml".to_string(),
            )
        );
    }
}
