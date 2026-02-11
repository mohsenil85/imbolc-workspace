use std::any::Any;

use crate::state::music::{Key, Scale};
use crate::state::{AppState, MusicalSettings};
use crate::ui::action_id::{ActionId, FrameEditActionId, ModeActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, Pane, SessionAction, Style};
use crate::ui::widgets::TextInput;

/// Fields editable in the frame editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Bpm,
    TimeSig,
    Tuning,
    Key,
    Scale,
    Snap,
}

const FIELDS: [Field; 6] = [Field::Bpm, Field::TimeSig, Field::Tuning, Field::Key, Field::Scale, Field::Snap];

pub struct FrameEditPane {
    keymap: Keymap,
    settings: MusicalSettings,
    original_settings: MusicalSettings,
    selected: usize,
    editing: bool,
    edit_input: TextInput,
}

impl FrameEditPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            settings: MusicalSettings::default(),
            original_settings: MusicalSettings::default(),
            selected: 0,
            editing: false,
            edit_input: TextInput::new(""),
        }
    }

    /// Set musical settings to edit (called before switching to this pane)
    #[allow(dead_code)]
    pub fn set_settings(&mut self, settings: MusicalSettings) {
        self.settings = settings;
        self.original_settings = self.settings.clone();
        self.selected = 0;
        self.editing = false;
    }

    fn current_field(&self) -> Field {
        FIELDS[self.selected]
    }

    fn cycle_key(&mut self, forward: bool) {
        let idx = Key::ALL.iter().position(|k| *k == self.settings.key).unwrap_or(0);
        self.settings.key = if forward {
            Key::ALL[(idx + 1) % 12]
        } else {
            Key::ALL[(idx + 11) % 12]
        };
    }

    fn cycle_scale(&mut self, forward: bool) {
        let idx = Scale::ALL.iter().position(|s| *s == self.settings.scale).unwrap_or(0);
        let len = Scale::ALL.len();
        self.settings.scale = if forward {
            Scale::ALL[(idx + 1) % len]
        } else {
            Scale::ALL[(idx + len - 1) % len]
        };
    }

    const TIME_SIGS: [(u8, u8); 5] = [(4, 4), (3, 4), (6, 8), (5, 4), (7, 8)];

    fn cycle_time_sig(&mut self, forward: bool) {
        let idx = Self::TIME_SIGS.iter().position(|ts| *ts == self.settings.time_signature).unwrap_or(0);
        let len = Self::TIME_SIGS.len();
        self.settings.time_signature = if forward {
            Self::TIME_SIGS[(idx + 1) % len]
        } else {
            Self::TIME_SIGS[(idx + len - 1) % len]
        };
    }

    fn adjust(&mut self, increase: bool) {
        match self.current_field() {
            Field::Bpm => {
                let delta: i16 = if increase { 1 } else { -1 };
                self.settings.bpm = (self.settings.bpm as i16 + delta).clamp(20, 300) as u16;
            }
            Field::TimeSig => self.cycle_time_sig(increase),
            Field::Tuning => {
                let delta: f32 = if increase { 1.0 } else { -1.0 };
                self.settings.tuning_a4 = (self.settings.tuning_a4 + delta).clamp(400.0, 480.0);
            }
            Field::Key => self.cycle_key(increase),
            Field::Scale => self.cycle_scale(increase),
            Field::Snap => self.settings.snap = !self.settings.snap,
        }
    }

    fn field_label(field: Field) -> &'static str {
        match field {
            Field::Bpm => "BPM",
            Field::TimeSig => "Time Sig",
            Field::Tuning => "Tuning (A4)",
            Field::Key => "Key",
            Field::Scale => "Scale",
            Field::Snap => "Snap",
        }
    }

    fn field_value(&self, field: Field) -> String {
        match field {
            Field::Bpm => format!("{}", self.settings.bpm),
            Field::TimeSig => format!("{}/{}", self.settings.time_signature.0, self.settings.time_signature.1),
            Field::Tuning => format!("{:.1} Hz", self.settings.tuning_a4),
            Field::Key => self.settings.key.name().to_string(),
            Field::Scale => self.settings.scale.name().to_string(),
            Field::Snap => if self.settings.snap { "ON".into() } else { "OFF".into() },
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }
}

impl Default for FrameEditPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for FrameEditPane {
    fn id(&self) -> &'static str {
        "frame_edit"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        if let ActionId::Mode(mode_action) = action {
            return match mode_action {
            // Text edit layer actions
            ModeActionId::TextConfirm => {
                let text = self.edit_input.value().to_string();
                match self.current_field() {
                    Field::Bpm => {
                        if let Ok(v) = text.parse::<u16>() {
                            self.settings.bpm = v.clamp(20, 300);
                        }
                    }
                    Field::Tuning => {
                        if let Ok(v) = text.parse::<f32>() {
                            self.settings.tuning_a4 = v.clamp(400.0, 480.0);
                        }
                    }
                    _ => {}
                }
                self.editing = false;
                self.edit_input.set_focused(false);
                Action::Session(SessionAction::UpdateSession(self.settings.clone()))
            }
            ModeActionId::TextCancel => {
                self.editing = false;
                self.edit_input.set_focused(false);
                self.settings = self.original_settings.clone();
                Action::Session(SessionAction::UpdateSession(self.original_settings.clone()))
            }
            ModeActionId::PianoEscape
            | ModeActionId::PianoOctaveDown
            | ModeActionId::PianoOctaveUp
            | ModeActionId::PianoSpace
            | ModeActionId::PianoKey
            | ModeActionId::PadEscape
            | ModeActionId::PadKey
            | ModeActionId::PaletteConfirm
            | ModeActionId::PaletteCancel => Action::None,
            };
        }

        let ActionId::FrameEdit(action) = action else {
            return Action::None;
        };

        match action {
            FrameEditActionId::Prev => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            FrameEditActionId::Next => {
                if self.selected < FIELDS.len() - 1 {
                    self.selected += 1;
                }
                Action::None
            }
            FrameEditActionId::Decrease => {
                self.adjust(false);
                Action::Session(SessionAction::UpdateSessionLive(self.settings.clone()))
            }
            FrameEditActionId::Increase => {
                self.adjust(true);
                Action::Session(SessionAction::UpdateSessionLive(self.settings.clone()))
            }
            FrameEditActionId::Confirm => {
                let field = self.current_field();
                if matches!(field, Field::Bpm | Field::Tuning) {
                    let val = match field {
                        Field::Bpm => format!("{}", self.settings.bpm),
                        Field::Tuning => format!("{:.1}", self.settings.tuning_a4),
                        _ => unreachable!(),
                    };
                    self.edit_input.set_value(&val);
                    self.edit_input.select_all();
                    self.edit_input.set_focused(true);
                    self.editing = true;
                    Action::PushLayer("text_edit")
                } else {
                    Action::Session(SessionAction::UpdateSession(self.settings.clone()))
                }
            }
            FrameEditActionId::Cancel => {
                self.settings = self.original_settings.clone();
                Action::Session(SessionAction::UpdateSession(self.original_settings.clone()))
            }
        }
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        if self.editing {
            self.edit_input.handle_input(event);
        }
        Action::None
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 50, 13);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Session ", border_style, border_style);

        let label_col = inner.x + 2;
        let value_col = label_col + 15;

        for (i, field) in FIELDS.iter().enumerate() {
            let y = inner.y + 1 + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let is_selected = i == self.selected;
            let sel_bg = Style::new().bg(Color::SELECTION_BG);

            // Indicator
            if is_selected {
                buf.set_cell(label_col, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
            }

            // Label
            let label_style = if is_selected {
                Style::new().fg(Color::CYAN).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::CYAN)
            };
            let label = format!("{:14}", Self::field_label(*field));
            buf.draw_line(Rect::new(label_col + 2, y, 14, 1), &[(&label, label_style)]);

            // Value
            if is_selected && self.editing {
                // Render TextInput inline
                self.edit_input.render_buf(buf.raw_buf(), value_col, y, inner.width.saturating_sub(18));
            } else {
                let val_style = if is_selected {
                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::WHITE)
                };
                let val = self.field_value(*field);
                buf.draw_line(Rect::new(value_col, y, inner.width.saturating_sub(18), 1), &[(&val, val_style)]);

                // Fill rest of line with selection bg
                if is_selected {
                    let fill_start = value_col + val.len() as u16;
                    let fill_end = inner.x + inner.width;
                    for x in fill_start..fill_end {
                        buf.set_cell(x, y, ' ', sel_bg);
                    }
                }
            }
        }

    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn on_enter(&mut self, state: &AppState) {
        self.set_settings(state.session.musical_settings());
    }


    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::ui::{InputEvent, KeyCode, Modifiers};

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn confirm_non_text_field_commits() {
        use crate::ui::action_id::{ActionId, FrameEditActionId};
        let mut pane = FrameEditPane::new(Keymap::new());
        let state = AppState::new();
        let settings = MusicalSettings::default();
        pane.set_settings(settings);

        for _ in 0..3 {
            pane.handle_action(ActionId::FrameEdit(FrameEditActionId::Next), &dummy_event(), &state);
        }

        let action = pane.handle_action(ActionId::FrameEdit(FrameEditActionId::Confirm), &dummy_event(), &state);
        match action {
            Action::Session(SessionAction::UpdateSession(updated)) => {
                assert_eq!(updated.key, pane.settings.key);
            }
            _ => panic!("Expected UpdateSession for non-text field confirm"),
        }
    }

    #[test]
    fn text_confirm_commits_and_returns() {
        use crate::ui::action_id::{ActionId, ModeActionId};
        let mut pane = FrameEditPane::new(Keymap::new());
        let state = AppState::new();
        let settings = MusicalSettings::default();
        pane.set_settings(settings);
        pane.edit_input.set_value("180");

        let action = pane.handle_action(ActionId::Mode(ModeActionId::TextConfirm), &dummy_event(), &state);
        match action {
            Action::Session(SessionAction::UpdateSession(updated)) => {
                assert_eq!(updated.bpm, 180);
                assert!(!pane.editing);
            }
            _ => panic!("Expected UpdateSession on text confirm"),
        }
    }

    #[test]
    fn cancel_reverts_to_original_settings() {
        use crate::ui::action_id::{ActionId, FrameEditActionId};
        let mut pane = FrameEditPane::new(Keymap::new());
        let state = AppState::new();
        let settings = MusicalSettings { bpm: 140, ..Default::default() };
        pane.set_settings(settings.clone());

        pane.settings.bpm = 200;

        let action = pane.handle_action(ActionId::FrameEdit(FrameEditActionId::Cancel), &dummy_event(), &state);
        match action {
            Action::Session(SessionAction::UpdateSession(updated)) => {
                assert_eq!(updated, settings);
                assert_eq!(pane.settings, settings);
            }
            _ => panic!("Expected UpdateSession on cancel"),
        }
    }

    #[test]
    fn text_cancel_reverts_to_original_settings() {
        use crate::ui::action_id::{ActionId, ModeActionId};
        let mut pane = FrameEditPane::new(Keymap::new());
        let state = AppState::new();
        let settings = MusicalSettings { tuning_a4: 432.0, ..Default::default() };
        pane.set_settings(settings.clone());

        pane.settings.tuning_a4 = 450.0;
        pane.editing = true;

        let action = pane.handle_action(ActionId::Mode(ModeActionId::TextCancel), &dummy_event(), &state);
        match action {
            Action::Session(SessionAction::UpdateSession(updated)) => {
                assert_eq!(updated, settings);
                assert_eq!(pane.settings, settings);
                assert!(!pane.editing);
            }
            _ => panic!("Expected UpdateSession on text cancel"),
        }
    }
}
