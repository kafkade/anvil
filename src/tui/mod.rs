//! TUI module — Ratatui-based terminal user interface
//!
//! This module provides the interactive TUI framework used for
//! rich terminal dashboards (e.g., installation progress).
//! When stdout/stdin are not a TTY or `--no-tui` is set,
//! the CLI falls back to the existing indicatif-based output.

pub mod events;
pub mod theme;
pub mod views;
pub mod widgets;

use std::io::{self, Stdout};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// Check if the TUI should be activated
///
/// Returns true when both stdout and stdin are TTYs.
pub fn should_use_tui() -> bool {
    atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stdin)
}

/// Terminal wrapper that manages setup/teardown of the raw terminal
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    restored: bool,
}

impl Tui {
    /// Create a new TUI, entering alternate screen and raw mode
    pub fn new() -> Result<Self> {
        // Install panic hook that restores the terminal
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = Self::force_restore();
            original_hook(panic_info);
        }));

        terminal::enable_raw_mode().context("Failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;

        Ok(Self {
            terminal,
            restored: false,
        })
    }

    /// Draw a frame using the provided closure
    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Poll for a crossterm event with timeout
    ///
    /// Returns `None` if no event is available within the timeout.
    pub fn poll_event(&self, timeout: std::time::Duration) -> Result<Option<Event>> {
        if event::poll(timeout).context("Failed to poll for event")? {
            let evt = event::read().context("Failed to read event")?;
            Ok(Some(evt))
        } else {
            Ok(None)
        }
    }

    /// Check if a key event is a quit signal (q or Ctrl+C)
    #[allow(dead_code)]
    pub fn is_quit_key(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    }

    /// Restore the terminal to its original state
    pub fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;

        terminal::disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        self.terminal.show_cursor()?;

        Ok(())
    }

    /// Force-restore the terminal (for use in panic hooks)
    fn force_restore() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_tui_returns_bool() {
        // In test/CI, this may return false — just verify it doesn't panic
        let _ = should_use_tui();
    }

    #[test]
    fn test_is_quit_key() {
        let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(Tui::is_quit_key(&q_key));

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(Tui::is_quit_key(&ctrl_c));

        let other = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(!Tui::is_quit_key(&other));

        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!Tui::is_quit_key(&enter));
    }
}
