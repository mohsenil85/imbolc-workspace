pub mod arpeggiator;
pub mod audio_feedback;
pub mod automation;
pub mod arrangement;
pub mod clipboard;
pub mod custom_synthdef;
pub mod drum_sequencer;
pub mod grid;
pub mod instrument;
pub mod instrument_state;
pub mod midi_connection;
pub mod midi_recording;
pub mod music;
pub mod param;
pub mod persistence;
pub mod piano_roll;
pub mod recent_projects;
pub mod sampler;
pub mod session;
pub mod undo;
pub mod vst_plugin;

pub use audio_feedback::AudioFeedbackState;
pub use automation::AutomationTarget;
pub use midi_connection::MidiConnectionState;
pub use arrangement::{ArrangementState, Clip, ClipId, ClipPlacement, PlayMode, PlacementId};
pub use clipboard::{Clipboard, ClipboardContents, ClipboardNote};
pub use custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};
pub use instrument::*;
pub use instrument::{InstrumentSection, instrument_row_count, instrument_section_for_row, instrument_row_info};
pub use instrument::{SourceTypeExt, EffectTypeExt};
pub use instrument_state::InstrumentState;
pub use param::{Param, ParamValue, adjust_freq_semitone, adjust_musical_step, is_freq_param};
pub use sampler::{BufferId, SampleBuffer, SampleRegistry, SamplerConfig, Slice, SliceId};
pub use session::{MixerSelection, MixerState, MusicalSettings, SessionState, MAX_BUSES, DEFAULT_BUS_COUNT};
pub use undo::UndoHistory;
pub use vst_plugin::{VstParamSpec, VstPlugin, VstPluginId, VstPluginKind, VstPluginRegistry};

// Re-export types moved to imbolc-types
pub use imbolc_types::{
    BusId, ClientDisplayInfo, IoGeneration, IoState, KeyboardLayout, NetworkConnectionStatus,
    NetworkDisplayContext, OwnershipDisplayStatus, PendingExport, PendingRender, ProjectMeta,
    RecordingState, VisualizationState,
};

/// Top-level application state, owned by main.rs and passed to panes by reference.
pub struct AppState {
    pub session: SessionState,
    pub instruments: InstrumentState,
    pub clipboard: Clipboard,
    /// I/O state for render and export operations
    pub io: IoState,
    pub keyboard_layout: KeyboardLayout,
    /// Recording state (audio recording + automation recording)
    pub recording: RecordingState,
    /// Audio feedback state (visualization, playhead, bpm, server_status)
    pub audio: AudioFeedbackState,
    pub recorded_waveform_peaks: Option<Vec<f32>>,
    /// Undo/redo history (owned by state so dispatch can manage it)
    pub undo_history: UndoHistory,
    /// Project metadata (path, dirty flag, default settings)
    pub project: ProjectMeta,
    /// MIDI hardware connection state
    pub midi: MidiConnectionState,
    /// Network collaboration context (None when running standalone)
    pub network: Option<NetworkDisplayContext>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            session: SessionState::new(),
            instruments: InstrumentState::new(),
            clipboard: Clipboard::default(),
            io: IoState::default(),
            keyboard_layout: KeyboardLayout::default(),
            recording: RecordingState::default(),
            audio: AudioFeedbackState::default(),
            recorded_waveform_peaks: None,
            undo_history: UndoHistory::new(500),
            project: ProjectMeta::default(),
            midi: MidiConnectionState::default(),
            network: None,
        }
    }

    pub fn new_with_defaults(defaults: MusicalSettings) -> Self {
        Self {
            session: SessionState::new_with_defaults(defaults.clone(), session::DEFAULT_BUS_COUNT),
            instruments: InstrumentState::new(),
            clipboard: Clipboard::default(),
            io: IoState::default(),
            keyboard_layout: KeyboardLayout::default(),
            recording: RecordingState::default(),
            audio: AudioFeedbackState::default(),
            recorded_waveform_peaks: None,
            undo_history: UndoHistory::new(500),
            project: ProjectMeta::new_with_defaults(defaults),
            midi: MidiConnectionState::default(),
            network: None,
        }
    }

    /// Get the ownership status for an instrument (for UI display).
    pub fn ownership_status(&self, instrument_id: InstrumentId) -> OwnershipDisplayStatus {
        match &self.network {
            Some(ctx) => ctx.ownership.get(&instrument_id).cloned().unwrap_or(OwnershipDisplayStatus::Unowned),
            None => OwnershipDisplayStatus::Local,
        }
    }

    /// Add an instrument, with custom synthdef param setup and piano roll track auto-creation.
    pub fn add_instrument(&mut self, source: SourceType) -> InstrumentId {
        let id = self.instruments.add_instrument(source);
        imbolc_types::reduce::initialize_instrument_from_registries(
            id, source, &mut self.instruments, &self.session,
        );
        self.session.piano_roll.add_track(id);
        id
    }

    /// Remove an instrument and its piano roll track.
    pub fn remove_instrument(&mut self, id: InstrumentId) {
        self.instruments.remove_instrument(id);
        self.session.piano_roll.remove_track(id);
        self.session.automation.remove_lanes_for_instrument(id);
        self.session.arrangement.remove_instrument_data(id);
    }

    /// Compute effective mute for an instrument, considering solo state and master mute.
    pub fn effective_instrument_mute(&self, inst: &Instrument) -> bool {
        if self.instruments.any_instrument_solo() {
            !inst.mixer.solo
        } else {
            inst.mixer.mute || self.session.mixer.master_mute
        }
    }

}

impl imbolc_audio::AudioStateProvider for AppState {
    fn session(&self) -> &SessionState { &self.session }
    fn instruments(&self) -> &InstrumentState { &self.instruments }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_instrument_clears_automation_lanes() {
        let mut state = AppState::new();
        let instrument_id = state.add_instrument(SourceType::Saw);

        assert_eq!(state.session.piano_roll.track_order.len(), 1);
        assert_eq!(state.session.piano_roll.track_order[0], instrument_id);

        state
            .session
            .automation
            .add_lane(AutomationTarget::level(instrument_id));
        state
            .session
            .automation
            .add_lane(AutomationTarget::pan(instrument_id));

        assert_eq!(
            state.session.automation.lanes_for_instrument(instrument_id).len(),
            2
        );

        state.remove_instrument(instrument_id);

        assert!(state
            .session
            .automation
            .lanes_for_instrument(instrument_id)
            .is_empty());
        assert!(state.session.piano_roll.track_order.is_empty());
    }

    #[test]
    fn effective_instrument_mute_no_solo() {
        let mut state = AppState::new();
        state.add_instrument(SourceType::Saw);
        let inst = &state.instruments.instruments[0];
        // Not muted, no solo, no master mute
        assert!(!state.effective_instrument_mute(inst));

        // Mute the instrument
        state.instruments.instruments[0].mixer.mute = true;
        let inst = &state.instruments.instruments[0];
        assert!(state.effective_instrument_mute(inst));

        // Unmute instrument but mute master
        state.instruments.instruments[0].mixer.mute = false;
        state.session.mixer.master_mute = true;
        let inst = &state.instruments.instruments[0];
        assert!(state.effective_instrument_mute(inst));
    }

    #[test]
    fn effective_instrument_mute_with_solo() {
        let mut state = AppState::new();
        state.add_instrument(SourceType::Saw);
        state.add_instrument(SourceType::Sin);
        state.instruments.instruments[0].mixer.solo = true;

        let inst0 = &state.instruments.instruments[0];
        assert!(!state.effective_instrument_mute(inst0)); // soloed — not muted

        let inst1 = &state.instruments.instruments[1];
        assert!(state.effective_instrument_mute(inst1)); // not soloed — muted
    }

    #[test]
    fn add_instrument_creates_piano_roll_track() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        assert_eq!(state.session.piano_roll.track_order.len(), 1);
        assert!(state.session.piano_roll.tracks.contains_key(&id));
    }

    #[test]
    fn remove_instrument_cleans_up_all() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        state.session.automation.add_lane(AutomationTarget::level(id));
        assert_eq!(state.session.automation.lanes.len(), 1);

        state.remove_instrument(id);
        assert!(state.session.piano_roll.track_order.is_empty());
        assert!(state.session.automation.lanes.is_empty());
    }
}
