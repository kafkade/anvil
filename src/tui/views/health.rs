//! Interactive health report viewer
//!
//! This view renders a full-screen TUI showing the results of
//! `anvil health` with collapsible sections and expandable item details.

use std::collections::HashSet;
use std::sync::mpsc;
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

/// The result of running the health view
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HealthOutcome {
    /// User went back to browser (Esc / Backspace)
    Back,
    /// User quit the app (q / Ctrl+C)
    Quit,
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
    outcome: Option<HealthOutcome>,
    /// Log messages from the health check process
    logs: Vec<String>,
    /// Whether the health check is still running
    checking: bool,
    /// Current checking phase label
    checking_phase: String,
    /// Items checked so far / total expected
    checked_count: usize,
    checked_total: usize,
    /// Animation tick
    tick: usize,
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
            outcome: None,
            logs: Vec::new(),
            checking: false,
            checking_phase: String::new(),
            checked_count: 0,
            checked_total: 0,
            tick: 0,
            theme: Theme::dark(),
        }
    }

    /// Create a new health viewer in "checking" mode (no results yet)
    pub fn new_checking(workload_name: String) -> Self {
        Self {
            report: HealthReport {
                workload_name,
                sections: Vec::new(),
                duration: Duration::ZERO,
            },
            selected: 0,
            expanded: HashSet::new(),
            section_collapsed: HashSet::new(),
            filter_failures: false,
            scroll_offset: 0,
            quit: false,
            outcome: None,
            logs: Vec::new(),
            checking: true,
            checking_phase: "Starting...".to_string(),
            checked_count: 0,
            checked_total: 0,
            tick: 0,
            theme: Theme::dark(),
        }
    }

    /// Process a health event, updating state
    pub fn handle_health_event(&mut self, event: crate::tui::events::HealthEvent) {
        use crate::tui::events::HealthEvent;
        match event {
            HealthEvent::PhaseStart { phase, total } => {
                self.checking_phase = format!("Checking {}...", phase.label());
                self.checked_total += total;
                self.logs
                    .push(format!("Phase: {} ({} items)", phase.label(), total));
            }
            HealthEvent::ItemComplete {
                phase: _,
                name,
                passed,
                message,
            } => {
                self.checked_count += 1;
                let icon = if passed { "✓" } else { "✗" };
                let mut log = format!("{} {}", icon, name);
                if let Some(msg) = &message {
                    log.push_str(&format!(" — {}", msg));
                }
                self.logs.push(log);

                // Keep log size bounded
                if self.logs.len() > 200 {
                    self.logs.remove(0);
                }
            }
            HealthEvent::PhaseComplete { phase } => {
                self.logs.push(format!("✓ {} complete", phase.label()));
            }
            HealthEvent::Log { message } => {
                self.logs.push(message);
                if self.logs.len() > 200 {
                    self.logs.remove(0);
                }
            }
            HealthEvent::Done { duration } => {
                self.checking = false;
                self.report.duration = duration;
                self.logs.push(format!(
                    "Health check complete in {:.1}s",
                    duration.as_secs_f64()
                ));
            }
        }
    }

    /// Set the full report (called when health check completes)
    pub fn set_report(&mut self, report: HealthReport) {
        self.report = report;
        self.checking = false;
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                self.outcome = Some(HealthOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.outcome = Some(HealthOutcome::Quit);
                self.quit = true;
            }
            KeyCode::Esc | KeyCode::Backspace => {
                self.outcome = Some(HealthOutcome::Back);
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

    /// Returns the outcome after the view exits
    pub fn outcome(&self) -> HealthOutcome {
        self.outcome.clone().unwrap_or(HealthOutcome::Back)
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
        if self.checking {
            self.render_checking(frame);
        } else {
            self.render_results(frame);
        }
    }

    /// Render the "checking in progress" view with progress + logs
    fn render_checking(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // branded header
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // progress bar
            Constraint::Length(1), // phase label
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // hairline divider
            Constraint::Min(3),    // log pane
            Constraint::Length(1), // key hints
        ])
        .split(area);

        // Header
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_frames[self.tick % spinner_frames.len()];
        render_header(
            frame,
            chunks[0],
            &self.theme,
            &["Health", &self.report.workload_name],
            Some(Line::from(vec![Span::styled(
                format!("{} Checking...", spinner),
                self.theme.running_style(),
            )])),
        );

        // Progress bar
        let ratio = if self.checked_total > 0 {
            self.checked_count as f64 / self.checked_total as f64
        } else {
            0.0
        };
        let pct = (ratio * 100.0) as u16;
        let gauge_w = chunks[2].width.saturating_sub(30) as usize;
        let filled = (ratio * gauge_w as f64) as usize;
        let empty = gauge_w.saturating_sub(filled);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled("█".repeat(filled), self.theme.running_style()),
                Span::styled("░".repeat(empty), self.theme.faint_style()),
                Span::styled(
                    format!(
                        "  {}/{} checks  {}%",
                        self.checked_count, self.checked_total, pct
                    ),
                    self.theme.dimmed(),
                ),
            ])),
            chunks[2],
        );

        // Phase label
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&self.checking_phase, self.theme.running_style()),
            ])),
            chunks[3],
        );

        // Hairline
        self.render_hairline(frame, chunks[5]);

        // Log pane
        let log_area = chunks[6];
        let visible_height = log_area.height as usize;
        let log_start = self.logs.len().saturating_sub(visible_height);
        let lines: Vec<Line> = self.logs[log_start..]
            .iter()
            .map(|log| {
                let style = if log.starts_with('✓') {
                    self.theme.success_style()
                } else if log.starts_with('✗') {
                    self.theme.error_style()
                } else {
                    self.theme.dimmed()
                };
                Line::styled(format!("  {}", log), style)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), log_area);

        // Key hints (minimal during check)
        render_keyhints(
            frame,
            chunks[7],
            &[KeyHint {
                key: "q",
                desc: "cancel",
            }],
            &self.theme,
        );
    }

    /// Render the results view (after checking completes)
    fn render_results(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // branded header
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // summary gauge bar
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // hairline divider
            Constraint::Min(3),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_branded_header(frame, chunks[0]);
        self.render_summary(frame, chunks[2]);
        self.render_hairline(frame, chunks[4]);
        self.render_main(frame, chunks[5]);
        self.render_keyhints(frame, chunks[6]);
    }

    /// Render the branded header bar
    fn render_branded_header(&self, frame: &mut Frame, area: Rect) {
        let (_, pass, _, _, _) = self.count_statuses();
        let total_items: usize = self.report.sections.iter().map(|s| s.items.len()).sum();
        let pct = if total_items > 0 {
            (pass as f64 / total_items as f64 * 100.0) as u16
        } else {
            100
        };
        let pct_style = if pct >= 80 {
            self.theme.success_style()
        } else if pct >= 50 {
            self.theme.warning_style()
        } else {
            self.theme.error_style()
        };

        render_header(
            frame,
            area,
            &self.theme,
            &["Health", &self.report.workload_name],
            Some(Line::from(vec![Span::styled(
                format!("{}%", pct),
                pct_style,
            )])),
        );
    }

    /// Render the summary gauge bar
    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let (total, pass, fail, _, _) = self.count_statuses();
        let ratio = if total > 0 {
            pass as f64 / total as f64
        } else {
            1.0
        };
        let pct = (ratio * 100.0) as u16;

        // Build gauge bar
        let gauge_w = 16usize;
        let filled = (ratio * gauge_w as f64) as usize;
        let empty = gauge_w.saturating_sub(filled);
        let bar_filled = "█".repeat(filled);
        let bar_empty = "░".repeat(empty);

        let issues_text = format!("{} ISSUES", fail);
        let issues_badge = if fail > 0 {
            badge(&issues_text, self.theme.error)
        } else {
            badge("ALL PASS", self.theme.success)
        };

        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(&bar_filled, self.theme.success_style()),
            Span::styled(&bar_empty, self.theme.faint_style()),
            Span::raw("  "),
            Span::styled(
                format!("{}%", pct),
                if pct >= 100 {
                    self.theme.success_style()
                } else {
                    self.theme.normal()
                },
            ),
            Span::styled("  │  ", self.theme.faint_style()),
            Span::styled(format!("{} pass", pass), self.theme.success_style()),
            Span::raw("  "),
            Span::styled(format!("{} fail", fail), self.theme.error_style()),
            Span::raw("    "),
            issues_badge,
            Span::raw("  "),
            Span::styled(
                format!("{:.1}s", self.report.duration.as_secs_f64()),
                self.theme.dimmed(),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    /// Render a hairline divider
    fn render_hairline(&self, frame: &mut Frame, area: Rect) {
        let divider_text = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                divider_text,
                Style::default().fg(self.theme.hairline),
            ))),
            area,
        );
    }

    /// Render the main content area with sections and items
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        let mut flat_index: usize = 0;
        let content_width = area.width as usize;

        for (si, section) in self.report.sections.iter().enumerate() {
            let is_collapsed = self.section_collapsed.contains(&si);
            let (section_pass, section_total) = self.section_counts(section);
            let is_selected = flat_index == self.selected;

            // Build heatmap data for this section
            let heatmap_results: Vec<bool> = section
                .items
                .iter()
                .map(|i| i.status == HealthStatus::Pass)
                .collect();

            // Section header with heatmap + right-aligned count
            let arrow = if is_collapsed { "▸" } else { "▾" };
            let count_text = format!("{}/{}", section_pass, section_total);
            let count_style = if section_pass == section_total {
                self.theme.success_style()
            } else {
                self.theme.error_style()
            };

            let heatmap_spans: Vec<Span> = heatmap_results
                .iter()
                .take(section_total.min(30))
                .map(|&pass| {
                    let c = if pass {
                        self.theme.success
                    } else {
                        self.theme.error
                    };
                    Span::styled("▮", Style::default().fg(c))
                })
                .collect();

            // Calculate padding for right-aligned count
            let header_prefix_len = 4 + section.name.len() + 1; // "  ▾ Name "
            let heatmap_len = heatmap_results.len().min(30);
            let used = header_prefix_len + heatmap_len + 1 + count_text.len();
            let pad = content_width.saturating_sub(used);

            if is_selected {
                let mut spans = vec![
                    Span::styled("▌", Style::default().fg(self.theme.accent)),
                    Span::styled(
                        format!(" {} {} ", arrow, section.name),
                        self.theme
                            .selection()
                            .add_modifier(Modifier::BOLD)
                            .bg(self.theme.bg_inset),
                    ),
                ];
                spans.extend(heatmap_spans);
                spans.push(Span::raw(" ".repeat(pad)));
                spans.push(Span::styled(count_text, count_style));
                lines.push(Line::from(spans));
            } else {
                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{} {} ", arrow, section.name),
                        self.theme.normal().add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(heatmap_spans);
                spans.push(Span::raw(" ".repeat(pad)));
                spans.push(Span::styled(count_text, count_style));
                lines.push(Line::from(spans));
            }
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

                    // Build the item line with right-aligned error text
                    let name_len = item.name.len();
                    let detail_text = if item.status == HealthStatus::Fail {
                        item.detail.as_deref().unwrap_or("")
                    } else {
                        ""
                    };
                    let affordance = if item.status == HealthStatus::Fail && item.detail.is_some() {
                        if is_expanded {
                            "▾"
                        } else {
                            "▸"
                        }
                    } else {
                        ""
                    };

                    // Truncate detail if needed
                    let prefix_len = 6 + name_len; // indent + icon + space + name
                    let detail_max =
                        content_width.saturating_sub(prefix_len + 4 + affordance.len());
                    let detail_display: String = if detail_text.len() > detail_max && detail_max > 3
                    {
                        format!("{}…", &detail_text[..detail_max.saturating_sub(1)])
                    } else {
                        detail_text.to_string()
                    };

                    let detail_pad = content_width
                        .saturating_sub(prefix_len + detail_display.len() + affordance.len() + 2);

                    if is_item_selected {
                        let mut spans = vec![
                            Span::raw("  "),
                            Span::styled("▌", Style::default().fg(self.theme.accent)),
                            Span::styled(format!(" {} ", icon), icon_style.bg(self.theme.bg_inset)),
                            Span::styled(
                                item.name.clone(),
                                self.theme.selection().bg(self.theme.bg_inset),
                            ),
                        ];
                        if !detail_display.is_empty() {
                            spans.push(Span::raw(" ".repeat(detail_pad)));
                            spans.push(Span::styled(detail_display, self.theme.error_style()));
                            spans.push(Span::styled(affordance, self.theme.error_style()));
                        }
                        lines.push(Line::from(spans));
                    } else {
                        let mut spans = vec![
                            Span::raw("    "),
                            Span::styled(format!("{} ", icon), icon_style),
                            Span::styled(item.name.clone(), self.theme.normal()),
                        ];
                        if !detail_display.is_empty() {
                            spans.push(Span::raw(" ".repeat(detail_pad)));
                            spans.push(Span::styled(detail_display, self.theme.error_style()));
                            spans.push(Span::styled(affordance, self.theme.error_style()));
                        }
                        lines.push(Line::from(spans));
                    }

                    // Expanded error detail — bg_inset box
                    if is_expanded {
                        if let Some(detail) = &item.detail {
                            lines.push(Line::raw(""));
                            // Wrap detail in a padded bg_inset block
                            let box_width = content_width.saturating_sub(8);
                            let mut remaining = detail.as_str();
                            while !remaining.is_empty() {
                                let chunk_len = remaining.len().min(box_width);
                                let split = if chunk_len < remaining.len() {
                                    remaining[..chunk_len].rfind(' ').unwrap_or(chunk_len)
                                } else {
                                    chunk_len
                                };
                                let chunk = &remaining[..split];
                                let pad = box_width.saturating_sub(chunk.len());
                                lines.push(Line::from(vec![
                                    Span::raw("      "),
                                    Span::styled(
                                        format!(" {}{} ", chunk, " ".repeat(pad)),
                                        self.theme.error_style().bg(self.theme.bg_inset),
                                    ),
                                ]));
                                remaining = remaining[split..].trim_start();
                            }
                            lines.push(Line::raw(""));
                        }
                    }

                    flat_index += 1;
                }
            }

            // Blank line between sections
            lines.push(Line::raw(""));
        }

        // Apply scroll offset
        let visible_height = area.height as usize;
        let max_scroll = lines.len().saturating_sub(visible_height);
        let scroll = self.scroll_offset.min(max_scroll);
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(scroll)
            .take(visible_height)
            .collect();

        let paragraph = Paragraph::new(visible_lines);
        frame.render_widget(paragraph, area);

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
                key: "↑↓",
                desc: "navigate",
            },
            KeyHint {
                key: "↵",
                desc: "expand error",
            },
            KeyHint {
                key: "f",
                desc: filter_desc,
            },
            KeyHint {
                key: "r",
                desc: "re-run",
            },
            KeyHint {
                key: "Esc",
                desc: "back",
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
pub fn run_health_viewer(report: HealthReport) -> anyhow::Result<HealthOutcome> {
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
    Ok(viewer.outcome())
}

/// Run the health viewer with live events from a channel.
///
/// Shows a progress view while the health check runs, then transitions
/// to the results view when done. Returns whether the user went back or quit.
#[allow(dead_code)]
pub fn run_health_with_events(
    workload_name: String,
    rx: mpsc::Receiver<crate::tui::events::HealthEvent>,
    report_rx: mpsc::Receiver<HealthReport>,
) -> anyhow::Result<HealthOutcome> {
    let mut tui = Tui::new()?;
    let mut viewer = HealthViewer::new_checking(workload_name);

    let tick_rate = Duration::from_millis(100);

    loop {
        // Drain pending health events
        loop {
            match rx.try_recv() {
                Ok(event) => viewer.handle_health_event(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Check if we got a final report
                    if let Ok(report) = report_rx.try_recv() {
                        viewer.set_report(report);
                    } else if viewer.checking {
                        viewer.checking = false;
                    }
                    break;
                }
            }
        }

        // Check for final report
        if let Ok(report) = report_rx.try_recv() {
            viewer.set_report(report);
        }

        viewer.tick += 1;
        tui.draw(|f| viewer.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            viewer.handle_key(key);

            if viewer.should_quit() {
                break;
            }
        }
    }

    tui.restore()?;
    Ok(viewer.outcome())
}

/// Run the health viewer inline within an existing TUI session.
///
/// Used when launched from the browser — keeps the same alternate screen.
pub fn run_health_inline(
    tui: &mut Tui,
    workload_name: String,
    rx: mpsc::Receiver<crate::tui::events::HealthEvent>,
    report_rx: mpsc::Receiver<HealthReport>,
) -> anyhow::Result<HealthOutcome> {
    let mut viewer = HealthViewer::new_checking(workload_name);
    let tick_rate = Duration::from_millis(100);

    loop {
        // Drain pending health events
        loop {
            match rx.try_recv() {
                Ok(event) => viewer.handle_health_event(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if let Ok(report) = report_rx.try_recv() {
                        viewer.set_report(report);
                    } else if viewer.checking {
                        viewer.checking = false;
                    }
                    break;
                }
            }
        }

        if let Ok(report) = report_rx.try_recv() {
            viewer.set_report(report);
        }

        viewer.tick += 1;
        tui.draw(|f| viewer.render(f))?;

        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            viewer.handle_key(key);

            if viewer.should_quit() {
                break;
            }
        }
    }

    Ok(viewer.outcome())
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
