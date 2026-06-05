//! Reusable Anvil TUI widgets
//!
//! Small composable helpers shared across views: the branded header bar,
//! status badges, gauges, heatmap strips, and the expandable error block.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph, Wrap},
    Frame,
};

use ratatui::layout::Alignment;

use crate::tui::theme::Theme;

/// The `anvil` wordmark, followed by a breadcrumb trail.
///
/// Renders a single styled line:  `anvil │ Workloads › essentials`
/// The last crumb is bold/primary; earlier crumbs are muted.
pub fn header_line<'a>(theme: &Theme, crumbs: &'a [&'a str]) -> Line<'a> {
    let mut spans: Vec<Span> = vec![
        Span::styled("anvil", theme.brand_style()),
        Span::styled(" │ ", Style::default().fg(theme.border)),
    ];

    for (i, crumb) in crumbs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" › ", theme.faint_style()));
        }
        let last = i == crumbs.len() - 1;
        let style = if last {
            theme.normal().add_modifier(Modifier::BOLD)
        } else {
            theme.dimmed()
        };
        spans.push(Span::styled(*crumb, style));
    }

    Line::from(spans)
}

/// Render the header bar into `area` (typically Constraint::Length(1) inside
/// a bg_alt block). `right` lines are drawn right-aligned on the same row.
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    crumbs: &[&str],
    right: Option<Line>,
) {
    // bg_alt fill
    let bar = Block::default().style(Style::default().bg(theme.bg_alt));
    frame.render_widget(bar, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    };

    frame.render_widget(Paragraph::new(header_line(theme, crumbs)), inner);

    if let Some(right_line) = right {
        frame.render_widget(
            Paragraph::new(right_line).alignment(Alignment::Right),
            inner,
        );
    }
}

/// A small uppercase pill badge, e.g. `[local]`, `OK`, `3 ISSUES`.
pub fn badge<'a>(text: &'a str, fg: ratatui::style::Color) -> Span<'a> {
    Span::styled(
        format!(" {text} "),
        Style::default().fg(fg).add_modifier(Modifier::BOLD),
    )
}

/// Build a Gauge widget styled to the theme.
///
/// `ratio` in 0.0..=1.0. `color` is the fill (accent / success / running).
pub fn gauge<'a>(
    theme: &Theme,
    ratio: f64,
    color: ratatui::style::Color,
    label: Option<String>,
) -> Gauge<'a> {
    let mut g = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(theme.bg_inset))
        .ratio(ratio.clamp(0.0, 1.0))
        .use_unicode(true);
    if let Some(l) = label {
        g = g.label(Span::styled(
            l,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
    }
    g
}

/// A heatmap strip: one colored cell per check result.
/// Renders into `area` as a row of `▮` blocks (green = pass, ember = fail).
///
/// `results`: true = pass, false = fail. Cells beyond `area.width` are dropped.
#[allow(dead_code)]
pub fn render_heatmap(frame: &mut Frame, area: Rect, theme: &Theme, results: &[bool]) {
    let cells: Vec<Span> = results
        .iter()
        .take(area.width as usize)
        .map(|&pass| {
            let c = if pass { theme.success } else { theme.error };
            Span::styled("▮", Style::default().fg(c))
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(cells)), area);
}

/// A mini sparkline-style health bar: 8 cells filled proportional to `health`
/// (0..=100). Used in Status / Browser rows. Returns a Line of styled Spans.
#[allow(dead_code)]
pub fn health_bar<'a>(theme: &Theme, health: u8) -> Line<'a> {
    let filled = ((health as usize * 8) / 100).min(8);
    let color = if health >= 90 {
        theme.success
    } else if health >= 70 {
        theme.warning
    } else {
        theme.error
    };
    let mut spans: Vec<Span> = Vec::with_capacity(9);
    for i in 0..8 {
        let style = if i < filled {
            Style::default().fg(color)
        } else {
            Style::default().fg(theme.bg_inset)
        };
        spans.push(Span::styled("▮", style));
    }
    spans.push(Span::styled(
        format!(" {health}%"),
        Style::default().fg(color),
    ));
    Line::from(spans)
}

/// Render an expandable error block beneath a failed health item.
///
/// When collapsed, the caller renders the (possibly truncated) message
/// inline & right-aligned. When `expanded`, call this to draw the full
/// wrapped message in a tinted, bordered block.
#[allow(dead_code)]
pub fn render_error_block(frame: &mut Frame, area: Rect, theme: &Theme, message: &str) {
    let block = Block::bordered()
        .border_style(Style::default().fg(theme.error))
        .style(Style::default().bg(theme.bg_inset));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let para = Paragraph::new(Span::styled(message, theme.error_style())).wrap(Wrap { trim: true });
    frame.render_widget(para, inner);
}

/// Truncate a string to `max` display columns, appending `…` if cut.
/// (Column-accurate enough for ASCII / typical winget IDs.)
#[allow(dead_code)]
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let keep = max.saturating_sub(1);
        format!("{}…", s.chars().take(keep).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_header_line_single_crumb() {
        let theme = Theme::dark();
        let line = header_line(&theme, &["Workloads"]);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_header_line_multiple_crumbs() {
        let theme = Theme::dark();
        let line = header_line(&theme, &["Workloads", "essentials"]);
        // Should have: anvil, separator, crumb1, separator, crumb2
        assert!(line.spans.len() >= 5);
    }

    #[test]
    fn test_badge_styling() {
        let b = badge("OK", ratatui::style::Color::Green);
        assert!(b.content.contains("OK"));
    }

    #[test]
    fn test_gauge_clamping() {
        let theme = Theme::dark();
        let g = gauge(&theme, 1.5, theme.accent, Some("over".to_string()));
        // Should not panic — ratio is clamped
        let _ = g;
    }

    #[test]
    fn test_health_bar_values() {
        let theme = Theme::dark();
        let bar_100 = health_bar(&theme, 100);
        assert!(!bar_100.spans.is_empty());
        let bar_0 = health_bar(&theme, 0);
        assert!(!bar_0.spans.is_empty());
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 6), "hello…");
    }

    #[test]
    fn test_render_header_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        terminal
            .draw(|f| {
                render_header(f, f.area(), &theme, &["Workloads"], None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_heatmap_no_panic() {
        let backend = TestBackend::new(20, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let results = vec![true, true, false, true, false];
        terminal
            .draw(|f| {
                render_heatmap(f, f.area(), &theme, &results);
            })
            .unwrap();
    }

    #[test]
    fn test_render_error_block_no_panic() {
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        terminal
            .draw(|f| {
                render_error_block(f, f.area(), &theme, "Something went wrong");
            })
            .unwrap();
    }
}
