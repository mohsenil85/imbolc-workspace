use std::any::Any;

use crate::state::{AppState, EqBandType, EqConfig, InstrumentId};
use crate::ui::action_id::{ActionId, EqActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, InstrumentAction, Keymap, Pane, Style};
use imbolc_types::EqParamKind;

use crate::state::instrument::EqBand;

pub struct EqPane {
    keymap: Keymap,
    selected_band: usize,   // 0-11
    selected_param: usize,  // 0=freq, 1=gain, 2=q, 3=enabled
}

impl EqPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected_band: 0,
            selected_param: 1, // default to gain
        }
    }
}

impl Default for EqPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for EqPane {
    fn id(&self) -> &'static str {
        "eq"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let instrument = match state.instruments.selected_instrument() {
            Some(i) => i,
            None => return Action::None,
        };
        let instrument_id = instrument.id;

        match action {
            ActionId::Eq(EqActionId::PrevBand) => {
                self.selected_band = self.selected_band.saturating_sub(1);
                Action::None
            }
            ActionId::Eq(EqActionId::NextBand) => {
                self.selected_band = (self.selected_band + 1).min(11);
                Action::None
            }
            ActionId::Eq(EqActionId::PrevParam) => {
                self.selected_param = self.selected_param.saturating_sub(1);
                Action::None
            }
            ActionId::Eq(EqActionId::NextParam) => {
                self.selected_param = (self.selected_param + 1).min(3);
                Action::None
            }
            ActionId::Eq(EqActionId::Increase) | ActionId::Eq(EqActionId::IncreaseBig) | ActionId::Eq(EqActionId::IncreaseTiny) => {
                adjust_param(instrument_id, instrument.eq(), self.selected_band, self.selected_param, true, action)
            }
            ActionId::Eq(EqActionId::Decrease) | ActionId::Eq(EqActionId::DecreaseBig) | ActionId::Eq(EqActionId::DecreaseTiny) => {
                adjust_param(instrument_id, instrument.eq(), self.selected_band, self.selected_param, false, action)
            }
            ActionId::Eq(EqActionId::ToggleEq) => {
                Action::Instrument(InstrumentAction::ToggleEq(instrument_id))
            }
            ActionId::Eq(EqActionId::ToggleBand) => {
                if let Some(eq) = instrument.eq() {
                    let band = &eq.bands[self.selected_band];
                    let new_val = if band.enabled { 0.0 } else { 1.0 };
                    Action::Instrument(InstrumentAction::SetEqParam(
                        instrument_id, self.selected_band, EqParamKind::Enabled, new_val,
                    ))
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 78, 24);

        let instrument = state.instruments.selected_instrument();
        let title = match instrument {
            Some(i) => format!(" EQ: {} ", i.name),
            None => " EQ: (none) ".to_string(),
        };

        let border_color = Color::new(100, 180, 255);
        let border_style = Style::new().fg(border_color);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let instrument = match instrument {
            Some(i) => i,
            None => {
                render_centered_text(inner, buf, "(no instrument selected)", Color::DARK_GRAY);
                return;
            }
        };

        let eq = match instrument.eq() {
            Some(eq) => eq,
            None => {
                render_centered_text(inner, buf, "EQ off — press 'e' to enable", Color::DARK_GRAY);
                return;
            }
        };

        // -- Frequency response curve --
        let curve_y = inner.y;
        let curve_height = inner.height.saturating_sub(10).max(4);
        let curve_width = inner.width.saturating_sub(6);
        let curve_x = inner.x + 5;

        render_frequency_curve(curve_x, curve_y, curve_width, curve_height, eq, self.selected_band, buf);

        // dB axis labels
        let db_labels = ["+24", "+12", "  0", "-12", "-24"];
        let db_style = Style::new().fg(Color::DARK_GRAY);
        for (i, label) in db_labels.iter().enumerate() {
            let y = curve_y + (i as u16) * (curve_height.saturating_sub(1)) / 4;
            if y < inner.y + inner.height {
                for (ci, ch) in label.chars().enumerate() {
                    let x = inner.x + ci as u16;
                    if x < inner.x + 4 {
                        buf.set_cell(x, y, ch, db_style);
                    }
                }
            }
        }

        // Frequency axis labels
        let freq_labels = ["20", "100", "500", "1k", "5k", "10k", "20k"];
        let freq_axis_y = curve_y + curve_height;
        let freq_style = Style::new().fg(Color::DARK_GRAY);
        if freq_axis_y < inner.y + inner.height {
            for (i, label) in freq_labels.iter().enumerate() {
                let frac = i as f32 / (freq_labels.len() - 1) as f32;
                let x = curve_x + (frac * (curve_width.saturating_sub(1) as f32)) as u16;
                for (ci, ch) in label.chars().enumerate() {
                    let px = x + ci as u16;
                    if px < inner.x + inner.width {
                        buf.set_cell(px, freq_axis_y, ch, freq_style);
                    }
                }
            }
        }

        // -- Band info (two rows of 6 bands each) --
        let info_y = freq_axis_y + 2;
        render_band_info(inner.x, info_y, inner.width, eq, self.selected_band, self.selected_param, buf);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// -- Helpers --

fn render_centered_text(area: Rect, buf: &mut RenderBuf, text: &str, color: Color) {
    let x = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
    let y = area.y + area.height / 2;
    let style = Style::new().fg(color);
    buf.draw_line(Rect::new(x, y, text.len() as u16, 1), &[(text, style)]);
}

fn adjust_param(
    instrument_id: InstrumentId,
    eq: Option<&EqConfig>,
    band_idx: usize,
    param_idx: usize,
    increase: bool,
    action: ActionId,
) -> Action {
    let eq = match eq {
        Some(eq) => eq,
        None => return Action::None,
    };
    let band = &eq.bands[band_idx];

    let (param, current, min, max) = match param_idx {
        0 => (EqParamKind::Freq, band.freq, 20.0, 20000.0),
        1 => (EqParamKind::Gain, band.gain, -24.0, 24.0),
        2 => (EqParamKind::Q, band.q, 0.1, 10.0),
        3 => return Action::None, // toggle handled by toggle_band
        _ => return Action::None,
    };

    let delta = match (param_idx, action) {
        // Freq: log-ish steps
        (0, ActionId::Eq(EqActionId::IncreaseBig)) | (0, ActionId::Eq(EqActionId::DecreaseBig)) => current * 0.2,
        (0, ActionId::Eq(EqActionId::IncreaseTiny)) | (0, ActionId::Eq(EqActionId::DecreaseTiny)) => current * 0.01,
        (0, ActionId::Eq(EqActionId::Increase)) | (0, ActionId::Eq(EqActionId::Decrease)) => current * 0.05,
        // Gain: dB steps
        (1, ActionId::Eq(EqActionId::IncreaseBig)) | (1, ActionId::Eq(EqActionId::DecreaseBig)) => 3.0,
        (1, ActionId::Eq(EqActionId::IncreaseTiny)) | (1, ActionId::Eq(EqActionId::DecreaseTiny)) => 0.1,
        (1, ActionId::Eq(EqActionId::Increase)) | (1, ActionId::Eq(EqActionId::Decrease)) => 0.5,
        // Q
        (2, ActionId::Eq(EqActionId::IncreaseBig)) | (2, ActionId::Eq(EqActionId::DecreaseBig)) => 1.0,
        (2, ActionId::Eq(EqActionId::IncreaseTiny)) | (2, ActionId::Eq(EqActionId::DecreaseTiny)) => 0.05,
        (2, ActionId::Eq(EqActionId::Increase)) | (2, ActionId::Eq(EqActionId::Decrease)) => 0.1,
        _ => 0.0,
    };

    let new_val = if increase {
        (current + delta).min(max)
    } else {
        (current - delta).max(min)
    };

    Action::Instrument(InstrumentAction::SetEqParam(
        instrument_id, band_idx, param, new_val,
    ))
}

/// Compute biquad magnitude response at a given frequency for one EQ band.
fn band_response_db(band: &EqBand, freq: f32) -> f32 {
    if !band.enabled || band.gain.abs() < 0.001 {
        return 0.0;
    }

    let w = freq / band.freq;
    let q = band.q.max(0.1);

    match band.band_type {
        EqBandType::Peaking => {
            let a = 10.0_f32.powf(band.gain / 40.0);
            let w2 = w * w;
            let num = (w2 - 1.0).powi(2) + (w * a / q).powi(2);
            let den = (w2 - 1.0).powi(2) + (w / (a * q)).powi(2);
            10.0 * (num / den).log10()
        }
        EqBandType::LowShelf => {
            let a = 10.0_f32.powf(band.gain / 20.0);
            let w2 = w * w;
            let blend = 1.0 / (1.0 + w2);
            let lin = a * blend + (1.0 - blend);
            20.0 * lin.log10()
        }
        EqBandType::HighShelf => {
            let a = 10.0_f32.powf(band.gain / 20.0);
            let w2 = w * w;
            let blend = w2 / (1.0 + w2);
            let lin = a * blend + (1.0 - blend);
            20.0 * lin.log10()
        }
    }
}

/// Compute composite EQ magnitude response at a given frequency.
fn composite_response_db(eq: &EqConfig, freq: f32) -> f32 {
    eq.bands.iter().map(|b| band_response_db(b, freq)).sum()
}

/// Render the frequency response curve.
fn render_frequency_curve(
    x: u16, y: u16, width: u16, height: u16,
    eq: &EqConfig,
    selected_band: usize,
    buf: &mut RenderBuf,
) {
    if width < 2 || height < 2 {
        return;
    }

    // Zero-line
    let grid_style = Style::new().fg(Color::new(40, 40, 40));
    let zero_row = y + height / 2;
    for col in x..x + width {
        buf.set_cell(col, zero_row, '-', grid_style);
    }

    // Compute response at each column (log-spaced 20Hz..20kHz)
    let curve_color = Color::new(100, 200, 255);
    let curve_style = Style::new().fg(curve_color);

    let db_range = 24.0_f32;
    let mut responses = Vec::with_capacity(width as usize);
    for col in 0..width {
        let frac = col as f32 / (width - 1) as f32;
        let freq = 20.0 * (1000.0_f32).powf(frac);
        let db = composite_response_db(eq, freq);
        responses.push(db);
    }

    // Map dB to row
    for (col, &db) in responses.iter().enumerate() {
        let frac = (-db / db_range + 1.0) / 2.0;
        let row_f = frac * (height - 1) as f32;
        let row = (row_f.round() as u16).min(height - 1);
        let px = x + col as u16;
        let py = y + row;
        if py >= y && py < y + height {
            let sub = (row_f - row_f.floor()) * 2.0;
            let ch = if sub < 1.0 { '\u{2584}' } else { '\u{2588}' };
            buf.set_cell(px, py, ch, curve_style);
        }
    }

    // Band markers
    for (i, band) in eq.bands.iter().enumerate() {
        let freq_frac = (band.freq / 20.0).log10() / 3.0;
        let col = (freq_frac * (width - 1) as f32).round() as u16;
        if col < width {
            let db = composite_response_db(eq, band.freq);
            let frac = (-db / db_range + 1.0) / 2.0;
            let row = (frac * (height - 1) as f32).round() as u16;
            let px = x + col;
            let py = y + row.min(height - 1);

            let marker_color = if i == selected_band {
                Color::new(255, 200, 50)
            } else if !band.enabled {
                Color::DARK_GRAY
            } else {
                Color::WHITE
            };
            buf.set_cell(px, py, '\u{25cf}', Style::new().fg(marker_color));
        }
    }
}

/// Render band info in two rows of 6 bands each.
fn render_band_info(
    x: u16, y: u16, width: u16,
    eq: &EqConfig,
    selected_band: usize,
    selected_param: usize,
    buf: &mut RenderBuf,
) {
    let bands_per_row = 6;
    let band_width = (width / bands_per_row as u16).max(10);
    let row_height = 4; // type+freq, gain, Q, on/off

    for (i, band) in eq.bands.iter().enumerate() {
        let row = i / bands_per_row;
        let col_in_row = i % bands_per_row;
        let bx = x + (col_in_row as u16) * band_width;
        let by = y + (row as u16) * (row_height + 1);
        let is_selected = i == selected_band;

        let type_color = if is_selected { Color::new(255, 200, 50) } else { Color::WHITE };

        // Row 0: type + freq
        let label = format!("{} {}",
            band.band_type.name(),
            format_freq(band.freq),
        );
        let label_style = Style::new().fg(if is_selected && selected_param == 0 { Color::new(255, 200, 50) } else { type_color });
        render_text_at(bx, by, &label, label_style, width, buf);

        // Row 1: gain
        let gain_str = format!("{:+.1}dB", band.gain);
        let gain_style = Style::new().fg(if is_selected && selected_param == 1 { Color::new(255, 200, 50) } else { Color::WHITE });
        render_text_at(bx, by + 1, &gain_str, gain_style, width, buf);

        // Row 2: Q
        let q_str = format!("Q:{:.2}", band.q);
        let q_style = Style::new().fg(if is_selected && selected_param == 2 { Color::new(255, 200, 50) } else { Color::WHITE });
        render_text_at(bx, by + 2, &q_str, q_style, width, buf);

        // Row 3: enabled
        let on_str = if band.enabled { "[ON]" } else { "[OFF]" };
        let on_color = if !band.enabled {
            Color::DARK_GRAY
        } else if is_selected && selected_param == 3 {
            Color::new(255, 200, 50)
        } else {
            Color::new(80, 200, 80)
        };
        render_text_at(bx, by + 3, on_str, Style::new().fg(on_color), width, buf);
    }
}

fn render_text_at(x: u16, y: u16, text: &str, style: Style, max_width: u16, buf: &mut RenderBuf) {
    for (i, ch) in text.chars().enumerate() {
        let px = x + i as u16;
        if px < x + max_width {
            buf.set_cell(px, y, ch, style);
        }
    }
}

fn format_freq(freq: f32) -> String {
    if freq >= 1000.0 {
        format!("{:.1}k", freq / 1000.0)
    } else {
        format!("{:.0}", freq)
    }
}
