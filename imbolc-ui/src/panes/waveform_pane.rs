use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, WaveformActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, Pane, Style};

/// Waveform display characters (8 levels) - used for spectrum/meters
const WAVEFORM_CHARS: [char; 8] = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];

/// Braille dot pattern offsets (2 columns x 4 rows)
/// Column 0: bits 0,1,2,6  Column 1: bits 3,4,5,7
const BRAILLE_DOT_OFFSETS: [[u8; 4]; 2] = [
    [0, 1, 2, 6],  // left column (x=0): rows 0,1,2,3
    [3, 4, 5, 7],  // right column (x=1): rows 0,1,2,3
];

/// Convert a set of dot coordinates to a braille character
/// Each dot is (x, y) where x is 0-1 and y is 0-3
fn dots_to_braille(dots: &[(u8, u8)]) -> char {
    let mut pattern: u8 = 0;
    for &(x, y) in dots {
        if x < 2 && y < 4 {
            pattern |= 1 << BRAILLE_DOT_OFFSETS[x as usize][y as usize];
        }
    }
    char::from_u32(0x2800 + pattern as u32).unwrap_or(' ')
}

/// Spectrum band labels
const SPECTRUM_LABELS: [&str; 7] = ["60", "150", "400", "1k", "2.5k", "6k", "15k"];

/// Color a waveform/meter row by its distance from center (0.0=center, 1.0=edge)
fn waveform_color(frac: f32) -> Color {
    if frac > 0.85 {
        Color::new(220, 40, 40)   // red
    } else if frac > 0.7 {
        Color::new(220, 120, 30)  // orange
    } else if frac > 0.5 {
        Color::new(200, 200, 40)  // yellow
    } else {
        Color::new(60, 200, 80)   // green
    }
}

/// Convert linear amplitude to dB
fn amp_to_db(amp: f32) -> f32 {
    if amp <= 0.0 { -96.0 } else { 20.0 * amp.log10() }
}

/// Display mode for the waveform pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveformMode {
    Waveform,
    Spectrum,
    Oscilloscope,
    LufsMeter,
}

#[allow(dead_code)]
impl WaveformMode {
    fn next(self) -> Self {
        match self {
            WaveformMode::Waveform => WaveformMode::Spectrum,
            WaveformMode::Spectrum => WaveformMode::Oscilloscope,
            WaveformMode::Oscilloscope => WaveformMode::LufsMeter,
            WaveformMode::LufsMeter => WaveformMode::Waveform,
        }
    }

    fn name(self) -> &'static str {
        match self {
            WaveformMode::Waveform => "Waveform",
            WaveformMode::Spectrum => "Spectrum",
            WaveformMode::Oscilloscope => "Oscilloscope",
            WaveformMode::LufsMeter => "Level Meter",
        }
    }
}

pub struct WaveformPane {
    keymap: Keymap,
    /// Live waveform from audio input
    pub audio_in_waveform: Option<Vec<f32>>,
    /// Current display mode
    mode: WaveformMode,
}

impl WaveformPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            audio_in_waveform: None,
            mode: WaveformMode::Waveform,
        }
    }
}

impl Default for WaveformPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl WaveformPane {
    fn render_waveform(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let is_recorded = state.recorded_waveform_peaks.is_some();
        let waveform = state.recorded_waveform_peaks.as_deref()
            .or(self.audio_in_waveform.as_deref())
            .unwrap_or(&[]);

        let rect = center_rect(area, 97, 29);
        let header_height: u16 = 2;
        let footer_height: u16 = 2;
        let grid_x = rect.x + 1;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(2);
        let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

        let title = if is_recorded {
            " Recorded Waveform ".to_string()
        } else if let Some(inst) = state.instruments.selected_instrument() {
            format!(" Audio Input: {} ", inst.name)
        } else {
            " Audio Input ".to_string()
        };
        self.render_border(rect, buf, &title, Color::AUDIO_IN_COLOR);
        self.render_header(rect, buf, state, "Waveform");

        // Braille grid dimensions (2 dots per char width, 4 dots per char height)
        let dot_width = grid_width as usize * 2;
        let dot_height = grid_height as usize * 4;
        let center_dot_y = dot_height / 2;

        // Center line (using braille dots on center row)
        let dark_gray = Style::new().fg(Color::DARK_GRAY);
        let center_char_row = grid_y + (grid_height / 2);
        for x in 0..grid_width {
            buf.set_cell(grid_x + x, center_char_row, '\u{2500}', dark_gray);
        }

        // Draw waveform using braille
        let waveform_len = waveform.len();
        if waveform_len == 0 {
            let status_y = grid_y + grid_height;
            let status = "Samples: 0  [Tab: cycle mode]";
            buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
                &[(status, Style::new().fg(Color::GRAY))]);
            return;
        }

        // Use fixed display size for live audio-in to prevent jumping
        // Recorded waveform peaks are already fixed at 512 samples
        const DISPLAY_SAMPLES: usize = 200;

        // Normalize live waveform to fixed size
        // - Recorded: use as-is (already fixed size)
        // - Live >= DISPLAY_SAMPLES: use last DISPLAY_SAMPLES
        // - Live < DISPLAY_SAMPLES: stretch to fill display
        let display_buffer: Vec<f32> = if is_recorded {
            waveform.to_vec()
        } else if waveform_len >= DISPLAY_SAMPLES {
            waveform[waveform_len - DISPLAY_SAMPLES..].to_vec()
        } else {
            // Stretch available samples to fill display (nearest-neighbor interpolation)
            (0..DISPLAY_SAMPLES)
                .map(|i| waveform[i * waveform_len / DISPLAY_SAMPLES])
                .collect()
        };
        let display_len = display_buffer.len();

        // Build a 2D grid of dots
        let mut dot_grid: Vec<Vec<bool>> = vec![vec![false; dot_height]; dot_width];

        // Map samples to dots - for waveform, we show amplitude mirrored around center
        for dot_x in 0..dot_width {
            let sample_idx = (dot_x * display_len / dot_width).min(display_len - 1);
            let amplitude = display_buffer[sample_idx].abs().min(1.0);

            // Calculate how many dots above/below center to fill
            let half_dot_height = center_dot_y;
            let bar_dots = (amplitude * half_dot_height as f32) as usize;

            // Fill dots above center
            for dy in 0..bar_dots {
                let y = center_dot_y.saturating_sub(dy + 1);
                if y < dot_height {
                    dot_grid[dot_x][y] = true;
                }
            }
            // Fill dots below center (mirror)
            for dy in 0..bar_dots {
                let y = center_dot_y + dy;
                if y < dot_height {
                    dot_grid[dot_x][y] = true;
                }
            }
        }

        // Convert dot grid to braille characters
        for char_col in 0..grid_width as usize {
            for char_row in 0..grid_height as usize {
                let mut dots: Vec<(u8, u8)> = Vec::new();

                // Each braille char covers 2 dot columns and 4 dot rows
                for dx in 0..2 {
                    for dy in 0..4 {
                        let dot_x = char_col * 2 + dx;
                        let dot_y = char_row * 4 + dy;
                        if dot_x < dot_width && dot_y < dot_height && dot_grid[dot_x][dot_y] {
                            dots.push((dx as u8, dy as u8));
                        }
                    }
                }

                if !dots.is_empty() {
                    let braille = dots_to_braille(&dots);
                    // Color based on distance from center
                    let char_center_dist = (char_row as f32 - (grid_height as f32 / 2.0)).abs();
                    let frac = char_center_dist / (grid_height as f32 / 2.0);
                    let color = waveform_color(frac);
                    let style = Style::new().fg(color);
                    buf.set_cell(grid_x + char_col as u16, grid_y + char_row as u16, braille, style);
                }
            }
        }

        let status_y = grid_y + grid_height;
        let status = format!("Samples: {}  [Tab: cycle mode]", waveform_len);
        buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
            &[(&status, Style::new().fg(Color::GRAY))]);
    }

    fn render_spectrum(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 97, 29);
        let header_height: u16 = 2;
        let footer_height: u16 = 3;
        let grid_x = rect.x + 1;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(2);
        let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

        self.render_border(rect, buf, " Spectrum Analyzer ", Color::METER_LOW);
        self.render_header(rect, buf, state, "Spectrum");

        let bands = &state.audio.visualization.spectrum_bands;
        let num_bands = bands.len();
        let band_width = grid_width as usize / num_bands;
        let gap = 1_usize; // gap between bands

        for (i, &amp) in bands.iter().enumerate() {
            let bar_x = grid_x + (i * band_width) as u16 + 1;
            let bar_width = (band_width - gap).max(1);
            let bar_height = (amp.min(1.0) * grid_height as f32) as u16;

            // Draw bar from bottom up
            for dy in 0..bar_height.min(grid_height) {
                let y = grid_y + grid_height - 1 - dy;
                let frac = (dy + 1) as f32 / grid_height as f32;
                let color = waveform_color(frac);
                let style = Style::new().fg(color);
                for bx in 0..bar_width as u16 {
                    if bar_x + bx < grid_x + grid_width {
                        buf.set_cell(bar_x + bx, y, WAVEFORM_CHARS[7], style);
                    }
                }
            }

            // Label below
            let label_y = grid_y + grid_height;
            let label = SPECTRUM_LABELS[i];
            let label_x = bar_x + (bar_width as u16 / 2).saturating_sub(label.len() as u16 / 2);
            buf.draw_line(Rect::new(label_x, label_y, label.len() as u16 + 1, 1),
                &[(label, Style::new().fg(Color::GRAY))]);

            // dB value above
            let db = amp_to_db(amp);
            let db_str = if db <= -60.0 { "-inf".to_string() } else { format!("{:.0}", db) };
            let db_x = bar_x + (bar_width as u16 / 2).saturating_sub(db_str.len() as u16 / 2);
            let db_y = grid_y + grid_height + 1;
            if db_y < rect.y + rect.height - 1 {
                buf.draw_line(Rect::new(db_x, db_y, 5, 1),
                    &[(&db_str, Style::new().fg(Color::DARK_GRAY))]);
            }
        }

        let status_y = rect.y + rect.height - 2;
        buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
            &[("[Tab: cycle mode]", Style::new().fg(Color::DARK_GRAY))]);
    }

    fn render_oscilloscope(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 97, 29);
        let header_height: u16 = 2;
        let footer_height: u16 = 2;
        let grid_x = rect.x + 1;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(2);
        let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

        self.render_border(rect, buf, " Oscilloscope ", Color::MIDI_COLOR);
        self.render_header(rect, buf, state, "Oscilloscope");

        let scope = &state.audio.visualization.scope_buffer;
        let scope_len = scope.len();

        // Braille grid dimensions
        let dot_width = grid_width as usize * 2;
        let dot_height = grid_height as usize * 4;

        // Draw center line
        let dark_gray = Style::new().fg(Color::DARK_GRAY);
        let center_char_row = grid_y + grid_height / 2;
        for x in 0..grid_width {
            buf.set_cell(grid_x + x, center_char_row, '\u{2500}', dark_gray);
        }

        // +1/-1 labels
        buf.draw_line(Rect::new(grid_x, grid_y, 2, 1), &[("+1", dark_gray)]);
        buf.draw_line(Rect::new(grid_x, grid_y + grid_height - 1, 2, 1), &[("-1", dark_gray)]);

        // Use fixed display size to prevent jumping from variable OSC receive rate
        const DISPLAY_SAMPLES: usize = 200;

        // Normalize buffer to fixed size
        // - If >= DISPLAY_SAMPLES: use last DISPLAY_SAMPLES (stable mapping)
        // - If < DISPLAY_SAMPLES: stretch to fill (prevents half-empty display)
        let display_buffer: Vec<f32> = if scope_len == 0 {
            vec![0.0; DISPLAY_SAMPLES]
        } else if scope_len >= DISPLAY_SAMPLES {
            scope.iter().skip(scope_len - DISPLAY_SAMPLES).copied().collect()
        } else {
            // Stretch available samples to fill display (nearest-neighbor interpolation)
            (0..DISPLAY_SAMPLES)
                .map(|i| scope[i * scope_len / DISPLAY_SAMPLES])
                .collect()
        };

        if scope_len == 0 {
            let status_y = grid_y + grid_height;
            let status = "Samples: 0  [Tab: cycle mode]";
            buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
                &[(status, Style::new().fg(Color::GRAY))]);
            return;
        }

        // Build a 2D grid of dots for the oscilloscope trace
        let mut dot_grid: Vec<Vec<bool>> = vec![vec![false; dot_height]; dot_width];

        // Map samples to dots - oscilloscope shows actual waveform (not mirrored)
        let mut prev_dot_y: Option<usize> = None;
        for dot_x in 0..dot_width {
            let sample_idx = (dot_x * DISPLAY_SAMPLES / dot_width).min(DISPLAY_SAMPLES - 1);
            let sample = display_buffer[sample_idx].clamp(-1.0, 1.0);

            // Map sample (-1 to 1) to dot y coordinate (0 to dot_height-1)
            // -1 -> bottom (dot_height-1), +1 -> top (0)
            let normalized = (1.0 - sample) / 2.0; // 0 to 1
            let dot_y = ((normalized * (dot_height - 1) as f32) as usize).min(dot_height - 1);

            dot_grid[dot_x][dot_y] = true;

            // Connect to previous point for smooth lines
            if let Some(prev_y) = prev_dot_y {
                let (y_min, y_max) = if dot_y < prev_y { (dot_y, prev_y) } else { (prev_y, dot_y) };
                for fill_y in y_min..=y_max {
                    if fill_y < dot_height {
                        dot_grid[dot_x][fill_y] = true;
                    }
                }
            }
            prev_dot_y = Some(dot_y);
        }

        // Convert dot grid to braille characters
        let green = Style::new().fg(Color::new(60, 200, 80));
        for char_col in 0..grid_width as usize {
            for char_row in 0..grid_height as usize {
                let mut dots: Vec<(u8, u8)> = Vec::new();

                // Each braille char covers 2 dot columns and 4 dot rows
                for dx in 0..2 {
                    for dy in 0..4 {
                        let dot_x = char_col * 2 + dx;
                        let dot_y = char_row * 4 + dy;
                        if dot_x < dot_width && dot_y < dot_height && dot_grid[dot_x][dot_y] {
                            dots.push((dx as u8, dy as u8));
                        }
                    }
                }

                if !dots.is_empty() {
                    let braille = dots_to_braille(&dots);
                    buf.set_cell(grid_x + char_col as u16, grid_y + char_row as u16, braille, green);
                }
            }
        }

        let status_y = grid_y + grid_height;
        let status = format!("Samples: {}  [Tab: cycle mode]", scope_len);
        buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
            &[(&status, Style::new().fg(Color::GRAY))]);
    }

    fn render_lufs_meter(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 97, 29);
        let header_height: u16 = 2;
        let footer_height: u16 = 2;
        let grid_x = rect.x + 1;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(2);
        let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

        self.render_border(rect, buf, " Level Meter ", Color::METER_LOW);
        self.render_header(rect, buf, state, "Level Meter");

        let viz = &state.audio.visualization;
        let meter_width = grid_width / 2 - 4; // space for each channel

        // Left channel
        self.render_single_meter(grid_x + 2, grid_y, meter_width, grid_height, viz.peak_l, viz.rms_l, "L", buf);

        // Right channel
        self.render_single_meter(grid_x + grid_width / 2 + 2, grid_y, meter_width, grid_height, viz.peak_r, viz.rms_r, "R", buf);

        // Numeric readout at bottom
        let status_y = grid_y + grid_height;
        let peak_db_l = amp_to_db(viz.peak_l);
        let peak_db_r = amp_to_db(viz.peak_r);
        let rms_db_l = amp_to_db(viz.rms_l);
        let rms_db_r = amp_to_db(viz.rms_r);
        let status = format!(
            "L: peak {:.1}dB  rms {:.1}dB    R: peak {:.1}dB  rms {:.1}dB    [Tab: cycle mode]",
            peak_db_l, rms_db_l, peak_db_r, rms_db_r,
        );
        buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
            &[(&status, Style::new().fg(Color::GRAY))]);
    }

    fn render_single_meter(&self, x: u16, y: u16, width: u16, height: u16, peak: f32, rms: f32, label: &str, buf: &mut RenderBuf) {
        // dB scale: -60 to 0
        let db_range = 60.0_f32;
        let peak_db = amp_to_db(peak).max(-db_range);
        let rms_db = amp_to_db(rms).max(-db_range);
        let peak_frac = ((peak_db + db_range) / db_range).clamp(0.0, 1.0);
        let rms_frac = ((rms_db + db_range) / db_range).clamp(0.0, 1.0);

        let peak_height = (peak_frac * height as f32) as u16;
        let rms_height = (rms_frac * height as f32) as u16;

        // Split width: RMS bars take most of it, peak indicator on the side
        let rms_width = width.saturating_sub(2);

        // Draw RMS bars from bottom up
        for dy in 0..rms_height.min(height) {
            let row = y + height - 1 - dy;
            let frac = (dy + 1) as f32 / height as f32;
            let color = waveform_color(frac);
            let style = Style::new().fg(color);
            for bx in 0..rms_width {
                buf.set_cell(x + bx, row, WAVEFORM_CHARS[7], style);
            }
        }

        // Draw peak indicator (single character on the right side)
        if peak_height > 0 {
            let peak_y = y + height - peak_height.min(height);
            let peak_frac_color = peak_height as f32 / height as f32;
            let peak_color = waveform_color(peak_frac_color);
            buf.set_cell(x + rms_width + 1, peak_y, '\u{2501}', Style::new().fg(peak_color));
        }

        // Channel label
        let label_x = x + rms_width / 2;
        let label_y = y + height;
        if label_y < y + height + 2 {
            buf.draw_line(Rect::new(label_x, label_y, 2, 1),
                &[(label, Style::new().fg(Color::WHITE))]);
        }

        // dB scale markers on the left side of meter
        let dark_gray = Style::new().fg(Color::DARK_GRAY);
        let markers = [("0", 0.0), ("-6", 6.0), ("-12", 12.0), ("-24", 24.0), ("-48", 48.0)];
        for (text, db_offset) in markers {
            let frac = (db_range - db_offset) / db_range;
            let marker_y = y + ((1.0 - frac) * height as f32) as u16;
            if marker_y >= y && marker_y < y + height {
                // Tick mark
                if x > 0 {
                    buf.draw_line(Rect::new(x.saturating_sub(text.len() as u16 + 1), marker_y, text.len() as u16, 1),
                        &[(text, dark_gray)]);
                }
            }
        }
    }

    fn render_border(&self, rect: Rect, buf: &mut RenderBuf, title: &str, color: Color) {
        let border_style = Style::new().fg(color);
        buf.draw_block(rect, title, border_style, border_style);
    }

    fn render_header(&self, rect: Rect, buf: &mut RenderBuf, state: &AppState, mode_name: &str) {
        let header_y = rect.y + 1;
        let play_icon = if state.audio.playing { "||" } else { "> " };
        let header_text = format!(
            " BPM:{:.0}  {}  {}",
            state.audio.bpm, play_icon, mode_name,
        );
        buf.draw_line(Rect::new(rect.x + 1, header_y, rect.width.saturating_sub(2), 1),
            &[(&header_text, Style::new().fg(Color::WHITE))]);
    }
}

impl Pane for WaveformPane {
    fn id(&self) -> &'static str {
        "waveform"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::Waveform(WaveformActionId::CycleMode) => {
                self.mode = self.mode.next();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        match self.mode {
            WaveformMode::Waveform => self.render_waveform(area, buf, state),
            WaveformMode::Spectrum => self.render_spectrum(area, buf, state),
            WaveformMode::Oscilloscope => self.render_oscilloscope(area, buf, state),
            WaveformMode::LufsMeter => self.render_lufs_meter(area, buf, state),
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
