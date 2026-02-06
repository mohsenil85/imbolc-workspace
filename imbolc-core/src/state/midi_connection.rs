//! MIDI connection state (local hardware).

/// MIDI hardware connection state.
#[derive(Debug, Clone, Default)]
pub struct MidiConnectionState {
    /// Available MIDI input port names
    pub port_names: Vec<String>,
    /// Currently connected MIDI port name
    pub connected_port: Option<String>,
}
