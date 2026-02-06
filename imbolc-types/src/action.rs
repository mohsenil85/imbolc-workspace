//! Action types for the dispatch system.
//!
//! Actions represent user intents that flow through the dispatch system.
//! This module contains all action enums and related types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    AutomationLaneId, AutomationTarget, ClipId, ClipboardNote, CurveType, DrumStep,
    EffectId, EffectType, EqConfig, EffectSlot, EnvConfig, FilterConfig, FilterType,
    InstrumentId, LfoConfig, MixerSelection, MusicalSettings, Param, PlacementId,
    ServerStatus, SourceType, VstPluginKind,
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
    Remove(u8),
    /// Rename a bus
    Rename(u8, String),
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
// AudioDirty and DispatchResult
// ============================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioDirty {
    pub instruments: bool,
    pub session: bool,
    pub piano_roll: bool,
    pub automation: bool,
    pub routing: bool,
    /// When set, only rebuild routing for this specific instrument (optimization).
    /// If `routing` is also true, this is ignored and a full rebuild is performed.
    pub routing_instrument: Option<InstrumentId>,
    pub mixer_params: bool,
    /// Targeted filter param update: (instrument_id, param_kind, value).
    /// Sends /n_set directly to the filter node without routing rebuild.
    pub filter_param: Option<(InstrumentId, FilterParamKind, f32)>,
    /// Targeted effect param update: (instrument_id, effect_id, param_index, value).
    /// Sends /n_set directly to the effect node without routing rebuild.
    /// The param name is resolved from the instrument state at send time.
    pub effect_param: Option<(InstrumentId, EffectId, usize, f32)>,
    /// Targeted LFO param update: (instrument_id, param_kind, value).
    /// Sends /n_set directly to the LFO node without routing rebuild.
    pub lfo_param: Option<(InstrumentId, LfoParamKind, f32)>,
}

impl AudioDirty {
    pub fn all() -> Self {
        Self {
            instruments: true,
            session: true,
            piano_roll: true,
            automation: true,
            routing: true,
            routing_instrument: None,
            mixer_params: true,
            filter_param: None,
            effect_param: None,
            lfo_param: None,
        }
    }

    pub fn any(&self) -> bool {
        self.instruments
            || self.session
            || self.piano_roll
            || self.automation
            || self.routing
            || self.routing_instrument.is_some()
            || self.mixer_params
            || self.filter_param.is_some()
            || self.effect_param.is_some()
            || self.lfo_param.is_some()
    }

    pub fn merge(&mut self, other: AudioDirty) {
        self.instruments |= other.instruments;
        self.session |= other.session;
        self.piano_roll |= other.piano_roll;
        self.automation |= other.automation;
        self.routing |= other.routing;
        // Merge routing_instrument: if both have a targeted instrument but they differ,
        // escalate to full rebuild. If only one has it, keep it.
        match (self.routing_instrument, other.routing_instrument) {
            (Some(a), Some(b)) if a != b => {
                // Different instruments targeted — escalate to full rebuild
                self.routing = true;
                self.routing_instrument = None;
            }
            (None, Some(id)) => {
                self.routing_instrument = Some(id);
            }
            _ => {} // keep existing
        }
        self.mixer_params |= other.mixer_params;
        // Targeted param updates: last one wins (these are real-time tweaks)
        if other.filter_param.is_some() {
            self.filter_param = other.filter_param;
        }
        if other.effect_param.is_some() {
            self.effect_param = other.effect_param;
        }
        if other.lfo_param.is_some() {
            self.lfo_param = other.lfo_param;
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Result of dispatching an action — contains side effects for the UI layer to process.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub quit: bool,
    pub nav: Vec<NavIntent>,
    pub status: Vec<StatusEvent>,
    pub project_name: Option<String>,
    pub audio_dirty: AudioDirty,
    /// Signal that playback should be stopped (processed by the UI layer, not dispatch)
    pub stop_playback: bool,
    /// Signal that the playhead should be reset to 0 (processed by the UI layer, not dispatch)
    pub reset_playhead: bool,
}

impl Default for DispatchResult {
    fn default() -> Self {
        Self {
            quit: false,
            nav: Vec::new(),
            status: Vec::new(),
            project_name: None,
            audio_dirty: AudioDirty::default(),
            stop_playback: false,
            reset_playhead: false,
        }
    }
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

    pub fn mark_audio_dirty(&mut self, dirty: AudioDirty) {
        self.audio_dirty.merge(dirty);
    }

    pub fn merge(&mut self, other: DispatchResult) {
        self.quit = self.quit || other.quit;
        self.nav.extend(other.nav);
        self.status.extend(other.status);
        if other.project_name.is_some() {
            self.project_name = other.project_name;
        }
        self.audio_dirty.merge(other.audio_dirty);
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
    AdjustSend(u8, f32),
    ToggleSend(u8),
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
}

/// Data carried by InstrumentAction::Update to apply edits without dispatch reading pane state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentUpdate {
    pub id: InstrumentId,
    pub source: SourceType,
    pub source_params: Vec<Param>,
    pub filter: Option<FilterConfig>,
    pub eq: Option<EqConfig>,
    pub effects: Vec<EffectSlot>,
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
    MoveEffect(InstrumentId, EffectId, i8),
    SetFilter(InstrumentId, Option<FilterType>),
    ToggleEffectBypass(InstrumentId, EffectId),
    ToggleFilter(InstrumentId),
    CycleFilterType(InstrumentId),
    AdjustFilterCutoff(InstrumentId, f32),
    AdjustFilterResonance(InstrumentId, f32),
    AdjustEffectParam(InstrumentId, EffectId, usize, f32),
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
    SetEqParam(InstrumentId, usize, String, f32), // instrument_id, band_index, param_name, value
    ToggleEq(InstrumentId),
    LinkLayer(InstrumentId, InstrumentId),
    UnlinkLayer(InstrumentId),
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
    SetLfoTarget(InstrumentId, crate::LfoTarget),
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
            | Self::MoveEffect(id, _, _)
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
// Main Action enum
// ============================================================================

/// Actions that can be returned from pane input handling.
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
    VstParam(VstParamAction),
    Click(ClickAction),
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
