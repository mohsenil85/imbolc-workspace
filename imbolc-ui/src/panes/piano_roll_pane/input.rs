use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId, PianoRollActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    translate_key, Action, InputEvent, KeyCode, MouseButton, MouseEvent, MouseEventKind,
    PianoRollAction, Rect,
};
use imbolc_types::InstrumentId;

use super::PianoRollPane;

impl PianoRollPane {
    /// Get the instrument ID for the current track from state
    fn current_instrument_id(&self, state: &AppState) -> InstrumentId {
        state
            .session
            .piano_roll
            .track_order
            .get(self.current_track)
            .copied()
            .unwrap_or(InstrumentId::new(0))
    }
}

impl PianoRollPane {
    pub(super) fn handle_action_impl(
        &mut self,
        action: ActionId,
        event: &InputEvent,
        state: &AppState,
    ) -> Action {
        match action {
            // Piano mode actions (from piano layer)
            ActionId::Mode(ModeActionId::PianoEscape) => {
                self.piano.deactivate();
                Action::ExitPerformanceMode
            }
            ActionId::Mode(ModeActionId::PianoOctaveDown) => {
                if self.piano.octave_down() {
                    self.center_view_on_piano_octave();
                }
                Action::None
            }
            ActionId::Mode(ModeActionId::PianoOctaveUp) => {
                if self.piano.octave_up() {
                    self.center_view_on_piano_octave();
                }
                Action::None
            }
            ActionId::Mode(ModeActionId::PianoSpace) => {
                Action::PianoRoll(PianoRollAction::PlayStopRecord)
            }
            ActionId::Mode(ModeActionId::PianoKey) => {
                if let KeyCode::Char(c) = event.key {
                    let c = translate_key(c, state.keyboard_layout);
                    if let Some(pitches) = self.piano.key_to_pitches(c) {
                        // Check if this is a new press or key repeat (sustain)
                        if let Some(new_pitches) = self.piano.key_pressed(
                            c,
                            pitches.clone(),
                            event.timestamp,
                            event.is_repeat,
                        ) {
                            // NEW press - spawn voice(s)
                            let instrument_id = self.current_instrument_id(state);
                            let track = self.current_track;
                            if new_pitches.len() == 1 {
                                return Action::PianoRoll(PianoRollAction::PlayNote {
                                    pitch: new_pitches[0],
                                    velocity: 100,
                                    instrument_id,
                                    track,
                                });
                            } else {
                                return Action::PianoRoll(PianoRollAction::PlayNotes {
                                    pitches: new_pitches,
                                    velocity: 100,
                                    instrument_id,
                                    track,
                                });
                            }
                        }
                        // Key repeat - sustain, no action needed
                    }
                }
                Action::None
            }
            // Normal grid navigation
            ActionId::PianoRoll(PianoRollActionId::Up) => {
                self.selection_anchor = None;
                if self.cursor_pitch < 127 {
                    self.cursor_pitch += 1;
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::Down) => {
                self.selection_anchor = None;
                if self.cursor_pitch > 0 {
                    self.cursor_pitch -= 1;
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::Right) => {
                self.selection_anchor = None;
                self.cursor_tick += self.ticks_per_cell();
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::Left) => {
                self.selection_anchor = None;
                let step = self.ticks_per_cell();
                self.cursor_tick = self.cursor_tick.saturating_sub(step);
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::SelectUp) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_tick, self.cursor_pitch));
                }
                if self.cursor_pitch < 127 {
                    self.cursor_pitch += 1;
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::SelectDown) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_tick, self.cursor_pitch));
                }
                if self.cursor_pitch > 0 {
                    self.cursor_pitch -= 1;
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::SelectRight) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_tick, self.cursor_pitch));
                }
                self.cursor_tick += self.ticks_per_cell();
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::SelectLeft) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_tick, self.cursor_pitch));
                }
                let step = self.ticks_per_cell();
                self.cursor_tick = self.cursor_tick.saturating_sub(step);
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::ToggleNote) => {
                Action::PianoRoll(PianoRollAction::ToggleNote {
                    pitch: self.cursor_pitch,
                    tick: self.cursor_tick,
                    duration: self.default_duration,
                    velocity: self.default_velocity,
                    track: self.current_track,
                })
            }
            ActionId::PianoRoll(PianoRollActionId::GrowDuration) => {
                self.adjust_default_duration(self.ticks_per_cell() as i32);
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::ShrinkDuration) => {
                self.adjust_default_duration(-(self.ticks_per_cell() as i32));
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::VelUp) => {
                self.adjust_default_velocity(10);
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::VelDown) => {
                self.adjust_default_velocity(-10);
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::PlayStop) => {
                Action::PianoRoll(PianoRollAction::PlayStop)
            }
            ActionId::PianoRoll(PianoRollActionId::Loop) => {
                Action::PianoRoll(PianoRollAction::ToggleLoop)
            }
            ActionId::PianoRoll(PianoRollActionId::LoopStart) => {
                Action::PianoRoll(PianoRollAction::SetLoopStart(self.cursor_tick))
            }
            ActionId::PianoRoll(PianoRollActionId::LoopEnd) => {
                Action::PianoRoll(PianoRollAction::SetLoopEnd(self.cursor_tick))
            }
            ActionId::PianoRoll(PianoRollActionId::OctaveUp) => {
                self.selection_anchor = None;
                self.cursor_pitch = (self.cursor_pitch as i16 + 12).min(127) as u8;
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::OctaveDown) => {
                self.selection_anchor = None;
                self.cursor_pitch = (self.cursor_pitch as i16 - 12).max(0) as u8;
                self.scroll_to_cursor();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::Home) => {
                self.selection_anchor = None;
                self.cursor_tick = 0;
                self.view_start_tick = 0;
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::End) => {
                self.selection_anchor = None;
                self.jump_to_end();
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::ZoomIn) => {
                if self.zoom_level > 1 {
                    self.zoom_level -= 1;
                    self.cursor_tick = self.snap_tick(self.cursor_tick);
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::ZoomOut) => {
                if self.zoom_level < 5 {
                    self.zoom_level += 1;
                    self.cursor_tick = self.snap_tick(self.cursor_tick);
                    self.scroll_to_cursor();
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::TimeSig) => {
                Action::PianoRoll(PianoRollAction::CycleTimeSig)
            }
            ActionId::PianoRoll(PianoRollActionId::TogglePoly) => {
                Action::PianoRoll(PianoRollAction::TogglePolyMode(self.current_track))
            }
            ActionId::PianoRoll(PianoRollActionId::RenderToWav) => Action::PianoRoll(
                PianoRollAction::RenderToWav(self.current_instrument_id(state)),
            ),
            ActionId::PianoRoll(PianoRollActionId::BounceToWav) => {
                if state.io.pending_export.is_some() {
                    Action::PianoRoll(PianoRollAction::CancelExport)
                } else {
                    Action::PianoRoll(PianoRollAction::BounceToWav)
                }
            }
            ActionId::PianoRoll(PianoRollActionId::ExportStems) => {
                if state.io.pending_export.is_some() {
                    Action::PianoRoll(PianoRollAction::CancelExport)
                } else {
                    Action::PianoRoll(PianoRollAction::ExportStems)
                }
            }
            ActionId::PianoRoll(PianoRollActionId::ToggleAutomation) => {
                self.automation_overlay_visible = !self.automation_overlay_visible;
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::AutomationLanePrev) => {
                if self.automation_overlay_visible {
                    match self.automation_overlay_lane_idx {
                        Some(idx) if idx > 0 => {
                            self.automation_overlay_lane_idx = Some(idx - 1);
                        }
                        _ => {}
                    }
                }
                Action::None
            }
            ActionId::PianoRoll(PianoRollActionId::AutomationLaneNext) => {
                if self.automation_overlay_visible {
                    let next = match self.automation_overlay_lane_idx {
                        Some(idx) => idx + 1,
                        None => 1,
                    };
                    // Will be clamped during render based on actual lane count
                    self.automation_overlay_lane_idx = Some(next);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub(super) fn handle_mouse_impl(
        &mut self,
        event: &MouseEvent,
        area: Rect,
        _state: &AppState,
    ) -> Action {
        let rect = center_rect(area, 97, 29);
        let key_col_width: u16 = 5;
        let header_height: u16 = 2;
        let footer_height: u16 = 2;
        let grid_x = rect.x + key_col_width;
        let grid_y = rect.y + header_height;
        let grid_width = rect.width.saturating_sub(key_col_width + 1);
        let grid_height = rect
            .height
            .saturating_sub(header_height + footer_height + 1);

        let col = event.column;
        let row = event.row;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.selection_anchor = None;
                // Click on the grid area
                if col >= grid_x
                    && col < grid_x + grid_width
                    && row >= grid_y
                    && row < grid_y + grid_height
                {
                    let grid_col = col - grid_x;
                    let grid_row = row - grid_y;
                    let pitch = self
                        .view_bottom_pitch
                        .saturating_add((grid_height - 1 - grid_row) as u8);
                    let tick = self.view_start_tick + grid_col as u32 * self.ticks_per_cell();

                    if pitch <= 127 {
                        self.cursor_pitch = pitch;
                        self.cursor_tick = tick;
                        return Action::PianoRoll(PianoRollAction::ToggleNote {
                            pitch,
                            tick,
                            duration: self.default_duration,
                            velocity: self.default_velocity,
                            track: self.current_track,
                        });
                    }
                }
                // Click on piano key column to set pitch
                if col >= rect.x && col < grid_x && row >= grid_y && row < grid_y + grid_height {
                    let grid_row = row - grid_y;
                    let pitch = self
                        .view_bottom_pitch
                        .saturating_add((grid_height - 1 - grid_row) as u8);
                    if pitch <= 127 {
                        self.cursor_pitch = pitch;
                    }
                }
                Action::None
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // Right-click on grid: just move cursor (no toggle)
                if col >= grid_x
                    && col < grid_x + grid_width
                    && row >= grid_y
                    && row < grid_y + grid_height
                {
                    let grid_col = col - grid_x;
                    let grid_row = row - grid_y;
                    let pitch = self
                        .view_bottom_pitch
                        .saturating_add((grid_height - 1 - grid_row) as u8);
                    let tick = self.view_start_tick + grid_col as u32 * self.ticks_per_cell();
                    if pitch <= 127 {
                        self.cursor_pitch = pitch;
                        self.cursor_tick = tick;
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                if event.modifiers.shift {
                    // Horizontal scroll
                    let step = self.ticks_per_cell() * 4;
                    self.view_start_tick = self.view_start_tick.saturating_sub(step);
                } else {
                    // Vertical scroll - pitch up
                    self.view_bottom_pitch = self.view_bottom_pitch.saturating_add(3).min(127);
                }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                if event.modifiers.shift {
                    // Horizontal scroll
                    let step = self.ticks_per_cell() * 4;
                    self.view_start_tick += step;
                } else {
                    // Vertical scroll - pitch down
                    self.view_bottom_pitch = self.view_bottom_pitch.saturating_sub(3);
                }
                Action::None
            }
            _ => Action::None,
        }
    }
}
