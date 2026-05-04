//! Key hints bar widget
//!
//! Renders a bottom bar showing available keybindings.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::theme::Theme;

/// A single key hint (key + description)
pub struct KeyHint {
    pub key: &'static str,
    pub desc: &'static str,
}

/// Render the key hints bar at the given area
pub fn render_keyhints(frame: &mut Frame, area: Rect, hints: &[KeyHint], theme: &Theme) {
    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, hint)| {
            let mut v = vec![
                Span::styled(
                    format!(" {} ", hint.key),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(hint.desc, theme.dimmed()),
            ];
            if i < hints.len() - 1 {
                v.push(Span::raw("   "));
            }
            v
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_keyhints() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        let hints = vec![
            KeyHint {
                key: "↑/↓",
                desc: "scroll",
            },
            KeyHint {
                key: "q",
                desc: "quit",
            },
            KeyHint {
                key: "v",
                desc: "verbose",
            },
        ];

        terminal
            .draw(|f| {
                let area = f.area();
                render_keyhints(f, area, &hints, &theme);
            })
            .unwrap();
    }
}
