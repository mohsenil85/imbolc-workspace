use super::InstrumentEditPane;
use crate::state::{AppState, Param, ParamValue};
use crate::ui::layout_helpers::center_rect;
use crate::ui::widgets::TextInput;
use crate::ui::{Color, Rect, RenderBuf, Style};
use imbolc_types::ProcessingStage;

impl InstrumentEditPane {
    pub(super) fn render_impl(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 97, 29);

        let title = format!(" Edit: {} ({}) ", self.instrument_name, self.source.name());
        let border_style = Style::new().fg(Color::ORANGE);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let content_x = inner.x + 1;
        let base_y = inner.y + 1;

        // Calculate visible content area (reserve 2 lines for help text at bottom)
        let raw_visible_height = inner.height.saturating_sub(2) as usize;
        let visual_overhead = self.visual_overhead();
        let effective_visible = raw_visible_height.saturating_sub(visual_overhead);
        let total_rows = self.total_rows();
        self.scroll_offset =
            Self::calc_scroll_offset(self.selected_row, total_rows, effective_visible);

        // Max Y we can render to (leave room for help text)
        let max_y = inner.y + inner.height.saturating_sub(2);

        // Track visual Y position for rendering
        let mut visual_y = base_y;

        // Mode indicators in header
        let mode_x = rect.x + rect.width - 18;
        let poly_style = Style::new().fg(if self.polyphonic {
            Color::LIME
        } else {
            Color::DARK_GRAY
        });
        let poly_str = if self.polyphonic { " POLY " } else { " MONO " };
        buf.draw_line(Rect::new(mode_x, rect.y, 6, 1), &[(poly_str, poly_style)]);

        // Channel config indicator (STEREO/MONO)
        let channel_style = Style::new().fg(if self.channel_config.is_stereo() {
            Color::CYAN
        } else {
            Color::YELLOW
        });
        let channel_str = if self.channel_config.is_stereo() {
            " ST "
        } else {
            " M "
        };
        buf.draw_line(
            Rect::new(mode_x + 6, rect.y, channel_str.len() as u16, 1),
            &[(channel_str, channel_style)],
        );

        // Active/Inactive indicator for AudioIn instruments
        if self.source.is_audio_input() {
            let active_style = Style::new().fg(if self.active {
                Color::LIME
            } else {
                Color::new(220, 40, 40)
            });
            let active_str = if self.active {
                " ACTIVE "
            } else {
                " INACTIVE "
            };
            let active_x = mode_x.saturating_sub(active_str.len() as u16 + 1);
            buf.draw_line(
                Rect::new(active_x, rect.y, active_str.len() as u16, 1),
                &[(active_str, active_style)],
            );
        }

        // Piano/Pad mode indicator
        if self.perf.pad.is_active() {
            let pad_str = self.perf.pad.status_label();
            let pad_style = Style::new().fg(Color::BLACK).bg(Color::KIT_COLOR);
            buf.draw_line(
                Rect::new(rect.x + 1, rect.y, pad_str.len() as u16, 1),
                &[(&pad_str, pad_style)],
            );
        } else if self.perf.piano.is_active() {
            let piano_str = self.perf.piano.status_label();
            let piano_style = Style::new().fg(Color::BLACK).bg(Color::PINK);
            buf.draw_line(
                Rect::new(rect.x + 1, rect.y, piano_str.len() as u16, 1),
                &[(&piano_str, piano_style)],
            );
        }

        let mut global_row = 0;

        // Helper to check if a row is in the visible range
        let is_visible = |row: usize| -> bool {
            row >= self.scroll_offset && row < self.scroll_offset + raw_visible_height
        };

        // === SOURCE SECTION ===
        let source_row_count = if self.source.is_sample() { 1 } else { 0 }
            + if self.source_params.is_empty() {
                1
            } else {
                self.source_params.len()
            };
        let source_start = global_row;
        let source_end = source_start + source_row_count;

        // Render source section if any rows visible
        if (source_start..source_end).any(&is_visible) && visual_y < max_y {
            let source_header = if self.source.is_sample() {
                format!("SOURCE: {}  (o: load)", self.source.name())
            } else {
                format!("SOURCE: {}", self.source.name())
            };
            buf.draw_line(
                Rect::new(content_x, visual_y, inner.width.saturating_sub(2), 1),
                &[(&source_header, Style::new().fg(Color::CYAN).bold())],
            );
            visual_y += 1;
        }

        // Sample name row for sampler instruments
        if self.source.is_sample() {
            if is_visible(global_row) && visual_y < max_y {
                let is_sel = self.selected_row == global_row;
                let display_name = self.sample_name.as_deref().unwrap_or("(no sample)");
                render_label_value_row_buf(
                    buf,
                    content_x,
                    visual_y,
                    "Sample",
                    display_name,
                    Color::CYAN,
                    is_sel,
                );
                visual_y += 1;
            }
            global_row += 1;
        }

        if self.source_params.is_empty() {
            if is_visible(global_row) && visual_y < max_y {
                let is_sel = self.selected_row == global_row;
                let style = if is_sel {
                    Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::DARK_GRAY)
                };
                buf.draw_line(
                    Rect::new(content_x + 2, visual_y, inner.width.saturating_sub(4), 1),
                    &[("(no parameters)", style)],
                );
                visual_y += 1;
            }
            global_row += 1;
        } else {
            for param in &self.source_params {
                if is_visible(global_row) && visual_y < max_y {
                    let is_sel = self.selected_row == global_row;
                    render_param_row_buf(
                        buf,
                        content_x,
                        visual_y,
                        param,
                        is_sel,
                        self.editing && is_sel,
                        &mut self.edit_input,
                    );
                    visual_y += 1;
                }
                global_row += 1;
            }
        }

        // Separator after source section
        if visual_y > base_y && visual_y < max_y {
            visual_y += 1;
        }

        // === PROCESSING CHAIN ===
        if self.processing_chain.is_empty() {
            // Placeholder row for empty chain
            if is_visible(global_row) && visual_y < max_y {
                let is_sel = self.selected_row == global_row;
                let style = if is_sel {
                    Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::DARK_GRAY)
                };
                buf.draw_line(
                    Rect::new(content_x + 2, visual_y, inner.width.saturating_sub(4), 1),
                    &[("(no processing)", style)],
                );
                visual_y += 1;
            }
            global_row += 1;
        } else {
            for stage in &self.processing_chain {
                match stage {
                    ProcessingStage::Filter(f) => {
                        let rc = stage.row_count();
                        let stage_start = global_row;
                        let stage_end = stage_start + rc;

                        // Filter header (non-selectable)
                        if (stage_start..stage_end).any(&is_visible) && visual_y < max_y {
                            let filter_label =
                                format!("FILTER: {}  (f: off, t: cycle)", f.filter_type.name());
                            buf.draw_line(
                                Rect::new(content_x, visual_y, inner.width.saturating_sub(2), 1),
                                &[(&filter_label, Style::new().fg(Color::FILTER_COLOR).bold())],
                            );
                            visual_y += 1;
                        }

                        // Type row
                        if is_visible(global_row) && visual_y < max_y {
                            let is_sel = self.selected_row == global_row;
                            render_label_value_row_buf(
                                buf,
                                content_x,
                                visual_y,
                                "Type",
                                f.filter_type.name(),
                                Color::FILTER_COLOR,
                                is_sel,
                            );
                            visual_y += 1;
                        }
                        global_row += 1;

                        // Cutoff row
                        if is_visible(global_row) && visual_y < max_y {
                            let is_sel = self.selected_row == global_row;
                            render_value_row_buf(
                                buf,
                                content_x,
                                visual_y,
                                "Cutoff",
                                f.cutoff.value,
                                f.cutoff.min,
                                f.cutoff.max,
                                is_sel,
                                self.editing && is_sel,
                                &mut self.edit_input,
                            );
                            visual_y += 1;
                        }
                        global_row += 1;

                        // Resonance row
                        if is_visible(global_row) && visual_y < max_y {
                            let is_sel = self.selected_row == global_row;
                            render_value_row_buf(
                                buf,
                                content_x,
                                visual_y,
                                "Resonance",
                                f.resonance.value,
                                f.resonance.min,
                                f.resonance.max,
                                is_sel,
                                self.editing && is_sel,
                                &mut self.edit_input,
                            );
                            visual_y += 1;
                        }
                        global_row += 1;

                        // Extra filter params
                        for param in &f.extra_params {
                            if is_visible(global_row) && visual_y < max_y {
                                let is_sel = self.selected_row == global_row;
                                render_param_row_buf(
                                    buf,
                                    content_x,
                                    visual_y,
                                    param,
                                    is_sel,
                                    self.editing && is_sel,
                                    &mut self.edit_input,
                                );
                                visual_y += 1;
                            }
                            global_row += 1;
                        }
                    }
                    ProcessingStage::Eq(_eq) => {
                        // Single selectable row: "EQ [ON]  (e: toggle)"
                        if is_visible(global_row) && visual_y < max_y {
                            let is_sel = self.selected_row == global_row;
                            let eq_text = "EQ [ON]  (e: toggle)";
                            render_label_value_row_buf(
                                buf,
                                content_x,
                                visual_y,
                                "EQ",
                                "ON",
                                Color::FILTER_COLOR,
                                is_sel,
                            );
                            let _ = eq_text; // suppress unused
                            visual_y += 1;
                        }
                        global_row += 1;
                    }
                    ProcessingStage::Effect(effect) => {
                        // Header row: effect name + enabled badge
                        if is_visible(global_row) && visual_y < max_y {
                            let is_sel = self.selected_row == global_row;
                            if is_sel {
                                buf.set_cell(
                                    content_x,
                                    visual_y,
                                    '>',
                                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
                                );
                            }

                            let enabled_str = if effect.enabled { "ON " } else { "OFF" };
                            let effect_text =
                                format!("{:10} [{}]", effect.effect_type.name(), enabled_str);
                            let effect_style = if is_sel {
                                Style::new().fg(Color::FX_COLOR).bg(Color::SELECTION_BG)
                            } else {
                                Style::new().fg(Color::FX_COLOR)
                            };
                            buf.draw_line(
                                Rect::new(content_x + 2, visual_y, 18, 1),
                                &[(&effect_text, effect_style)],
                            );
                            visual_y += 1;
                        }
                        global_row += 1;

                        // Per-param rows with sliders
                        for param in &effect.params {
                            if is_visible(global_row) && visual_y < max_y {
                                let is_sel = self.selected_row == global_row;
                                render_param_row_buf(
                                    buf,
                                    content_x,
                                    visual_y,
                                    param,
                                    is_sel,
                                    self.editing && is_sel,
                                    &mut self.edit_input,
                                );
                                visual_y += 1;
                            }
                            global_row += 1;
                        }
                    }
                }
            }
        }

        // Separator after processing chain
        if visual_y < max_y {
            visual_y += 1;
        }

        // === LFO SECTION ===
        let lfo_row_count = 4;
        let lfo_start = global_row;
        let lfo_end = lfo_start + lfo_row_count;

        if (lfo_start..lfo_end).any(&is_visible) && visual_y < max_y {
            let lfo_status = if self.lfo.enabled { "ON" } else { "OFF" };
            let lfo_header = format!("LFO [{}]  (l: toggle, s: shape, m: target)", lfo_status);
            buf.draw_line(
                Rect::new(content_x, visual_y, inner.width.saturating_sub(2), 1),
                &[(&lfo_header, Style::new().fg(Color::PINK).bold())],
            );
            visual_y += 1;
        }

        // Row 0: Enabled
        if is_visible(global_row) && visual_y < max_y {
            let is_sel = self.selected_row == global_row;
            let enabled_val = if self.lfo.enabled { "ON" } else { "OFF" };
            render_label_value_row_buf(
                buf,
                content_x,
                visual_y,
                "Enabled",
                enabled_val,
                Color::PINK,
                is_sel,
            );
            visual_y += 1;
        }
        global_row += 1;

        // Row 1: Rate
        if is_visible(global_row) && visual_y < max_y {
            let is_sel = self.selected_row == global_row;
            render_value_row_buf(
                buf,
                content_x,
                visual_y,
                "Rate",
                self.lfo.rate,
                0.1,
                32.0,
                is_sel,
                self.editing && is_sel,
                &mut self.edit_input,
            );
            // Hz label
            let hz_style = if is_sel {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            for (j, ch) in "Hz".chars().enumerate() {
                buf.set_cell(content_x + 44 + j as u16, visual_y, ch, hz_style);
            }
            visual_y += 1;
        }
        global_row += 1;

        // Row 2: Depth
        if is_visible(global_row) && visual_y < max_y {
            let is_sel = self.selected_row == global_row;
            render_value_row_buf(
                buf,
                content_x,
                visual_y,
                "Depth",
                self.lfo.depth,
                0.0,
                1.0,
                is_sel,
                self.editing && is_sel,
                &mut self.edit_input,
            );
            visual_y += 1;
        }
        global_row += 1;

        // Row 3: Shape and Target
        if is_visible(global_row) && visual_y < max_y {
            let is_sel = self.selected_row == global_row;
            let shape_val = format!("{} → {}", self.lfo.shape.name(), self.lfo.target.name());
            render_label_value_row_buf(
                buf,
                content_x,
                visual_y,
                "Shape/Dest",
                &shape_val,
                Color::PINK,
                is_sel,
            );
            visual_y += 1;
        }
        global_row += 1;

        // Separator after LFO section
        if visual_y < max_y {
            visual_y += 1;
        }

        // === ENVELOPE SECTION === (hidden for VSTi — plugin has own envelope)
        if !self.source.is_vst() {
            let env_row_count = 4;
            let env_start = global_row;
            let env_end = env_start + env_row_count;

            if (env_start..env_end).any(&is_visible) && visual_y < max_y {
                buf.draw_line(
                    Rect::new(content_x, visual_y, inner.width.saturating_sub(2), 1),
                    &[(
                        "ENVELOPE (ADSR)  (p: poly, r: track)",
                        Style::new().fg(Color::ENV_COLOR).bold(),
                    )],
                );
                visual_y += 1;
            }

            // Get source-type-specific default envelope
            let default_env = self.source.default_envelope();

            let env_labels = ["Attack", "Decay", "Sustain", "Release"];
            let env_values = [
                self.amp_envelope.attack,
                self.amp_envelope.decay,
                self.amp_envelope.sustain,
                self.amp_envelope.release,
            ];
            let env_defaults = [
                default_env.attack,
                default_env.decay,
                default_env.sustain,
                default_env.release,
            ];
            let env_maxes = [5.0, 5.0, 1.0, 5.0];

            for ((label, val), (default, max)) in env_labels
                .iter()
                .zip(env_values.iter())
                .zip(env_defaults.iter().zip(env_maxes.iter()))
            {
                if is_visible(global_row) && visual_y < max_y {
                    let is_sel = self.selected_row == global_row;
                    render_value_row_with_default_buf(
                        buf,
                        content_x,
                        visual_y,
                        label,
                        *val,
                        0.0,
                        *max,
                        Some(*default),
                        is_sel,
                        self.editing && is_sel,
                        &mut self.edit_input,
                    );
                    visual_y += 1;
                }
                global_row += 1;
            }
        }

        // Suppress unused variable warning
        let _ = global_row;
        let _ = visual_y;
    }
}

fn render_slider_with_default(
    value: f32,
    min: f32,
    max: f32,
    width: usize,
    default: Option<f32>,
) -> String {
    let normalized = (value - min) / (max - min);
    let pos = (normalized * width as f32) as usize;
    let pos = pos.min(width);

    // Calculate default marker position if provided
    let default_pos = default
        .map(|d| {
            let d_norm = (d - min) / (max - min);
            (d_norm * width as f32) as usize
        })
        .map(|p| p.min(width));

    let mut s = String::with_capacity(width + 2);
    s.push('[');
    for i in 0..width {
        if i == pos {
            s.push('|');
        } else if Some(i) == default_pos {
            // Show default marker (only if current value isn't there)
            s.push('▾');
        } else if i < pos {
            s.push('=');
        } else {
            s.push('-');
        }
    }
    s.push(']');
    s
}

fn render_slider(value: f32, min: f32, max: f32, width: usize) -> String {
    render_slider_with_default(value, min, max, width, None)
}

fn render_param_row_buf(
    buf: &mut RenderBuf,
    x: u16,
    y: u16,
    param: &Param,
    is_selected: bool,
    is_editing: bool,
    edit_input: &mut TextInput,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(
            x,
            y,
            '>',
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
        );
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

#[allow(clippy::too_many_arguments)]
fn render_value_row_buf(
    buf: &mut RenderBuf,
    x: u16,
    y: u16,
    name: &str,
    value: f32,
    min: f32,
    max: f32,
    is_selected: bool,
    is_editing: bool,
    edit_input: &mut TextInput,
) {
    render_value_row_with_default_buf(
        buf,
        x,
        y,
        name,
        value,
        min,
        max,
        None,
        is_selected,
        is_editing,
        edit_input,
    );
}

/// Render a value row with an optional default marker
#[allow(clippy::too_many_arguments)]
fn render_value_row_with_default_buf(
    buf: &mut RenderBuf,
    x: u16,
    y: u16,
    name: &str,
    value: f32,
    min: f32,
    max: f32,
    default: Option<f32>,
    is_selected: bool,
    is_editing: bool,
    edit_input: &mut TextInput,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(
            x,
            y,
            '>',
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
        );
    }

    // Label with optional default annotation
    let name_style = if is_selected {
        Style::new().fg(Color::CYAN).bg(Color::SELECTION_BG)
    } else {
        Style::new().fg(Color::CYAN)
    };
    let name_str = format!("{:12}", name);
    for (j, ch) in name_str.chars().enumerate() {
        buf.set_cell(x + 2 + j as u16, y, ch, name_style);
    }

    // Slider with default marker
    let slider = render_slider_with_default(value, min, max, 16, default);
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

        // Show default value annotation after current value
        if let Some(def) = default {
            let def_style = if is_selected {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let def_str = format!(" (def: {:.2})", def);
            for (j, ch) in def_str.chars().enumerate() {
                buf.set_cell(x + 34 + formatted.len() as u16 + j as u16, y, ch, def_style);
            }
        }
    }
}

/// Render a label-value row (no slider, for type/enabled/shape rows)
fn render_label_value_row_buf(
    buf: &mut RenderBuf,
    x: u16,
    y: u16,
    label: &str,
    value: &str,
    color: Color,
    is_selected: bool,
) {
    // Selection indicator
    if is_selected {
        buf.set_cell(
            x,
            y,
            '>',
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
        );
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
