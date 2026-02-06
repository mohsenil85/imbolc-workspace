use crate::action::{DispatchResult, MidiAction};
use crate::state::AppState;
use crate::state::midi_recording::MidiCcMapping;

pub(super) fn dispatch_midi(action: &MidiAction, state: &mut AppState) -> DispatchResult {
    match action {
        MidiAction::ConnectPort(_port_index) => {
            // Port connection is intercepted in main.rs (needs MidiInputManager)
            DispatchResult::none()
        }
        MidiAction::DisconnectPort => {
            // Port disconnection is intercepted in main.rs (needs MidiInputManager)
            state.midi.connected_port = None;
            DispatchResult::none()
        }
        MidiAction::AddCcMapping { cc, channel, target } => {
            let mut mapping = MidiCcMapping::new(*cc, target.clone());
            mapping.channel = *channel;
            state.session.midi_recording.add_cc_mapping(mapping);
            DispatchResult::none()
        }
        MidiAction::RemoveCcMapping { cc, channel } => {
            state.session.midi_recording.remove_cc_mapping(*cc, *channel);
            DispatchResult::none()
        }
        MidiAction::SetChannelFilter(channel) => {
            state.session.midi_recording.channel_filter = *channel;
            DispatchResult::none()
        }
        MidiAction::SetLiveInputInstrument(instrument_id) => {
            state.session.midi_recording.live_input_instrument = *instrument_id;
            DispatchResult::none()
        }
        MidiAction::ToggleNotePassthrough => {
            state.session.midi_recording.note_passthrough = !state.session.midi_recording.note_passthrough;
            DispatchResult::none()
        }
    }
}
