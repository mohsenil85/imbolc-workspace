//! Action types for the dispatch system.
//!
//! Actions represent user intents that flow through the dispatch system.
//! This module contains all action enums and related types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    AutomationLaneId, AutomationTarget, BusId, ClipId, ClipboardNote, CurveType, DrumStep,
    EffectId, EffectType, EnvConfig, FilterType,
    InstrumentId, LfoConfig, MixerSelection, MusicalSettings, Param, ParamIndex, PlacementId,
    ProcessingStage, ServerStatus, SourceType, VstPluginKind,
};

// ============================================================================
// Simple enums with no dependencies
// ============================================================================

/// Navigation actions (pane switching, modal stack).
#[derive(Debug, Clone, PartialEq)]
pub enum NavAction {
    SwitchPane(&'static str),
    PushPane(&'static str),
    PopPane,
}

/// Result of toggling performance mode (piano/pad keyboard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToggleResult {
    /// Pane doesn't support performance mode
    NotSupported,
    /// Piano keyboard was activated
    ActivatedPiano,
    /// Pad keyboard was activated
    ActivatedPad,
    /// Layout cycled (still in piano mode)
    CycledLayout,
    /// Performance mode was deactivated
    Deactivated,
}

/// Identifies a filter parameter for targeted /n_set updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterParamKind {
    Cutoff,
    Resonance,
}

impl FilterParamKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterParamKind::Cutoff => "cutoff",
            FilterParamKind::Resonance => "resonance",
        }
    }
}

/// Identifies an LFO parameter for targeted /n_set updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoParamKind {
    Rate,
    Depth,
}

impl LfoParamKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LfoParamKind::Rate => "rate",
            LfoParamKind::Depth => "depth",
        }
    }
}

/// Identifies an EQ band parameter for targeted /n_set updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqParamKind {
    Freq,
    Gain,
    Q,
    Enabled,
}

impl EqParamKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EqParamKind::Freq => "freq",
            EqParamKind::Gain => "gain",
            EqParamKind::Q => "q",
            EqParamKind::Enabled => "on",
        }
    }
}

/// Identifies whether a VST operation targets the instrument source or an effect slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VstTarget {
    Source,
    Effect(EffectId), // stable effect ID
}

// ============================================================================
// Server / Bus / Chopper actions
// ============================================================================

/// Audio server actions — Start/Restart carry device selections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerAction {
    Connect,
    Disconnect,
    Start {
        input_device: Option<String>,
        output_device: Option<String>,
        buffer_size: u32,
        sample_rate: u32,
    },
    Stop,
    CompileSynthDefs,
    CompileVstSynthDefs,
    LoadSynthDefs,
    Restart {
        input_device: Option<String>,
        output_device: Option<String>,
        buffer_size: u32,
        sample_rate: u32,
    },
    RecordMaster,
    RecordInput,
}

/// Bus management actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BusAction {
    /// Add a new bus
    Add,
    /// Remove a bus by ID
    Remove(BusId),
    /// Rename a bus
    Rename(BusId, String),
    /// Add an effect to a bus
    AddEffect(BusId, EffectType),
    /// Remove an effect from a bus
    RemoveEffect(BusId, EffectId),
    /// Move an effect up/down on a bus
    MoveEffect(BusId, EffectId, i8),
    /// Toggle bypass on a bus effect
    ToggleEffectBypass(BusId, EffectId),
    /// Adjust a parameter on a bus effect
    AdjustEffectParam(BusId, EffectId, ParamIndex, f32),
}

/// Layer group actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerGroupAction {
    /// Add an effect to a layer group
    AddEffect(u32, EffectType),
    /// Remove an effect from a layer group
    RemoveEffect(u32, EffectId),
    /// Move an effect up/down on a layer group
    MoveEffect(u32, EffectId, i8),
    /// Toggle bypass on a layer group effect
    ToggleEffectBypass(u32, EffectId),
    /// Adjust a parameter on a layer group effect
    AdjustEffectParam(u32, EffectId, ParamIndex, f32),
    /// Toggle EQ on/off for a layer group
    ToggleEq(u32),
    /// Set an EQ band parameter on a layer group (group_id, band_index, param, value)
    SetEqParam(u32, usize, EqParamKind, f32),
}

/// Sample chopper actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChopperAction {
    LoadSample,
    LoadSampleResult(PathBuf),
    AddSlice(f32),           // cursor_pos
    RemoveSlice,
    AssignToPad(usize),
    AutoSlice(usize),
    PreviewSlice,
    SelectSlice(i8),         // +1/-1
    NudgeSliceStart(f32),
    NudgeSliceEnd(f32),
    MoveCursor(i8),          // direction
    CommitAll,               // assign all slices to pads and return
}

// ============================================================================
// File selection and navigation
// ============================================================================

/// Action to take when a file is selected in the file browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileSelectAction {
    ImportCustomSynthDef,
    ImportVstInstrument,
    ImportVstEffect,
    LoadDrumSample(usize), // pad index
    LoadChopperSample,
    LoadPitchedSample(InstrumentId),
    LoadImpulseResponse(InstrumentId, EffectId), // instrument_id, effect_id
    ImportProject,
}

/// Navigation intent returned from dispatch — processed by the UI layer.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum NavIntent {
    SwitchTo(&'static str),
    PushTo(&'static str),
    Pop,
    /// Pop only if the active pane matches the given id
    ConditionalPop(&'static str),
    /// Pop, falling back to SwitchTo if stack is empty
    PopOrSwitchTo(&'static str),
    /// Configure and push to the file browser
    OpenFileBrowser(FileSelectAction),
    /// Configure and push to the VST param pane for a specific target
    OpenVstParams(InstrumentId, VstTarget),
}

/// Status event returned from dispatch — forwarded to the server pane by the UI layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEvent {
    pub status: ServerStatus,
    pub message: String,
    pub server_running: Option<bool>,
}

// ============================================================================
// AudioEffect and DispatchResult
// ============================================================================

/// Typed audio effect event — replaces boolean-flag AudioDirty.
///
/// Dispatch handlers push these to signal what the audio thread needs to do.
/// The UI accumulates them per-frame and flushes once via `apply_effects()`.
/// No fixed-size arrays or overflow escalation — just push events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioEffect {
    // Structural rebuilds
    RebuildInstruments,
    RebuildSession,
    RebuildRouting,
    RebuildRoutingForInstrument(InstrumentId),
    AddInstrumentRouting(InstrumentId),
    DeleteInstrumentRouting(InstrumentId),
    RebuildBusProcessing,
    UpdateMixerParams,
    UpdatePianoRoll,
    UpdateAutomation,

    // Targeted /n_set params (no rebuild needed)
    SetFilterParam(InstrumentId, FilterParamKind, f32),
    SetEffectParam(InstrumentId, EffectId, ParamIndex, f32),
    SetLfoParam(InstrumentId, LfoParamKind, f32),
    SetBusEffectParam(BusId, EffectId, ParamIndex, f32),
    SetLayerGroupEffectParam(u32, EffectId, ParamIndex, f32),
}

impl AudioEffect {
    /// All structural rebuild effects (for undo/redo/load/full sync).
    pub fn all() -> Vec<AudioEffect> {
        vec![
            AudioEffect::RebuildInstruments,
            AudioEffect::RebuildSession,
            AudioEffect::UpdatePianoRoll,
            AudioEffect::UpdateAutomation,
            AudioEffect::RebuildRouting,
            AudioEffect::UpdateMixerParams,
        ]
    }

    /// Effects for a single instrument rebuild + targeted routing.
    pub fn for_instrument(id: InstrumentId) -> Vec<AudioEffect> {
        vec![
            AudioEffect::RebuildInstruments,
            AudioEffect::RebuildRoutingForInstrument(id),
        ]
    }

    /// Whether this effect requests any routing rebuild.
    pub fn is_routing(&self) -> bool {
        matches!(
            self,
            AudioEffect::RebuildRouting
                | AudioEffect::RebuildRoutingForInstrument(_)
                | AudioEffect::AddInstrumentRouting(_)
                | AudioEffect::DeleteInstrumentRouting(_)
        )
    }
}

/// Result of dispatching an action — contains side effects for the UI layer to process.
#[derive(Debug, Clone, Default)]
pub struct DispatchResult {
    pub quit: bool,
    pub nav: Vec<NavIntent>,
    pub status: Vec<StatusEvent>,
    pub project_name: Option<String>,
    pub audio_effects: Vec<AudioEffect>,
    /// Signal that playback should be stopped (processed by the UI layer, not dispatch)
    pub stop_playback: bool,
    /// Signal that the playhead should be reset to 0 (processed by the UI layer, not dispatch)
    pub reset_playhead: bool,
    /// True if the action could not be incrementally projected to the audio thread
    /// and requires a full state sync via `apply_effects()`.
    pub needs_full_sync: bool,
}

impl DispatchResult {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_quit() -> Self {
        Self { quit: true, ..Self::default() }
    }

    pub fn with_nav(intent: NavIntent) -> Self {
        Self { nav: vec![intent], ..Self::default() }
    }

    pub fn with_status(status: ServerStatus, message: impl Into<String>) -> Self {
        Self {
            status: vec![StatusEvent { status, message: message.into(), server_running: None }],
            ..Self::default()
        }
    }

    pub fn push_nav(&mut self, intent: NavIntent) {
        self.nav.push(intent);
    }

    pub fn push_status(&mut self, status: ServerStatus, message: impl Into<String>) {
        self.status.push(StatusEvent { status, message: message.into(), server_running: None });
    }

    pub fn push_status_with_running(&mut self, status: ServerStatus, message: impl Into<String>, running: bool) {
        self.status.push(StatusEvent { status, message: message.into(), server_running: Some(running) });
    }

    pub fn merge(&mut self, other: DispatchResult) {
        self.quit = self.quit || other.quit;
        self.nav.extend(other.nav);
        self.status.extend(other.status);
        if other.project_name.is_some() {
            self.project_name = other.project_name;
        }
        self.audio_effects.extend(other.audio_effects);
        self.stop_playback |= other.stop_playback;
        self.reset_playhead |= other.reset_playhead;
    }
}

// ============================================================================
// Domain-specific action enums
// ============================================================================

/// VST parameter actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VstParamAction {
    SetParam(InstrumentId, VstTarget, u32, f32),       // instrument_id, target, param_index, value
    AdjustParam(InstrumentId, VstTarget, u32, f32),    // instrument_id, target, param_index, delta
    ResetParam(InstrumentId, VstTarget, u32),          // instrument_id, target, param_index
    DiscoverParams(InstrumentId, VstTarget),
    SaveState(InstrumentId, VstTarget),
}

/// Mixer actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MixerAction {
    Move(i8),
    Jump(i8),
    SelectAt(MixerSelection),
    AdjustLevel(f32),
    ToggleMute,
    ToggleSolo,
    CycleSection,
    CycleOutput,
    CycleOutputReverse,
    AdjustSend(BusId, f32),
    ToggleSend(BusId),
    CycleSendTapPoint(BusId),
    AdjustPan(f32),
}

/// Session/file actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionAction {
    Save,
    SaveAs(PathBuf),
    Load,
    LoadFrom(PathBuf),
    NewProject,
    UpdateSession(MusicalSettings),
    UpdateSessionLive(MusicalSettings),
    OpenFileBrowser(FileSelectAction),
    ImportCustomSynthDef(PathBuf),
    ImportVstPlugin(PathBuf, VstPluginKind),
    AdjustHumanizeVelocity(f32),
    AdjustHumanizeTiming(f32),
    ToggleMasterMute,
    /// Cycle through available themes (dark -> light -> high contrast)
    CycleTheme,
    /// Create a named checkpoint (persistent restore point)
    CreateCheckpoint(String),
    /// Restore project state to a checkpoint
    RestoreCheckpoint(i64),
    /// Delete a checkpoint
    DeleteCheckpoint(i64),
}

/// MIDI configuration actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiAction {
    ConnectPort(usize),
    DisconnectPort,
    AddCcMapping { cc: u8, channel: Option<u8>, target: AutomationTarget },
    RemoveCcMapping { cc: u8, channel: Option<u8> },
    SetChannelFilter(Option<u8>),
    SetLiveInputInstrument(Option<InstrumentId>),
    ToggleNotePassthrough,
}

/// Automation actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AutomationAction {
    AddLane(AutomationTarget),
    RemoveLane(AutomationLaneId),
    ToggleLaneEnabled(AutomationLaneId),
    AddPoint(AutomationLaneId, u32, f32),          // lane, tick, value
    RemovePoint(AutomationLaneId, u32),            // lane, tick
    MovePoint(AutomationLaneId, u32, u32, f32),    // lane, old_tick, new_tick, new_value
    SetCurveType(AutomationLaneId, u32, CurveType), // lane, tick, curve
    SelectLane(i8),                                 // +1/-1
    ClearLane(AutomationLaneId),
    ToggleRecording,
    ToggleLaneArm(AutomationLaneId),
    ArmAllLanes,
    DisarmAllLanes,
    RecordValue(AutomationTarget, f32),
    /// Delete automation points in tick range on a lane
    DeletePointsInRange(AutomationLaneId, u32, u32),
    /// Paste automation points at offset
    PastePoints(AutomationLaneId, u32, Vec<(u32, f32)>),
    /// Copy automation points within a tick range to the clipboard
    CopyPoints(AutomationLaneId, u32, u32),
}

/// Arrangement/timeline actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArrangementAction {
    TogglePlayMode,
    CreateClip { instrument_id: InstrumentId, length_ticks: u32 },
    CaptureClipFromPianoRoll { instrument_id: InstrumentId },
    DeleteClip(ClipId),
    RenameClip(ClipId, String),
    PlaceClip { clip_id: ClipId, instrument_id: InstrumentId, start_tick: u32 },
    RemovePlacement(PlacementId),
    MovePlacement { placement_id: PlacementId, new_start_tick: u32 },
    ResizePlacement { placement_id: PlacementId, new_length: Option<u32> },
    DuplicatePlacement(PlacementId),
    SelectPlacement(Option<usize>),
    SelectLane(usize),
    MoveCursor(i32),
    ScrollView(i32),
    ZoomIn,
    ZoomOut,
    EnterClipEdit(ClipId),
    ExitClipEdit,
    PlayStop,
}

/// Piano roll actions — all variants carry the data they need.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PianoRollAction {
    ToggleNote { pitch: u8, tick: u32, duration: u32, velocity: u8, track: usize },
    PlayStop,
    ToggleLoop,
    SetLoopStart(u32),
    SetLoopEnd(u32),
    CycleTimeSig,
    TogglePolyMode(usize),
    PlayNote { pitch: u8, velocity: u8, instrument_id: InstrumentId, track: usize },
    PlayNotes { pitches: Vec<u8>, velocity: u8, instrument_id: InstrumentId, track: usize },
    /// Release a sustained note (key-up via timeout detection)
    ReleaseNote { pitch: u8, instrument_id: InstrumentId },
    /// Release multiple sustained notes (for chords)
    ReleaseNotes { pitches: Vec<u8>, instrument_id: InstrumentId },
    PlayStopRecord,
    AdjustSwing(f32),               // delta for swing amount
    RenderToWav(InstrumentId),
    /// Delete all notes in the given region (used by Cut)
    DeleteNotesInRegion {
        track: usize,
        start_tick: u32,
        end_tick: u32,
        start_pitch: u8,
        end_pitch: u8,
    },
    /// Paste notes at a position from clipboard
    PasteNotes {
        track: usize,
        anchor_tick: u32,
        anchor_pitch: u8,
        notes: Vec<ClipboardNote>,
    },
    BounceToWav,
    ExportStems,
    CancelExport,
    /// Copy notes within a region to the clipboard
    CopyNotes { track: usize, start_tick: u32, end_tick: u32, start_pitch: u8, end_pitch: u8 },
}

impl PianoRollAction {
    /// Returns the target instrument ID for ownership validation, if applicable.
    /// Returns None for actions that don't explicitly specify an instrument ID.
    /// Note: Actions with `track` index need state to resolve to InstrumentId.
    pub fn target_instrument_id(&self) -> Option<InstrumentId> {
        match self {
            // Actions with explicit instrument_id
            Self::PlayNote { instrument_id, .. } => Some(*instrument_id),
            Self::PlayNotes { instrument_id, .. } => Some(*instrument_id),
            Self::ReleaseNote { instrument_id, .. } => Some(*instrument_id),
            Self::ReleaseNotes { instrument_id, .. } => Some(*instrument_id),
            Self::RenderToWav(id) => Some(*id),

            // Actions without explicit instrument_id (use track index, need state to resolve)
            Self::ToggleNote { .. }
            | Self::PlayStop
            | Self::ToggleLoop
            | Self::SetLoopStart(_)
            | Self::SetLoopEnd(_)
            | Self::CycleTimeSig
            | Self::TogglePolyMode(_)
            | Self::PlayStopRecord
            | Self::AdjustSwing(_)
            | Self::DeleteNotesInRegion { .. }
            | Self::PasteNotes { .. }
            | Self::BounceToWav
            | Self::ExportStems
            | Self::CancelExport
            | Self::CopyNotes { .. } => None,
        }
    }
}

/// Drum sequencer actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SequencerAction {
    ToggleStep(usize, usize),         // (pad_idx, step_idx)
    AdjustVelocity(usize, usize, i8), // (pad_idx, step_idx, delta)
    PlayStop,
    LoadSample(usize),              // pad_idx
    ClearPad(usize),                // pad_idx
    ClearPattern,
    CyclePatternLength,
    NextPattern,
    PrevPattern,
    AdjustPadLevel(usize, f32),     // (pad_idx, delta)
    LoadSampleResult(usize, PathBuf), // (pad_idx, path) — from file browser
    AdjustSwing(f32),               // delta for swing amount
    ApplyEuclidean { pad: usize, pulses: usize, steps: usize, rotation: usize },
    AdjustProbability(usize, usize, f32), // (pad_idx, step_idx, delta)
    ToggleChain,
    AddChainStep(usize),            // pattern_index
    RemoveChainStep(usize),         // position in chain
    MoveChainStep(usize, usize),    // from_position, to_position
    ToggleReverse(usize),              // pad_idx
    AdjustPadPitch(usize, i8),         // (pad_idx, delta semitones)
    AdjustStepPitch(usize, usize, i8), // (pad_idx, step_idx, delta)
    /// Delete steps in region (used by Cut)
    DeleteStepsInRegion {
        start_pad: usize,
        end_pad: usize,
        start_step: usize,
        end_step: usize,
    },
    /// Paste drum steps at cursor
    PasteSteps {
        anchor_pad: usize,
        anchor_step: usize,
        steps: Vec<(usize, usize, DrumStep)>,
    },
    /// Copy steps within a region to the clipboard
    CopySteps { start_pad: usize, end_pad: usize, start_step: usize, end_step: usize },
    /// Assign an instrument to a pad for one-shot triggering
    SetPadInstrument(usize, InstrumentId, f32), // pad_idx, instrument_id, freq
    /// Clear instrument assignment from a pad
    ClearPadInstrument(usize), // pad_idx
    /// Adjust the trigger frequency for a pad
    SetPadTriggerFreq(usize, f32), // pad_idx, freq
    /// Set the editing pad and open instrument picker
    OpenInstrumentPicker(usize), // pad_idx
    /// Cycle step resolution (1/4 -> 1/8 -> 1/16 -> 1/32)
    CycleStepResolution,
}

/// Data carried by InstrumentAction::Update to apply edits without dispatch reading pane state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentUpdate {
    pub id: InstrumentId,
    pub source: SourceType,
    pub source_params: Vec<Param>,
    pub processing_chain: Vec<ProcessingStage>,
    pub lfo: LfoConfig,
    pub amp_envelope: EnvConfig,
    pub polyphonic: bool,
    pub active: bool,
}

/// Instrument actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentAction {
    Add(SourceType),
    Delete(InstrumentId),
    Edit(InstrumentId),
    Update(Box<InstrumentUpdate>),
    AddEffect(InstrumentId, EffectType),
    RemoveEffect(InstrumentId, EffectId),
    MoveStage(InstrumentId, usize, i8),
    SetFilter(InstrumentId, Option<FilterType>),
    ToggleEffectBypass(InstrumentId, EffectId),
    ToggleFilter(InstrumentId),
    CycleFilterType(InstrumentId),
    AdjustFilterCutoff(InstrumentId, f32),
    AdjustFilterResonance(InstrumentId, f32),
    AdjustEffectParam(InstrumentId, EffectId, ParamIndex, f32),
    PlayNote(u8, u8),
    PlayNotes(Vec<u8>, u8),
    Select(usize),
    SelectNext,
    SelectPrev,
    SelectFirst,
    SelectLast,
    PlayDrumPad(usize),
    LoadSampleResult(InstrumentId, PathBuf),
    ToggleArp(InstrumentId),
    CycleArpDirection(InstrumentId),
    CycleArpRate(InstrumentId),
    AdjustArpOctaves(InstrumentId, i8),
    AdjustArpGate(InstrumentId, f32),
    CycleChordShape(InstrumentId),
    ClearChordShape(InstrumentId),
    LoadIRResult(InstrumentId, EffectId, PathBuf), // instrument_id, effect_id, path
    OpenVstEffectParams(InstrumentId, EffectId), // instrument_id, effect_id
    SetEqParam(InstrumentId, usize, EqParamKind, f32), // instrument_id, band_index, param, value
    ToggleEq(InstrumentId),
    LinkLayer(InstrumentId, InstrumentId),
    UnlinkLayer(InstrumentId),
    AdjustLayerOctaveOffset(InstrumentId, i8),
    // Per-track groove settings
    SetTrackSwing(InstrumentId, Option<f32>),
    SetTrackSwingGrid(InstrumentId, Option<crate::SwingGrid>),
    AdjustTrackSwing(InstrumentId, f32),
    SetTrackHumanizeVelocity(InstrumentId, Option<f32>),
    AdjustTrackHumanizeVelocity(InstrumentId, f32),
    SetTrackHumanizeTiming(InstrumentId, Option<f32>),
    AdjustTrackHumanizeTiming(InstrumentId, f32),
    SetTrackTimingOffset(InstrumentId, f32),
    AdjustTrackTimingOffset(InstrumentId, f32),
    ResetTrackGroove(InstrumentId),
    // Per-track time signature
    SetTrackTimeSignature(InstrumentId, Option<(u8, u8)>),
    CycleTrackTimeSignature(InstrumentId),
    // LFO actions
    ToggleLfo(InstrumentId),
    AdjustLfoRate(InstrumentId, f32),
    AdjustLfoDepth(InstrumentId, f32),
    SetLfoShape(InstrumentId, crate::LfoShape),
    SetLfoTarget(InstrumentId, crate::ParameterTarget),
    // Envelope actions
    AdjustEnvelopeAttack(InstrumentId, f32),
    AdjustEnvelopeDecay(InstrumentId, f32),
    AdjustEnvelopeSustain(InstrumentId, f32),
    AdjustEnvelopeRelease(InstrumentId, f32),
    // Channel config
    ToggleChannelConfig(InstrumentId),
}

impl InstrumentAction {
    /// Returns the target instrument ID for ownership validation, if applicable.
    /// Returns None for actions that don't target a specific instrument (Add, Select, PlayNote, etc).
    pub fn target_instrument_id(&self) -> Option<InstrumentId> {
        match self {
            // Actions that don't target a specific instrument
            Self::Add(_) => None,
            Self::PlayNote(_, _) => None,
            Self::PlayNotes(_, _) => None,
            Self::Select(_) => None,
            Self::SelectNext => None,
            Self::SelectPrev => None,
            Self::SelectFirst => None,
            Self::SelectLast => None,
            Self::PlayDrumPad(_) => None,

            // Actions targeting a specific instrument
            Self::Delete(id)
            | Self::Edit(id)
            | Self::AddEffect(id, _)
            | Self::RemoveEffect(id, _)
            | Self::MoveStage(id, _, _)
            | Self::SetFilter(id, _)
            | Self::ToggleEffectBypass(id, _)
            | Self::ToggleFilter(id)
            | Self::CycleFilterType(id)
            | Self::AdjustFilterCutoff(id, _)
            | Self::AdjustFilterResonance(id, _)
            | Self::AdjustEffectParam(id, _, _, _)
            | Self::LoadSampleResult(id, _)
            | Self::ToggleArp(id)
            | Self::CycleArpDirection(id)
            | Self::CycleArpRate(id)
            | Self::AdjustArpOctaves(id, _)
            | Self::AdjustArpGate(id, _)
            | Self::CycleChordShape(id)
            | Self::ClearChordShape(id)
            | Self::LoadIRResult(id, _, _)
            | Self::OpenVstEffectParams(id, _)
            | Self::SetEqParam(id, _, _, _)
            | Self::ToggleEq(id)
            | Self::LinkLayer(id, _)
            | Self::UnlinkLayer(id)
            | Self::AdjustLayerOctaveOffset(id, _)
            | Self::SetTrackSwing(id, _)
            | Self::SetTrackSwingGrid(id, _)
            | Self::AdjustTrackSwing(id, _)
            | Self::SetTrackHumanizeVelocity(id, _)
            | Self::AdjustTrackHumanizeVelocity(id, _)
            | Self::SetTrackHumanizeTiming(id, _)
            | Self::AdjustTrackHumanizeTiming(id, _)
            | Self::SetTrackTimingOffset(id, _)
            | Self::AdjustTrackTimingOffset(id, _)
            | Self::ResetTrackGroove(id)
            | Self::SetTrackTimeSignature(id, _)
            | Self::CycleTrackTimeSignature(id)
            | Self::ToggleLfo(id)
            | Self::AdjustLfoRate(id, _)
            | Self::AdjustLfoDepth(id, _)
            | Self::SetLfoShape(id, _)
            | Self::SetLfoTarget(id, _)
            | Self::AdjustEnvelopeAttack(id, _)
            | Self::AdjustEnvelopeDecay(id, _)
            | Self::AdjustEnvelopeSustain(id, _)
            | Self::AdjustEnvelopeRelease(id, _)
            | Self::ToggleChannelConfig(id) => Some(*id),

            Self::Update(update) => Some(update.id),
        }
    }
}

/// Reference tuner actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TunerAction {
    /// Play a reference tone at the given frequency
    PlayTone(f32),
    /// Stop the currently playing reference tone
    StopTone,
}

/// Click track (metronome) actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClickAction {
    /// Toggle click track enabled
    Toggle,
    /// Toggle mute (quick silence without disabling)
    ToggleMute,
    /// Adjust volume by delta
    AdjustVolume(f32),
    /// Set volume directly
    SetVolume(f32),
}

// ============================================================================
// Main Action enum (also serves as PaneAction — returned from pane handlers)
// ============================================================================

/// Actions returned from pane input handling. Contains both UI-layer mechanics
/// (Nav, PushLayer, PopLayer, ExitPerformanceMode, Quit, SaveAndQuit) and
/// domain mutations (Instrument, Mixer, etc.).
///
/// Use `to_domain()` to extract a `DomainAction` for dispatch.
#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    Nav(NavAction),
    Instrument(InstrumentAction),
    Mixer(MixerAction),
    PianoRoll(PianoRollAction),
    Arrangement(ArrangementAction),
    Server(ServerAction),
    Session(SessionAction),
    Sequencer(SequencerAction),
    Chopper(ChopperAction),
    Automation(AutomationAction),
    Midi(MidiAction),
    Bus(BusAction),
    LayerGroup(LayerGroupAction),
    VstParam(VstParamAction),
    Click(ClickAction),
    Tuner(TunerAction),
    AudioFeedback(crate::AudioFeedback),
    /// Pane signals: pop piano_mode/pad_mode layer
    ExitPerformanceMode,
    /// Push a named layer onto the layer stack
    PushLayer(&'static str),
    /// Pop a named layer from the layer stack
    PopLayer(&'static str),
    /// Undo the last undoable state change
    Undo,
    /// Redo the last undone state change
    Redo,
    /// Save the project then quit (used by quit prompt)
    SaveAndQuit,
}

// ============================================================================
// DomainAction — state mutations handled by core dispatch
// ============================================================================

/// Actions that mutate domain state (instruments, session, mixer, etc.).
/// Handled by `dispatch_action()` in imbolc-core. Does not include UI-layer
/// mechanics (navigation, layer stack, quit).
///
/// Extracted from `Action` via `Action::to_domain()`. Dispatch, undo, and
/// audio projection operate on `DomainAction` exclusively.
#[derive(Debug, Clone)]
pub enum DomainAction {
    Instrument(InstrumentAction),
    Mixer(MixerAction),
    PianoRoll(PianoRollAction),
    Arrangement(ArrangementAction),
    Server(ServerAction),
    Session(SessionAction),
    Sequencer(SequencerAction),
    Chopper(ChopperAction),
    Automation(AutomationAction),
    Midi(MidiAction),
    Bus(BusAction),
    LayerGroup(LayerGroupAction),
    VstParam(VstParamAction),
    Click(ClickAction),
    Tuner(TunerAction),
    AudioFeedback(crate::AudioFeedback),
    Undo,
    Redo,
}

impl Action {
    /// Convert to a `DomainAction` if this is a domain action.
    /// Returns `None` for UI-only actions (None, Quit, Nav, PushLayer, PopLayer,
    /// ExitPerformanceMode, SaveAndQuit).
    pub fn to_domain(&self) -> Option<DomainAction> {
        match self {
            Self::Instrument(a) => Some(DomainAction::Instrument(a.clone())),
            Self::Mixer(a) => Some(DomainAction::Mixer(a.clone())),
            Self::PianoRoll(a) => Some(DomainAction::PianoRoll(a.clone())),
            Self::Arrangement(a) => Some(DomainAction::Arrangement(a.clone())),
            Self::Server(a) => Some(DomainAction::Server(a.clone())),
            Self::Session(a) => Some(DomainAction::Session(a.clone())),
            Self::Sequencer(a) => Some(DomainAction::Sequencer(a.clone())),
            Self::Chopper(a) => Some(DomainAction::Chopper(a.clone())),
            Self::Automation(a) => Some(DomainAction::Automation(a.clone())),
            Self::Midi(a) => Some(DomainAction::Midi(a.clone())),
            Self::Bus(a) => Some(DomainAction::Bus(a.clone())),
            Self::LayerGroup(a) => Some(DomainAction::LayerGroup(a.clone())),
            Self::VstParam(a) => Some(DomainAction::VstParam(a.clone())),
            Self::Click(a) => Some(DomainAction::Click(a.clone())),
            Self::Tuner(a) => Some(DomainAction::Tuner(a.clone())),
            Self::AudioFeedback(f) => Some(DomainAction::AudioFeedback(f.clone())),
            Self::Undo => Some(DomainAction::Undo),
            Self::Redo => Some(DomainAction::Redo),
            // UI-only actions
            Self::None | Self::Quit | Self::Nav(_) | Self::ExitPerformanceMode
            | Self::PushLayer(_) | Self::PopLayer(_) | Self::SaveAndQuit => None,
        }
    }
}

impl From<DomainAction> for Action {
    fn from(d: DomainAction) -> Self {
        match d {
            DomainAction::Instrument(a) => Self::Instrument(a),
            DomainAction::Mixer(a) => Self::Mixer(a),
            DomainAction::PianoRoll(a) => Self::PianoRoll(a),
            DomainAction::Arrangement(a) => Self::Arrangement(a),
            DomainAction::Server(a) => Self::Server(a),
            DomainAction::Session(a) => Self::Session(a),
            DomainAction::Sequencer(a) => Self::Sequencer(a),
            DomainAction::Chopper(a) => Self::Chopper(a),
            DomainAction::Automation(a) => Self::Automation(a),
            DomainAction::Midi(a) => Self::Midi(a),
            DomainAction::Bus(a) => Self::Bus(a),
            DomainAction::LayerGroup(a) => Self::LayerGroup(a),
            DomainAction::VstParam(a) => Self::VstParam(a),
            DomainAction::Click(a) => Self::Click(a),
            DomainAction::Tuner(a) => Self::Tuner(a),
            DomainAction::AudioFeedback(f) => Self::AudioFeedback(f),
            DomainAction::Undo => Self::Undo,
            DomainAction::Redo => Self::Redo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_effect_all_contains_structural_effects() {
        let effects = AudioEffect::all();
        assert!(effects.contains(&AudioEffect::RebuildInstruments));
        assert!(effects.contains(&AudioEffect::RebuildSession));
        assert!(effects.contains(&AudioEffect::UpdatePianoRoll));
        assert!(effects.contains(&AudioEffect::UpdateAutomation));
        assert!(effects.contains(&AudioEffect::RebuildRouting));
        assert!(effects.contains(&AudioEffect::UpdateMixerParams));
    }

    #[test]
    fn audio_effect_for_instrument() {
        let id = InstrumentId::new(42);
        let effects = AudioEffect::for_instrument(id);
        assert!(effects.contains(&AudioEffect::RebuildInstruments));
        assert!(effects.contains(&AudioEffect::RebuildRoutingForInstrument(id)));
    }

    #[test]
    fn dispatch_result_none_is_empty() {
        let r = DispatchResult::none();
        assert!(!r.quit);
        assert!(r.nav.is_empty());
        assert!(r.status.is_empty());
        assert!(r.audio_effects.is_empty());
    }

    #[test]
    fn dispatch_result_with_quit() {
        let r = DispatchResult::with_quit();
        assert!(r.quit);
    }

    #[test]
    fn dispatch_result_merge_extends_effects() {
        let mut a = DispatchResult::none();
        a.audio_effects.push(AudioEffect::RebuildInstruments);
        let mut b = DispatchResult::none();
        b.audio_effects.push(AudioEffect::RebuildSession);
        a.merge(b);
        assert_eq!(a.audio_effects.len(), 2);
        assert!(a.audio_effects.contains(&AudioEffect::RebuildInstruments));
        assert!(a.audio_effects.contains(&AudioEffect::RebuildSession));
    }

    #[test]
    fn filter_param_kind_as_str() {
        assert_eq!(FilterParamKind::Cutoff.as_str(), "cutoff");
        assert_eq!(FilterParamKind::Resonance.as_str(), "resonance");
    }

    #[test]
    fn lfo_param_kind_as_str() {
        assert_eq!(LfoParamKind::Rate.as_str(), "rate");
        assert_eq!(LfoParamKind::Depth.as_str(), "depth");
    }

    #[test]
    fn audio_effect_is_routing() {
        assert!(AudioEffect::RebuildRouting.is_routing());
        assert!(AudioEffect::RebuildRoutingForInstrument(InstrumentId::new(1)).is_routing());
        assert!(AudioEffect::AddInstrumentRouting(InstrumentId::new(1)).is_routing());
        assert!(AudioEffect::DeleteInstrumentRouting(InstrumentId::new(1)).is_routing());
        assert!(!AudioEffect::RebuildInstruments.is_routing());
        assert!(!AudioEffect::UpdatePianoRoll.is_routing());
    }
}
