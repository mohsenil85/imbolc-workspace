use super::InstrumentEditPane;
use crate::state::{AppState, Param, ParamValue};
use crate::ui::layout_helpers::center_rect;
use crate::ui::widgets::TextInput;
use crate::ui::{Rect, RenderBuf, Color, Style};

impl InstrumentEditPane {
    pub(super) fn render_impl(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 97, 29);

        let title = format!(" Edit: {} ({}) ", self.instrument_name, self.source.name());
        let border_style = Style::new().fg(Color::ORANGE);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let content_x = inner.x + 1;
        let mut y = inner.y + 1;

        // Mode indicators in header
        let mode_x = rect.x + rect.width - 18;
        let poly_style = Style::new().fg(if self.polyphonic { Color::LIME } else { Color::DARK_GRAY });
        let poly_str = if self.polyphonic { " POLY " } else { " MONO " };
        buf.draw_line(Rect::new(mode_x, rect.y, 6, 1), &[(poly_str, poly_style)]);

        // Active/Inactive indicator for AudioIn instruments
        if self.source.is_audio_input() {
            let active_style = Style::new().fg(
                if self.active { Color::LIME } else { Color::new(220, 40, 40) }
            );
            let active_str = if self.active { " ACTIVE " } else { " INACTIVE " };
            let active_x = mode_x.saturating_sub(active_str.len() as u16 + 1);
            buf.draw_line(Rect::new(active_x, rect.y, active_str.len() as u16, 1), &[(active_str, active_style)]);
        }

        // Piano/Pad mode indicator
        if self.pad_keyboard.is_active() {
            let pad_str = self.pad_keyboard.status_label();
            let pad_style = Style::new().fg(Color::BLACK).bg(Color::KIT_COLOR);
            buf.draw_line(Rect::new(rect.x + 1, rect.y, pad_str.len() as u16, 1), &[(&pad_str, pad_style)]);
        } else if self.piano.is_active() {
            let piano_str = self.piano.status_label();
            let piano_style = Style::new().fg(Color::BLACK).bg(Color::PINK);
            buf.draw_line(Rect::new(rect.x + 1, rect.y, piano_str.len() as u16, 1), &[(&piano_str, piano_style)]);
        }

        let mut global_row = 0;

        // === SOURCE SECTION ===
        let source_header = if self.source.is_sample() {
            format!("SOURCE: {}  (o: load)", self.source.name())
        } else {
            format!("SOURCE: {}", self.source.name())
        };
        buf.draw_line(Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
            &[(&source_header, Style::new().fg(Color::CYAN).bold())]);
        y += 1;

        // Sample name row for sampler instruments
        if self.source.is_sample() {
            let is_sel = self.selected_row == global_row;
            let display_name = self.sample_name.as_deref().unwrap_or("(no sample)");
            render_label_value_row_buf(buf, content_x, y, "Sample", display_name, Color::CYAN, is_sel);
            y += 1;
            global_row += 1;
        }

        if self.source_params.is_empty() {
            let is_sel = self.selected_row == global_row;
            let style = if is_sel {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            buf.draw_line(Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1), &[("(no parameters)", style)]);
            global_row += 1;
        } else {
            for param in &self.source_params {
                let is_sel = self.selected_row == global_row;
                render_param_row_buf(buf, content_x, y, param, is_sel, self.editing && is_sel, &mut self.edit_input);
                y += 1;
                global_row += 1;
            }
        }
        y += 1;

        // === FILTER SECTION ===
        let filter_label = if let Some(ref f) = self.filter {
            format!("FILTER: {}  (f: off, t: cycle)", f.filter_type.name())
        } else {
            "FILTER: OFF  (f: enable)".to_string()
        };
        buf.draw_line(Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
            &[(&filter_label, Style::new().fg(Color::FILTER_COLOR).bold())]);
        y += 1;

        if let Some(ref f) = self.filter {
            // Type row
            {
                let is_sel = self.selected_row == global_row;
                render_label_value_row_buf(buf, content_x, y, "Type", &f.filter_type.name(), Color::FILTER_COLOR, is_sel);
                y += 1;
                global_row += 1;
            }
            // Cutoff row
            {
                let is_sel = self.selected_row == global_row;
                render_value_row_buf(buf, content_x, y, "Cutoff", f.cutoff.value, f.cutoff.min, f.cutoff.max, is_sel, self.editing && is_sel, &mut self.edit_input);
                y += 1;
                global_row += 1;
            }
            // Resonance row
            {
                let is_sel = self.selected_row == global_row;
                render_value_row_buf(buf, content_x, y, "Resonance", f.resonance.value, f.resonance.min, f.resonance.max, is_sel, self.editing && is_sel, &mut self.edit_input);
                y += 1;
                global_row += 1;
            }
            // Extra filter params (e.g. shape for Vowel, drive for ResDrive)
            for param in &f.extra_params {
                let is_sel = self.selected_row == global_row;
                render_param_row_buf(buf, content_x, y, param, is_sel, self.editing && is_sel, &mut self.edit_input);
                y += 1;
                global_row += 1;
            }
        } else {
            let is_sel = self.selected_row == global_row;
            let style = if is_sel {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            buf.draw_line(Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1), &[("(disabled)", style)]);
            y += 1;
            global_row += 1;
        }
        y += 1;

        // === EFFECTS SECTION ===
        buf.draw_line(Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
            &[("EFFECTS  (a: add effect, d: remove)", Style::new().fg(Color::FX_COLOR).bold())]);
        y += 1;

        if self.effects.is_empty() {
            let is_sel = self.selected_row == global_row;
            let style = if is_sel {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            buf.draw_line(Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1), &[("(no effects)", style)]);
            global_row += 1;
        } else {
            for effect in &self.effects {
                // Header row: effect name + enabled badge
                let is_sel = self.selected_row == global_row;
                if is_sel {
                    buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                }

                let enabled_str = if effect.enabled { "ON " } else { "OFF" };
                let effect_text = format!("{:10} [{}]", effect.effect_type.name(), enabled_str);
                let effect_style = if is_sel {
                    Style::new().fg(Color::FX_COLOR).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::FX_COLOR)
                };
                buf.draw_line(Rect::new(content_x + 2, y, 18, 1), &[(&effect_text, effect_style)]);

                y += 1;
                global_row += 1;

                // Per-param rows with sliders
                for param in &effect.params {
                    let is_sel = self.selected_row == global_row;
                    render_param_row_buf(buf, content_x, y, param, is_sel, self.editing && is_sel, &mut self.edit_input);
                    y += 1;
                    global_row += 1;
                }
            }
        }
        y += 1;

        // === LFO SECTION ===
        let lfo_status = if self.lfo.enabled { "ON" } else { "OFF" };
        let lfo_header = format!("LFO [{}]  (l: toggle, s: shape, m: target)", lfo_status);
        buf.draw_line(Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
            &[(&lfo_header, Style::new().fg(Color::PINK).bold())]);
        y += 1;

        // Row 0: Enabled
        {
            let is_sel = self.selected_row == global_row;
            let enabled_val = if self.lfo.enabled { "ON" } else { "OFF" };
            render_label_value_row_buf(buf, content_x, y, "Enabled", enabled_val, Color::PINK, is_sel);
            y += 1;
            global_row += 1;
        }

        // Row 1: Rate
        {
            let is_sel = self.selected_row == global_row;
            render_value_row_buf(buf, content_x, y, "Rate", self.lfo.rate, 0.1, 32.0, is_sel, self.editing && is_sel, &mut self.edit_input);
            // Hz label
            let hz_style = if is_sel {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            for (j, ch) in "Hz".chars().enumerate() {
                buf.set_cell(content_x + 44 + j as u16, y, ch, hz_style);
            }
            y += 1;
            global_row += 1;
        }

        // Row 2: Depth
        {
            let is_sel = self.selected_row == global_row;
            render_value_row_buf(buf, content_x, y, "Depth", self.lfo.depth, 0.0, 1.0, is_sel, self.editing && is_sel, &mut self.edit_input);
            y += 1;
            global_row += 1;
        }

        // Row 3: Shape and Target
        {
            let is_sel = self.selected_row == global_row;
            let shape_val = format!("{} → {}", self.lfo.shape.name(), self.lfo.target.name());
            render_label_value_row_buf(buf, content_x, y, "Shape/Dest", &shape_val, Color::PINK, is_sel);
            y += 1;
            global_row += 1;
        }
        y += 1;

        // === ENVELOPE SECTION === (hidden for VSTi — plugin has own envelope)
        if !self.source.is_vst() {
            buf.draw_line(Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
                &[("ENVELOPE (ADSR)  (p: poly, r: track)", Style::new().fg(Color::ENV_COLOR).bold())]);
            y += 1;

            let env_labels = ["Attack", "Decay", "Sustain", "Release"];
            let env_values = [
                self.amp_envelope.attack,
                self.amp_envelope.decay,
                self.amp_envelope.sustain,
                self.amp_envelope.release,
            ];
            let env_maxes = [5.0, 5.0, 1.0, 5.0];

            for (label, (val, max)) in env_labels.iter().zip(env_values.iter().zip(env_maxes.iter())) {
                let is_sel = self.selected_row == global_row;
                render_value_row_buf(buf, content_x, y, label, *val, 0.0, *max, is_sel, self.editing && is_sel, &mut self.edit_input);
                y += 1;
                global_row += 1;
            }
        }

        // Suppress unused variable warning
        let _ = global_row;

        // Help text
        let help_y = rect.y + rect.height - 2;
        let help_text = if self.pad_keyboard.is_active() {
            "R T Y U / F G H J / V B N M: trigger pads | /: cycle | Esc: exit"
        } else if self.piano.is_active() {
            "Play keys | [/]: octave | \u{2190}/\u{2192}: adjust | \\: zero | /: cycle | Esc: exit"
        } else {
            "\u{2191}/\u{2193}: move | Tab/S-Tab: section | \u{2190}/\u{2192}: adjust | \\: zero | /: piano"
        };
        buf.draw_line(Rect::new(content_x, help_y, inner.width.saturating_sub(2), 1),
            &[(help_text, Style::new().fg(Color::DARK_GRAY))]);
    }
}

fn render_slider(value: f32, min: f32, max: f32, width: usize) -> String {
    let normalized = (value - min) / (max - min);
    let pos = (normalized * width as f32) as usize;
    let pos = pos.min(width);
    let mut s = String::with_capacity(width + 2);
    s.push('[');
    for i in 0..width {
        if i == pos { s.push('|'); }
        else if i < pos { s.push('='); }
        else { s.push('-'); }
    }
    s.push(']');
    s
}

fn render_param_row_buf(
    buf: &mut RenderBuf,
    x: u16, y: u16,
    param: &Param,
    is_selected: bool,
    is_editing: bool,
    edit_input: &mut TextInput,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
    }

    // Param name
    let name_style = if is_selected {
        Style::new().fg(Color::CYAN).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(Color::CYAN)
    };
    let name_str = format!("{:12}", param.name);
    for (j, ch) in name_str.chars().enumerate() {
        buf.set_cell(x + 2 + j as u16, y, ch, name_style);
    }

    // Slider
    let (val, min, max) = match &param.value {
        ParamValue::Float(v) => (*v, param.min, param.max),
        ParamValue::Int(v) => (*v as f32, param.min, param.max),
        ParamValue::Bool(v) => (if *v { 1.0 } else { 0.0 }, 0.0, 1.0),
    };
    let slider = render_slider(val, min, max, 16);
    let slider_style = if is_selected {
        Style::new().fg(Color::LIME).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(Color::LIME)
    };
    for (j, ch) in slider.chars().enumerate() {
        buf.set_cell(x + 15 + j as u16, y, ch, slider_style);
    }

    // Value or text input
    if is_editing {
        edit_input.render_buf(buf.raw_buf(), x + 34, y, 10);
    } else {
        let value_str = match &param.value {
            ParamValue::Float(v) => format!("{:.2}", v),
            ParamValue::Int(v) => format!("{}", v),
            ParamValue::Bool(v) => format!("{}", v),
        };
        let val_style = if is_selected {
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
        } else {
            Style::new().fg(Color::WHITE)
        };
        let formatted = format!("{:10}", value_str);
        for (j, ch) in formatted.chars().enumerate() {
            buf.set_cell(x + 34 + j as u16, y, ch, val_style);
        }
    }
}

fn render_value_row_buf(
    buf: &mut RenderBuf,
    x: u16, y: u16,
    name: &str,
    value: f32, min: f32, max: f32,
    is_selected: bool,
    is_editing: bool,
    edit_input: &mut TextInput,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
    }

    // Label
    let name_style = if is_selected {
        Style::new().fg(Color::CYAN).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(Color::CYAN)
    };
    let name_str = format!("{:12}", name);
    for (j, ch) in name_str.chars().enumerate() {
        buf.set_cell(x + 2 + j as u16, y, ch, name_style);
    }

    // Slider
    let slider = render_slider(value, min, max, 16);
    let slider_style = if is_selected {
        Style::new().fg(Color::LIME).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(Color::LIME)
    };
    for (j, ch) in slider.chars().enumerate() {
        buf.set_cell(x + 15 + j as u16, y, ch, slider_style);
    }

    // Value or text input
    if is_editing {
        edit_input.render_buf(buf.raw_buf(), x + 34, y, 10);
    } else {
        let val_style = if is_selected {
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
        } else {
            Style::new().fg(Color::WHITE)
        };
        let formatted = format!("{:.2}", value);
        for (j, ch) in formatted.chars().enumerate() {
            buf.set_cell(x + 34 + j as u16, y, ch, val_style);
        }
    }
}

/// Render a label-value row (no slider, for type/enabled/shape rows)
fn render_label_value_row_buf(
    buf: &mut RenderBuf,
    x: u16, y: u16,
    label: &str,
    value: &str,
    color: Color,
    is_selected: bool,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
    }

    let text = format!("{:12}  {}", label, value);
    let style = if is_selected {
        Style::new().fg(color).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(color)
    };
    for (j, ch) in text.chars().enumerate() {
        buf.set_cell(x + 2 + j as u16, y, ch, style);
    }
}
