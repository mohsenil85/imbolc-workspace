//! Session state and musical settings.

use serde::{Deserialize, Serialize};

use super::arrangement::ArrangementState;
use super::automation::AutomationState;
use super::custom_synthdef::CustomSynthDefRegistry;
use super::humanize::HumanizeSettings;
use super::instrument::MixerBus;
use super::midi_recording::MidiRecordingState;
use super::mixer::{MixerState, DEFAULT_BUS_COUNT};
use super::music::{Key, Scale};
use super::piano_roll::PianoRollState;
use super::theme::Theme;
use super::vst::VstPluginRegistry;

/// Click track (metronome) state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClickTrackState {
    /// Whether the click track is enabled
    pub enabled: bool,
    /// Volume (0.0 - 1.0)
    pub volume: f32,
    /// Quick mute without disabling
    pub muted: bool,
}

impl Default for ClickTrackState {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.7,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MixerSelection {
    Instrument(usize), // index into instruments vec
    LayerGroup(u32),   // layer group ID
    Bus(u8),           // 1-8
    Master,
}

impl Default for MixerSelection {
    fn default() -> Self {
        Self::Instrument(0)
    }
}

/// The subset of session fields that are cheap to clone for editing (BPM, key, scale, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicalSettings {
    pub key: Key,
    pub scale: Scale,
    pub bpm: u16,
    pub tuning_a4: f32,
    pub snap: bool,
    pub time_signature: (u8, u8),
}

impl Default for MusicalSettings {
    fn default() -> Self {
        Self {
            key: Key::C,
            scale: Scale::Major,
            bpm: 120,
            tuning_a4: 440.0,
            snap: false,
            time_signature: (4, 4),
        }
    }
}

/// Project-level state container.
/// Owns musical settings, piano roll, automation, mixer buses, and other project data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    // Musical settings (flat, not nested)
    pub key: Key,
    pub scale: Scale,
    pub bpm: u16,
    pub tuning_a4: f32,
    pub snap: bool,
    pub time_signature: (u8, u8),

    // Project state (hoisted from InstrumentState)
    pub piano_roll: PianoRollState,
    pub arrangement: ArrangementState,
    pub automation: AutomationState,
    pub midi_recording: MidiRecordingState,
    pub custom_synthdefs: CustomSynthDefRegistry,
    pub vst_plugins: VstPluginRegistry,

    // Mixer state (extracted)
    pub mixer: MixerState,

    // Humanize settings (extracted)
    pub humanize: HumanizeSettings,

    // Click track / metronome
    #[serde(default)]
    pub click_track: ClickTrackState,

    // UI theme
    #[serde(default)]
    pub theme: Theme,
}

impl SessionState {
    pub fn new() -> Self {
        Self::new_with_defaults(MusicalSettings::default(), DEFAULT_BUS_COUNT)
    }

    pub fn new_with_defaults(defaults: MusicalSettings, bus_count: u8) -> Self {
        Self {
            key: defaults.key,
            scale: defaults.scale,
            bpm: defaults.bpm,
            tuning_a4: defaults.tuning_a4,
            snap: defaults.snap,
            time_signature: defaults.time_signature,
            piano_roll: PianoRollState::new(),
            arrangement: ArrangementState::new(),
            automation: AutomationState::new(),
            midi_recording: MidiRecordingState::new(),
            custom_synthdefs: CustomSynthDefRegistry::new(),
            vst_plugins: VstPluginRegistry::new(),
            mixer: MixerState::new_with_bus_count(bus_count),
            humanize: HumanizeSettings::default(),
            click_track: ClickTrackState::default(),
            theme: Theme::default(),
        }
    }

    /// Extract the cheap musical settings for editing
    pub fn musical_settings(&self) -> MusicalSettings {
        MusicalSettings {
            key: self.key,
            scale: self.scale,
            bpm: self.bpm,
            tuning_a4: self.tuning_a4,
            snap: self.snap,
            time_signature: self.time_signature,
        }
    }

    /// Apply edited musical settings back
    pub fn apply_musical_settings(&mut self, settings: &MusicalSettings) {
        self.key = settings.key;
        self.scale = settings.scale;
        self.bpm = settings.bpm;
        self.tuning_a4 = settings.tuning_a4;
        self.snap = settings.snap;
        self.time_signature = settings.time_signature;
        // Sync to piano_roll (invariant: piano_roll mirrors session settings)
        self.piano_roll.bpm = self.bpm as f32;
        self.piano_roll.time_signature = self.time_signature;
    }

    /// Set BPM and sync to piano_roll.
    /// Use this instead of direct assignment to maintain invariant.
    pub fn set_bpm(&mut self, bpm: u16) {
        self.bpm = bpm;
        self.piano_roll.bpm = bpm as f32;
    }

    /// Set time signature and sync to piano_roll.
    /// Use this instead of direct assignment to maintain invariant.
    pub fn set_time_signature(&mut self, ts: (u8, u8)) {
        self.time_signature = ts;
        self.piano_roll.time_signature = ts;
    }

    // ========== Delegation methods for MixerState ==========
    // These preserve backwards compatibility for method calls.
    // Direct field access should use state.session.mixer.* instead.

    /// Get a bus by ID
    pub fn bus(&self, id: u8) -> Option<&MixerBus> {
        self.mixer.bus(id)
    }

    /// Get a mutable bus by ID
    pub fn bus_mut(&mut self, id: u8) -> Option<&mut MixerBus> {
        self.mixer.bus_mut(id)
    }

    /// Get an iterator over current bus IDs in order
    pub fn bus_ids(&self) -> impl Iterator<Item = u8> + '_ {
        self.mixer.bus_ids()
    }

    /// Add a new bus. Returns the new bus ID, or None if at max capacity.
    pub fn add_bus(&mut self) -> Option<u8> {
        self.mixer.add_bus()
    }

    /// Remove a bus by ID. Returns true if the bus was found and removed.
    pub fn remove_bus(&mut self, id: u8) -> bool {
        self.mixer.remove_bus(id)
    }

    /// Check if any bus is soloed
    pub fn any_bus_solo(&self) -> bool {
        self.mixer.any_bus_solo()
    }

    /// Compute effective mute for a bus, considering solo state
    pub fn effective_bus_mute(&self, bus: &MixerBus) -> bool {
        self.mixer.effective_bus_mute(bus)
    }

    /// Cycle between instrument/bus/master sections
    pub fn mixer_cycle_section(&mut self) {
        self.mixer.cycle_section()
    }

    /// Recompute next_bus_id from current buses (call after loading from persistence)
    pub fn recompute_next_bus_id(&mut self) {
        self.mixer.recompute_next_bus_id()
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mixer::MAX_BUSES;

    #[test]
    fn bus_lookup_by_id() {
        let session = SessionState::new();
        assert!(session.bus(1).is_some());
        assert_eq!(session.bus(1).unwrap().id, 1);
        assert!(session.bus(8).is_some());
        assert_eq!(session.bus(8).unwrap().id, 8);
        // Non-existent IDs return None
        assert!(session.bus(0).is_none());
        assert!(session.bus(9).is_none());
        assert!(session.bus(100).is_none());
    }

    #[test]
    fn add_bus_increments_id() {
        let mut session = SessionState::new();
        assert_eq!(session.mixer.buses.len(), DEFAULT_BUS_COUNT as usize);
        let new_id = session.add_bus().unwrap();
        assert_eq!(new_id, DEFAULT_BUS_COUNT + 1);
        assert_eq!(session.mixer.buses.len(), DEFAULT_BUS_COUNT as usize + 1);
        assert!(session.bus(new_id).is_some());
    }

    #[test]
    fn add_bus_respects_max_limit() {
        let mut session = SessionState::new_with_defaults(MusicalSettings::default(), MAX_BUSES);
        assert_eq!(session.mixer.buses.len(), MAX_BUSES as usize);
        assert!(session.add_bus().is_none());
        assert_eq!(session.mixer.buses.len(), MAX_BUSES as usize);
    }

    #[test]
    fn remove_bus() {
        let mut session = SessionState::new();
        assert!(session.bus(3).is_some());
        assert!(session.remove_bus(3));
        assert!(session.bus(3).is_none());
        // Removing non-existent bus returns false
        assert!(!session.remove_bus(3));
        assert!(!session.remove_bus(100));
    }

    #[test]
    fn bus_ids_iterator() {
        let session = SessionState::new();
        let ids: Vec<u8> = session.bus_ids().collect();
        assert_eq!(ids, (1..=DEFAULT_BUS_COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn recompute_next_bus_id() {
        let mut session = SessionState::new();
        session.mixer.next_bus_id = 0; // Simulate loading from persistence
        session.recompute_next_bus_id();
        assert_eq!(session.mixer.next_bus_id, DEFAULT_BUS_COUNT + 1);
    }

    #[test]
    fn effective_bus_mute_no_solo() {
        let session = SessionState::new();
        let bus = session.bus(1).unwrap();
        assert!(!session.effective_bus_mute(bus));

        let mut bus_copy = bus.clone();
        bus_copy.mute = true;
        assert!(session.effective_bus_mute(&bus_copy));
    }

    #[test]
    fn effective_bus_mute_with_solo() {
        let mut session = SessionState::new();
        session.bus_mut(1).unwrap().solo = true;
        // Bus 1 is soloed — should not be muted
        assert!(!session.effective_bus_mute(session.bus(1).unwrap()));
        // Bus 2 is not soloed — should be muted
        assert!(session.effective_bus_mute(session.bus(2).unwrap()));
    }

    #[test]
    fn mixer_cycle_section_full_cycle() {
        let mut session = SessionState::new();
        assert!(matches!(
            session.mixer.selection,
            MixerSelection::Instrument(0)
        ));
        session.mixer_cycle_section();
        assert!(matches!(session.mixer.selection, MixerSelection::Bus(1)));
        session.mixer_cycle_section();
        assert!(matches!(session.mixer.selection, MixerSelection::Master));
        session.mixer_cycle_section();
        assert!(matches!(
            session.mixer.selection,
            MixerSelection::Instrument(0)
        ));
    }

    #[test]
    fn musical_settings_round_trip() {
        let mut session = SessionState::new();
        session.bpm = 140;
        session.key = Key::D;
        session.scale = Scale::Minor;
        session.tuning_a4 = 442.0;
        session.snap = true;
        session.time_signature = (3, 4);

        let settings = session.musical_settings();
        assert_eq!(settings.bpm, 140);
        assert_eq!(settings.key, Key::D);
        assert_eq!(settings.time_signature, (3, 4));

        // Modify and apply back
        let mut modified = settings.clone();
        modified.bpm = 160;
        modified.key = Key::E;
        session.apply_musical_settings(&modified);
        assert_eq!(session.bpm, 160);
        assert_eq!(session.key, Key::E);
    }

    #[test]
    fn any_bus_solo() {
        let mut session = SessionState::new();
        assert!(!session.any_bus_solo());
        session.bus_mut(3).unwrap().solo = true;
        assert!(session.any_bus_solo());
    }
}
