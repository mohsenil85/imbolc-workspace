//! DAW theme bridge for rat-widget styling.
//!
//! Maps our semantic color constants from `style.rs` to `ratatui::style::Style`
//! values suitable for rat-widget configuration. Provides a centralized place
//! to control how all rat-widget components look within the DAW.

use ratatui::style::{Color as RatatuiColor, Style as RatatuiStyle};

use super::style::Color;

/// Centralized theme for rat-widget components in the DAW.
///
/// Each method returns a `ratatui::style::Style` that can be passed directly
/// to rat-widget builder methods (`.style()`, `.focus_style()`, etc.).
pub struct DawTheme;

impl DawTheme {
    // ── Text Input ────────────────────────────────────────────────

    /// Base style for text input widgets
    pub fn text_input_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::WHITE))
    }

    /// Style when a text input has focus
    pub fn text_input_focus_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::WHITE))
    }

    /// Style for selected text in inputs
    pub fn text_input_select_style() -> RatatuiStyle {
        RatatuiStyle::default()
            .fg(RatatuiColor::from(Color::WHITE))
            .bg(RatatuiColor::from(Color::SELECTION_BG))
    }

    /// Cursor style for text inputs
    pub fn text_input_cursor_style() -> RatatuiStyle {
        RatatuiStyle::default()
            .fg(RatatuiColor::from(Color::WHITE))
            .bg(RatatuiColor::from(Color::SELECTION_BG))
    }

    // ── Number Input ──────────────────────────────────────────────

    /// Base style for number inputs
    pub fn number_input_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::LIME))
    }

    /// Focus style for number inputs
    pub fn number_input_focus_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::WHITE))
    }

    /// Selection style for number inputs
    pub fn number_input_select_style() -> RatatuiStyle {
        RatatuiStyle::default()
            .fg(RatatuiColor::from(Color::WHITE))
            .bg(RatatuiColor::from(Color::SELECTION_BG))
    }

    // ── Checkbox ──────────────────────────────────────────────────

    /// Base style for checkboxes
    pub fn checkbox_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::WHITE))
    }

    /// Focus style for checkboxes
    pub fn checkbox_focus_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::CYAN))
    }

    // ── Slider ────────────────────────────────────────────────────

    /// Track style for sliders
    pub fn slider_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::LIME))
    }

    /// Focus style for sliders
    pub fn slider_focus_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::CYAN))
    }

    /// Knob style for sliders
    pub fn slider_knob_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::WHITE))
    }

    // ── Dialog / Popup ────────────────────────────────────────────

    /// Border style for dialogs
    pub fn dialog_border_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::CYAN))
    }

    /// Title style for dialogs
    pub fn dialog_title_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::CYAN))
    }

    /// Warning dialog border
    pub fn warning_border_style() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::YELLOW))
    }

    // ── Selection ─────────────────────────────────────────────────

    /// Background for selected items in lists
    pub fn selection_bg() -> RatatuiStyle {
        RatatuiStyle::default().bg(RatatuiColor::from(Color::SELECTION_BG))
    }

    /// Selected item text
    pub fn selection_text() -> RatatuiStyle {
        RatatuiStyle::default()
            .fg(RatatuiColor::from(Color::WHITE))
            .bg(RatatuiColor::from(Color::SELECTION_BG))
    }

    // ── General ───────────────────────────────────────────────────

    /// Muted/disabled text
    pub fn muted_text() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::DARK_GRAY))
    }

    /// Help/hint text
    pub fn help_text() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::DARK_GRAY))
    }

    /// Error text
    pub fn error_text() -> RatatuiStyle {
        RatatuiStyle::default().fg(RatatuiColor::from(Color::MUTE_COLOR))
    }
}
