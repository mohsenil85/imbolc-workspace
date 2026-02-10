use imbolc_types::state::theme::{Theme, ThemeColor};
use ratatui::style::{Color as RatatuiColor, Modifier, Style as RatatuiStyle};

/// RGB color. Construct with `Color::new(r, g, b)` or use named constants
/// (e.g. `Color::WHITE`, `Color::PINK`, `Color::MIDI_COLOR`, `Color::METER_LOW`).
///
/// No `Color::rgb()` alias exists — use `Color::new()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // Basic colors
    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0);
    pub const GREEN: Color = Color::new(0, 255, 0);
    pub const DEEP_GREEN: Color = Color::new(0, 100, 0);
    pub const BLUE: Color = Color::new(0, 0, 255);
    pub const YELLOW: Color = Color::new(255, 255, 0);
    pub const CYAN: Color = Color::new(0, 255, 255);
    pub const MAGENTA: Color = Color::new(255, 0, 255);
    pub const GRAY: Color = Color::new(128, 128, 128);
    pub const DARK_GRAY: Color = Color::new(100, 100, 100);

    // DAW accent colors
    pub const ORANGE: Color = Color::new(255, 165, 0);
    pub const PINK: Color = Color::new(255, 105, 180);
    pub const PURPLE: Color = Color::new(147, 112, 219);
    pub const LIME: Color = Color::new(50, 205, 50);
    pub const TEAL: Color = Color::new(0, 128, 128);
    #[allow(dead_code)]
    pub const CORAL: Color = Color::new(255, 127, 80);
    pub const SKY_BLUE: Color = Color::new(135, 206, 235);
    pub const GOLD: Color = Color::new(255, 215, 0);

    // Module type colors
    #[allow(dead_code)]
    pub const MIDI_COLOR: Color = Color::new(255, 100, 160);   // Magenta - MIDI/note source
    pub const OSC_COLOR: Color = Color::new(100, 180, 255);    // Blue - oscillators
    pub const FILTER_COLOR: Color = Color::new(255, 140, 90);  // Orange - filters
    pub const ENV_COLOR: Color = Color::new(180, 130, 255);    // Purple - envelopes
    #[allow(dead_code)]
    pub const LFO_COLOR: Color = Color::new(130, 255, 180);    // Mint - LFOs
    pub const FX_COLOR: Color = Color::new(255, 180, 220);     // Pink - effects
    pub const EQ_COLOR: Color = Color::new(100, 200, 255);     // Light blue - EQ
    #[allow(dead_code)]
    pub const OUTPUT_COLOR: Color = Color::new(255, 220, 100); // Gold - output
    pub const AUDIO_IN_COLOR: Color = Color::new(100, 255, 200); // Teal/Cyan - audio input
    pub const SAMPLE_COLOR: Color = Color::new(255, 200, 100); // Warm orange - sample
    pub const CUSTOM_COLOR: Color = Color::new(200, 150, 255); // Light purple - custom synthdef
    pub const KIT_COLOR: Color = Color::new(255, 165, 0);    // Orange - kit
    pub const BUS_IN_COLOR: Color = Color::new(180, 220, 100); // Yellow-green - bus input
    pub const VST_COLOR: Color = Color::new(255, 120, 200);    // Hot pink - VST plugins

    // Port type colors
    #[allow(dead_code)]
    pub const AUDIO_PORT: Color = Color::new(80, 200, 255);    // Cyan - audio
    #[allow(dead_code)]
    pub const CONTROL_PORT: Color = Color::new(100, 255, 150); // Green - control
    #[allow(dead_code)]
    pub const GATE_PORT: Color = Color::new(255, 230, 80);     // Yellow - gate

    // Meter colors
    pub const METER_LOW: Color = Color::new(80, 220, 100);     // Green
    pub const METER_MID: Color = Color::new(255, 220, 50);     // Yellow
    pub const METER_HIGH: Color = Color::new(255, 80, 80);     // Red

    // UI colors
    pub const SELECTION_BG: Color = Color::new(60, 100, 180);  // Selection highlight
    pub const MUTE_COLOR: Color = Color::new(255, 100, 100);   // Muted state
    pub const SOLO_COLOR: Color = Color::new(255, 220, 80);    // Solo state
}

impl From<ThemeColor> for Color {
    fn from(tc: ThemeColor) -> Self {
        Color::new(tc.r, tc.g, tc.b)
    }
}

// === Theme-aware style functions ===
// Use these instead of hardcoded Color constants when rendering with a theme.

/// Get the background color from theme
pub fn theme_bg(theme: &Theme) -> Color {
    theme.background.into()
}

/// Get the foreground color from theme
pub fn theme_fg(theme: &Theme) -> Color {
    theme.foreground.into()
}

/// Get the border color from theme
pub fn theme_border(theme: &Theme) -> Color {
    theme.border.into()
}

/// Get the selection background color from theme
pub fn theme_selection_bg(theme: &Theme) -> Color {
    theme.selection_bg.into()
}

/// Get the selection foreground color from theme
pub fn theme_selection_fg(theme: &Theme) -> Color {
    theme.selection_fg.into()
}

/// Get the muted/disabled color from theme
pub fn theme_muted(theme: &Theme) -> Color {
    theme.muted.into()
}

/// Get the error color from theme
pub fn theme_error(theme: &Theme) -> Color {
    theme.error.into()
}

/// Get the warning color from theme
pub fn theme_warning(theme: &Theme) -> Color {
    theme.warning.into()
}

/// Get the success color from theme
pub fn theme_success(theme: &Theme) -> Color {
    theme.success.into()
}

/// Get the oscillator module color from theme
pub fn theme_osc_color(theme: &Theme) -> Color {
    theme.osc_color.into()
}

/// Get the filter module color from theme
pub fn theme_filter_color(theme: &Theme) -> Color {
    theme.filter_color.into()
}

/// Get the envelope module color from theme
pub fn theme_env_color(theme: &Theme) -> Color {
    theme.env_color.into()
}

/// Get the LFO module color from theme
pub fn theme_lfo_color(theme: &Theme) -> Color {
    theme.lfo_color.into()
}

/// Get the effects module color from theme
pub fn theme_fx_color(theme: &Theme) -> Color {
    theme.fx_color.into()
}

/// Get the sample module color from theme
pub fn theme_sample_color(theme: &Theme) -> Color {
    theme.sample_color.into()
}

/// Get the MIDI module color from theme
pub fn theme_midi_color(theme: &Theme) -> Color {
    theme.midi_color.into()
}

/// Get the audio input module color from theme
pub fn theme_audio_in_color(theme: &Theme) -> Color {
    theme.audio_in_color.into()
}

/// Get meter color for low level
pub fn theme_meter_low(theme: &Theme) -> Color {
    theme.meter_low.into()
}

/// Get meter color for mid level
pub fn theme_meter_mid(theme: &Theme) -> Color {
    theme.meter_mid.into()
}

/// Get meter color for high level
pub fn theme_meter_high(theme: &Theme) -> Color {
    theme.meter_high.into()
}

/// Get the playing status color from theme
pub fn theme_playing(theme: &Theme) -> Color {
    theme.playing.into()
}

/// Get the recording status color from theme
pub fn theme_recording(theme: &Theme) -> Color {
    theme.recording.into()
}

/// Get the armed status color from theme
pub fn theme_armed(theme: &Theme) -> Color {
    theme.armed.into()
}

/// Get waveform gradient colors from theme (array of 4 colors from center outward)
pub fn theme_waveform_gradient(theme: &Theme) -> [Color; 4] {
    [
        theme.waveform_gradient[0].into(),
        theme.waveform_gradient[1].into(),
        theme.waveform_gradient[2].into(),
        theme.waveform_gradient[3].into(),
    ]
}

/// Build a selection style from theme
pub fn selection_style(theme: &Theme) -> Style {
    Style::new()
        .fg(theme_selection_fg(theme))
        .bg(theme_selection_bg(theme))
}

/// Create style with conditional selection background.
/// Useful for list items that highlight when selected.
pub fn selected_style(is_selected: bool, fg: Color) -> Style {
    if is_selected {
        Style::new().fg(fg).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(fg)
    }
}

/// Create bold style with conditional selection background.
/// Useful for list items that need bold text and highlight when selected.
pub fn selected_style_bold(is_selected: bool, fg: Color) -> Style {
    if is_selected {
        Style::new().fg(fg).bg(Color::SELECTION_BG).bold()
    } else {
        Style::new().fg(fg).bold()
    }
}

/// Text style with foreground, background, and attributes.
///
/// Builder methods (all const, chainable):
/// - `fg(Color)` — set foreground color
/// - `bg(Color)` — set background color
/// - `bold()` — enable bold
/// - `underline()` — enable underline
///
/// No `italic()`, `dim()`, or `reset()` methods exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub underline: bool,
}


impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            underline: false,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    #[allow(dead_code)]
    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

// --- Conversions to ratatui types ---

impl From<Color> for RatatuiColor {
    fn from(c: Color) -> Self {
        RatatuiColor::Rgb(c.r, c.g, c.b)
    }
}

impl From<Style> for RatatuiStyle {
    fn from(s: Style) -> Self {
        let mut rs = RatatuiStyle::default();
        if let Some(fg) = s.fg {
            rs = rs.fg(RatatuiColor::from(fg));
        }
        if let Some(bg) = s.bg {
            rs = rs.bg(RatatuiColor::from(bg));
        }
        if s.bold {
            rs = rs.add_modifier(Modifier::BOLD);
        }
        if s.underline {
            rs = rs.add_modifier(Modifier::UNDERLINED);
        }
        rs
    }
}
