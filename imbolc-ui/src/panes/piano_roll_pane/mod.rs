mod input;
mod rendering;

use std::any::Any;
use std::time::Instant;

use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, InputEvent, Keymap, MouseEvent, Pane, PianoKeyboard, PianoRollAction, ToggleResult};
use crate::ui::action_id::ActionId;
use imbolc_types::InstrumentId;

pub struct PianoRollPane {
    keymap: Keymap,
    // Cursor state
    pub(crate) cursor_pitch: u8,   // MIDI note 0-127
    pub(crate) cursor_tick: u32,   // Position in ticks
    // View state
    pub(crate) current_track: usize,
    pub(super) view_bottom_pitch: u8,  // Lowest visible pitch
    pub(super) view_start_tick: u32,   // Leftmost visible tick
    pub(super) zoom_level: u8,         // 1=finest, higher=wider beats. Ticks per cell.
    // Note placement defaults
    pub(super) default_duration: u32,
    pub(super) default_velocity: u8,
    // Piano keyboard mode
    pub(super) piano: PianoKeyboard,
    pub(super) recording: bool,            // True when recording notes from piano keyboard
    // Automation overlay
    pub(super) automation_overlay_visible: bool,
    pub(super) automation_overlay_lane_idx: Option<usize>, // index into automation.lanes for overlay display
    /// Selection anchor — set when Shift+Arrow begins. None = no active selection.
    pub(crate) selection_anchor: Option<(u32, u8)>,  // (tick, pitch)
}

impl PianoRollPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            cursor_pitch: 60, // C4
            cursor_tick: 0,
            current_track: 0,
            view_bottom_pitch: 48, // C3
            view_start_tick: 0,
            zoom_level: 3, // Each cell = 120 ticks (1/4 beat at 480 tpb)
            default_duration: 480, // One beat
            default_velocity: 100,
            piano: PianoKeyboard::new(),
            recording: false,
            automation_overlay_visible: false,
            automation_overlay_lane_idx: None,
            selection_anchor: None,
        }
    }

    /// Set current track index directly (for external syncing from global instrument selection)
    #[allow(dead_code)]
    pub fn current_track(&self) -> usize { self.current_track }

    pub fn set_enhanced_keyboard(&mut self, enabled: bool) {
        self.piano.set_enhanced_keyboard(enabled);
    }

    pub fn adjust_default_duration(&mut self, delta: i32) {
        let new_dur = (self.default_duration as i32 + delta).max(self.ticks_per_cell() as i32);
        self.default_duration = new_dur as u32;
    }

    pub fn adjust_default_velocity(&mut self, delta: i8) {
        let new_vel = (self.default_velocity as i16 + delta as i16).clamp(1, 127);
        self.default_velocity = new_vel as u8;
    }

    #[allow(dead_code)]
    pub fn change_track(&mut self, delta: i8, track_count: usize) {
        if track_count == 0 { return; }
        let new_idx = (self.current_track as i32 + delta as i32).clamp(0, track_count as i32 - 1);
        self.current_track = new_idx as usize;
    }

    /// Set current track index directly (for external syncing from global instrument selection)
    pub fn set_current_track(&mut self, idx: usize) {
        self.current_track = idx;
    }

    pub fn jump_to_end(&mut self) {
        // Jump to a reasonable far position (e.g., 16 bars worth)
        self.cursor_tick = 480 * 4 * 16; // 16 bars at 4/4
        self.scroll_to_cursor();
    }

    /// Returns the selection region as (track, start_tick, end_tick, start_pitch, end_pitch),
    /// or a single-cell region at the cursor if no selection is active.
    pub(crate) fn selection_region(&self) -> (usize, u32, u32, u8, u8) {
        if let Some((anchor_tick, anchor_pitch)) = self.selection_anchor {
            let (t0, t1, p0, p1) = crate::state::grid::normalize_2d_region(
                anchor_tick, anchor_pitch,
                self.cursor_tick, self.cursor_pitch,
                self.ticks_per_cell(),
            );
            (self.current_track, t0, t1, p0, p1)
        } else {
            // No selection: single cell at cursor
            (self.current_track, self.cursor_tick, self.cursor_tick + 1, self.cursor_pitch, self.cursor_pitch)
        }
    }

    /// Ticks per grid cell based on zoom level
    pub(crate) fn ticks_per_cell(&self) -> u32 {
        crate::state::grid::ticks_per_cell(self.zoom_level)
    }

    /// Snap cursor tick to grid
    fn snap_tick(&self, tick: u32) -> u32 {
        crate::state::grid::snap_to_grid(tick, self.zoom_level)
    }

    /// Ensure cursor is visible by adjusting view
    pub(crate) fn scroll_to_cursor(&mut self) {
        // Vertical: keep cursor within visible range
        let visible_rows = 24u8;
        if self.cursor_pitch < self.view_bottom_pitch {
            self.view_bottom_pitch = self.cursor_pitch;
        } else if self.cursor_pitch >= self.view_bottom_pitch.saturating_add(visible_rows) {
            self.view_bottom_pitch = self.cursor_pitch.saturating_sub(visible_rows - 1);
        }

        // Horizontal: keep cursor within visible range
        let visible_cols = 60u32;
        let visible_ticks = visible_cols * self.ticks_per_cell();
        if self.cursor_tick < self.view_start_tick {
            self.view_start_tick = self.snap_tick(self.cursor_tick);
        } else if self.cursor_tick >= self.view_start_tick + visible_ticks {
            self.view_start_tick = self.snap_tick(self.cursor_tick.saturating_sub(visible_ticks - self.ticks_per_cell()));
        }
    }

    /// Center the view vertically on the current piano octave
    fn center_view_on_piano_octave(&mut self) {
        // Piano octave base note: octave 4 = C4 = MIDI 60
        let base_pitch = ((self.piano.octave() as i16 + 1) * 12).clamp(0, 127) as u8;
        // Center the view so the octave is roughly in the middle
        // visible_rows is about 24, so offset by ~12 to center
        let visible_rows = 24u8;
        self.view_bottom_pitch = base_pitch.saturating_sub(visible_rows / 2);
        // Also move cursor to the base note of this octave
        self.cursor_pitch = base_pitch;
    }
}

impl Default for PianoRollPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for PianoRollPane {
    fn id(&self) -> &'static str {
        "piano_roll"
    }

    fn on_enter(&mut self, state: &AppState) {
        self.selection_anchor = None;
        // Sync current_track to the globally selected instrument
        if let Some(selected_idx) = state.instruments.selected {
            if let Some(inst) = state.instruments.instruments.get(selected_idx) {
                if let Some(track_idx) = state.session.piano_roll.track_order.iter()
                    .position(|&id| id == inst.id)
                {
                    self.current_track = track_idx;
                }
            }
        }
    }

    fn tick(&mut self, state: &AppState) -> Vec<Action> {
        if !self.piano.is_active() || !self.piano.has_active_keys() {
            return vec![];
        }
        let now = Instant::now();
        let released = self.piano.check_releases(now);
        if released.is_empty() {
            return vec![];
        }
        let instrument_id = state.session.piano_roll.track_order
            .get(self.current_track)
            .copied()
            .unwrap_or(InstrumentId::new(0));
        // Flatten all released pitches (handles chords)
        released.into_iter()
            .map(|(_, pitches)| {
                if pitches.len() == 1 {
                    Action::PianoRoll(PianoRollAction::ReleaseNote {
                        pitch: pitches[0],
                        instrument_id,
                    })
                } else {
                    Action::PianoRoll(PianoRollAction::ReleaseNotes {
                        pitches,
                        instrument_id,
                    })
                }
            })
            .collect()
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.handle_action_impl(action, event, state)
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action {
        self.handle_mouse_impl(event, area, state)
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        self.render_notes_buf(buf, area, state);

        // Automation overlay
        if self.automation_overlay_visible {
            let rect = center_rect(area, 97, 29);
            let key_col_width: u16 = 5;
            let header_height: u16 = 2;
            let footer_height: u16 = 2;
            let grid_x = rect.x + key_col_width;
            let grid_width = rect.width.saturating_sub(key_col_width + 1);
            let grid_height = rect.height.saturating_sub(header_height + footer_height + 1);

            // Overlay occupies the bottom 4 rows of the grid area
            let overlay_rows = 4u16.min(grid_height / 2);
            let overlay_y = rect.y + header_height + grid_height - overlay_rows;
            let overlay_area = Rect::new(rect.x, overlay_y, rect.width, overlay_rows);

            self.render_automation_overlay(buf, overlay_area, grid_x, grid_width, state);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn toggle_performance_mode(&mut self, _state: &AppState) -> ToggleResult {
        if self.piano.is_active() {
            self.piano.handle_escape();
            if self.piano.is_active() {
                ToggleResult::CycledLayout
            } else {
                ToggleResult::Deactivated
            }
        } else {
            self.piano.activate();
            ToggleResult::ActivatedPiano
        }
    }

    fn activate_piano(&mut self) {
        if !self.piano.is_active() { self.piano.activate(); }
    }

    fn deactivate_performance(&mut self) {
        self.piano.release_all();
        self.piano.deactivate();
    }

    fn supports_performance_mode(&self) -> bool { true }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::ui::{InputEvent, KeyCode, Modifiers, PianoRollAction};
    use crate::ui::action_id::PianoRollActionId;

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn cursor_moves_with_arrow_actions() {
        let mut pane = PianoRollPane::new(Keymap::new());
        let state = AppState::new();

        let start_pitch = pane.cursor_pitch;
        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::Up), &dummy_event(), &state);
        assert_eq!(pane.cursor_pitch, start_pitch + 1);

        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::Down), &dummy_event(), &state);
        assert_eq!(pane.cursor_pitch, start_pitch);

        let start_tick = pane.cursor_tick;
        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::Right), &dummy_event(), &state);
        assert!(pane.cursor_tick > start_tick);

        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::Left), &dummy_event(), &state);
        assert_eq!(pane.cursor_tick, start_tick);
    }

    #[test]
    fn zoom_in_out_clamps() {
        let mut pane = PianoRollPane::new(Keymap::new());
        let state = AppState::new();

        pane.zoom_level = 1;
        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::ZoomIn), &dummy_event(), &state);
        assert_eq!(pane.zoom_level, 1);

        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::ZoomOut), &dummy_event(), &state);
        assert_eq!(pane.zoom_level, 2);
    }

    #[test]
    fn home_resets_cursor_and_view() {
        let mut pane = PianoRollPane::new(Keymap::new());
        let state = AppState::new();

        pane.cursor_tick = 960;
        pane.view_start_tick = 480;
        pane.handle_action(ActionId::PianoRoll(PianoRollActionId::Home), &dummy_event(), &state);
        assert_eq!(pane.cursor_tick, 0);
        assert_eq!(pane.view_start_tick, 0);
    }

    #[test]
    fn toggle_note_returns_action() {
        let mut pane = PianoRollPane::new(Keymap::new());
        let state = AppState::new();

        let action = pane.handle_action(ActionId::PianoRoll(PianoRollActionId::ToggleNote), &dummy_event(), &state);
        assert!(matches!(action, Action::PianoRoll(PianoRollAction::ToggleNote { .. })));
    }
}
