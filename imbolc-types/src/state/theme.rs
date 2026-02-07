//! Theme definitions for the Imbolc UI.
//!
//! Provides color schemes that can be applied across all panes.

use serde::{Deserialize, Serialize};

/// RGB color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to a tuple for convenience
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }
}

impl Default for ThemeColor {
    fn default() -> Self {
        Self::new(255, 255, 255)
    }
}

/// Complete UI theme with all color definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,

    // Base colors
    pub background: ThemeColor,
    pub foreground: ThemeColor,
    pub border: ThemeColor,

    // Semantic UI colors
    pub selection_bg: ThemeColor,
    pub selection_fg: ThemeColor,
    pub muted: ThemeColor,
    pub error: ThemeColor,
    pub warning: ThemeColor,
    pub success: ThemeColor,

    // Module type colors (for instrument editor sections)
    pub osc_color: ThemeColor,
    pub filter_color: ThemeColor,
    pub env_color: ThemeColor,
    pub lfo_color: ThemeColor,
    pub fx_color: ThemeColor,
    pub sample_color: ThemeColor,
    pub midi_color: ThemeColor,
    pub audio_in_color: ThemeColor,

    // Meter colors
    pub meter_low: ThemeColor,
    pub meter_mid: ThemeColor,
    pub meter_high: ThemeColor,

    // Waveform gradient (from center outward)
    pub waveform_gradient: [ThemeColor; 4],

    // Status colors
    pub playing: ThemeColor,
    pub recording: ThemeColor,
    pub armed: ThemeColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Default dark theme (current Imbolc colors)
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),

            // Base
            background: ThemeColor::new(0, 0, 0),
            foreground: ThemeColor::new(255, 255, 255),
            border: ThemeColor::new(100, 100, 100),

            // Semantic
            selection_bg: ThemeColor::new(60, 60, 80),
            selection_fg: ThemeColor::new(255, 255, 255),
            muted: ThemeColor::new(100, 100, 100),
            error: ThemeColor::new(255, 80, 80),
            warning: ThemeColor::new(255, 180, 0),
            success: ThemeColor::new(80, 255, 80),

            // Module types
            osc_color: ThemeColor::new(100, 200, 255),  // Cyan-ish
            filter_color: ThemeColor::new(255, 150, 100), // Orange
            env_color: ThemeColor::new(150, 255, 150),   // Green
            lfo_color: ThemeColor::new(255, 100, 255),   // Magenta
            fx_color: ThemeColor::new(255, 255, 100),    // Yellow
            sample_color: ThemeColor::new(100, 255, 200), // Teal
            midi_color: ThemeColor::new(255, 100, 100),   // Red
            audio_in_color: ThemeColor::new(200, 100, 255), // Purple

            // Meters
            meter_low: ThemeColor::new(0, 180, 0),
            meter_mid: ThemeColor::new(180, 180, 0),
            meter_high: ThemeColor::new(180, 0, 0),

            // Waveform gradient
            waveform_gradient: [
                ThemeColor::new(30, 60, 90),
                ThemeColor::new(60, 120, 180),
                ThemeColor::new(90, 180, 255),
                ThemeColor::new(120, 200, 255),
            ],

            // Status
            playing: ThemeColor::new(0, 255, 0),
            recording: ThemeColor::new(255, 0, 0),
            armed: ThemeColor::new(255, 100, 100),
        }
    }

    /// Light theme for bright environments
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),

            // Base
            background: ThemeColor::new(245, 245, 245),
            foreground: ThemeColor::new(30, 30, 30),
            border: ThemeColor::new(180, 180, 180),

            // Semantic
            selection_bg: ThemeColor::new(100, 140, 200),
            selection_fg: ThemeColor::new(255, 255, 255),
            muted: ThemeColor::new(150, 150, 150),
            error: ThemeColor::new(200, 50, 50),
            warning: ThemeColor::new(200, 140, 0),
            success: ThemeColor::new(50, 180, 50),

            // Module types (darker for visibility)
            osc_color: ThemeColor::new(0, 120, 200),
            filter_color: ThemeColor::new(200, 100, 50),
            env_color: ThemeColor::new(50, 150, 50),
            lfo_color: ThemeColor::new(180, 50, 180),
            fx_color: ThemeColor::new(180, 140, 0),
            sample_color: ThemeColor::new(0, 150, 120),
            midi_color: ThemeColor::new(200, 60, 60),
            audio_in_color: ThemeColor::new(140, 60, 200),

            // Meters
            meter_low: ThemeColor::new(50, 180, 50),
            meter_mid: ThemeColor::new(200, 180, 0),
            meter_high: ThemeColor::new(200, 50, 50),

            // Waveform gradient
            waveform_gradient: [
                ThemeColor::new(180, 200, 220),
                ThemeColor::new(100, 150, 200),
                ThemeColor::new(50, 100, 180),
                ThemeColor::new(30, 80, 160),
            ],

            // Status
            playing: ThemeColor::new(50, 200, 50),
            recording: ThemeColor::new(220, 50, 50),
            armed: ThemeColor::new(220, 100, 100),
        }
    }

    /// High contrast theme for accessibility
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),

            // Base
            background: ThemeColor::new(0, 0, 0),
            foreground: ThemeColor::new(255, 255, 255),
            border: ThemeColor::new(255, 255, 255),

            // Semantic
            selection_bg: ThemeColor::new(255, 255, 0),
            selection_fg: ThemeColor::new(0, 0, 0),
            muted: ThemeColor::new(180, 180, 180),
            error: ThemeColor::new(255, 0, 0),
            warning: ThemeColor::new(255, 255, 0),
            success: ThemeColor::new(0, 255, 0),

            // Module types (bright, saturated)
            osc_color: ThemeColor::new(0, 255, 255),
            filter_color: ThemeColor::new(255, 128, 0),
            env_color: ThemeColor::new(0, 255, 0),
            lfo_color: ThemeColor::new(255, 0, 255),
            fx_color: ThemeColor::new(255, 255, 0),
            sample_color: ThemeColor::new(0, 255, 128),
            midi_color: ThemeColor::new(255, 0, 0),
            audio_in_color: ThemeColor::new(128, 0, 255),

            // Meters
            meter_low: ThemeColor::new(0, 255, 0),
            meter_mid: ThemeColor::new(255, 255, 0),
            meter_high: ThemeColor::new(255, 0, 0),

            // Waveform gradient
            waveform_gradient: [
                ThemeColor::new(0, 50, 100),
                ThemeColor::new(0, 100, 200),
                ThemeColor::new(0, 200, 255),
                ThemeColor::new(100, 255, 255),
            ],

            // Status
            playing: ThemeColor::new(0, 255, 0),
            recording: ThemeColor::new(255, 0, 0),
            armed: ThemeColor::new(255, 255, 0),
        }
    }

    /// Get all built-in themes
    pub fn built_in_themes() -> Vec<Theme> {
        vec![Self::dark(), Self::light(), Self::high_contrast()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme.name, "Dark");
    }

    #[test]
    fn test_built_in_themes() {
        let themes = Theme::built_in_themes();
        assert_eq!(themes.len(), 3);
        assert_eq!(themes[0].name, "Dark");
        assert_eq!(themes[1].name, "Light");
        assert_eq!(themes[2].name, "High Contrast");
    }

    #[test]
    fn test_theme_color_rgb() {
        let color = ThemeColor::new(10, 20, 30);
        assert_eq!(color.to_rgb(), (10, 20, 30));
    }
}
