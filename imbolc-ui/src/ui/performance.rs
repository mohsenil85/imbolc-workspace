use std::time::Instant;

use super::pad_keyboard::PadKeyboard;
use super::piano_keyboard::PianoKeyboard;
use super::pane::ToggleResult;
use crate::ui::{Action, PianoRollAction};
use imbolc_types::InstrumentId;

/// Shared piano/pad keyboard lifecycle controller.
///
/// Used by InstrumentPane, InstrumentEditPane, and (piano-only subset) PianoRollPane
/// to eliminate duplicated toggle/activate/deactivate/tick logic.
pub struct PerformanceController {
    pub piano: PianoKeyboard,
    pub pad: PadKeyboard,
}

impl PerformanceController {
    pub fn new() -> Self {
        Self {
            piano: PianoKeyboard::new(),
            pad: PadKeyboard::new(),
        }
    }

    /// Toggle between piano/pad/off. Uses `is_kit` to decide which mode to activate.
    pub fn toggle(&mut self, is_kit: bool) -> ToggleResult {
        if self.pad.is_active() {
            self.pad.deactivate();
            ToggleResult::Deactivated
        } else if self.piano.is_active() {
            self.piano.handle_escape();
            if self.piano.is_active() {
                ToggleResult::CycledLayout
            } else {
                ToggleResult::Deactivated
            }
        } else if is_kit {
            self.pad.activate();
            ToggleResult::ActivatedPad
        } else {
            self.piano.activate();
            ToggleResult::ActivatedPiano
        }
    }

    pub fn set_enhanced_keyboard(&mut self, enabled: bool) {
        self.piano.set_enhanced_keyboard(enabled);
    }

    pub fn activate_piano(&mut self) {
        if !self.piano.is_active() { self.piano.activate(); }
        self.pad.deactivate();
    }

    pub fn activate_pad(&mut self) {
        if !self.pad.is_active() { self.pad.activate(); }
        self.piano.deactivate();
    }

    pub fn deactivate(&mut self) {
        self.piano.release_all();
        self.piano.deactivate();
        self.pad.deactivate();
    }

    /// Check for released keys and return ReleaseNote/ReleaseNotes actions.
    pub fn tick_releases(&mut self, instrument_id: InstrumentId) -> Vec<Action> {
        if !self.piano.is_active() || !self.piano.has_active_keys() {
            return vec![];
        }
        let now = Instant::now();
        let released = self.piano.check_releases(now);
        if released.is_empty() {
            return vec![];
        }
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
}
