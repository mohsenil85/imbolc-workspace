use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Color, Style};

use super::VstParamPane;

impl VstParamPane {
    pub(super) fn render_impl(&self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 80.min(area.width), 30.min(area.height));

        // Determine plugin name and instrument number
        let plugin_name = self.get_plugin_id(state)
            .and_then(|pid| state.session.vst_plugins.get(pid))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "—".to_string());
        let inst_label = self.instrument_id
            .map(|id| format!("Inst {}", id))
            .unwrap_or_else(|| "—".to_string());

        let title = match self.target {
            crate::action::VstTarget::Source => {
                format!(" VST Params: {} — {} ", plugin_name, inst_label)
            }
            crate::action::VstTarget::Effect(idx) => {
                format!(" VST Effect Params: {} — {} FX {} ", plugin_name, inst_label, idx)
            }
        };

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        // Search bar (top row)
        let search_y = inner.y;
        let search_style = if self.search_active {
            Style::new().fg(Color::WHITE)
        } else {
            Style::new().fg(Color::DARK_GRAY)
        };
        let search_text = if self.search_active || !self.search_text.is_empty() {
            format!("/ {}", self.search_text)
        } else {
            "/ [search]".to_string()
        };
        buf.draw_line(
            Rect::new(inner.x + 1, search_y, inner.width.saturating_sub(1), 1),
            &[(&search_text, search_style)],
        );

        // Param list area
        let list_y = search_y + 1;
        let list_height = inner.height.saturating_sub(2) as usize; // -1 for search, -1 for help

        // Get params to display
        let (params, param_values) = self.get_plugin_id(state)
            .and_then(|pid| state.session.vst_plugins.get(pid))
            .map(|plugin| {
                let param_vals = self.instrument_id
                    .and_then(|id| state.instruments.instrument(id))
                    .map(|inst| match self.target {
                        crate::action::VstTarget::Source => inst.vst_source_params().to_vec(),
                        crate::action::VstTarget::Effect(effect_id) => {
                            inst.effect_by_id(effect_id)
                                .map(|e| e.vst_param_values.clone())
                                .unwrap_or_default()
                        }
                    })
                    .unwrap_or_default();
                (plugin.params.clone(), param_vals)
            })
            .unwrap_or_default();

        if params.is_empty() {
            buf.draw_line(
                Rect::new(inner.x + 2, list_y + 1, inner.width.saturating_sub(2), 1),
                &[("No params discovered. Press 'd' to discover.", Style::new().fg(Color::DARK_GRAY))],
            );
        }

        // Adjust scroll offset
        let scroll = if self.selected_param >= self.scroll_offset + list_height {
            self.selected_param.saturating_sub(list_height - 1)
        } else {
            self.scroll_offset
        };

        let bar_width = (inner.width as usize).saturating_sub(30).max(8);

        for (row_idx, &filtered_idx) in self.filtered_indices.iter()
            .skip(scroll)
            .take(list_height)
            .enumerate()
        {
            let y = list_y + row_idx as u16;
            if y >= inner.y + inner.height - 1 { break; } // Leave room for help line

            let Some(spec) = params.get(filtered_idx) else { continue };

            let value = param_values.iter()
                .find(|(idx, _)| *idx == spec.index)
                .map(|(_, v)| *v)
                .unwrap_or(spec.default);

            let is_selected = scroll + row_idx == self.selected_param;

            // Indicator
            let indicator = if is_selected { ">" } else { " " };

            // Format: > 001 Cutoff          [===|======-------] 0.72 Hz
            let index_str = format!("{:03}", spec.index);
            let name = &spec.name;
            let value_str = format!("{:.2}", value);
            let label_suffix = spec.label.as_deref()
                .filter(|l| !l.is_empty())
                .map(|l| format!(" {}", l))
                .unwrap_or_default();

            // Build bar
            let filled = (value * bar_width as f32).round() as usize;
            let bar: String = (0..bar_width).map(|i| {
                if i < filled { '=' } else { '-' }
            }).collect();

            let line = format!(
                "{} {} {:<20} [{}] {}{}",
                indicator, index_str,
                if name.len() > 20 { &name[..20] } else { name },
                bar, value_str, label_suffix,
            );

            let style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::new(180, 180, 180))
            };

            buf.draw_line(
                Rect::new(inner.x + 1, y, inner.width.saturating_sub(1), 1),
                &[(&line, style)],
            );
            // Fill rest of row with selection bg if selected
            if is_selected {
                for x in (inner.x + 1 + line.len() as u16)..inner.x + inner.width {
                    buf.set_cell(x, y, ' ', style);
                }
            }
        }

    }
}
