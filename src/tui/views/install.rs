//! Interactive installation progress dashboard
//!
//! This view renders a full-screen TUI showing real-time
//! installation progress across all phases.

use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::events::{InstallEvent, InstallPhase, ItemResult};
use crate::tui::theme::Theme;
use crate::tui::widgets::chrome::render_header;
use crate::tui::widgets::keyhints::{render_keyhints, KeyHint};
use crate::tui::widgets::status::{status_line, ItemStatus};
use crate::tui::Tui;

/// State for a single item within a phase
#[derive(Debug, Clone)]
struct ItemState {
    name: String,
    status: ItemStatus,
    message: Option<String>,
}

/// State for a single install phase
#[derive(Debug, Clone)]
struct PhaseState {
    phase: InstallPhase,
    status: ItemStatus,
    items: Vec<ItemState>,
    total: usize,
    completed: usize,
}

/// The install dashboard state
pub struct InstallDashboard {
    workload_name: String,
    phases: Vec<PhaseState>,
    scroll_offset: usize,
    verbose: bool,
    logs: Vec<String>,
    done: bool,
    quit_requested: bool,
    summary_text: String,
    total_duration: Option<Duration>,
    tick: usize,
    theme: Theme,
}

impl InstallDashboard {
    /// Create a new dashboard for the given workload
    pub fn new(workload_name: String) -> Self {
        Self {
            workload_name,
            phases: Vec::new(),
            scroll_offset: 0,
            verbose: false,
            logs: Vec::new(),
            done: false,
            quit_requested: false,
            summary_text: String::new(),
            total_duration: None,
            tick: 0,
            theme: Theme::dark(),
        }
    }

    /// Process an install event, updating state
    pub fn handle_event(&mut self, event: InstallEvent) {
        match event {
            InstallEvent::PhaseStart { phase, total } => {
                self.phases.push(PhaseState {
                    phase,
                    status: ItemStatus::Running,
                    items: Vec::new(),
                    total,
                    completed: 0,
                });
            }

            InstallEvent::ItemStart { phase, name } => {
                if let Some(ps) = self.find_phase_mut(phase) {
                    ps.items.push(ItemState {
                        name,
                        status: ItemStatus::Running,
                        message: None,
                    });
                }
            }

            InstallEvent::ItemComplete {
                phase,
                name,
                result,
                message,
            } => {
                if let Some(ps) = self.find_phase_mut(phase) {
                    let status = match result {
                        ItemResult::Success => ItemStatus::Success,
                        ItemResult::Skipped => ItemStatus::Skipped,
                        ItemResult::Failed => ItemStatus::Failed,
                    };

                    if let Some(item) = ps.items.iter_mut().find(|i| i.name == name) {
                        item.status = status;
                        if !message.is_empty() {
                            item.message = Some(message);
                        }
                    }
                    ps.completed += 1;
                }
            }

            InstallEvent::PhaseComplete { phase } => {
                if let Some(ps) = self.find_phase_mut(phase) {
                    ps.status = ItemStatus::Success;
                }
            }

            InstallEvent::Log { message } => {
                self.logs.push(message);
                // Auto-scroll log to bottom
                if self.logs.len() > 100 {
                    self.logs.remove(0);
                }
            }

            InstallEvent::Done {
                success: _,
                summary,
                duration,
            } => {
                self.done = true;
                self.summary_text = summary;
                self.total_duration = Some(duration);
            }
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                self.quit_requested = true;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.quit_requested = true;
            }
            KeyCode::Char('v') if key.modifiers == KeyModifiers::NONE => {
                self.verbose = !self.verbose;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            _ => {}
        }
    }

    /// Should the TUI exit?
    pub fn should_quit(&self) -> bool {
        self.quit_requested
    }

    /// Is the install done?
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Render the dashboard
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // branded header
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // overall gauge
            Constraint::Length(1), // blank padding
            Constraint::Length(1), // phase chips
            Constraint::Length(1), // hairline divider
            Constraint::Min(3),    // main content (split body)
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_branded_header(frame, chunks[0]);
        self.render_gauge(frame, chunks[2]);
        self.render_phase_chips(frame, chunks[4]);
        self.render_hairline(frame, chunks[5]);
        self.render_main(frame, chunks[6]);
        self.render_keyhints(frame, chunks[7]);
    }

    /// Render the branded header bar
    fn render_branded_header(&self, frame: &mut Frame, area: Rect) {
        let status_text = if self.done { "Complete" } else { "Installing" };

        let mut right_spans = vec![];
        if let Some(dur) = self.total_duration {
            right_spans.push(Span::styled(
                format!("⏱ {}", crate::cli::progress::format_duration(dur)),
                self.theme.dimmed(),
            ));
        } else if !self.done {
            // Show elapsed estimate
            right_spans.push(Span::styled(status_text, self.theme.running_style()));
        } else {
            right_spans.push(Span::styled(status_text, self.theme.success_style()));
        }

        render_header(
            frame,
            area,
            &self.theme,
            &[status_text, &self.workload_name],
            Some(Line::from(right_spans)),
        );
    }

    /// Render the overall progress gauge
    fn render_gauge(&self, frame: &mut Frame, area: Rect) {
        let (completed, total) = self.overall_progress();
        let ratio = if total > 0 {
            completed as f64 / total as f64
        } else {
            0.0
        };
        let pct = (ratio * 100.0) as u16;
        let label = format!("{} / {} packages", completed, total);

        // Build: "  Overall  ████░░░░░░░  label  pct%"
        let gauge_width = area.width.saturating_sub(30) as usize;
        let filled = (ratio * gauge_width as f64) as usize;
        let empty = gauge_width.saturating_sub(filled);
        let bar_filled = "█".repeat(filled);
        let bar_empty = "░".repeat(empty);

        let line = Line::from(vec![
            Span::styled("  Overall  ", self.theme.dimmed()),
            Span::styled(bar_filled, self.theme.error_style()),
            Span::styled(bar_empty, self.theme.faint_style()),
            Span::styled(format!("  {}  ", label), self.theme.normal()),
            Span::styled(format!("{}%", pct), self.theme.dimmed()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    /// Render phase indicator chips
    fn render_phase_chips(&self, frame: &mut Frame, area: Rect) {
        let all_phases = [
            InstallPhase::PreCommands,
            InstallPhase::Packages,
            InstallPhase::Fonts,
            InstallPhase::Features,
            InstallPhase::Terminal,
            InstallPhase::Files,
            InstallPhase::PostCommands,
        ];

        let mut spans: Vec<Span> = vec![Span::raw(" ")];
        for (idx, phase) in all_phases.iter().enumerate() {
            let state = self.phases.iter().find(|p| p.phase == *phase);
            let (icon, style) = match state.map(|p| p.status) {
                Some(ItemStatus::Success) => ("✓", self.theme.success_style()),
                Some(ItemStatus::Running) => match self.tick % 10 {
                    0 => ("⠋", self.theme.running_style()),
                    1 => ("⠙", self.theme.running_style()),
                    2 => ("⠹", self.theme.running_style()),
                    3 => ("⠸", self.theme.running_style()),
                    4 => ("⠼", self.theme.running_style()),
                    5 => ("⠴", self.theme.running_style()),
                    6 => ("⠦", self.theme.running_style()),
                    7 => ("⠧", self.theme.running_style()),
                    8 => ("⠇", self.theme.running_style()),
                    _ => ("⠏", self.theme.running_style()),
                },
                _ => ("·", self.theme.faint_style()),
            };
            if idx > 0 {
                spans.push(Span::styled(" · ", self.theme.faint_style()));
            }
            spans.push(Span::styled(format!("{} {}", icon, phase.label()), style));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Render a hairline divider
    fn render_hairline(&self, frame: &mut Frame, area: Rect) {
        let divider_text = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                divider_text,
                ratatui::style::Style::default().fg(self.theme.hairline),
            ))),
            area,
        );
    }

    /// Compute overall completed/total across all phases
    fn overall_progress(&self) -> (usize, usize) {
        let completed: usize = self.phases.iter().map(|p| p.completed).sum();
        let total: usize = self.phases.iter().map(|p| p.total).sum();
        (completed, total)
    }

    /// Render the main content area — always split: items left, log right
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        if self.verbose {
            // Split body: items (60%) + divider + log (40%)
            let body = Layout::horizontal([
                Constraint::Percentage(60),
                Constraint::Length(1),
                Constraint::Percentage(40),
            ])
            .split(area);

            self.render_items(frame, body[0]);

            // Vertical divider
            let divider_lines: Vec<Line> = (0..body[1].height)
                .map(|_| {
                    Line::from(Span::styled(
                        "│",
                        ratatui::style::Style::default().fg(self.theme.hairline),
                    ))
                })
                .collect();
            frame.render_widget(Paragraph::new(divider_lines), body[1]);

            self.render_log(frame, body[2]);
        } else {
            self.render_items(frame, area);
        }
    }

    /// Render the items pane (no bordered block — just content)
    fn render_items(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        let width = area.width;

        if self.done {
            // Show completion summary
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "  Installation complete!",
                self.theme.success_style().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::raw(""));
            for line in self.summary_text.lines() {
                lines.push(Line::raw(format!("  {}", line)));
            }
            if let Some(dur) = self.total_duration {
                lines.push(Line::raw(""));
                lines.push(Line::raw(format!(
                    "  Duration: {}",
                    crate::cli::progress::format_duration(dur)
                )));
            }
            lines.push(Line::raw(""));
            lines.push(Line::styled("  Press q to exit", self.theme.dimmed()));
        } else {
            // Show phases and items
            for ps in &self.phases {
                if ps.status == ItemStatus::Running {
                    // Active phase header with inline mini gauge
                    let pct = ps
                        .total
                        .checked_div(1)
                        .filter(|_| ps.total > 0)
                        .map(|_| (ps.completed as f64 / ps.total as f64 * 100.0) as u16)
                        .unwrap_or(0);
                    let gauge_w = 12usize;
                    let filled = (ps.completed * gauge_w).checked_div(ps.total).unwrap_or(0);
                    let empty = gauge_w.saturating_sub(filled);
                    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(
                                "  {} Installing {} [{}/{}]  ",
                                ItemStatus::Running.symbol(self.tick),
                                ps.phase.label(),
                                ps.completed,
                                ps.total,
                            ),
                            self.theme.running_style(),
                        ),
                        Span::styled(bar, self.theme.running_style()),
                        Span::styled(format!("  {}%", pct), self.theme.dimmed()),
                    ]));
                    lines.push(Line::raw(""));

                    // Items
                    for item in &ps.items {
                        lines.push(status_line(
                            &item.name,
                            item.status,
                            item.message.as_deref(),
                            self.tick,
                            &self.theme,
                            width,
                        ));
                    }

                    // Show pending items count
                    let started = ps.items.len();
                    if started < ps.total {
                        let remaining = ps.total - started;
                        lines.push(Line::raw(""));
                        lines.push(Line::styled(
                            format!("  ... {} more pending", remaining),
                            self.theme.dimmed(),
                        ));
                    }
                } else if ps.status == ItemStatus::Success {
                    // Completed phase — summary line
                    let summary = format!("  ✓ {} ({} items)", ps.phase.label(), ps.completed);
                    lines.push(Line::styled(summary, self.theme.success_style()));
                }
            }
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

    /// Render the log pane with header and auto-scrolling entries
    fn render_log(&self, frame: &mut Frame, area: Rect) {
        // Split: LOG header + content
        let parts = Layout::vertical([
            Constraint::Length(1), // LOG header
            Constraint::Min(1),    // log content
        ])
        .split(area);

        // LOG header
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("  LOG", self.theme.label_style()))),
            parts[0],
        );

        // Log content — auto-scroll to bottom
        let visible_height = parts[1].height as usize;
        let log_start = self.logs.len().saturating_sub(visible_height);
        let lines: Vec<Line> = self.logs[log_start..]
            .iter()
            .map(|log| {
                let style = if log.contains('✓') || log.contains("installed") {
                    self.theme.success_style()
                } else if log.starts_with('$') || log.starts_with('>') || log.contains("$ ") {
                    self.theme.faint_style()
                } else {
                    self.theme.dimmed()
                };
                Line::styled(format!("  {}", log), style)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, parts[1]);
    }

    /// Render the key hints bar
    fn render_keyhints(&self, frame: &mut Frame, area: Rect) {
        let hints = if self.done {
            vec![KeyHint {
                key: "q",
                desc: "exit",
            }]
        } else {
            vec![
                KeyHint {
                    key: "↑↓",
                    desc: "scroll",
                },
                KeyHint {
                    key: "v",
                    desc: "toggle log",
                },
                KeyHint {
                    key: "q",
                    desc: "cancel",
                },
            ]
        };

        render_keyhints(frame, area, &hints, &self.theme);
    }

    fn find_phase_mut(&mut self, phase: InstallPhase) -> Option<&mut PhaseState> {
        self.phases.iter_mut().find(|p| p.phase == phase)
    }
}

/// Run the TUI dashboard, consuming events from the channel
///
/// This function is designed to be called from a spawned thread.
/// It takes ownership of the receiver and runs until Done is received
/// or the channel disconnects.
///
/// Returns whether a quit was requested (to signal cancellation to the install thread).
pub fn run_dashboard(
    workload_name: String,
    rx: mpsc::Receiver<InstallEvent>,
    cancel_tx: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    let mut tui = Tui::new()?;
    let mut dashboard = InstallDashboard::new(workload_name);

    let tick_rate = Duration::from_millis(100);

    loop {
        // Drain all pending install events
        loop {
            match rx.try_recv() {
                Ok(event) => dashboard.handle_event(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !dashboard.is_done() {
                        dashboard.handle_event(InstallEvent::Done {
                            success: false,
                            summary: "Channel disconnected".to_string(),
                            duration: Duration::ZERO,
                        });
                    }
                    break;
                }
            }
        }

        // Render
        dashboard.tick += 1;
        tui.draw(|f| dashboard.render(f))?;

        // Poll for keyboard events
        if let Some(Event::Key(key)) = tui.poll_event(tick_rate)? {
            dashboard.handle_key(key);

            if dashboard.should_quit() {
                if dashboard.is_done() {
                    break;
                }
                // Signal cancellation to install thread
                let _ = cancel_tx.send(());
                // Don't break yet — wait for Done event
            }
        }

        // Exit when done and user pressed quit
        if dashboard.is_done() && dashboard.quit_requested {
            break;
        }

        // Auto-exit after done + brief pause for user to see summary
        if dashboard.is_done() {
            // Wait for user to press a key
            if let Some(Event::Key(_)) = tui.poll_event(tick_rate)? {
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

    #[test]
    fn test_dashboard_initial_state() {
        let dashboard = InstallDashboard::new("test-workload".to_string());
        assert!(!dashboard.is_done());
        assert!(!dashboard.should_quit());
        assert!(!dashboard.verbose);
        assert!(dashboard.phases.is_empty());
    }

    #[test]
    fn test_dashboard_phase_lifecycle() {
        let mut dashboard = InstallDashboard::new("test".to_string());

        // Start a phase
        dashboard.handle_event(InstallEvent::PhaseStart {
            phase: InstallPhase::Packages,
            total: 3,
        });
        assert_eq!(dashboard.phases.len(), 1);
        assert_eq!(dashboard.phases[0].status, ItemStatus::Running);

        // Start items
        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "Package.A".to_string(),
        });
        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "Package.B".to_string(),
        });
        assert_eq!(dashboard.phases[0].items.len(), 2);

        // Complete an item
        dashboard.handle_event(InstallEvent::ItemComplete {
            phase: InstallPhase::Packages,
            name: "Package.A".to_string(),
            result: ItemResult::Success,
            message: "v1.0.0".to_string(),
        });
        assert_eq!(dashboard.phases[0].completed, 1);
        assert_eq!(dashboard.phases[0].items[0].status, ItemStatus::Success);

        // Complete the phase
        dashboard.handle_event(InstallEvent::PhaseComplete {
            phase: InstallPhase::Packages,
        });
        assert_eq!(dashboard.phases[0].status, ItemStatus::Success);
    }

    #[test]
    fn test_dashboard_done_event() {
        let mut dashboard = InstallDashboard::new("test".to_string());

        dashboard.handle_event(InstallEvent::Done {
            success: true,
            summary: "3 installed, 1 skipped".to_string(),
            duration: Duration::from_secs(42),
        });

        assert!(dashboard.is_done());
        assert_eq!(dashboard.summary_text, "3 installed, 1 skipped");
        assert_eq!(dashboard.total_duration, Some(Duration::from_secs(42)));
    }

    #[test]
    fn test_dashboard_keyboard_handling() {
        let mut dashboard = InstallDashboard::new("test".to_string());

        // Toggle verbose
        assert!(!dashboard.verbose);
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(dashboard.verbose);
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(!dashboard.verbose);

        // Scroll
        dashboard.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(dashboard.scroll_offset, 1);
        dashboard.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(dashboard.scroll_offset, 0);

        // Quit
        assert!(!dashboard.should_quit());
        dashboard.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(dashboard.quit_requested);
    }

    #[test]
    fn test_dashboard_log_events() {
        let mut dashboard = InstallDashboard::new("test".to_string());

        dashboard.handle_event(InstallEvent::Log {
            message: "Test log message".to_string(),
        });
        assert_eq!(dashboard.logs.len(), 1);
        assert_eq!(dashboard.logs[0], "Test log message");
    }

    #[test]
    fn test_dashboard_render_in_test_backend() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut dashboard = InstallDashboard::new("rust-developer".to_string());

        // Add some state
        dashboard.handle_event(InstallEvent::PhaseStart {
            phase: InstallPhase::Packages,
            total: 3,
        });
        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "Rustlang.Rustup".to_string(),
        });
        dashboard.handle_event(InstallEvent::ItemComplete {
            phase: InstallPhase::Packages,
            name: "Rustlang.Rustup".to_string(),
            result: ItemResult::Success,
            message: "installed".to_string(),
        });
        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "LLVM.LLVM".to_string(),
        });

        // Should render without panicking
        terminal.draw(|f| dashboard.render(f)).unwrap();
    }

    #[test]
    fn test_dashboard_skipped_and_failed_items() {
        let mut dashboard = InstallDashboard::new("test".to_string());

        dashboard.handle_event(InstallEvent::PhaseStart {
            phase: InstallPhase::Packages,
            total: 3,
        });

        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "Pkg.Skip".to_string(),
        });
        dashboard.handle_event(InstallEvent::ItemComplete {
            phase: InstallPhase::Packages,
            name: "Pkg.Skip".to_string(),
            result: ItemResult::Skipped,
            message: "already installed".to_string(),
        });

        dashboard.handle_event(InstallEvent::ItemStart {
            phase: InstallPhase::Packages,
            name: "Pkg.Fail".to_string(),
        });
        dashboard.handle_event(InstallEvent::ItemComplete {
            phase: InstallPhase::Packages,
            name: "Pkg.Fail".to_string(),
            result: ItemResult::Failed,
            message: "download error".to_string(),
        });

        assert_eq!(dashboard.phases[0].items[0].status, ItemStatus::Skipped);
        assert_eq!(dashboard.phases[0].items[1].status, ItemStatus::Failed);
        assert_eq!(dashboard.phases[0].completed, 2);
    }
}
