//! MIDI recording state and CC/pitch bend mappings.

use super::automation::AutomationTarget;
use crate::InstrumentId;
use serde::{Deserialize, Serialize};

/// Recording mode for MIDI automation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordMode {
    /// Not recording
    Off,
    /// Armed for recording (waiting for play to start)
    Armed,
    /// Actively recording
    Recording,
}

impl Default for RecordMode {
    fn default() -> Self {
        Self::Off
    }
}

/// Mapping of a MIDI CC to an automation target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiCcMapping {
    /// MIDI CC number (0-127)
    pub cc_number: u8,
    /// MIDI channel (0-15, or None for any channel)
    pub channel: Option<u8>,
    /// Target parameter to control
    pub target: AutomationTarget,
    /// Min value when CC is 0
    pub min_value: f32,
    /// Max value when CC is 127
    pub max_value: f32,
}

impl MidiCcMapping {
    pub fn new(cc_number: u8, target: AutomationTarget) -> Self {
        let (min_value, max_value) = target.default_range();
        Self {
            cc_number,
            channel: None,
            target,
            min_value,
            max_value,
        }
    }

    /// Map a CC value (0-127) to the target range
    pub fn map_value(&self, cc_value: u8) -> f32 {
        let t = cc_value as f32 / 127.0;
        self.min_value + t * (self.max_value - self.min_value)
    }

    /// Map a value back to CC (0-127)
    pub fn unmap_value(&self, value: f32) -> u8 {
        let t = (value - self.min_value) / (self.max_value - self.min_value);
        (t * 127.0).clamp(0.0, 127.0) as u8
    }
}

/// Pitch bend configuration for scratching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchBendConfig {
    /// Target parameter (usually SampleRate for scratching)
    pub target: AutomationTarget,
    /// Value when pitch bend is at center (0)
    pub center_value: f32,
    /// Range: center_value - range to center_value + range
    pub range: f32,
    /// Sensitivity multiplier
    pub sensitivity: f32,
}

impl PitchBendConfig {
    pub fn new_for_sampler(instrument_id: InstrumentId) -> Self {
        Self {
            target: AutomationTarget::sample_rate(instrument_id),
            center_value: 1.0, // Normal playback speed
            range: 1.0,        // -0.0 (stopped/reverse) to 2.0 (double speed)
            sensitivity: 1.0,
        }
    }

    /// Map pitch bend value (-8192 to 8191) to target value
    pub fn map_value(&self, pitch_bend: i16) -> f32 {
        let t = pitch_bend as f32 / 8192.0; // -1.0 to ~1.0
        self.center_value + t * self.range * self.sensitivity
    }
}

/// State for MIDI recording and mapping
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MidiRecordingState {
    /// Current recording mode
    #[serde(skip)]
    pub record_mode: RecordMode,
    /// CC to automation mappings
    pub cc_mappings: Vec<MidiCcMapping>,
    /// Pitch bend configurations per instrument
    pub pitch_bend_configs: Vec<PitchBendConfig>,
    /// Currently selected instrument for live MIDI input
    pub live_input_instrument: Option<InstrumentId>,
    /// Whether to pass-through MIDI notes to audio engine
    pub note_passthrough: bool,
    /// MIDI channel filter (None = all channels)
    pub channel_filter: Option<u8>,
}

impl MidiRecordingState {
    pub fn new() -> Self {
        Self {
            record_mode: RecordMode::Off,
            cc_mappings: Vec::new(),
            pitch_bend_configs: Vec::new(),
            live_input_instrument: None,
            note_passthrough: true,
            channel_filter: None,
        }
    }

    /// Add a CC mapping
    pub fn add_cc_mapping(&mut self, mapping: MidiCcMapping) {
        // Remove existing mapping for same CC/channel
        self.cc_mappings.retain(|m| {
            !(m.cc_number == mapping.cc_number && m.channel == mapping.channel)
        });
        self.cc_mappings.push(mapping);
    }

    /// Remove a CC mapping
    pub fn remove_cc_mapping(&mut self, cc_number: u8, channel: Option<u8>) {
        self.cc_mappings
            .retain(|m| !(m.cc_number == cc_number && m.channel == channel));
    }

    /// Find mapping for a CC message
    pub fn find_cc_mapping(&self, cc_number: u8, channel: u8) -> Option<&MidiCcMapping> {
        self.cc_mappings.iter().find(|m| {
            m.cc_number == cc_number && (m.channel.is_none() || m.channel == Some(channel))
        })
    }

    /// Add pitch bend config for an instrument
    pub fn add_pitch_bend_config(&mut self, config: PitchBendConfig) {
        // Remove existing config for same target instrument
        let instrument_id = config.target.instrument_id();
        self.pitch_bend_configs
            .retain(|c| c.target.instrument_id() != instrument_id);
        self.pitch_bend_configs.push(config);
    }

    /// Find pitch bend config for an instrument
    pub fn find_pitch_bend_config(&self, instrument_id: InstrumentId) -> Option<&PitchBendConfig> {
        self.pitch_bend_configs
            .iter()
            .find(|c| c.target.instrument_id() == Some(instrument_id))
    }

    /// Arm for recording
    pub fn arm(&mut self) {
        self.record_mode = RecordMode::Armed;
    }

    /// Start recording (called when playback starts if armed)
    pub fn start_recording(&mut self) {
        if self.record_mode == RecordMode::Armed {
            self.record_mode = RecordMode::Recording;
        }
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.record_mode = RecordMode::Off;
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.record_mode == RecordMode::Recording
    }

    /// Check if armed for recording
    pub fn is_armed(&self) -> bool {
        self.record_mode == RecordMode::Armed
    }

    /// Set the instrument for live MIDI input
    pub fn set_live_input_instrument(&mut self, instrument_id: Option<InstrumentId>) {
        self.live_input_instrument = instrument_id;
    }

    /// Check if a MIDI channel should be processed
    pub fn should_process_channel(&self, channel: u8) -> bool {
        self.channel_filter.map_or(true, |f| f == channel)
    }
}

/// Common CC numbers for reference
pub mod cc {
    pub const MOD_WHEEL: u8 = 1;
    pub const BREATH: u8 = 2;
    pub const FOOT: u8 = 4;
    pub const PORTAMENTO_TIME: u8 = 5;
    pub const DATA_ENTRY: u8 = 6;
    pub const VOLUME: u8 = 7;
    pub const BALANCE: u8 = 8;
    pub const PAN: u8 = 10;
    pub const EXPRESSION: u8 = 11;
    pub const SUSTAIN: u8 = 64;
    pub const PORTAMENTO: u8 = 65;
    pub const SOSTENUTO: u8 = 66;
    pub const SOFT_PEDAL: u8 = 67;
    pub const ALL_SOUNDS_OFF: u8 = 120;
    pub const RESET_ALL_CONTROLLERS: u8 = 121;
    pub const ALL_NOTES_OFF: u8 = 123;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_mapping() {
        let mapping = MidiCcMapping::new(1, AutomationTarget::filter_cutoff(0));

        // CC 0 -> min (20 Hz for filter cutoff)
        let val_min = mapping.map_value(0);
        assert!((val_min - 20.0).abs() < 1.0);

        // CC 127 -> max (20000 Hz)
        let val_max = mapping.map_value(127);
        assert!((val_max - 20000.0).abs() < 1.0);

        // CC 64 -> middle
        let val_mid = mapping.map_value(64);
        assert!(val_mid > 100.0 && val_mid < 19000.0);
    }

    #[test]
    fn test_pitch_bend_config() {
        let config = PitchBendConfig::new_for_sampler(0);

        // Center = normal playback
        let val_center = config.map_value(0);
        assert!((val_center - 1.0).abs() < 0.01);

        // Full up = double speed
        let val_up = config.map_value(8191);
        assert!(val_up > 1.9 && val_up < 2.1);

        // Full down = reverse/stopped
        let val_down = config.map_value(-8192);
        assert!(val_down < 0.1);
    }

    #[test]
    fn test_midi_recording_state() {
        let mut state = MidiRecordingState::new();

        // Add CC mapping
        state.add_cc_mapping(MidiCcMapping::new(1, AutomationTarget::filter_cutoff(0)));
        assert!(state.find_cc_mapping(1, 0).is_some());
        assert!(state.find_cc_mapping(2, 0).is_none());

        // Record mode cycling
        assert_eq!(state.record_mode, RecordMode::Off);
        state.arm();
        assert_eq!(state.record_mode, RecordMode::Armed);
        state.start_recording();
        assert_eq!(state.record_mode, RecordMode::Recording);
        state.stop_recording();
        assert_eq!(state.record_mode, RecordMode::Off);
    }
}
