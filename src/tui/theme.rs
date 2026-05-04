//! Color theme for the Anvil TUI
//!
//! Defines the forge-branded color palette used across all TUI views.

use ratatui::style::{Color, Modifier, Style};

/// TUI color theme
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    /// Primary foreground
    pub fg: Color,
    /// Primary background
    pub bg: Color,
    /// Accent color (for highlights, borders)
    pub accent: Color,
    /// Success indicator
    pub success: Color,
    /// Warning indicator
    pub warning: Color,
    /// Error indicator
    pub error: Color,
    /// Muted/dimmed text
    pub muted: Color,
    /// Border color
    pub border: Color,
    /// Running/active indicator
    pub running: Color,
    /// Title/header text
    pub title: Color,
}

#[allow(dead_code)]
impl Theme {
    /// Dark theme — forge branding (molten gradient palette)
    pub fn dark() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Reset,
            accent: Color::Rgb(253, 224, 71), // amber/gold
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            border: Color::DarkGray,
            running: Color::Cyan,
            title: Color::Rgb(254, 243, 199), // warm white
        }
    }

    /// Style for normal text
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Style for muted/dimmed text
    pub fn dimmed(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for success text
    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for error text
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Style for warning text
    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Style for running/active text
    pub fn running_style(&self) -> Style {
        Style::default().fg(self.running)
    }

    /// Style for titles
    pub fn title_style(&self) -> Style {
        Style::default().fg(self.title).add_modifier(Modifier::BOLD)
    }

    /// Style for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_colors() {
        let theme = Theme::dark();
        assert_eq!(theme.fg, Color::White);
        assert_eq!(theme.success, Color::Green);
        assert_eq!(theme.error, Color::Red);
    }

    #[test]
    fn test_theme_styles_are_distinct() {
        let theme = Theme::dark();
        assert_ne!(theme.success_style(), theme.error_style());
        assert_ne!(theme.dimmed(), theme.normal());
    }
}
