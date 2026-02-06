//! Audio command and feedback types for the audio thread abstraction.
//!
//! Phase 3: AudioHandle serializes commands through an MPSC channel to a
//! dedicated audio thread and consumes feedback updates each frame.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::action::VstTarget;
use crate::audio::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use crate::state::automation::AutomationTarget;
use crate::state::{BufferId, EffectId, InstrumentId};

/// Commands sent from the main thread to the audio engine.
///
/// Commands either carry their own data, use reply channels for
/// synchronous operations, or rely on snapshots previously provided via
/// UpdateState / UpdatePianoRollData / UpdateAutomationLanes.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AudioCmd {
    // ── Server lifecycle ──────────────────────────────────────────
    Connect {
        server_addr: String,
        reply: Sender<std::io::Result<()>>,
    },
    Disconnect,
    StartServer {
        input_device: Option<String>,
        output_device: Option<String>,
        buffer_size: u32,
        sample_rate: u32,
        reply: Sender<Result<(), String>>,
    },
    StopServer,
    RestartServer {
        input_device: Option<String>,
        output_device: Option<String>,
        server_addr: String,
        buffer_size: u32,
        sample_rate: u32,
    },
    CompileSynthDefs {
        scd_path: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    LoadSynthDefs {
        dir: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    LoadSynthDefFile {
        path: PathBuf,
        reply: Sender<Result<(), String>>,
    },

    // ── State snapshots ───────────────────────────────────────────
    UpdateState {
        instruments: InstrumentSnapshot,
        session: SessionSnapshot,
    },
    UpdatePianoRollData {
        piano_roll: PianoRollSnapshot,
    },
    UpdateAutomationLanes {
        lanes: AutomationSnapshot,
    },

    // ── Playback control ──────────────────────────────────────────
    SetPlaying {
        playing: bool,
    },
    ResetPlayhead,
    SetBpm {
        bpm: f32,
    },

    // ── Routing & mixing ──────────────────────────────────────────
    RebuildRouting,
    RebuildInstrumentRouting {
        instrument_id: InstrumentId,
    },
    UpdateMixerParams,
    SetBusMixerParams {
        bus_id: u8,
        level: f32,
        mute: bool,
        pan: f32,
    },
    SetSourceParam {
        instrument_id: InstrumentId,
        param: String,
        value: f32,
    },
    SetEqParam {
        instrument_id: InstrumentId,
        param: String,
        value: f32,
    },
    /// Targeted /n_set to filter node (no routing rebuild).
    SetFilterParam {
        instrument_id: InstrumentId,
        param: String,
        value: f32,
    },
    /// Targeted /n_set to effect node (no routing rebuild).
    SetEffectParam {
        instrument_id: InstrumentId,
        effect_id: EffectId,
        param: String,
        value: f32,
    },
    /// Targeted /n_set to LFO node (no routing rebuild).
    SetLfoParam {
        instrument_id: InstrumentId,
        param: String,
        value: f32,
    },
    SetInstrumentMixerParams {
        instrument_id: InstrumentId,
        level: f32,
        pan: f32,
        mute: bool,
        solo: bool,
    },
    SetMasterParams {
        level: f32,
        mute: bool,
    },

    // ── Voice management ──────────────────────────────────────────
    SpawnVoice {
        instrument_id: InstrumentId,
        pitch: u8,
        velocity: f32,
        offset_secs: f64,
    },
    ReleaseVoice {
        instrument_id: InstrumentId,
        pitch: u8,
        offset_secs: f64,
    },
    RegisterActiveNote {
        instrument_id: InstrumentId,
        pitch: u8,
        duration_ticks: u32,
    },
    ClearActiveNotes,
    ReleaseAllVoices,
    PlayDrumHit {
        buffer_id: BufferId,
        amp: f32,
        instrument_id: InstrumentId,
        slice_start: f32,
        slice_end: f32,
        rate: f32,
        offset_secs: f64,
    },

    // ── Samples ───────────────────────────────────────────────────
    LoadSample {
        buffer_id: BufferId,
        path: String,
        reply: Sender<Result<i32, String>>,
    },
    FreeSamples {
        buffer_ids: Vec<BufferId>,
    },

    // ── Recording ─────────────────────────────────────────────────
    StartRecording {
        bus: i32,
        path: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    StopRecording {
        reply: Sender<Option<PathBuf>>,
    },
    StartInstrumentRender {
        instrument_id: InstrumentId,
        path: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    StartMasterBounce {
        path: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    StartStemExport {
        stems: Vec<(InstrumentId, PathBuf)>,
        reply: Sender<Result<(), String>>,
    },
    CancelExport,

    // ── Automation ────────────────────────────────────────────────
    ApplyAutomation {
        target: AutomationTarget,
        value: f32,
    },

    // ── VST parameter control ──────────────────────────────────
    QueryVstParams {
        instrument_id: InstrumentId,
        target: VstTarget,
    },
    SetVstParam {
        instrument_id: InstrumentId,
        target: VstTarget,
        param_index: u32,
        value: f32,
    },
    SaveVstState {
        instrument_id: InstrumentId,
        target: VstTarget,
        path: PathBuf,
    },
    LoadVstState {
        instrument_id: InstrumentId,
        target: VstTarget,
        path: PathBuf,
    },

    // ── Lifecycle ─────────────────────────────────────────────────
    Shutdown,
}

impl AudioCmd {
    /// Returns true if this command is time-critical and should use the priority channel.
    /// Priority commands: voice management, param changes, playback control.
    /// Normal commands: state sync, routing rebuilds, recording, server lifecycle.
    pub fn is_priority(&self) -> bool {
        matches!(
            self,
            // Voice management (most time-critical)
            AudioCmd::SpawnVoice { .. }
                | AudioCmd::ReleaseVoice { .. }
                | AudioCmd::PlayDrumHit { .. }
                | AudioCmd::RegisterActiveNote { .. }
                | AudioCmd::ClearActiveNotes
                | AudioCmd::ReleaseAllVoices
                // Param changes (need low latency for knob tweaks)
                | AudioCmd::SetSourceParam { .. }
                | AudioCmd::SetEqParam { .. }
                | AudioCmd::SetFilterParam { .. }
                | AudioCmd::SetEffectParam { .. }
                | AudioCmd::SetLfoParam { .. }
                | AudioCmd::SetVstParam { .. }
                | AudioCmd::SetInstrumentMixerParams { .. }
                | AudioCmd::SetMasterParams { .. }
                | AudioCmd::SetBusMixerParams { .. }
                // Playback control
                | AudioCmd::SetPlaying { .. }
                | AudioCmd::SetBpm { .. }
                | AudioCmd::ResetPlayhead
                // Automation (applied during playback)
                | AudioCmd::ApplyAutomation { .. }
        )
    }
}

// Re-export AudioFeedback and ExportKind from imbolc-types
pub use imbolc_types::{AudioFeedback, ExportKind};
