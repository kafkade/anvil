//! Progress gauge widget
//!
//! Renders a labeled progress bar for phase progress.

use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Gauge},
    Frame,
};

use crate::tui::theme::Theme;

/// Render a phase progress gauge
#[allow(dead_code)]
pub fn render_progress(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    completed: usize,
    total: usize,
    theme: &Theme,
) {
    if total == 0 {
        return;
    }

    let ratio = completed as f64 / total as f64;
    let percent = (ratio * 100.0) as u16;

    let label_text = format!("  {}  [{}/{}] {}%", label, completed, total, percent);

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(theme.accent)
                .bg(ratatui::style::Color::DarkGray),
        )
        .ratio(ratio)
        .label(Span::raw(label_text));

    frame.render_widget(gauge, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_progress_zero_total() {
        let backend = TestBackend::new(60, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        terminal
            .draw(|f| {
                let area = f.area();
                render_progress(f, area, "Test", 0, 0, &theme);
            })
            .unwrap();
        // Should not panic with zero total
    }

    #[test]
    fn test_render_progress_partial() {
        let backend = TestBackend::new(60, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        terminal
            .draw(|f| {
                let area = f.area();
                render_progress(f, area, "Packages", 3, 5, &theme);
            })
            .unwrap();
    }
}
