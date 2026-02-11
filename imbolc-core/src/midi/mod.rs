#![allow(dead_code)]

use midir::{MidiInput, MidiInputConnection};
use std::sync::mpsc::{self, Receiver, Sender};

/// MIDI event types with optional timestamp for sample-accurate scheduling.
/// Timestamp is in microseconds from a driver-specific epoch.
#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    /// Event timestamp in microseconds (driver-specific epoch)
    pub timestamp_us: u64,
    /// The actual MIDI event data
    pub kind: MidiEventKind,
}

/// The specific type of MIDI event
#[derive(Debug, Clone, Copy)]
pub enum MidiEventKind {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    PitchBend {
        channel: u8,
        /// Pitch bend value: -8192 (full down) to +8191 (full up), 0 = center
        value: i16,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    Aftertouch {
        channel: u8,
        pressure: u8,
    },
    PolyAftertouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
}

impl MidiEvent {
    /// Create a new MidiEvent with timestamp
    pub fn new(timestamp_us: u64, kind: MidiEventKind) -> Self {
        Self { timestamp_us, kind }
    }

    /// Get the relative offset in seconds from when the event was captured to now.
    /// Returns 0.0 if the event is in the past (most common case).
    /// This uses the difference between event timestamp and a reference time.
    pub fn offset_secs_from(&self, reference_us: u64) -> f64 {
        if self.timestamp_us > reference_us {
            // Event is scheduled slightly in the future (rare but possible with some drivers)
            (self.timestamp_us - reference_us) as f64 / 1_000_000.0
        } else {
            // Event is in the past - use 0 offset for immediate playback
            0.0
        }
    }
}

/// Information about an available MIDI port
#[derive(Debug, Clone)]
pub struct MidiPortInfo {
    pub index: usize,
    pub name: String,
}

/// MIDI input manager
pub struct MidiInputManager {
    midi_in: Option<MidiInput>,
    connection: Option<MidiInputConnection<()>>,
    event_receiver: Option<Receiver<MidiEvent>>,
    event_sender: Option<Sender<MidiEvent>>,
    connected_port_name: Option<String>,
    available_ports: Vec<MidiPortInfo>,
}

impl MidiInputManager {
    pub fn new() -> Self {
        let midi_in = MidiInput::new("imbolc").ok();
        Self {
            midi_in,
            connection: None,
            event_receiver: None,
            event_sender: None,
            connected_port_name: None,
            available_ports: Vec::new(),
        }
    }

    /// Refresh the list of available MIDI input ports
    pub fn refresh_ports(&mut self) {
        self.available_ports.clear();

        if let Some(ref midi_in) = self.midi_in {
            let ports = midi_in.ports();
            for (index, port) in ports.iter().enumerate() {
                if let Ok(name) = midi_in.port_name(port) {
                    self.available_ports.push(MidiPortInfo { index, name });
                }
            }
        }
    }

    /// Get list of available MIDI input ports
    pub fn list_ports(&self) -> &[MidiPortInfo] {
        &self.available_ports
    }

    /// Check if connected to a MIDI port
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Get the name of the connected port
    pub fn connected_port_name(&self) -> Option<&str> {
        self.connected_port_name.as_deref()
    }

    /// Connect to a MIDI input port by index
    pub fn connect(&mut self, port_index: usize) -> Result<(), String> {
        // Disconnect existing connection first
        self.disconnect();

        // Need to recreate MidiInput after taking ownership for connection
        let midi_in = MidiInput::new("imbolc").map_err(|e| e.to_string())?;
        let ports = midi_in.ports();

        if port_index >= ports.len() {
            return Err(format!("Invalid port index: {}", port_index));
        }

        let port = &ports[port_index];
        let port_name = midi_in
            .port_name(port)
            .unwrap_or_else(|_| "Unknown".to_string());

        let (tx, rx) = mpsc::channel();
        self.event_sender = Some(tx.clone());
        self.event_receiver = Some(rx);

        let connection = midi_in
            .connect(
                port,
                "imbolc-input",
                move |timestamp, message, _| {
                    if let Some(kind) = parse_midi_message(message) {
                        let _ = tx.send(MidiEvent::new(timestamp, kind));
                    }
                },
                (),
            )
            .map_err(|e| e.to_string())?;

        self.connection = Some(connection);
        self.connected_port_name = Some(port_name);

        // Recreate MidiInput for future port listing
        self.midi_in = MidiInput::new("imbolc").ok();

        Ok(())
    }

    /// Disconnect from the current MIDI input port
    pub fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close();
        }
        self.event_receiver = None;
        self.event_sender = None;
        self.connected_port_name = None;
    }

    /// Poll for pending MIDI events (non-blocking)
    pub fn poll_events(&self) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        if let Some(ref rx) = self.event_receiver {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        events
    }

    /// Poll for a single MIDI event (non-blocking)
    pub fn poll_event(&self) -> Option<MidiEvent> {
        self.event_receiver.as_ref()?.try_recv().ok()
    }
}

impl Default for MidiInputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MidiInputManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Parse a raw MIDI message into a MidiEventKind
fn parse_midi_message(data: &[u8]) -> Option<MidiEventKind> {
    if data.is_empty() {
        return None;
    }

    let status = data[0];
    let channel = status & 0x0F;
    let message_type = status & 0xF0;

    match message_type {
        0x80 => {
            // Note Off
            if data.len() >= 3 {
                Some(MidiEventKind::NoteOff {
                    channel,
                    note: data[1],
                })
            } else {
                None
            }
        }
        0x90 => {
            // Note On (velocity 0 = note off)
            if data.len() >= 3 {
                let velocity = data[2];
                if velocity == 0 {
                    Some(MidiEventKind::NoteOff {
                        channel,
                        note: data[1],
                    })
                } else {
                    Some(MidiEventKind::NoteOn {
                        channel,
                        note: data[1],
                        velocity,
                    })
                }
            } else {
                None
            }
        }
        0xA0 => {
            // Polyphonic Aftertouch
            if data.len() >= 3 {
                Some(MidiEventKind::PolyAftertouch {
                    channel,
                    note: data[1],
                    pressure: data[2],
                })
            } else {
                None
            }
        }
        0xB0 => {
            // Control Change
            if data.len() >= 3 {
                Some(MidiEventKind::ControlChange {
                    channel,
                    controller: data[1],
                    value: data[2],
                })
            } else {
                None
            }
        }
        0xC0 => {
            // Program Change
            if data.len() >= 2 {
                Some(MidiEventKind::ProgramChange {
                    channel,
                    program: data[1],
                })
            } else {
                None
            }
        }
        0xD0 => {
            // Channel Aftertouch
            if data.len() >= 2 {
                Some(MidiEventKind::Aftertouch {
                    channel,
                    pressure: data[1],
                })
            } else {
                None
            }
        }
        0xE0 => {
            // Pitch Bend
            if data.len() >= 3 {
                let lsb = data[1] as i16;
                let msb = data[2] as i16;
                let value = ((msb << 7) | lsb) - 8192; // Center at 0
                Some(MidiEventKind::PitchBend { channel, value })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let data = [0x90, 60, 100]; // Note On, channel 0, note 60, velocity 100
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::NoteOn {
                channel,
                note,
                velocity,
            } => {
                assert_eq!(channel, 0);
                assert_eq!(note, 60);
                assert_eq!(velocity, 100);
            }
            _ => panic!("Expected NoteOn"),
        }
    }

    #[test]
    fn test_parse_note_off() {
        let data = [0x80, 60, 0]; // Note Off, channel 0, note 60
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::NoteOff { channel, note } => {
                assert_eq!(channel, 0);
                assert_eq!(note, 60);
            }
            _ => panic!("Expected NoteOff"),
        }
    }

    #[test]
    fn test_parse_note_on_velocity_zero() {
        let data = [0x90, 60, 0]; // Note On with velocity 0 = Note Off
        let event = parse_midi_message(&data).unwrap();
        assert!(matches!(event, MidiEventKind::NoteOff { .. }));
    }

    #[test]
    fn test_parse_pitch_bend() {
        // Center (no bend)
        let data = [0xE0, 0x00, 0x40]; // LSB=0, MSB=64 = 8192 = center
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::PitchBend { channel, value } => {
                assert_eq!(channel, 0);
                assert_eq!(value, 0);
            }
            _ => panic!("Expected PitchBend"),
        }

        // Full up
        let data = [0xE0, 0x7F, 0x7F]; // LSB=127, MSB=127 = 16383 - 8192 = 8191
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::PitchBend { value, .. } => {
                assert_eq!(value, 8191);
            }
            _ => panic!("Expected PitchBend"),
        }

        // Full down
        let data = [0xE0, 0x00, 0x00]; // LSB=0, MSB=0 = 0 - 8192 = -8192
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::PitchBend { value, .. } => {
                assert_eq!(value, -8192);
            }
            _ => panic!("Expected PitchBend"),
        }
    }

    #[test]
    fn test_parse_control_change() {
        let data = [0xB0, 1, 64]; // CC, channel 0, controller 1 (mod wheel), value 64
        let event = parse_midi_message(&data).unwrap();
        match event {
            MidiEventKind::ControlChange {
                channel,
                controller,
                value,
            } => {
                assert_eq!(channel, 0);
                assert_eq!(controller, 1);
                assert_eq!(value, 64);
            }
            _ => panic!("Expected ControlChange"),
        }
    }

    #[test]
    fn test_parse_empty_message_returns_none() {
        assert!(parse_midi_message(&[]).is_none());
    }

    #[test]
    fn test_parse_short_messages_return_none() {
        assert!(parse_midi_message(&[0x90, 60]).is_none());
        assert!(parse_midi_message(&[0xB0, 1]).is_none());
        assert!(parse_midi_message(&[0xE0, 0x00]).is_none());
    }

    #[test]
    fn test_parse_unknown_status_returns_none() {
        assert!(parse_midi_message(&[0x00]).is_none());
        assert!(parse_midi_message(&[0xF0, 0x01, 0x02]).is_none());
    }
}
