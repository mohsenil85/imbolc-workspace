use crate::action::{Action, AutomationAction, InstrumentAction};
use crate::midi::{MidiEvent, MidiEventKind};
use crate::state::AppState;

/// Process a MIDI event and return an Action if one should be dispatched.
/// The timestamp in MidiEvent can be used for sample-accurate scheduling
/// (passed through InstrumentAction::PlayNoteWithOffset if needed).
pub fn process_midi_event(event: &MidiEvent, state: &AppState) -> Option<Action> {
    let midi_rec = &state.session.midi_recording;

    match &event.kind {
        MidiEventKind::ControlChange {
            channel,
            controller,
            value,
        } => {
            // Check channel filter
            if !midi_rec.should_process_channel(*channel) {
                return None;
            }

            // Look up CC mapping
            let mapping = midi_rec.find_cc_mapping(*controller, *channel)?;
            let target = mapping.target.clone();
            let mapped_value = mapping.map_value(*value);

            // RecordValue always applies to audio engine; recording logic is in the dispatch handler
            let normalized = target.normalize_value(mapped_value);
            Some(Action::Automation(AutomationAction::RecordValue(
                target, normalized,
            )))
        }

        MidiEventKind::NoteOn {
            channel,
            note,
            velocity,
        } => {
            if !midi_rec.should_process_channel(*channel) {
                return None;
            }

            if !midi_rec.note_passthrough {
                return None;
            }

            // PlayNote uses the selected instrument
            Some(Action::Instrument(InstrumentAction::PlayNote(
                *note, *velocity,
            )))
        }

        MidiEventKind::NoteOff { channel, .. } => {
            // Note release is handled by voice duration in the audio engine
            if !midi_rec.should_process_channel(*channel) {
                return None;
            }
            None
        }

        MidiEventKind::PitchBend { channel, value } => {
            if !midi_rec.should_process_channel(*channel) {
                return None;
            }

            // Look up pitch bend config for the target instrument
            let instrument_id = midi_rec
                .live_input_instrument
                .or_else(|| state.instruments.selected_instrument().map(|i| i.id))?;

            let config = midi_rec.find_pitch_bend_config(instrument_id)?;
            let target = config.target.clone();
            let mapped_value = config.map_value(*value);

            let normalized = target.normalize_value(mapped_value);
            Some(Action::Automation(AutomationAction::RecordValue(
                target, normalized,
            )))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::automation::AutomationTarget;
    use crate::state::midi_recording::MidiCcMapping;
    use imbolc_types::InstrumentId;

    fn test_state() -> AppState {
        let mut state = AppState::new();
        // Add a CC mapping: CC 1 -> FilterCutoff of instrument 0
        state
            .session
            .midi_recording
            .add_cc_mapping(MidiCcMapping::new(
                1,
                AutomationTarget::filter_cutoff(InstrumentId::new(0)),
            ));
        state
    }

    #[test]
    fn test_cc_mapped_returns_action() {
        let state = test_state();
        let event = MidiEvent::new(
            0,
            MidiEventKind::ControlChange {
                channel: 0,
                controller: 1,
                value: 64,
            },
        );
        let action = process_midi_event(&event, &state);
        assert!(action.is_some());
    }

    #[test]
    fn test_cc_unmapped_returns_none() {
        let state = test_state();
        let event = MidiEvent::new(
            0,
            MidiEventKind::ControlChange {
                channel: 0,
                controller: 99,
                value: 64,
            },
        );
        let action = process_midi_event(&event, &state);
        assert!(action.is_none());
    }

    #[test]
    fn test_channel_filter_blocks() {
        let mut state = test_state();
        state.session.midi_recording.channel_filter = Some(1); // Only channel 1
        let event = MidiEvent::new(
            0,
            MidiEventKind::ControlChange {
                channel: 0,
                controller: 1,
                value: 64,
            },
        );
        let action = process_midi_event(&event, &state);
        assert!(action.is_none());
    }

    #[test]
    fn test_note_passthrough_no_instrument() {
        let state = test_state();
        // No instrument selected, so note passthrough should return action anyway (PlayNote)
        let event = MidiEvent::new(
            0,
            MidiEventKind::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            },
        );
        let action = process_midi_event(&event, &state);
        // PlayNote dispatches to selected instrument, which will be a no-op if none
        assert!(action.is_some());
    }

    #[test]
    fn test_note_passthrough_disabled() {
        let mut state = test_state();
        state.session.midi_recording.note_passthrough = false;
        let event = MidiEvent::new(
            0,
            MidiEventKind::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            },
        );
        let action = process_midi_event(&event, &state);
        assert!(action.is_none());
    }
}
