use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Color, Style};

use super::PianoRollPane;

/// MIDI note name for a given pitch (0-127)
pub(super) fn note_name(pitch: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (pitch / 12) as i8 - 1;
    let name = names[(pitch % 12) as usize];
    format!("{}{}", name, octave)
}

/// Check if a pitch is a black key
pub(super) fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

/// Block characters for value graph (8 levels, bottom to top)
pub(super) const AUTOMATION_BLOCKS: [char; 8] = [
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
    '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

impl PianoRollPane {
    /// Render the automation overlay strip at the bottom of the note grid
    pub(super) fn render_automation_overlay(
        &self,
        buf: &mut RenderBuf,
        overlay_area: Rect,
        grid_x: u16,
        grid_width: u16,
        state: &AppState,
    ) {
        let automation = &state.session.automation;
        let inst_id = state.instruments.selected_instrument().map(|i| i.id);

        // Find the lane to display
        let lane = if let Some(idx) = self.automation_overlay_lane_idx {
            automation.lanes.get(idx)
        } else {
            // Default: show first lane for current instrument
            inst_id.and_then(|id| {
                automation.lanes.iter().find(|l| l.target.instrument_id() == Some(id))
            })
        };

        let overlay_height = overlay_area.height;
        if overlay_height == 0 { return; }

        // Separator line
        let sep_style = Style::new().fg(Color::new(50, 40, 60));
        for x in overlay_area.x..overlay_area.x + overlay_area.width {
            buf.set_cell(x, overlay_area.y, '─', sep_style);
        }

        // Lane name on left edge
        let lane_name = lane
            .map(|l| l.target.short_name())
            .unwrap_or("—");
        let label_style = Style::new().fg(Color::CYAN);
        for (i, ch) in lane_name.chars().enumerate() {
            let x = overlay_area.x + i as u16;
            if x >= grid_x { break; }
            let y = overlay_area.y + 1;
            if y < overlay_area.y + overlay_height {
                buf.set_cell(x, y, ch, label_style);
            }
        }

        // REC indicator
        if state.recording.automation_recording {
            let rec_str = "REC";
            let rec_style = Style::new().fg(Color::WHITE).bg(Color::RED);
            for (i, ch) in rec_str.chars().enumerate() {
                let x = overlay_area.x + i as u16;
                let y = overlay_area.y + 2.min(overlay_height - 1);
                if x < grid_x && y < overlay_area.y + overlay_height {
                    buf.set_cell(x, y, ch, rec_style);
                }
            }
        }

        let Some(lane) = lane else { return; };
        if lane.points.is_empty() { return; }

        let tpc = self.ticks_per_cell();
        let graph_rows = overlay_height.saturating_sub(1); // Minus separator row
        if graph_rows == 0 { return; }

        let curve_color = if lane.enabled { Color::CYAN } else { Color::DARK_GRAY };
        let curve_style = Style::new().fg(curve_color);

        for col in 0..grid_width {
            let tick = self.view_start_tick + col as u32 * tpc;
            if let Some(raw_value) = lane.value_at(tick) {
                // Normalize to 0-1
                let normalized = if lane.max_value > lane.min_value {
                    ((raw_value - lane.min_value) / (lane.max_value - lane.min_value)).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                // Map to block character index (0-7)
                let block_idx = (normalized * 7.0) as usize;
                let block_char = AUTOMATION_BLOCKS[block_idx.min(7)];

                // Render at the bottom row(s) of the overlay
                let x = grid_x + col;
                let y = overlay_area.y + 1; // First row below separator
                if y < overlay_area.y + overlay_height && x < overlay_area.x + overlay_area.width {
                    buf.set_cell(x, y, block_char, curve_style);
                }

                // For taller overlays, fill upward with full blocks
                if graph_rows > 1 {
                    let filled_rows = ((normalized * (graph_rows - 1) as f32) as u16).min(graph_rows - 1);
                    for r in 1..filled_rows {
                        let y = overlay_area.y + graph_rows - r;
                        if y > overlay_area.y && y < overlay_area.y + overlay_height && x < overlay_area.x + overlay_area.width {
                            buf.set_cell(x, y, '▁', curve_style);
                        }
                    }
                }
            }
        }
    }

    /// Render notes grid (buffer version)
    pub(super) fn render_notes_buf(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        let piano_roll = &state.session.piano_roll;
        let rect = center_rect(area, 97, 29);

        // Layout constants
        let key_col_width: u16 = 5;
        let header_height: u16 = 2;
        let footer_height: u16 = 2;
        let grid_x = rect.x + key_col_width;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(key_col_width + 1);
        let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

        // Border
        let track_label = if let Some(ref ctx) = state.session.arrangement.editing_clip {
            let clip_name = state.session.arrangement.clip(ctx.clip_id)
                .map(|c| c.name.as_str()).unwrap_or("?");
            format!(" Piano Roll - Editing: {} ", clip_name)
        } else if let Some(track) = piano_roll.track_at(self.current_track) {
            let mode = if track.polyphonic { "POLY" } else { "MONO" };
            format!(
                " Piano Roll: midi-{} [{}/{}] {} ",
                track.module_id,
                self.current_track + 1,
                piano_roll.track_order.len(),
                mode,
            )
        } else {
            " Piano Roll: (no tracks) ".to_string()
        };
        let border_style = Style::new().fg(Color::PINK);
        buf.draw_block(rect, &track_label, border_style, border_style);

        // Header: transport info
        let header_y = rect.y + 1;
        let play_icon = if state.audio.playing { "||" } else { "> " };
        let loop_icon = if piano_roll.looping { "L" } else { " " };
        let header_text = format!(
            " {}  {}  Beat:{:.1}",
            play_icon,
            loop_icon,
            piano_roll.tick_to_beat(state.audio.playhead),
        );
        buf.draw_line(Rect::new(rect.x + 1, header_y, rect.width.saturating_sub(2), 1),
            &[(&header_text, Style::new().fg(Color::WHITE))]);

        // Loop range indicator
        if piano_roll.looping {
            let loop_info = format!(
                "Loop:{:.1}-{:.1}",
                piano_roll.tick_to_beat(piano_roll.loop_start),
                piano_roll.tick_to_beat(piano_roll.loop_end),
            );
            let loop_x = rect.x + rect.width - loop_info.len() as u16 - 2;
            buf.draw_line(Rect::new(loop_x, header_y, rect.width.saturating_sub(loop_x - rect.x), 1),
                &[(&loop_info, Style::new().fg(Color::YELLOW))]);
        }

        // Rendering indicator
        if let Some(render) = &state.io.pending_render {
            if let Some(track_inst_id) = state.session.piano_roll.track_order.get(self.current_track) {
                if render.instrument_id == *track_inst_id {
                    let label = " RENDERING ";
                    let style = Style::new().fg(Color::WHITE).bg(Color::RED);
                    let x = rect.x + rect.width - label.len() as u16 - 2;
                    buf.draw_line(Rect::new(x, header_y, label.len() as u16, 1), &[(label, style)]);
                }
            }
        }

        // Export progress indicator
        if let Some(export) = &state.io.pending_export {
            let progress = state.io.export_progress;
            let bar_width: usize = 20;
            let filled = (progress * bar_width as f32) as usize;
            let empty = bar_width.saturating_sub(filled);
            let label = match export.kind {
                imbolc_audio::commands::ExportKind::MasterBounce => "BOUNCING",
                imbolc_audio::commands::ExportKind::StemExport => "STEMS",
            };
            let text = format!(
                " {} [{}{}] {:.0}% ",
                label,
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(empty),
                progress * 100.0,
            );
            let style = Style::new().fg(Color::WHITE).bg(Color::new(200, 120, 0));
            let x = rect.x + rect.width - text.len() as u16 - 2;
            buf.draw_line(Rect::new(x, header_y, text.len() as u16, 1), &[(&text, style)]);
        }

        // Piano keys column + grid rows
        for row in 0..grid_height {
            let pitch = self.view_bottom_pitch.saturating_add((grid_height - 1 - row) as u8);
            if pitch > 127 {
                continue;
            }
            let y = grid_y + row;

            // Piano key label
            let name = note_name(pitch);
            let is_black = is_black_key(pitch);
            let key_style = if pitch == self.cursor_pitch {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
            } else if is_black {
                Style::new().fg(Color::GRAY)
            } else {
                Style::new().fg(Color::WHITE)
            };
            let key_str = format!("{:>3}", name);
            for (j, ch) in key_str.chars().enumerate() {
                buf.set_cell(rect.x + 1 + j as u16, y, ch, key_style);
            }

            // Separator
            buf.set_cell(rect.x + key_col_width - 1, y, '|', Style::new().fg(Color::GRAY));

            // Grid cells
            for col in 0..grid_width {
                let tick = self.view_start_tick + col as u32 * self.ticks_per_cell();
                let x = grid_x + col;

                let has_note = piano_roll.track_at(self.current_track).is_some_and(|track| {
                    track.notes.iter().any(|n| {
                        n.pitch == pitch && tick >= n.tick && tick < n.tick + n.duration
                    })
                });

                let is_note_start = piano_roll.track_at(self.current_track).is_some_and(|track| {
                    track.notes.iter().any(|n| n.pitch == pitch && n.tick == tick)
                });

                let is_cursor = pitch == self.cursor_pitch && tick == self.cursor_tick;
                let is_playhead = state.audio.playing
                    && tick <= state.audio.playhead
                    && state.audio.playhead < tick + self.ticks_per_cell();

                let tpb = piano_roll.ticks_per_beat;
                let tpbar = piano_roll.ticks_per_bar();
                let is_bar_line = tick.is_multiple_of(tpbar);
                let is_beat_line = tick.is_multiple_of(tpb);

                let in_selection = self.selection_anchor.is_some_and(|(anchor_tick, anchor_pitch)| {
                    let (t0, t1) = if anchor_tick <= self.cursor_tick {
                        (anchor_tick, self.cursor_tick + self.ticks_per_cell())
                    } else {
                        (self.cursor_tick, anchor_tick + self.ticks_per_cell())
                    };
                    let (p0, p1) = if anchor_pitch <= self.cursor_pitch {
                        (anchor_pitch, self.cursor_pitch)
                    } else {
                        (self.cursor_pitch, anchor_pitch)
                    };
                    tick >= t0 && tick < t1 && pitch >= p0 && pitch <= p1
                });

                let (ch, style) = if is_cursor {
                    if has_note {
                        ('█', Style::new().fg(Color::BLACK).bg(Color::WHITE))
                    } else {
                        ('▒', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG))
                    }
                } else if in_selection && has_note {
                    // Selected note
                    ('█', Style::new().fg(Color::WHITE).bg(Color::new(60, 30, 80)))
                } else if in_selection {
                    // Selection region background
                    ('░', Style::new().fg(Color::new(60, 30, 80)))
                } else if has_note {
                    if is_note_start {
                        ('█', Style::new().fg(Color::PINK))
                    } else {
                        ('█', Style::new().fg(Color::MAGENTA))
                    }
                } else if is_playhead {
                    ('│', Style::new().fg(Color::GREEN))
                } else if is_bar_line {
                    ('┊', Style::new().fg(Color::GRAY))
                } else if is_beat_line {
                    ('·', Style::new().fg(Color::new(40, 40, 40)))
                } else if is_black {
                    ('·', Style::new().fg(Color::new(25, 25, 25)))
                } else {
                    (' ', Style::new())
                };

                buf.set_cell(x, y, ch, style);
            }
        }

        // Footer: beat markers
        let footer_y = grid_y + grid_height;
        for col in 0..grid_width {
            let tick = self.view_start_tick + col as u32 * self.ticks_per_cell();
            let tpb = piano_roll.ticks_per_beat;
            let tpbar = piano_roll.ticks_per_bar();
            let x = grid_x + col;

            if tick.is_multiple_of(tpbar) {
                let bar = tick / tpbar + 1;
                let label = format!("{}", bar);
                let white = Style::new().fg(Color::WHITE);
                for (j, ch) in label.chars().enumerate() {
                    buf.set_cell(x + j as u16, footer_y, ch, white);
                }
            } else if tick.is_multiple_of(tpb) {
                buf.set_cell(x, footer_y, '·', Style::new().fg(Color::GRAY));
            }
        }

        // Status line
        let status_y = footer_y + 1;
        let vel_str = if let Some((anchor_tick, anchor_pitch)) = self.selection_anchor {
            let t_diff = (self.cursor_tick as i64 - anchor_tick as i64).unsigned_abs() as u32 + self.ticks_per_cell();
            let p_diff = (self.cursor_pitch as i16 - anchor_pitch as i16).abs() + 1;
            format!("Sel: {:.1} beats x {} pitches", t_diff as f32 / piano_roll.ticks_per_beat as f32, p_diff)
        } else {
            format!(
                "Note:{} Tick:{} Vel:{} Dur:{}",
                note_name(self.cursor_pitch),
                self.cursor_tick,
                self.default_velocity,
                self.default_duration,
            )
        };
        buf.draw_line(Rect::new(rect.x + 1, status_y, rect.width.saturating_sub(2), 1),
            &[(&vel_str, Style::new().fg(Color::GRAY))]);

        // Piano mode indicator
        if self.piano.is_active() {
            let piano_str = self.piano.status_label();
            let mut indicator_x = rect.x + rect.width - piano_str.len() as u16 - 1;

            if self.recording {
                let rec_str = " REC ";
                indicator_x -= rec_str.len() as u16;
                let rec_style = Style::new().fg(Color::WHITE).bg(Color::RED);
                for (j, ch) in rec_str.chars().enumerate() {
                    buf.set_cell(indicator_x + j as u16, status_y, ch, rec_style);
                }
                indicator_x += rec_str.len() as u16;
            }

            let piano_style = Style::new().fg(Color::BLACK).bg(Color::PINK);
            for (j, ch) in piano_str.chars().enumerate() {
                buf.set_cell(indicator_x + j as u16, status_y, ch, piano_style);
            }
        } else {
            let hint_str = "/=piano";
            let hint_x = rect.x + rect.width - hint_str.len() as u16 - 2;
            buf.draw_line(Rect::new(hint_x, status_y, hint_str.len() as u16, 1),
                &[(hint_str, Style::new().fg(Color::GRAY))]);
        }
    }
}
