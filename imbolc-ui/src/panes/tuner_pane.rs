use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, TunerActionId};
use crate::ui::layout_helpers::center_rect;
use crate::action::TunerAction;
use crate::ui::{Action, Color, InputEvent, Keymap, Pane, Rect, RenderBuf, Style};

// ── Instrument presets ──────────────────────────────────────────────────

struct TunerPreset {
    name: &'static str,
    strings: &'static [(&'static str, u8)], // (note_name, midi_note) high→low
}

const PRESETS: &[TunerPreset] = &[
    TunerPreset {
        name: "Guitar",
        strings: &[("E4", 64), ("B3", 59), ("G3", 55), ("D3", 50), ("A2", 45), ("E2", 40)],
    },
    TunerPreset {
        name: "Bass",
        strings: &[("G2", 43), ("D2", 38), ("A1", 33), ("E1", 28)],
    },
    TunerPreset {
        name: "Ukulele",
        strings: &[("A4", 69), ("E4", 64), ("C4", 60), ("G4", 67)],
    },
    TunerPreset {
        name: "Guitulele",
        strings: &[("A4", 69), ("E4", 64), ("C4", 60), ("G3", 55), ("D3", 50), ("A2", 45)],
    },
];

// ── Pane state ──────────────────────────────────────────────────────────

pub struct TunerPane {
    keymap: Keymap,
    instrument_idx: usize,
    string_idx: usize,
    playing: bool,
}

impl TunerPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            instrument_idx: 0,
            string_idx: 0,
            playing: false,
        }
    }

    fn preset(&self) -> &TunerPreset {
        &PRESETS[self.instrument_idx]
    }

    fn midi_to_freq(midi: u8, tuning_a4: f32) -> f32 {
        tuning_a4 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
    }
}

impl Default for TunerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for TunerPane {
    fn id(&self) -> &'static str {
        "tuner"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let ActionId::Tuner(a) = action else {
            return Action::None;
        };

        match a {
            TunerActionId::PrevInstrument => {
                let was_playing = self.playing;
                if self.instrument_idx == 0 {
                    self.instrument_idx = PRESETS.len() - 1;
                } else {
                    self.instrument_idx -= 1;
                }
                self.string_idx = 0;
                self.playing = false;
                if was_playing {
                    Action::Tuner(TunerAction::StopTone)
                } else {
                    Action::None
                }
            }
            TunerActionId::NextInstrument => {
                let was_playing = self.playing;
                self.instrument_idx = (self.instrument_idx + 1) % PRESETS.len();
                self.string_idx = 0;
                self.playing = false;
                if was_playing {
                    Action::Tuner(TunerAction::StopTone)
                } else {
                    Action::None
                }
            }
            TunerActionId::PrevString => {
                let count = self.preset().strings.len();
                if self.string_idx == 0 {
                    self.string_idx = count - 1;
                } else {
                    self.string_idx -= 1;
                }
                if self.playing {
                    let midi = self.preset().strings[self.string_idx].1;
                    let freq = Self::midi_to_freq(midi, state.session.tuning_a4);
                    return Action::Tuner(TunerAction::PlayTone(freq));
                }
                Action::None
            }
            TunerActionId::NextString => {
                let count = self.preset().strings.len();
                self.string_idx = (self.string_idx + 1) % count;
                if self.playing {
                    let midi = self.preset().strings[self.string_idx].1;
                    let freq = Self::midi_to_freq(midi, state.session.tuning_a4);
                    return Action::Tuner(TunerAction::PlayTone(freq));
                }
                Action::None
            }
            TunerActionId::PlayStop => {
                if self.playing {
                    self.playing = false;
                    Action::Tuner(TunerAction::StopTone)
                } else {
                    self.playing = true;
                    let midi = self.preset().strings[self.string_idx].1;
                    let freq = Self::midi_to_freq(midi, state.session.tuning_a4);
                    Action::Tuner(TunerAction::PlayTone(freq))
                }
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let preset = self.preset();
        let height = (preset.strings.len() as u16) + 6; // title + instrument + a4 + gap + strings
        let width = 44;
        let inner = center_rect(area, width, height);

        // Background
        let bg = Color::new(25, 25, 35);
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                buf.set_cell(x, y, ' ', Style::new().bg(bg));
            }
        }

        let mut y = inner.y;

        // Title
        let title = "Reference Tuner";
        let tx = inner.x + (inner.width.saturating_sub(title.len() as u16)) / 2;
        buf.draw_str(tx, y, title, Style::new().fg(Color::WHITE).bg(bg));
        y += 2;

        // Instrument selector
        let inst_line = format!("<  {}  >", preset.name);
        let ix = inner.x + (inner.width.saturating_sub(inst_line.len() as u16)) / 2;
        buf.draw_str(ix, y, &inst_line, Style::new().fg(Color::new(255, 200, 100)).bg(bg));
        y += 1;

        // A4 tuning value
        let a4_line = format!("A4 = {:.1} Hz", state.session.tuning_a4);
        let ax = inner.x + (inner.width.saturating_sub(a4_line.len() as u16)) / 2;
        buf.draw_str(ax, y, &a4_line, Style::new().fg(Color::new(120, 120, 140)).bg(bg));
        y += 2;

        // String list
        for (i, (note_name, midi)) in preset.strings.iter().enumerate() {
            let freq = Self::midi_to_freq(*midi, state.session.tuning_a4);
            let is_selected = i == self.string_idx;

            let marker = if is_selected && self.playing {
                ">>"
            } else if is_selected {
                " >"
            } else {
                "  "
            };

            let line = format!("{} {:>3}  {:>8.2} Hz", marker, note_name, freq);
            let sx = inner.x + (inner.width.saturating_sub(line.len() as u16)) / 2;

            let style = if is_selected {
                Style::new().fg(Color::new(100, 220, 255)).bg(bg)
            } else {
                Style::new().fg(Color::new(180, 180, 190)).bg(bg)
            };

            buf.draw_str(sx, y, &line, style);
            y += 1;
        }

    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn on_exit(&mut self, _state: &AppState) {
        self.playing = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::KeyCode;

    fn make_state() -> AppState {
        AppState::new()
    }

    fn make_event() -> InputEvent {
        InputEvent::new(KeyCode::Enter, crate::ui::input::Modifiers::none())
    }

    #[test]
    fn test_play_stop_toggle() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        // Initially not playing
        assert!(!pane.playing);

        // Play
        let action = pane.handle_action(ActionId::Tuner(TunerActionId::PlayStop), &event, &state);
        assert!(pane.playing);
        assert!(matches!(action, Action::Tuner(TunerAction::PlayTone(_))));

        // Stop
        let action = pane.handle_action(ActionId::Tuner(TunerActionId::PlayStop), &event, &state);
        assert!(!pane.playing);
        assert!(matches!(action, Action::Tuner(TunerAction::StopTone)));
    }

    #[test]
    fn test_next_instrument_wraps() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        assert_eq!(pane.instrument_idx, 0); // Guitar
        pane.handle_action(ActionId::Tuner(TunerActionId::NextInstrument), &event, &state);
        assert_eq!(pane.instrument_idx, 1); // Bass
        pane.handle_action(ActionId::Tuner(TunerActionId::NextInstrument), &event, &state);
        assert_eq!(pane.instrument_idx, 2); // Ukulele
        pane.handle_action(ActionId::Tuner(TunerActionId::NextInstrument), &event, &state);
        assert_eq!(pane.instrument_idx, 3); // Guitulele
        pane.handle_action(ActionId::Tuner(TunerActionId::NextInstrument), &event, &state);
        assert_eq!(pane.instrument_idx, 0); // Wraps to Guitar
    }

    #[test]
    fn test_prev_instrument_wraps() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        assert_eq!(pane.instrument_idx, 0);
        pane.handle_action(ActionId::Tuner(TunerActionId::PrevInstrument), &event, &state);
        assert_eq!(pane.instrument_idx, PRESETS.len() - 1);
    }

    #[test]
    fn test_string_navigation() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        // Guitar has 6 strings
        assert_eq!(pane.string_idx, 0);
        pane.handle_action(ActionId::Tuner(TunerActionId::NextString), &event, &state);
        assert_eq!(pane.string_idx, 1);
        // Go back
        pane.handle_action(ActionId::Tuner(TunerActionId::PrevString), &event, &state);
        assert_eq!(pane.string_idx, 0);
        // Wrap backwards
        pane.handle_action(ActionId::Tuner(TunerActionId::PrevString), &event, &state);
        assert_eq!(pane.string_idx, 5); // last string of guitar
    }

    #[test]
    fn test_instrument_switch_resets_string_and_stops() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        pane.string_idx = 3;
        pane.playing = true;
        let action = pane.handle_action(ActionId::Tuner(TunerActionId::NextInstrument), &event, &state);
        assert_eq!(pane.string_idx, 0);
        assert!(!pane.playing);
        assert!(matches!(action, Action::Tuner(TunerAction::StopTone)));
    }

    #[test]
    fn test_on_exit_resets_playing() {
        let mut pane = TunerPane::default();
        let state = make_state();
        pane.playing = true;
        pane.on_exit(&state);
        assert!(!pane.playing);
    }

    #[test]
    fn test_midi_to_freq() {
        // A4 = 69 should give exactly the tuning frequency
        let freq = TunerPane::midi_to_freq(69, 432.0);
        assert!((freq - 432.0).abs() < 0.01);

        // A4 = 69 at 440 Hz
        let freq = TunerPane::midi_to_freq(69, 440.0);
        assert!((freq - 440.0).abs() < 0.01);

        // E2 = 40 at 432 Hz
        let freq = TunerPane::midi_to_freq(40, 432.0);
        let expected = 432.0 * 2.0_f32.powf((40.0 - 69.0) / 12.0);
        assert!((freq - expected).abs() < 0.01);
    }

    #[test]
    fn test_string_change_updates_freq_when_playing() {
        let mut pane = TunerPane::default();
        let state = make_state();
        let event = make_event();

        // Start playing
        pane.handle_action(ActionId::Tuner(TunerActionId::PlayStop), &event, &state);
        assert!(pane.playing);

        // Navigate to next string — should emit PlayTone with new freq
        let action = pane.handle_action(ActionId::Tuner(TunerActionId::NextString), &event, &state);
        assert!(matches!(action, Action::Tuner(TunerAction::PlayTone(_))));
    }
}
