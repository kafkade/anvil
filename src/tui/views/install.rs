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
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::events::{InstallEvent, InstallPhase, ItemResult};
use crate::tui::theme::Theme;
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

        // Main layout: content + key hints bar
        let chunks = Layout::vertical([
            Constraint::Min(3),    // main content
            Constraint::Length(1), // key hints
        ])
        .split(area);

        self.render_main(frame, chunks[0]);
        self.render_keyhints(frame, chunks[1]);
    }

    /// Render the main content area
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" Anvil — Installing {} ", self.workload_name);
        let block = Block::default()
            .title(Span::styled(title, self.theme.title_style()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Build content lines
        let mut lines: Vec<Line> = Vec::new();
        let width = inner.width;

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
                lines.push(Line::raw(""));

                if ps.status == ItemStatus::Running {
                    // Active phase — show progress and items
                    // Phase header with progress
                    let phase_line = Line::from(vec![
                        Span::styled(
                            format!("  Phase: {}", ps.phase.label()),
                            self.theme.running_style().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("  [{}/{}]", ps.completed, ps.total)),
                    ]);
                    lines.push(phase_line);

                    // Progress bar area (rendered separately)
                    if ps.total > 0 {
                        lines.push(Line::raw("")); // spacer for gauge
                    }

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

                    // Show pending items count if not all started
                    let started = ps.items.len();
                    if started < ps.total {
                        let remaining = ps.total - started;
                        lines.push(Line::styled(
                            format!("  ... {} more pending", remaining),
                            self.theme.dimmed(),
                        ));
                    }
                } else if ps.status == ItemStatus::Success {
                    // Completed phase — summary line
                    let summary = format!("  ✓ {} ({} items)", ps.phase.label(), ps.completed);
                    lines.push(Line::styled(summary, self.theme.success_style()));
                } else {
                    // Pending phase
                    lines.push(Line::styled(
                        format!("  Phase: {}  pending", ps.phase.label()),
                        self.theme.dimmed(),
                    ));
                }
            }

            // Show future phases as pending
            let active_phases: Vec<InstallPhase> = self.phases.iter().map(|p| p.phase).collect();
            let all_phases = [
                InstallPhase::PreCommands,
                InstallPhase::Packages,
                InstallPhase::Fonts,
                InstallPhase::Files,
                InstallPhase::PostCommands,
            ];
            for phase in &all_phases {
                if !active_phases.contains(phase) {
                    lines.push(Line::styled(
                        format!("  Phase: {}  pending", phase.label()),
                        self.theme.dimmed(),
                    ));
                }
            }

            // Verbose log section
            if self.verbose && !self.logs.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled("  ─── Log ───", self.theme.dimmed()));
                let log_start = self.logs.len().saturating_sub(10);
                for log in &self.logs[log_start..] {
                    lines.push(Line::styled(format!("  {}", log), self.theme.dimmed()));
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
        let hints = if self.done {
            vec![KeyHint {
                key: "q",
                desc: "exit",
            }]
        } else {
            vec![
                KeyHint {
                    key: "↑/↓",
                    desc: "scroll",
                },
                KeyHint {
                    key: "q",
                    desc: "stop after current",
                },
                KeyHint {
                    key: "v",
                    desc: if self.verbose { "hide log" } else { "show log" },
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
