use std::any::Any;

use crate::state::{AppState, SourceType};
use crate::state::arrangement::PlayMode;
use crate::ui::action_id::{ActionId, TrackActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, ArrangementAction, Color, InputEvent, Keymap, Pane, Style};

fn source_color(source: SourceType) -> Color {
    match source {
        // Oscillators and synths
        SourceType::Saw | SourceType::Sin | SourceType::Sqr | SourceType::Tri
        | SourceType::Noise | SourceType::Pulse | SourceType::SuperSaw | SourceType::Sync
        | SourceType::Ring | SourceType::FBSin | SourceType::FM | SourceType::PhaseMod
        | SourceType::FMBell | SourceType::FMBrass
        | SourceType::Pluck | SourceType::Formant | SourceType::Gendy | SourceType::Chaos
        | SourceType::Additive | SourceType::Wavetable | SourceType::Granular
        | SourceType::Bowed | SourceType::Blown | SourceType::Membrane
        // Mallet percussion
        | SourceType::Marimba | SourceType::Vibes | SourceType::Kalimba | SourceType::SteelDrum
        | SourceType::TubularBell | SourceType::Glockenspiel
        // Plucked strings
        | SourceType::Guitar | SourceType::BassGuitar | SourceType::Harp | SourceType::Koto
        // Drums
        | SourceType::Kick | SourceType::Snare | SourceType::HihatClosed | SourceType::HihatOpen
        | SourceType::Clap | SourceType::Cowbell | SourceType::Rim | SourceType::Tom
        | SourceType::Clave | SourceType::Conga
        // Classic synths
        | SourceType::Choir | SourceType::EPiano | SourceType::Organ | SourceType::BrassStab
        | SourceType::Strings | SourceType::Acid => Color::OSC_COLOR,
        SourceType::AudioIn => Color::AUDIO_IN_COLOR,
        SourceType::PitchedSampler | SourceType::TimeStretch => Color::SAMPLE_COLOR,
        SourceType::Kit => Color::KIT_COLOR,
        SourceType::BusIn => Color::BUS_IN_COLOR,
        SourceType::Custom(_) => Color::CUSTOM_COLOR,
        SourceType::Vst(_) => Color::VST_COLOR,
    }
}

pub struct TrackPane {
    keymap: Keymap,
    /// Index into current instrument's clips list for placement selection
    selected_clip_index: usize,
}

impl TrackPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected_clip_index: 0,
        }
    }

    fn ticks_per_bar(&self, state: &AppState) -> u32 {
        let (beats, _) = state.session.time_signature;
        beats as u32 * 480
    }
}

impl Default for TrackPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for TrackPane {
    fn id(&self) -> &'static str {
        "track"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let arr = &state.session.arrangement;
        let num_instruments = state.instruments.instruments.len();
        if num_instruments == 0 {
            return Action::None;
        }

        let lane = arr.selected_lane.min(num_instruments.saturating_sub(1));
        let instrument_id = state.instruments.instruments[lane].id;

        match action {
            ActionId::Track(TrackActionId::LaneUp) => {
                if lane > 0 {
                    Action::Arrangement(ArrangementAction::SelectLane(lane - 1))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::LaneDown) => {
                if lane + 1 < num_instruments {
                    Action::Arrangement(ArrangementAction::SelectLane(lane + 1))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::CursorLeft) => Action::Arrangement(ArrangementAction::MoveCursor(-1)),
            ActionId::Track(TrackActionId::CursorRight) => Action::Arrangement(ArrangementAction::MoveCursor(1)),
            ActionId::Track(TrackActionId::CursorHome) => {
                // Jump to tick 0
                let delta = -(arr.cursor_tick as i32 / arr.ticks_per_col.max(1) as i32);
                Action::Arrangement(ArrangementAction::MoveCursor(delta))
            }
            ActionId::Track(TrackActionId::CursorEnd) => {
                let end = arr.arrangement_length();
                if end > arr.cursor_tick {
                    let delta = (end - arr.cursor_tick) as i32 / arr.ticks_per_col.max(1) as i32;
                    Action::Arrangement(ArrangementAction::MoveCursor(delta))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::NewClip) => {
                Action::Arrangement(ArrangementAction::CaptureClipFromPianoRoll {
                    instrument_id,
                })
            }
            ActionId::Track(TrackActionId::NewEmptyClip) => {
                let tpb = self.ticks_per_bar(state);
                Action::Arrangement(ArrangementAction::CreateClip {
                    instrument_id,
                    length_ticks: tpb,
                })
            }
            ActionId::Track(TrackActionId::PlaceClip) => {
                // Place the selected clip at cursor position
                let clips = arr.clips_for_instrument(instrument_id);
                if clips.is_empty() {
                    return Action::None;
                }
                let idx = self.selected_clip_index.min(clips.len().saturating_sub(1));
                let clip_id = clips[idx].id;
                Action::Arrangement(ArrangementAction::PlaceClip {
                    clip_id,
                    instrument_id,
                    start_tick: arr.cursor_tick,
                })
            }
            ActionId::Track(TrackActionId::EditClip) => {
                // Edit clip under cursor
                if let Some(placement) = arr.placement_at(instrument_id, arr.cursor_tick) {
                    Action::Arrangement(ArrangementAction::EnterClipEdit(placement.clip_id))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::Delete) => {
                // Delete selected placement
                if let Some(placement) = arr.placement_at(instrument_id, arr.cursor_tick) {
                    Action::Arrangement(ArrangementAction::RemovePlacement(placement.id))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::DeleteClip) => {
                // Delete clip and all placements
                let clips = arr.clips_for_instrument(instrument_id);
                if clips.is_empty() {
                    return Action::None;
                }
                let idx = self.selected_clip_index.min(clips.len().saturating_sub(1));
                let clip_id = clips[idx].id;
                Action::Arrangement(ArrangementAction::DeleteClip(clip_id))
            }
            ActionId::Track(TrackActionId::Duplicate) => {
                if let Some(placement) = arr.placement_at(instrument_id, arr.cursor_tick) {
                    Action::Arrangement(ArrangementAction::DuplicatePlacement(placement.id))
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::ToggleMode) => Action::Arrangement(ArrangementAction::TogglePlayMode),
            ActionId::Track(TrackActionId::PlayStop) => Action::Arrangement(ArrangementAction::PlayStop),
            ActionId::Track(TrackActionId::MoveLeft) => {
                if let Some(placement) = arr.placement_at(instrument_id, arr.cursor_tick) {
                    let new_start = placement.start_tick.saturating_sub(arr.ticks_per_col);
                    Action::Arrangement(ArrangementAction::MovePlacement {
                        placement_id: placement.id,
                        new_start_tick: new_start,
                    })
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::MoveRight) => {
                if let Some(placement) = arr.placement_at(instrument_id, arr.cursor_tick) {
                    let new_start = placement.start_tick + arr.ticks_per_col;
                    Action::Arrangement(ArrangementAction::MovePlacement {
                        placement_id: placement.id,
                        new_start_tick: new_start,
                    })
                } else {
                    Action::None
                }
            }
            ActionId::Track(TrackActionId::ZoomIn) => Action::Arrangement(ArrangementAction::ZoomIn),
            ActionId::Track(TrackActionId::ZoomOut) => Action::Arrangement(ArrangementAction::ZoomOut),
            ActionId::Track(TrackActionId::SelectNextPlacement) => {
                let placements = arr.placements_for_instrument(instrument_id);
                if placements.is_empty() {
                    return Action::None;
                }
                let next = match arr.selected_placement {
                    Some(i) => {
                        let next_idx = i + 1;
                        if next_idx < placements.len() { Some(next_idx) } else { Some(0) }
                    }
                    None => Some(0),
                };
                Action::Arrangement(ArrangementAction::SelectPlacement(next))
            }
            ActionId::Track(TrackActionId::SelectPrevPlacement) => {
                let placements = arr.placements_for_instrument(instrument_id);
                if placements.is_empty() {
                    return Action::None;
                }
                let prev = match arr.selected_placement {
                    Some(0) => Some(placements.len().saturating_sub(1)),
                    Some(i) => Some(i - 1),
                    None => Some(0),
                };
                Action::Arrangement(ArrangementAction::SelectPlacement(prev))
            }
            ActionId::Track(TrackActionId::SelectNextClip) => {
                let clips = arr.clips_for_instrument(instrument_id);
                if !clips.is_empty() {
                    self.selected_clip_index = (self.selected_clip_index + 1) % clips.len();
                }
                Action::None
            }
            ActionId::Track(TrackActionId::SelectPrevClip) => {
                let clips = arr.clips_for_instrument(instrument_id);
                if !clips.is_empty() {
                    if self.selected_clip_index == 0 {
                        self.selected_clip_index = clips.len() - 1;
                    } else {
                        self.selected_clip_index -= 1;
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 97, 29);
        let arr = &state.session.arrangement;
        let ticks_per_col = arr.ticks_per_col.max(1);

        // Mode indicator for title
        let mode_str = match arr.play_mode {
            PlayMode::Pattern => "Pattern",
            PlayMode::Song => "Song",
        };
        let title = format!(" Track [{}] ", mode_str);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        if state.instruments.instruments.is_empty() {
            let text = "(no instruments)";
            let x = inner.x + (inner.width.saturating_sub(text.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.draw_line(
                Rect::new(x, y, text.len() as u16, 1),
                &[(text, Style::new().fg(Color::DARK_GRAY))],
            );
            return;
        }

        // Layout: header(1) + lanes + footer(2)
        let label_width: u16 = 20;
        let timeline_x = inner.x + label_width + 1;
        let timeline_width = inner.width.saturating_sub(label_width + 2);
        let header_height: u16 = 1;
        let footer_height: u16 = 2;
        let lanes_area_y = inner.y + header_height;
        let lanes_area_height = inner.height.saturating_sub(header_height + footer_height);

        let num_instruments = state.instruments.instruments.len();
        let lane_height: u16 = 2;
        let max_visible = (lanes_area_height / lane_height) as usize;

        // Scroll to keep selected lane visible
        let selected_lane = arr.selected_lane.min(num_instruments.saturating_sub(1));
        let scroll = if selected_lane >= max_visible {
            selected_lane - max_visible + 1
        } else {
            0
        };

        let sel_bg = Style::new().bg(Color::SELECTION_BG);
        let bar_line_style = Style::new().fg(Color::new(50, 50, 50));
        let separator_style = Style::new().fg(Color::new(40, 40, 40));

        // Compute bar spacing in columns
        let (beats_per_bar, _) = state.session.time_signature;
        let ticks_per_bar = beats_per_bar as u32 * 480;
        let cols_per_bar = ticks_per_bar / ticks_per_col;
        let cols_per_beat = 480 / ticks_per_col;

        // --- Header: bar numbers ---
        let header_y = inner.y;
        let header_label_style = Style::new().fg(Color::DARK_GRAY);
        for col in 0..timeline_width as u32 {
            let tick = arr.view_start_tick + col * ticks_per_col;
            if cols_per_bar > 0 && (tick % ticks_per_bar) < ticks_per_col {
                let bar_num = tick / ticks_per_bar + 1;
                let label = format!("{}", bar_num);
                let x = timeline_x + col as u16;
                for (j, ch) in label.chars().enumerate() {
                    if x + (j as u16) < inner.x + inner.width {
                        buf.set_cell(x + j as u16, header_y, ch, header_label_style);
                    }
                }
            }
        }

        // --- Instrument lanes ---
        for (vi, i) in (scroll..num_instruments).enumerate() {
            if vi >= max_visible {
                break;
            }
            let instrument = &state.instruments.instruments[i];
            let is_selected = i == selected_lane;
            let lane_y = lanes_area_y + (vi as u16) * lane_height;

            if lane_y + lane_height > lanes_area_y + lanes_area_height {
                break;
            }

            let source_c = source_color(instrument.source);

            // Fill label area bg for selected (before drawing text)
            if is_selected {
                for row in 0..lane_height {
                    let y = lane_y + row;
                    if y >= lanes_area_y + lanes_area_height { break; }
                    for x in inner.x..timeline_x {
                        buf.set_cell(x, y, ' ', sel_bg);
                    }
                }
            }

            // Selection indicator
            if is_selected {
                buf.set_cell(inner.x, lane_y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
            }

            // Instrument number + name
            let num_str = format!("{:>2} ", i + 1);
            let name_str = &instrument.name[..instrument.name.len().min(11)];
            let src_short = format!(" {}", instrument.source.name());

            let num_style = if is_selected {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let name_style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::WHITE)
            };
            let src_style = if is_selected {
                Style::new().fg(source_c).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(source_c)
            };

            // Line 1: number + name
            buf.draw_line(
                Rect::new(inner.x + 1, lane_y, label_width, 1),
                &[(&num_str, num_style), (name_str, name_style)],
            );

            // Line 2: source type
            buf.draw_line(
                Rect::new(inner.x + 1, lane_y + 1, label_width, 1),
                &[(&src_short[..src_short.len().min(label_width as usize)], src_style)],
            );

            // Separator between label and timeline
            for row in 0..lane_height {
                let y = lane_y + row;
                if y >= lanes_area_y + lanes_area_height { break; }
                buf.set_cell(inner.x + label_width, y, '|', Style::new().fg(Color::GRAY));
            }

            // Timeline area: bar/beat lines + clip blocks
            let inst_id = instrument.id;

            // Draw bar/beat lines
            for col in 0..timeline_width as u32 {
                let tick = arr.view_start_tick + col * ticks_per_col;
                let x = timeline_x + col as u16;
                let is_bar = cols_per_bar > 0 && (tick % ticks_per_bar) < ticks_per_col;
                let is_beat = cols_per_beat > 0 && (tick % 480) < ticks_per_col;

                for row in 0..lane_height {
                    let y = lane_y + row;
                    if y >= lanes_area_y + lanes_area_height { break; }
                    if is_bar {
                        buf.set_cell(x, y, '|', bar_line_style);
                    } else if is_beat && row == 0 {
                        buf.set_cell(x, y, '.', Style::new().fg(Color::new(30, 30, 30)));
                    }
                }
            }

            // Draw clip placements for this instrument
            let placements = arr.placements_for_instrument(inst_id);
            for placement in &placements {
                if let Some(clip) = arr.clip(placement.clip_id) {
                    let _eff_len = placement.effective_length(clip);
                    let start_col = placement.start_tick.saturating_sub(arr.view_start_tick) / ticks_per_col;
                    let end_col = placement.end_tick(clip).saturating_sub(arr.view_start_tick) / ticks_per_col;

                    // Skip if entirely off-screen
                    if placement.end_tick(clip) <= arr.view_start_tick {
                        continue;
                    }
                    if placement.start_tick >= arr.view_start_tick + (timeline_width as u32) * ticks_per_col {
                        continue;
                    }

                    let vis_start = if placement.start_tick < arr.view_start_tick { 0 } else { start_col as u16 };
                    let vis_end = (end_col as u16).min(timeline_width);

                    if vis_start >= vis_end {
                        continue;
                    }

                    let clip_bg = source_c;
                    let clip_style = Style::new().fg(Color::BLACK).bg(clip_bg);
                    let sel_clip_style = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold();

                    let is_placement_selected = arr.selected_placement
                        .and_then(|idx| arr.placements.get(idx))
                        .map(|p| p.id == placement.id)
                        .unwrap_or(false);

                    let style = if is_placement_selected { sel_clip_style } else { clip_style };

                    // Render clip block
                    let block_width = vis_end - vis_start;
                    let name = &clip.name;
                    let display_name: String = if name.len() > block_width as usize {
                        name[..block_width as usize].to_string()
                    } else {
                        let padding = block_width as usize - name.len();
                        let left_pad = 0;
                        let right_pad = padding;
                        format!("{}{}{}", " ".repeat(left_pad), name, " ".repeat(right_pad))
                    };

                    // Fill both rows of the lane
                    for row in 0..lane_height {
                        let y = lane_y + row;
                        if y >= lanes_area_y + lanes_area_height { break; }
                        let x = timeline_x + vis_start;
                        if row == 0 {
                            // Name on first row
                            for (j, ch) in display_name.chars().enumerate() {
                                if x + (j as u16) < timeline_x + timeline_width {
                                    buf.set_cell(x + j as u16, y, ch, style);
                                }
                            }
                        } else {
                            // Fill second row
                            for j in 0..block_width {
                                if x + j < timeline_x + timeline_width {
                                    buf.set_cell(x + j, y, ' ', style);
                                }
                            }
                        }
                    }

                    // Clip boundary markers
                    if vis_start > 0 || placement.start_tick >= arr.view_start_tick {
                        let x = timeline_x + vis_start;
                        for row in 0..lane_height {
                            let y = lane_y + row;
                            if y >= lanes_area_y + lanes_area_height { break; }
                            buf.set_cell(x, y, '[', style);
                        }
                    }
                    if vis_end <= timeline_width && vis_end > vis_start {
                        let x = timeline_x + vis_end - 1;
                        for row in 0..lane_height {
                            let y = lane_y + row;
                            if y >= lanes_area_y + lanes_area_height { break; }
                            buf.set_cell(x, y, ']', style);
                        }
                    }
                }
            }

            // Horizontal separator below each lane
            if vi + 1 < max_visible && i + 1 < num_instruments {
                let sep_y = lane_y + lane_height;
                if sep_y < lanes_area_y + lanes_area_height {
                    for x in (inner.x + label_width + 1)..(inner.x + inner.width) {
                        buf.set_cell(x, sep_y, '-', separator_style);
                    }
                }
            }
        }

        // --- Playhead ---
        let playhead_tick = state.audio.playhead;
        if playhead_tick >= arr.view_start_tick {
            let playhead_col = (playhead_tick - arr.view_start_tick) / ticks_per_col;
            if (playhead_col as u16) < timeline_width {
                let x = timeline_x + playhead_col as u16;
                let ph_style = Style::new().fg(Color::WHITE).bold();
                for y in lanes_area_y..(lanes_area_y + lanes_area_height) {
                    buf.set_cell(x, y, '|', ph_style);
                }
            }
        }

        // --- Cursor --- (uses raw_buf for read-back to preserve clip visibility)
        if arr.cursor_tick >= arr.view_start_tick {
            let cursor_col = (arr.cursor_tick - arr.view_start_tick) / ticks_per_col;
            if (cursor_col as u16) < timeline_width {
                let x = timeline_x + cursor_col as u16;
                let lane_y = lanes_area_y + ((selected_lane - scroll.min(selected_lane)) as u16) * lane_height;
                let cursor_style = Style::new().fg(Color::CYAN);
                for row in 0..lane_height {
                    let y = lane_y + row;
                    if y < lanes_area_y + lanes_area_height {
                        if let Some(cell) = buf.raw_buf().cell_mut((x, y)) {
                            if cell.symbol() == " " {
                                cell.set_char('|').set_style(cursor_style);
                            }
                        }
                    }
                }
            }
        }

        // --- Footer ---
        let footer_y = inner.y + inner.height - 2;

        // Line 1: key hints
        let hints = "n:new  p:place  Enter:edit  d:del  m:mode  Space:play  z/x:zoom";
        buf.draw_line(
            Rect::new(inner.x + 1, footer_y, inner.width.saturating_sub(2), 1),
            &[(hints, Style::new().fg(Color::DARK_GRAY))],
        );

        // Line 2: cursor position + selected clip info
        let bar = arr.cursor_tick / ticks_per_bar + 1;
        let beat = (arr.cursor_tick % ticks_per_bar) / 480 + 1;
        let inst_id = state.instruments.instruments[selected_lane].id;
        let clips = arr.clips_for_instrument(inst_id);
        let clip_info = if clips.is_empty() {
            "No clips".to_string()
        } else {
            let idx = self.selected_clip_index.min(clips.len().saturating_sub(1));
            format!("Clip: {} [{}/{}]", clips[idx].name, idx + 1, clips.len())
        };

        let pos_str = format!("Bar {} Beat {}  |  {}", bar, beat, clip_info);
        buf.draw_line(
            Rect::new(inner.x + 1, footer_y + 1, inner.width.saturating_sub(2), 1),
            &[(&pos_str, Style::new().fg(Color::GRAY))],
        );
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
