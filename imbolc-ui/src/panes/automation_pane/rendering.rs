use crate::state::automation::{AutomationTargetExt, CurveType};
use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Color, Style};

use super::{AutomationFocus, AutomationPane, TargetPickerState};

/// Block characters for mini value graph (8 levels)
#[allow(dead_code)]
pub(super) const BLOCK_CHARS: [char; 8] = [
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
    '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

impl AutomationPane {
    pub(super) fn render_lane_list(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        if area.height < 2 || area.width < 10 {
            return;
        }

        let automation = &state.session.automation;

        // Filter lanes for the currently selected instrument (plus global lanes)
        let inst_id = state.instruments.selected_instrument().map(|i| i.id);
        let visible_lanes: Vec<(usize, &crate::state::automation::AutomationLane)> = automation
            .lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                match l.target.instrument_id() {
                    Some(id) => inst_id == Some(id),
                    None => true, // Global targets always visible
                }
            })
            .collect();

        if visible_lanes.is_empty() {
            let text = "(no automation lanes)";
            let style = Style::new().fg(Color::DARK_GRAY);
            let x = area.x + 1;
            let y = area.y;
            buf.draw_line(Rect::new(x, y, text.len() as u16, 1), &[(text, style)]);
            return;
        }

        // Header
        let header = format!("{:<6} {:<16} {:>3} {:>2} {:>4} {:<6}", "Lane", "Target", "En", "R", "Pts", "Curve");
        let header_style = Style::new().fg(Color::DARK_GRAY);
        for (i, ch) in header.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width { break; }
            buf.set_cell(area.x + 1 + i as u16, area.y, ch, header_style);
        }

        for (vi, (global_idx, lane)) in visible_lanes.iter().enumerate() {
            let y = area.y + 1 + vi as u16;
            if y >= area.y + area.height { break; }

            let is_selected = automation.selected_lane == Some(*global_idx);
            let in_focus = self.focus == AutomationFocus::LaneList;

            let enabled_char = if lane.enabled { "x" } else { " " };
            let point_count = lane.points.len();
            let curve_name = if let Some(p) = lane.points.first() {
                match p.curve {
                    CurveType::Linear => "Linear",
                    CurveType::Exponential => "Exp",
                    CurveType::Step => "Step",
                    CurveType::SCurve => "SCurve",
                }
            } else {
                "Linear"
            };

            let short = lane.target.short_name();
            let name = lane.target.name();
            let arm_char = if lane.record_armed { "R" } else { " " };
            let line_text = format!(
                "{}{:<5} {:<16} [{}] {} {:>3} {:<6}",
                if is_selected { ">" } else { " " },
                short,
                name,
                enabled_char,
                arm_char,
                point_count,
                curve_name
            );

            let style = if is_selected && in_focus {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::new(30, 30, 40))
            } else if !lane.enabled {
                Style::new().fg(Color::DARK_GRAY)
            } else {
                Style::new().fg(Color::GRAY)
            };

            // Arm indicator position: 1 (marker) + 5 (short) + 1 (sp) + 16 (name) + 2 (" [") + 1 (en) + 2 ("] ") = 28
            let arm_pos = 28;
            let arm_style = Style::new().fg(Color::MUTE_COLOR).bold();
            for (i, ch) in line_text.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width { break; }
                let cell_style = if lane.record_armed && i == arm_pos {
                    arm_style
                } else {
                    style
                };
                buf.set_cell(x, y, ch, cell_style);
            }
            // Fill remaining width for selected row
            if is_selected {
                for x in (area.x + line_text.len() as u16)..(area.x + area.width) {
                    buf.set_cell(x, y, ' ', style);
                }
            }
        }
    }

    pub(super) fn render_timeline(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        let automation = &state.session.automation;
        let lane = match automation.selected() {
            Some(l) => l,
            None => {
                let text = "(select a lane)";
                let style = Style::new().fg(Color::DARK_GRAY);
                let x = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
                let y = area.y + area.height / 2;
                buf.draw_line(Rect::new(x, y, text.len() as u16, 1), &[(text, style)]);
                return;
            }
        };

        let tpc = self.ticks_per_cell();
        let graph_height = area.height.saturating_sub(2); // Reserve 1 for beat markers, 1 for status
        let graph_width = area.width;
        let graph_y = area.y;

        // Draw the value graph area
        let bg_style = Style::new().fg(Color::new(30, 30, 30));
        let _beat_style = Style::new().fg(Color::new(45, 45, 45));
        let bar_style = Style::new().fg(Color::new(55, 55, 55));
        let in_focus = self.focus == AutomationFocus::Timeline;

        // Grid dots
        for col in 0..graph_width {
            let tick = self.view_start_tick + col as u32 * tpc;
            let is_bar = tick % 1920 == 0; // 4 beats
            let is_beat = tick % 480 == 0;

            for row in 0..graph_height {
                let y = graph_y + row;
                let x = area.x + col;
                if is_bar {
                    buf.set_cell(x, y, '┊', bar_style);
                } else if is_beat && row == 0 {
                    buf.set_cell(x, y, '·', bg_style);
                }
            }
        }

        // Draw automation curve
        let curve_color = if lane.enabled { Color::CYAN } else { Color::DARK_GRAY };
        let curve_style = Style::new().fg(curve_color);
        let point_style = Style::new().fg(Color::WHITE).bg(curve_color);

        if !lane.points.is_empty() && graph_height > 0 {
            for col in 0..graph_width {
                let tick = self.view_start_tick + col as u32 * tpc;
                if let Some(raw_value) = lane.value_at(tick) {
                    // Convert from actual range to normalized 0-1
                    let normalized = if lane.max_value > lane.min_value {
                        (raw_value - lane.min_value) / (lane.max_value - lane.min_value)
                    } else {
                        0.5
                    };
                    let row = ((1.0 - normalized) * (graph_height.saturating_sub(1)) as f32) as u16;
                    let y = graph_y + row;
                    let x = area.x + col;
                    if y < graph_y + graph_height {
                        // Check if there's a point exactly at this tick
                        if lane.point_at(tick).is_some() {
                            buf.set_cell(x, y, '●', point_style);
                        } else {
                            buf.set_cell(x, y, '─', curve_style);
                        }
                    }
                }
            }
        }

        // Draw cursor
        if in_focus {
            let cursor_col = if self.cursor_tick >= self.view_start_tick {
                ((self.cursor_tick - self.view_start_tick) / tpc) as u16
            } else {
                0
            };
            let cursor_row = ((1.0 - self.cursor_value) * (graph_height.saturating_sub(1)) as f32) as u16;

            if cursor_col < graph_width {
                let x = area.x + cursor_col;

                // Vertical line at cursor tick
                for row in 0..graph_height {
                    let y = graph_y + row;
                    if let Some(cell) = buf.raw_buf().cell_mut((x, y)) {
                        if row == cursor_row {
                            cell.set_char('◆').set_style(Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG));
                        } else if cell.symbol() == " " {
                            cell.set_char('│').set_style(Style::new().fg(Color::new(50, 50, 60)));
                        }
                    }
                }
            }
        }

        // Beat markers row
        let marker_y = graph_y + graph_height;
        if marker_y < area.y + area.height {
            let marker_style = Style::new().fg(Color::DARK_GRAY);
            for col in 0..graph_width {
                let tick = self.view_start_tick + col as u32 * tpc;
                if tick % 1920 == 0 {
                    // Bar number
                    let bar = tick / 1920 + 1;
                    let label = format!("B{}", bar);
                    for (j, ch) in label.chars().enumerate() {
                        let x = area.x + col + j as u16;
                        if x < area.x + graph_width {
                            buf.set_cell(x, marker_y, ch, marker_style);
                        }
                    }
                }
            }
        }

        // Status line
        let status_y = graph_y + graph_height + 1;
        if status_y < area.y + area.height {
            let curve_at_cursor = lane.point_at(self.cursor_tick)
                .map(|p| match p.curve {
                    CurveType::Linear => "Linear",
                    CurveType::Exponential => "Exp",
                    CurveType::Step => "Step",
                    CurveType::SCurve => "SCurve",
                })
                .unwrap_or("—");

            let rec_indicator = if state.recording.automation_recording { " [REC]" } else { "" };
            let status = format!(
                " Tick:{:<6} Val:{:.2}  Curve:{}{}",
                self.cursor_tick,
                self.cursor_value,
                curve_at_cursor,
                rec_indicator,
            );

            let normal_style = Style::new().fg(Color::GRAY);
            let rec_style = Style::new().fg(Color::WHITE).bg(Color::RED);

            // Render status text
            for (i, ch) in status.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + graph_width { break; }
                // Use red style for [REC]
                let is_rec_section = state.recording.automation_recording
                    && i >= status.len() - 6;
                let style = if is_rec_section { rec_style } else { normal_style };
                buf.set_cell(x, status_y, ch, style);
            }
        }
    }

    pub(super) fn render_target_picker(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        if let TargetPickerState::Active { ref options, cursor } = self.target_picker {
            let picker_width = 40u16.min(area.width.saturating_sub(4));
            let picker_height = (options.len() as u16 + 2).min(area.height.saturating_sub(2));
            let picker_rect = center_rect(area, picker_width, picker_height);

            // Clear background
            let clear_style = Style::new().bg(Color::new(20, 20, 30));
            for y in picker_rect.y..picker_rect.y + picker_rect.height {
                for x in picker_rect.x..picker_rect.x + picker_rect.width {
                    buf.set_cell(x, y, ' ', clear_style);
                }
            }

            let border_style = Style::new().fg(Color::CYAN);
            let title_style = Style::new().fg(Color::CYAN);
            let inner = buf.draw_block(picker_rect, " Add Lane ", border_style, title_style);

            let inst = state.instruments.selected_instrument();
            let vst_registry = &state.session.vst_plugins;

            // Scroll offset: keep cursor visible within the inner area
            let visible_rows = inner.height as usize;
            let scroll_offset = if cursor >= visible_rows {
                cursor - visible_rows + 1
            } else {
                0
            };

            for (vi, target) in options.iter().enumerate().skip(scroll_offset) {
                let row = vi - scroll_offset;
                let y = inner.y + row as u16;
                if y >= inner.y + inner.height { break; }

                let is_selected = vi == cursor;
                let display_name = target.name_with_context(inst, vst_registry);
                let text = format!(
                    "{} {}",
                    if is_selected { ">" } else { " " },
                    display_name
                );
                let style = if is_selected {
                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
                } else {
                    Style::new().fg(Color::GRAY)
                };

                for (j, ch) in text.chars().enumerate() {
                    let x = inner.x + j as u16;
                    if x >= inner.x + inner.width { break; }
                    buf.set_cell(x, y, ch, style);
                }
                // Fill remaining for selected
                if is_selected {
                    for x in (inner.x + text.len() as u16)..(inner.x + inner.width) {
                        buf.set_cell(x, y, ' ', style);
                    }
                }
            }
        }
    }
}
