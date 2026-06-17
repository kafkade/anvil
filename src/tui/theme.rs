//! Color theme for the Anvil TUI
//!
//! Forge-branded palette built on the kafkade "Flexoki" design system
//! (dark mode). Ember red is the single dominant accent; the remaining
//! hues are used sparingly for status, section identity, and metadata.
//!
//! Source tokens: kafkade colors_and_type.css → [data-theme="dark"].

use ratatui::style::{Color, Modifier, Style};

/// TUI color theme
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    // ---- surfaces ----
    /// Page background (terminal default — keeps user's transparency/theme)
    pub bg: Color,
    /// Secondary surface (header bars, footers)
    pub bg_alt: Color,
    /// Inset surface (code, inline highlight, gauge track)
    pub bg_inset: Color,

    // ---- text ----
    /// Primary text
    pub fg: Color,
    /// Secondary / body text
    pub ink2: Color,
    /// Muted meta text (dates, captions, counts)
    pub muted: Color,
    /// Faint text (disabled, pending, end-marks)
    pub faint: Color,

    // ---- structure ----
    /// Structural border
    pub border: Color,
    /// Hairline rule / row divider
    pub hairline: Color,

    // ---- accent ----
    /// THE brand accent — ember red. Links, selection, active, primary.
    pub accent: Color,
    /// Text/ink on an accent fill
    pub accent_ink: Color,

    // ---- status / section hues (Flexoki 400-level for dark) ----
    pub success: Color, // green   — pass / installed / stable
    pub warning: Color, // yellow  — warn / partial / alpha
    pub error: Color,   // red     — fail (== accent, intentional)
    pub running: Color, // cyan    — in-progress / tags
    pub info: Color,    // blue    — files / informational
    pub special: Color, // orange  — commands / emphasis
    pub alt: Color,     // purple  — fonts / secondary category
    pub title: Color,   // warm-white titles
}

#[allow(dead_code)]
impl Theme {
    /// Dark theme — kafkade Flexoki dark + ember accent
    pub fn dark() -> Self {
        Self {
            // surfaces
            bg: Color::Reset,
            bg_alt: Color::Rgb(0x1C, 0x1B, 0x1A),
            bg_inset: Color::Rgb(0x28, 0x27, 0x26),
            // text
            fg: Color::Rgb(0xCE, 0xCD, 0xC3),
            ink2: Color::Rgb(0xB7, 0xB5, 0xAC),
            muted: Color::Rgb(0x87, 0x85, 0x80),
            faint: Color::Rgb(0x57, 0x56, 0x53),
            // structure
            border: Color::Rgb(0x40, 0x3E, 0x3C),
            hairline: Color::Rgb(0x34, 0x33, 0x31),
            // accent
            accent: Color::Rgb(0xD1, 0x4D, 0x41),
            accent_ink: Color::Rgb(0x10, 0x0F, 0x0F),
            // status / section
            success: Color::Rgb(0x87, 0x9A, 0x39),
            warning: Color::Rgb(0xD0, 0xA2, 0x15),
            error: Color::Rgb(0xD1, 0x4D, 0x41),
            running: Color::Rgb(0x3A, 0xA9, 0x9F),
            info: Color::Rgb(0x43, 0x85, 0xBE),
            special: Color::Rgb(0xDA, 0x76, 0x35),
            alt: Color::Rgb(0x8B, 0x7E, 0xC8),
            title: Color::Rgb(0xCE, 0xCD, 0xC3),
        }
    }

    // ---------------------------------------------------------------
    // Text styles
    // ---------------------------------------------------------------

    /// Normal body text
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Secondary body text
    pub fn body(&self) -> Style {
        Style::default().fg(self.ink2)
    }

    /// Muted / dimmed meta text
    pub fn dimmed(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Faint text (pending, disabled)
    pub fn faint_style(&self) -> Style {
        Style::default().fg(self.faint)
    }

    /// Success text (green)
    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Error text (ember)
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Warning text (yellow)
    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Running / active text (cyan)
    pub fn running_style(&self) -> Style {
        Style::default().fg(self.running)
    }

    /// Accent text (ember)
    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Bold accent — used for the `anvil` wordmark and active emphasis
    pub fn brand_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Title text (bold)
    pub fn title_style(&self) -> Style {
        Style::default().fg(self.title).add_modifier(Modifier::BOLD)
    }

    /// Uppercase section label (muted, used as group dividers)
    pub fn label_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    // ---------------------------------------------------------------
    // Selection / highlight
    // ---------------------------------------------------------------

    /// Selected row — subtle ember wash + ember text (no full inversion).
    /// Pair with a `▌` accent prefix Span for the left border effect.
    pub fn selection(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Strong selection (full fill) — for list pickers where contrast matters.
    pub fn selection_filled(&self) -> Style {
        Style::default().bg(self.accent).fg(self.accent_ink)
    }

    // ---------------------------------------------------------------
    // Borders
    // ---------------------------------------------------------------

    /// Resting border
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Focused border (ember)
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.accent)
    }

    // ---------------------------------------------------------------
    // Section identity colors (Detail / Browser breakdown)
    // ---------------------------------------------------------------

    /// Color for a workload section by kind. Keeps section identity
    /// consistent across Browser breakdown, Detail tabs, and Health.
    pub fn section_color(&self, kind: SectionKind) -> Color {
        match kind {
            SectionKind::Packages => self.running, // cyan
            SectionKind::Files => self.info,       // blue
            SectionKind::Commands => self.special, // orange
            SectionKind::Fonts => self.alt,        // purple
            SectionKind::Features => self.warning, // yellow
            SectionKind::Environment => self.muted,
            SectionKind::Assertions => self.success, // green
            SectionKind::Inheritance => self.muted,
        }
    }
}

/// Identifies a workload section for consistent coloring & icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SectionKind {
    Inheritance,
    Packages,
    Files,
    Commands,
    Fonts,
    Features,
    Environment,
    Assertions,
}

#[allow(dead_code)]
impl SectionKind {
    /// A single-glyph icon for the section (terminal-safe).
    pub fn icon(self) -> &'static str {
        match self {
            SectionKind::Inheritance => "~",
            SectionKind::Packages => "#",
            SectionKind::Files => "=",
            SectionKind::Commands => ">",
            SectionKind::Fonts => "A",
            SectionKind::Features => "*",
            SectionKind::Environment => "$",
            SectionKind::Assertions => "+",
        }
    }

    /// Human label.
    pub fn label(self) -> &'static str {
        match self {
            SectionKind::Inheritance => "Inheritance",
            SectionKind::Packages => "Packages",
            SectionKind::Files => "Files",
            SectionKind::Commands => "Commands",
            SectionKind::Fonts => "Fonts",
            SectionKind::Features => "Features",
            SectionKind::Environment => "Environment",
            SectionKind::Assertions => "Assertions",
        }
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
    fn test_dark_theme_accent_is_ember() {
        let theme = Theme::dark();
        assert_eq!(theme.accent, Color::Rgb(0xD1, 0x4D, 0x41));
        assert_eq!(theme.error, theme.accent); // fail == accent, intentional
    }

    #[test]
    fn test_theme_styles_are_distinct() {
        let theme = Theme::dark();
        assert_ne!(theme.success_style(), theme.error_style());
        assert_ne!(theme.dimmed(), theme.normal());
        assert_ne!(theme.border_style(), theme.border_focused());
    }

    #[test]
    fn test_section_colors_distinct() {
        let theme = Theme::dark();
        assert_ne!(
            theme.section_color(SectionKind::Packages),
            theme.section_color(SectionKind::Files)
        );
    }
}
