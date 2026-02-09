mod effect;
mod envelope;
mod filter;
mod lfo;
mod source_type;

pub use effect::*;
pub use envelope::*;
pub use filter::*;
pub use lfo::*;
pub use source_type::*;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::arpeggiator::{ArpeggiatorConfig, ChordShape};
use super::drum_sequencer::DrumSequencerState;
use super::groove::GrooveConfig;
use super::sampler::SamplerConfig;
use crate::{EffectId, InstrumentId, Param};

/// Whether an instrument's signal chain is mono or stereo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChannelConfig {
    Mono,
    #[default]
    Stereo,
}

impl ChannelConfig {
    pub fn is_mono(&self) -> bool {
        matches!(self, ChannelConfig::Mono)
    }

    pub fn is_stereo(&self) -> bool {
        matches!(self, ChannelConfig::Stereo)
    }

    pub fn toggle(&self) -> Self {
        match self {
            ChannelConfig::Mono => ChannelConfig::Stereo,
            ChannelConfig::Stereo => ChannelConfig::Mono,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelConfig::Mono => "MONO",
            ChannelConfig::Stereo => "STEREO",
        }
    }

    /// Number of audio channels (1 for mono, 2 for stereo)
    pub fn channels(&self) -> usize {
        match self {
            ChannelConfig::Mono => 1,
            ChannelConfig::Stereo => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputTarget {
    Master,
    Bus(u8), // 1-8
}

impl Default for OutputTarget {
    fn default() -> Self {
        Self::Master
    }
}

/// Where a send taps the instrument signal chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SendTapPoint {
    /// Before filter and effects
    PreInsert,
    /// After all insert effects, before fader (industry-standard "pre-fader send")
    #[default]
    PostInsert,
}

impl SendTapPoint {
    pub fn cycle(&self) -> Self {
        match self {
            SendTapPoint::PreInsert => SendTapPoint::PostInsert,
            SendTapPoint::PostInsert => SendTapPoint::PreInsert,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SendTapPoint::PreInsert => "PRE",
            SendTapPoint::PostInsert => "POST",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerSend {
    pub bus_id: u8,
    pub level: f32,
    pub enabled: bool,
    #[serde(default)]
    pub tap_point: SendTapPoint,
}

impl MixerSend {
    pub fn new(bus_id: u8) -> Self {
        Self {
            bus_id,
            level: 0.0,
            enabled: false,
            tap_point: SendTapPoint::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerBus {
    pub id: u8,
    pub name: String,
    pub level: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    #[serde(default)]
    pub effects: Vec<EffectSlot>,
    #[serde(default)]
    pub next_effect_id: EffectId,
}

impl MixerBus {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            name: format!("Bus {}", id),
            level: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            effects: Vec::new(),
            next_effect_id: 0,
        }
    }

    /// Add an effect and return its stable EffectId.
    pub fn add_effect(&mut self, effect_type: EffectType) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        self.effects.push(EffectSlot::new(id, effect_type));
        id
    }

    /// Find an effect by its stable EffectId.
    pub fn effect_by_id(&self, id: EffectId) -> Option<&EffectSlot> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Find a mutable effect by its stable EffectId.
    pub fn effect_by_id_mut(&mut self, id: EffectId) -> Option<&mut EffectSlot> {
        self.effects.iter_mut().find(|e| e.id == id)
    }

    /// Get the position of an effect in the chain by EffectId.
    pub fn effect_position(&self, id: EffectId) -> Option<usize> {
        self.effects.iter().position(|e| e.id == id)
    }

    /// Remove an effect by its EffectId, returns true if removed.
    pub fn remove_effect(&mut self, id: EffectId) -> bool {
        if let Some(pos) = self.effect_position(id) {
            self.effects.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move an effect up or down by its EffectId.
    pub fn move_effect(&mut self, id: EffectId, direction: i8) -> bool {
        if let Some(pos) = self.effect_position(id) {
            let new_pos = (pos as i8 + direction).max(0) as usize;
            if new_pos < self.effects.len() {
                self.effects.swap(pos, new_pos);
                return true;
            }
        }
        false
    }

    /// Recalculate next_effect_id from existing effects (used after loading).
    pub fn recalculate_next_effect_id(&mut self) {
        self.next_effect_id = self
            .effects
            .iter()
            .map(|e| e.id)
            .max()
            .map_or(0, |m| m + 1);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerGroupMixer {
    pub group_id: u32,
    pub name: String,
    pub level: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub output_target: OutputTarget,
    pub sends: Vec<MixerSend>,
    #[serde(default)]
    pub effects: Vec<EffectSlot>,
    #[serde(default)]
    pub next_effect_id: EffectId,
    #[serde(default)]
    pub eq: Option<EqConfig>,
}

impl LayerGroupMixer {
    pub fn new(group_id: u32, bus_ids: &[u8]) -> Self {
        let sends = bus_ids.iter().map(|&id| MixerSend::new(id)).collect();
        Self {
            group_id,
            name: format!("Group {}", group_id),
            level: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            output_target: OutputTarget::Master,
            sends,
            effects: Vec::new(),
            next_effect_id: 0,
            eq: Some(EqConfig::default()),
        }
    }

    pub fn toggle_eq(&mut self) {
        if self.eq.is_some() {
            self.eq = None;
        } else {
            self.eq = Some(EqConfig::default());
        }
    }

    pub fn eq(&self) -> Option<&EqConfig> {
        self.eq.as_ref()
    }

    pub fn eq_mut(&mut self) -> Option<&mut EqConfig> {
        self.eq.as_mut()
    }

    pub fn sync_sends_with_buses(&mut self, bus_ids: &[u8]) {
        for &bus_id in bus_ids {
            if !self.sends.iter().any(|s| s.bus_id == bus_id) {
                self.sends.push(MixerSend::new(bus_id));
            }
        }
        self.sends.sort_by_key(|s| s.bus_id);
    }

    pub fn disable_send_for_bus(&mut self, bus_id: u8) {
        if let Some(send) = self.sends.iter_mut().find(|s| s.bus_id == bus_id) {
            send.enabled = false;
        }
    }

    /// Add an effect and return its stable EffectId.
    pub fn add_effect(&mut self, effect_type: EffectType) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        self.effects.push(EffectSlot::new(id, effect_type));
        id
    }

    /// Find an effect by its stable EffectId.
    pub fn effect_by_id(&self, id: EffectId) -> Option<&EffectSlot> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Find a mutable effect by its stable EffectId.
    pub fn effect_by_id_mut(&mut self, id: EffectId) -> Option<&mut EffectSlot> {
        self.effects.iter_mut().find(|e| e.id == id)
    }

    /// Get the position of an effect in the chain by EffectId.
    pub fn effect_position(&self, id: EffectId) -> Option<usize> {
        self.effects.iter().position(|e| e.id == id)
    }

    /// Remove an effect by its EffectId, returns true if removed.
    pub fn remove_effect(&mut self, id: EffectId) -> bool {
        if let Some(pos) = self.effect_position(id) {
            self.effects.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move an effect up or down by its EffectId.
    pub fn move_effect(&mut self, id: EffectId, direction: i8) -> bool {
        if let Some(pos) = self.effect_position(id) {
            let new_pos = (pos as i8 + direction).max(0) as usize;
            if new_pos < self.effects.len() {
                self.effects.swap(pos, new_pos);
                return true;
            }
        }
        false
    }

    /// Recalculate next_effect_id from existing effects (used after loading).
    pub fn recalculate_next_effect_id(&mut self) {
        self.next_effect_id = self
            .effects
            .iter()
            .map(|e| e.id)
            .max()
            .map_or(0, |m| m + 1);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulatedParam {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub mod_source: Option<ModSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModSource {
    Lfo(LfoConfig),
    Envelope(EnvConfig),
    InstrumentParam(InstrumentId, String),
}

/// A single stage in the instrument's processing chain.
/// Replaces the old separate filter/eq/effects fields with a unified,
/// user-orderable signal chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStage {
    Filter(FilterConfig),
    Eq(EqConfig),
    Effect(EffectSlot),
}

impl ProcessingStage {
    pub fn is_filter(&self) -> bool {
        matches!(self, ProcessingStage::Filter(_))
    }

    pub fn is_eq(&self) -> bool {
        matches!(self, ProcessingStage::Eq(_))
    }

    pub fn is_effect(&self) -> bool {
        matches!(self, ProcessingStage::Effect(_))
    }

    /// Number of editable rows this stage occupies in the instrument editor.
    pub fn row_count(&self) -> usize {
        match self {
            ProcessingStage::Filter(f) => 3 + f.extra_params.len(),
            ProcessingStage::Eq(_) => 1,
            ProcessingStage::Effect(e) => 1 + e.params.len(),
        }
    }
}

/// Decode an effect cursor position into (EffectId, Option<param_index>).
/// Returns None if cursor is out of range. Used by Instrument, MixerBus, and LayerGroupMixer.
pub fn decode_effect_cursor_from_slice(effects: &[EffectSlot], cursor: usize) -> Option<(EffectId, Option<usize>)> {
    let mut pos = 0;
    for effect in effects {
        if cursor == pos {
            return Some((effect.id, None));
        }
        pos += 1;
        for pi in 0..effect.params.len() {
            if cursor == pos {
                return Some((effect.id, Some(pi)));
            }
            pos += 1;
        }
    }
    None
}

/// Max cursor position for an effect chain. Returns 0 for empty chains.
pub fn effects_max_cursor(effects: &[EffectSlot]) -> usize {
    if effects.is_empty() {
        return 0;
    }
    let total: usize = effects.iter().map(|e| 1 + e.params.len()).sum();
    total.saturating_sub(1)
}

/// Which section of an instrument a given editing row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentSection {
    Source,
    Processing(usize), // chain index
    Lfo,
    Envelope,
}

/// Total number of selectable rows for instrument editing.
///
/// Free function variant: accepts decomposed fields so callers holding
/// cloned/shadow copies of instrument fields (e.g. InstrumentEditPane)
/// can call without constructing a temporary Instrument.
pub fn instrument_row_count(
    source: SourceType,
    source_params: &[Param],
    processing_chain: &[ProcessingStage],
) -> usize {
    let sample_row = if source.is_sample() || source.is_time_stretch() {
        1
    } else {
        0
    };
    let source_rows = sample_row + source_params.len().max(1);
    let processing_rows = if processing_chain.is_empty() {
        1 // placeholder row
    } else {
        processing_chain.iter().map(|s| s.row_count()).sum()
    };
    let lfo_rows = 4;
    let env_rows = if source.is_vst() { 0 } else { 4 };
    source_rows + processing_rows + lfo_rows + env_rows
}

/// Which section a given row belongs to.
///
/// Free function variant for use with decomposed fields.
pub fn instrument_section_for_row(
    row: usize,
    source: SourceType,
    source_params: &[Param],
    processing_chain: &[ProcessingStage],
) -> InstrumentSection {
    let (section, _) = instrument_row_info(row, source, source_params, processing_chain);
    section
}

/// Get section and local index for a given row.
///
/// Free function variant for use with decomposed fields.
pub fn instrument_row_info(
    row: usize,
    source: SourceType,
    source_params: &[Param],
    processing_chain: &[ProcessingStage],
) -> (InstrumentSection, usize) {
    let sample_row = if source.is_sample() || source.is_time_stretch() {
        1
    } else {
        0
    };
    let source_rows = sample_row + source_params.len().max(1);

    if row < source_rows {
        return (InstrumentSection::Source, row);
    }
    let mut offset = source_rows;

    // Processing chain
    if processing_chain.is_empty() {
        // One placeholder row mapped to Processing(0)
        if row < offset + 1 {
            return (InstrumentSection::Processing(0), 0);
        }
        offset += 1;
    } else {
        for (i, stage) in processing_chain.iter().enumerate() {
            let rc = stage.row_count();
            if row < offset + rc {
                return (InstrumentSection::Processing(i), row - offset);
            }
            offset += rc;
        }
    }

    let lfo_rows = 4;
    if row < offset + lfo_rows {
        return (InstrumentSection::Lfo, row - offset);
    }
    offset += lfo_rows;

    (InstrumentSection::Envelope, row - offset)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    pub source: SourceType,
    pub source_params: Vec<Param>,
    #[serde(default)]
    pub processing_chain: Vec<ProcessingStage>,
    pub lfo: LfoConfig,
    pub amp_envelope: EnvConfig,
    pub polyphonic: bool,
    // Integrated mixer
    pub level: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub active: bool,
    pub output_target: OutputTarget,
    /// Mono or stereo signal chain
    #[serde(default)]
    pub channel_config: ChannelConfig,
    pub sends: Vec<MixerSend>,
    // Sample configuration (only used when source is SourceType::PitchedSampler)
    pub sampler_config: Option<SamplerConfig>,
    // Kit sequencer (only used when source is SourceType::Kit)
    pub drum_sequencer: Option<DrumSequencerState>,
    // Per-instance VST parameter values: (param_index, normalized_value)
    pub vst_param_values: Vec<(u32, f32)>,
    // Path to saved VST plugin state file (.fxp)
    pub vst_state_path: Option<PathBuf>,
    /// Arpeggiator configuration
    pub arpeggiator: ArpeggiatorConfig,
    /// Chord shape (None = single notes, Some = expand to chord)
    pub chord_shape: Option<ChordShape>,
    /// Path to loaded impulse response file for convolution reverb
    pub convolution_ir_path: Option<String>,
    /// Layer group ID: instruments sharing the same group sound together
    pub layer_group: Option<u32>,
    /// Per-instrument octave offset for layer groups (-4 to +4, default 0)
    pub layer_octave_offset: i8,
    /// Counter for allocating unique EffectIds
    pub next_effect_id: EffectId,
    /// Per-track groove settings (swing, humanization, timing offset)
    pub groove: GrooveConfig,
}

impl Instrument {
    pub fn new(id: InstrumentId, source: SourceType) -> Self {
        // Sends are initialized empty; call sync_sends_with_buses() after creation
        let sends = Vec::new();
        // Sample instruments get a sampler config
        let sampler_config = if source.is_sample() || source.is_time_stretch() {
            Some(SamplerConfig::default())
        } else {
            None
        };
        // Kit instruments get a drum sequencer
        let drum_sequencer = if source.is_kit() {
            Some(DrumSequencerState::new())
        } else {
            None
        };
        Self {
            id,
            name: format!("{}-{}", source.short_name(), id),
            source,
            source_params: source.default_params(),
            processing_chain: Vec::new(),
            lfo: LfoConfig::default(),
            amp_envelope: source.default_envelope(),
            polyphonic: true,
            level: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            active: !source.is_audio_input(),
            output_target: OutputTarget::Master,
            channel_config: ChannelConfig::default(),
            sends,
            sampler_config,
            drum_sequencer,
            vst_param_values: Vec::new(),
            vst_state_path: None,
            arpeggiator: ArpeggiatorConfig::default(),
            chord_shape: None,
            convolution_ir_path: None,
            layer_group: None,
            layer_octave_offset: 0,
            next_effect_id: 0,
            groove: GrooveConfig::default(),
        }
    }

    // --- Processing chain read accessors ---

    /// Get the first filter in the processing chain.
    pub fn filter(&self) -> Option<&FilterConfig> {
        self.processing_chain.iter().find_map(|s| match s {
            ProcessingStage::Filter(f) => Some(f),
            _ => None,
        })
    }

    /// Get the first filter mutably.
    pub fn filter_mut(&mut self) -> Option<&mut FilterConfig> {
        self.processing_chain.iter_mut().find_map(|s| match s {
            ProcessingStage::Filter(f) => Some(f),
            _ => None,
        })
    }

    /// Get all filters in the processing chain.
    pub fn filters(&self) -> impl Iterator<Item = &FilterConfig> {
        self.processing_chain.iter().filter_map(|s| match s {
            ProcessingStage::Filter(f) => Some(f),
            _ => None,
        })
    }

    /// Get all filters mutably.
    pub fn filters_mut(&mut self) -> impl Iterator<Item = &mut FilterConfig> {
        self.processing_chain.iter_mut().filter_map(|s| match s {
            ProcessingStage::Filter(f) => Some(f),
            _ => None,
        })
    }

    /// Get the EQ config (single instance).
    pub fn eq(&self) -> Option<&EqConfig> {
        self.processing_chain.iter().find_map(|s| match s {
            ProcessingStage::Eq(eq) => Some(eq),
            _ => None,
        })
    }

    /// Get the EQ config mutably.
    pub fn eq_mut(&mut self) -> Option<&mut EqConfig> {
        self.processing_chain.iter_mut().find_map(|s| match s {
            ProcessingStage::Eq(eq) => Some(eq),
            _ => None,
        })
    }

    /// Check whether an EQ is present.
    pub fn has_eq(&self) -> bool {
        self.processing_chain.iter().any(|s| s.is_eq())
    }

    /// Get all effects in the processing chain.
    pub fn effects(&self) -> impl Iterator<Item = &EffectSlot> {
        self.processing_chain.iter().filter_map(|s| match s {
            ProcessingStage::Effect(e) => Some(e),
            _ => None,
        })
    }

    /// Get all effects mutably.
    pub fn effects_mut(&mut self) -> impl Iterator<Item = &mut EffectSlot> {
        self.processing_chain.iter_mut().filter_map(|s| match s {
            ProcessingStage::Effect(e) => Some(e),
            _ => None,
        })
    }

    /// Collect effects into a Vec (convenience for code that needs a slice).
    pub fn effects_vec(&self) -> Vec<&EffectSlot> {
        self.effects().collect()
    }

    /// Find an effect by its stable EffectId.
    pub fn effect_by_id(&self, id: EffectId) -> Option<&EffectSlot> {
        self.effects().find(|e| e.id == id)
    }

    /// Find a mutable effect by its stable EffectId.
    pub fn effect_by_id_mut(&mut self, id: EffectId) -> Option<&mut EffectSlot> {
        self.effects_mut().find(|e| e.id == id)
    }

    /// Get the position of an effect among effects only (not chain index).
    pub fn effect_position(&self, id: EffectId) -> Option<usize> {
        self.effects().position(|e| e.id == id)
    }

    // --- Index queries into the full chain ---

    /// Chain index of the first filter.
    pub fn filter_chain_index(&self) -> Option<usize> {
        self.processing_chain.iter().position(|s| s.is_filter())
    }

    /// Chain index of the EQ.
    pub fn eq_chain_index(&self) -> Option<usize> {
        self.processing_chain.iter().position(|s| s.is_eq())
    }

    /// Chain index of an effect by its EffectId.
    pub fn effect_chain_index(&self, id: EffectId) -> Option<usize> {
        self.processing_chain.iter().position(|s| matches!(s, ProcessingStage::Effect(e) if e.id == id))
    }

    // --- Mutation helpers ---

    /// Toggle filter: remove first filter if present, or insert Lpf at index 0.
    pub fn toggle_filter(&mut self) {
        if let Some(idx) = self.filter_chain_index() {
            self.processing_chain.remove(idx);
        } else {
            self.processing_chain.insert(0, ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        }
    }

    /// Set filter type. None removes; Some replaces or inserts at index 0.
    pub fn set_filter(&mut self, filter_type: Option<FilterType>) {
        match filter_type {
            None => {
                if let Some(idx) = self.filter_chain_index() {
                    self.processing_chain.remove(idx);
                }
            }
            Some(ft) => {
                if let Some(idx) = self.filter_chain_index() {
                    self.processing_chain[idx] = ProcessingStage::Filter(FilterConfig::new(ft));
                } else {
                    self.processing_chain.insert(0, ProcessingStage::Filter(FilterConfig::new(ft)));
                }
            }
        }
    }

    /// Toggle EQ: remove if present, or insert after last filter (single instance enforced).
    pub fn toggle_eq(&mut self) {
        if let Some(idx) = self.eq_chain_index() {
            self.processing_chain.remove(idx);
        } else {
            // Insert after the last filter, or at index 0 if no filters
            let insert_at = self.processing_chain.iter()
                .rposition(|s| s.is_filter())
                .map(|i| i + 1)
                .unwrap_or(0);
            self.processing_chain.insert(insert_at, ProcessingStage::Eq(EqConfig::default()));
        }
    }

    /// Add an effect to the end of the chain. Returns its stable EffectId.
    pub fn add_effect(&mut self, effect_type: EffectType) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        self.processing_chain.push(ProcessingStage::Effect(EffectSlot::new(id, effect_type)));
        id
    }

    /// Remove an effect by its EffectId. Returns true if removed.
    pub fn remove_effect(&mut self, id: EffectId) -> bool {
        if let Some(idx) = self.effect_chain_index(id) {
            self.processing_chain.remove(idx);
            true
        } else {
            false
        }
    }

    /// Move an effect within the full processing chain by its EffectId.
    pub fn move_effect(&mut self, id: EffectId, direction: i8) -> bool {
        if let Some(idx) = self.effect_chain_index(id) {
            self.move_stage(idx, direction)
        } else {
            false
        }
    }

    /// Move any stage within the processing chain by chain index.
    pub fn move_stage(&mut self, idx: usize, direction: i8) -> bool {
        if idx >= self.processing_chain.len() {
            return false;
        }
        let new_idx = (idx as i64 + direction as i64).max(0) as usize;
        if new_idx >= self.processing_chain.len() || new_idx == idx {
            return false;
        }
        self.processing_chain.swap(idx, new_idx);
        true
    }

    /// Apply layer octave offset to a pitch, clamping to MIDI range 0..=127.
    pub fn offset_pitch(&self, pitch: u8) -> u8 {
        ((pitch as i16) + (self.layer_octave_offset as i16 * 12)).clamp(0, 127) as u8
    }

    /// Recalculate next_effect_id from existing effects in the chain (used after loading).
    pub fn recalculate_next_effect_id(&mut self) {
        self.next_effect_id = self
            .effects()
            .map(|e| e.id)
            .max()
            .map_or(0, |m| m + 1);
    }

    /// Sync sends with current bus IDs. Adds missing sends, keeps existing ones.
    pub fn sync_sends_with_buses(&mut self, bus_ids: &[u8]) {
        for &bus_id in bus_ids {
            if !self.sends.iter().any(|s| s.bus_id == bus_id) {
                self.sends.push(MixerSend::new(bus_id));
            }
        }
        // Sort sends by bus_id for consistent ordering
        self.sends.sort_by_key(|s| s.bus_id);
    }

    /// Disable sends for a removed bus (keeps the entry for undo support)
    pub fn disable_send_for_bus(&mut self, bus_id: u8) {
        if let Some(send) = self.sends.iter_mut().find(|s| s.bus_id == bus_id) {
            send.enabled = false;
        }
    }

    // --- Structure navigation convenience methods ---

    /// Total number of selectable rows for instrument editing.
    pub fn total_editable_rows(&self) -> usize {
        instrument_row_count(self.source, &self.source_params, &self.processing_chain)
    }

    /// Which section a given row belongs to.
    pub fn section_for_row(&self, row: usize) -> InstrumentSection {
        instrument_section_for_row(
            row,
            self.source,
            &self.source_params,
            &self.processing_chain,
        )
    }

    /// Get section and local index for a given row.
    pub fn row_info(&self, row: usize) -> (InstrumentSection, usize) {
        instrument_row_info(
            row,
            self.source,
            &self.source_params,
            &self.processing_chain,
        )
    }

    /// Decode a flat cursor position over just the effects in the chain into (EffectId, Option<param_index>).
    /// Returns None if cursor is out of range. None param_index means the effect header row.
    pub fn decode_effect_cursor(&self, cursor: usize) -> Option<(EffectId, Option<usize>)> {
        let effects: Vec<_> = self.effects().cloned().collect();
        decode_effect_cursor_from_slice(&effects, cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_count_basic() {
        let inst = Instrument::new(1, SourceType::Saw);
        let count = inst.total_editable_rows();
        // Saw: no sample row, default params + processing(empty=1) + lfo(4) + env(4)
        let expected = inst.source_params.len().max(1) + 1 + 4 + 4;
        assert_eq!(count, expected);
    }

    #[test]
    fn section_for_row_first_is_source() {
        let inst = Instrument::new(1, SourceType::Saw);
        assert_eq!(inst.section_for_row(0), InstrumentSection::Source);
    }

    #[test]
    fn row_info_returns_local_index() {
        let inst = Instrument::new(1, SourceType::Saw);
        let source_rows = inst.source_params.len().max(1);
        // First row after source section should be Processing(0) with local_idx 0
        let (section, local_idx) = inst.row_info(source_rows);
        assert_eq!(section, InstrumentSection::Processing(0));
        assert_eq!(local_idx, 0);
    }

    #[test]
    fn row_info_roundtrips_all_sections() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.toggle_filter();
        inst.add_effect(EffectType::Delay);
        let total = inst.total_editable_rows();
        let mut has_source = false;
        let mut has_processing = false;
        let mut has_lfo = false;
        let mut has_envelope = false;
        for i in 0..total {
            match inst.section_for_row(i) {
                InstrumentSection::Source => has_source = true,
                InstrumentSection::Processing(_) => has_processing = true,
                InstrumentSection::Lfo => has_lfo = true,
                InstrumentSection::Envelope => has_envelope = true,
            }
        }
        assert!(has_source);
        assert!(has_processing);
        assert!(has_lfo);
        assert!(has_envelope);
    }

    #[test]
    fn vst_has_no_envelope_rows() {
        let inst = Instrument::new(1, SourceType::Vst(0));
        let total = inst.total_editable_rows();
        for i in 0..total {
            assert_ne!(inst.section_for_row(i), InstrumentSection::Envelope);
        }
    }

    #[test]
    fn decode_effect_cursor_empty() {
        let inst = Instrument::new(1, SourceType::Saw);
        assert!(inst.effects().next().is_none());
        assert_eq!(inst.decode_effect_cursor(0), None);
    }

    #[test]
    fn decode_effect_cursor_with_effects() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        assert_eq!(inst.decode_effect_cursor(0), Some((id1, None)));
        let params1 = inst.effects().next().unwrap().params.len();
        assert_eq!(inst.decode_effect_cursor(1 + params1), Some((id2, None)));
    }

    #[test]
    fn timestretch_has_sampler_config_and_correct_params() {
        let inst = Instrument::new(1, SourceType::TimeStretch);
        assert!(inst.sampler_config.is_some());
        let param_names: Vec<&str> = inst.source_params.iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"stretch"));
        assert!(param_names.contains(&"pitch"));
        assert!(param_names.contains(&"grain_size"));
        assert!(param_names.contains(&"overlap"));
        assert!(param_names.contains(&"amp"));
        assert!(!param_names.contains(&"rate"));
    }

    #[test]
    fn timestretch_has_sample_row_in_count() {
        let inst = Instrument::new(1, SourceType::TimeStretch);
        let count = inst.total_editable_rows();
        // TimeStretch: 1 sample row + params + processing(empty=1) + lfo(4) + env(4)
        let expected = 1 + inst.source_params.len().max(1) + 1 + 4 + 4;
        assert_eq!(count, expected);
    }

    #[test]
    fn channel_config_default_is_stereo() {
        assert_eq!(ChannelConfig::default(), ChannelConfig::Stereo);
    }

    #[test]
    fn channel_config_toggle() {
        assert_eq!(ChannelConfig::Mono.toggle(), ChannelConfig::Stereo);
        assert_eq!(ChannelConfig::Stereo.toggle(), ChannelConfig::Mono);
    }

    #[test]
    fn channel_config_as_str() {
        assert_eq!(ChannelConfig::Mono.as_str(), "MONO");
        assert_eq!(ChannelConfig::Stereo.as_str(), "STEREO");
    }

    #[test]
    fn channel_config_channels() {
        assert_eq!(ChannelConfig::Mono.channels(), 1);
        assert_eq!(ChannelConfig::Stereo.channels(), 2);
    }

    #[test]
    fn channel_config_is_mono_is_stereo() {
        assert!(ChannelConfig::Mono.is_mono());
        assert!(!ChannelConfig::Mono.is_stereo());
        assert!(ChannelConfig::Stereo.is_stereo());
        assert!(!ChannelConfig::Stereo.is_mono());
    }

    #[test]
    fn output_target_default_is_master() {
        assert_eq!(OutputTarget::default(), OutputTarget::Master);
    }

    #[test]
    fn mixer_send_new() {
        let send = MixerSend::new(3);
        assert_eq!(send.bus_id, 3);
        assert_eq!(send.level, 0.0);
        assert!(!send.enabled);
    }

    #[test]
    fn instrument_add_remove_effect() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id = inst.add_effect(EffectType::Delay);
        assert!(inst.effect_by_id(id).is_some());
        assert!(inst.remove_effect(id));
        assert!(inst.effect_by_id(id).is_none());
    }

    #[test]
    fn instrument_move_effect() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        assert_eq!(inst.effect_position(id1), Some(0));
        assert_eq!(inst.effect_position(id2), Some(1));
        inst.move_effect(id2, -1);
        assert_eq!(inst.effect_position(id2), Some(0));
        assert_eq!(inst.effect_position(id1), Some(1));
    }

    #[test]
    fn decode_effect_cursor_from_slice_navigates_headers_and_params() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        let effects: Vec<_> = inst.effects().cloned().collect();
        let params1 = effects[0].params.len();

        assert_eq!(decode_effect_cursor_from_slice(&effects, 0), Some((id1, None)));
        if params1 > 0 {
            assert_eq!(decode_effect_cursor_from_slice(&effects, 1), Some((id1, Some(0))));
        }
        assert_eq!(decode_effect_cursor_from_slice(&effects, 1 + params1), Some((id2, None)));
        let total: usize = effects.iter().map(|e| 1 + e.params.len()).sum();
        assert_eq!(decode_effect_cursor_from_slice(&effects, total), None);
    }

    #[test]
    fn effects_max_cursor_empty_and_populated() {
        assert_eq!(effects_max_cursor(&[]), 0);

        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay);
        inst.add_effect(EffectType::Reverb);
        let effects: Vec<_> = inst.effects().cloned().collect();
        let total: usize = effects.iter().map(|e| 1 + e.params.len()).sum();
        assert_eq!(effects_max_cursor(&effects), total - 1);
    }

    #[test]
    fn offset_pitch_identity_at_zero() {
        let inst = Instrument::new(1, SourceType::Saw);
        assert_eq!(inst.layer_octave_offset, 0);
        assert_eq!(inst.offset_pitch(60), 60);
        assert_eq!(inst.offset_pitch(0), 0);
        assert_eq!(inst.offset_pitch(127), 127);
    }

    #[test]
    fn offset_pitch_positive() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.layer_octave_offset = 2;
        assert_eq!(inst.offset_pitch(60), 84);
        assert_eq!(inst.offset_pitch(48), 72);
    }

    #[test]
    fn offset_pitch_negative() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.layer_octave_offset = -3;
        assert_eq!(inst.offset_pitch(60), 24);
    }

    #[test]
    fn offset_pitch_clamps_high() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.layer_octave_offset = 4;
        assert_eq!(inst.offset_pitch(120), 127);
    }

    #[test]
    fn offset_pitch_clamps_low() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.layer_octave_offset = -4;
        assert_eq!(inst.offset_pitch(10), 0);
    }

    #[test]
    fn layer_group_mixer_new_has_eq() {
        let gm = LayerGroupMixer::new(1, &[1, 2]);
        assert!(gm.eq.is_some());
        let eq = gm.eq().unwrap();
        assert!(eq.enabled);
        assert_eq!(eq.bands.len(), EQ_BAND_COUNT);
        for band in &eq.bands {
            assert_eq!(band.gain, 0.0);
        }
    }

    #[test]
    fn layer_group_mixer_toggle_eq() {
        let mut gm = LayerGroupMixer::new(1, &[]);
        assert!(gm.eq().is_some());
        gm.toggle_eq();
        assert!(gm.eq().is_none());
        gm.toggle_eq();
        assert!(gm.eq().is_some());
    }

    #[test]
    fn layer_group_mixer_eq_mut() {
        let mut gm = LayerGroupMixer::new(1, &[]);
        if let Some(eq) = gm.eq_mut() {
            eq.bands[0].gain = 3.0;
        }
        assert_eq!(gm.eq().unwrap().bands[0].gain, 3.0);
    }

    // --- ProcessingStage tests ---

    #[test]
    fn processing_stage_is_methods() {
        let f = ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf));
        let e = ProcessingStage::Eq(EqConfig::default());
        let fx = ProcessingStage::Effect(EffectSlot::new(0, EffectType::Delay));
        assert!(f.is_filter());
        assert!(!f.is_eq());
        assert!(!f.is_effect());
        assert!(e.is_eq());
        assert!(fx.is_effect());
    }

    #[test]
    fn processing_stage_row_count_filter() {
        let f = ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf));
        assert_eq!(f.row_count(), 3);
        let fv = ProcessingStage::Filter(FilterConfig::new(FilterType::Vowel));
        assert_eq!(fv.row_count(), 4); // 3 + 1 extra param
    }

    #[test]
    fn processing_stage_row_count_eq() {
        let eq = ProcessingStage::Eq(EqConfig::default());
        assert_eq!(eq.row_count(), 1);
    }

    #[test]
    fn processing_stage_row_count_effect() {
        let delay = ProcessingStage::Effect(EffectSlot::new(0, EffectType::Delay));
        let delay_params = EffectType::Delay.default_params().len();
        assert_eq!(delay.row_count(), 1 + delay_params);

        let vst = ProcessingStage::Effect(EffectSlot::new(1, EffectType::Vst(0)));
        assert_eq!(vst.row_count(), 1);
    }

    // --- Convenience accessor tests ---

    #[test]
    fn filter_accessors() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        assert!(inst.filter().is_none());
        assert_eq!(inst.filters().count(), 0);

        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Hpf)));
        assert_eq!(inst.filter().unwrap().filter_type, FilterType::Hpf);
        assert_eq!(inst.filters().count(), 1);

        inst.filter_mut().unwrap().cutoff.value = 5000.0;
        assert_eq!(inst.filter().unwrap().cutoff.value, 5000.0);
    }

    #[test]
    fn eq_accessors() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        assert!(inst.eq().is_none());
        assert!(!inst.has_eq());

        inst.processing_chain.push(ProcessingStage::Eq(EqConfig::default()));
        assert!(inst.eq().is_some());
        assert!(inst.has_eq());

        inst.eq_mut().unwrap().bands[0].gain = 6.0;
        assert_eq!(inst.eq().unwrap().bands[0].gain, 6.0);
    }

    #[test]
    fn effects_accessors() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        assert_eq!(inst.effects().count(), 0);

        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        assert_eq!(inst.effects().count(), 2);
        assert!(inst.effect_by_id(id1).is_some());
        assert!(inst.effect_by_id(id2).is_some());
        assert_eq!(inst.effect_position(id1), Some(0));
        assert_eq!(inst.effect_position(id2), Some(1));
    }

    #[test]
    fn effect_by_id_through_mixed_chain() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        let id = inst.add_effect(EffectType::Delay);
        inst.processing_chain.push(ProcessingStage::Eq(EqConfig::default()));
        assert!(inst.effect_by_id(id).is_some());
        assert_eq!(inst.effect_by_id(id).unwrap().effect_type, EffectType::Delay);
    }

    // --- Toggle/set operation tests ---

    #[test]
    fn toggle_filter_insert_remove() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        assert!(inst.filter().is_none());
        inst.toggle_filter();
        assert!(inst.filter().is_some());
        assert_eq!(inst.filter().unwrap().filter_type, FilterType::Lpf);
        inst.toggle_filter();
        assert!(inst.filter().is_none());
    }

    #[test]
    fn set_filter_replace_insert_remove() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.set_filter(Some(FilterType::Hpf));
        assert_eq!(inst.filter().unwrap().filter_type, FilterType::Hpf);
        inst.set_filter(Some(FilterType::Bpf));
        assert_eq!(inst.filter().unwrap().filter_type, FilterType::Bpf);
        assert_eq!(inst.filters().count(), 1);
        inst.set_filter(None);
        assert!(inst.filter().is_none());
    }

    #[test]
    fn toggle_eq_single_instance_and_position() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        inst.add_effect(EffectType::Delay);
        inst.toggle_eq();
        assert!(inst.has_eq());
        assert_eq!(inst.eq_chain_index(), Some(1)); // after filter at 0
        inst.toggle_eq();
        assert!(!inst.has_eq());
    }

    #[test]
    fn add_remove_effect_chain_integrity() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        assert_eq!(inst.processing_chain.len(), 3);
        assert!(inst.remove_effect(id1));
        assert_eq!(inst.processing_chain.len(), 2);
        assert!(inst.effect_by_id(id2).is_some());
        assert!(inst.filter().is_some());
    }

    // --- move_stage tests ---

    #[test]
    fn move_stage_reorder() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id = inst.add_effect(EffectType::Delay);
        inst.processing_chain.insert(0, ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        assert!(inst.processing_chain[0].is_filter());
        assert!(inst.move_stage(0, 1));
        assert!(inst.processing_chain[0].is_effect());
        assert!(inst.processing_chain[1].is_filter());
        assert!(inst.effect_by_id(id).is_some());
    }

    #[test]
    fn move_stage_boundary_cases() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay);
        assert!(!inst.move_stage(0, 1)); // only 1 element
        assert!(!inst.move_stage(0, -1));
        assert!(!inst.move_stage(5, 1)); // out of bounds
    }

    // --- Navigation table-driven tests ---

    #[test]
    fn nav_empty_chain() {
        let inst = Instrument::new(1, SourceType::Saw);
        let source_rows = inst.source_params.len().max(1);
        let (section, local) = inst.row_info(source_rows);
        assert_eq!(section, InstrumentSection::Processing(0));
        assert_eq!(local, 0);
    }

    #[test]
    fn nav_filter_only() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.toggle_filter();
        let source_rows = inst.source_params.len().max(1);
        let (section, local) = inst.row_info(source_rows);
        assert_eq!(section, InstrumentSection::Processing(0));
        assert_eq!(local, 0);
        let (section2, _) = inst.row_info(source_rows + 2);
        assert_eq!(section2, InstrumentSection::Processing(0));
        let (section3, _) = inst.row_info(source_rows + 3);
        assert_eq!(section3, InstrumentSection::Lfo);
    }

    #[test]
    fn nav_effects_only() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay);
        let source_rows = inst.source_params.len().max(1);
        let delay_rows = 1 + EffectType::Delay.default_params().len();
        let (section, local) = inst.row_info(source_rows);
        assert_eq!(section, InstrumentSection::Processing(0));
        assert_eq!(local, 0);
        let (section2, _) = inst.row_info(source_rows + delay_rows);
        assert_eq!(section2, InstrumentSection::Lfo);
    }

    #[test]
    fn nav_mixed_order_effect_filter_eq() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay);
        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        inst.processing_chain.push(ProcessingStage::Eq(EqConfig::default()));

        let source_rows = inst.source_params.len().max(1);
        let delay_rows = 1 + EffectType::Delay.default_params().len();
        let filter_rows = 3;
        let eq_rows = 1;

        let (s, _) = inst.row_info(source_rows);
        assert_eq!(s, InstrumentSection::Processing(0));
        let (s, _) = inst.row_info(source_rows + delay_rows);
        assert_eq!(s, InstrumentSection::Processing(1));
        let (s, _) = inst.row_info(source_rows + delay_rows + filter_rows);
        assert_eq!(s, InstrumentSection::Processing(2));
        let (s, _) = inst.row_info(source_rows + delay_rows + filter_rows + eq_rows);
        assert_eq!(s, InstrumentSection::Lfo);
    }

    #[test]
    fn nav_different_stage_sizes() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.processing_chain.push(ProcessingStage::Filter(FilterConfig::new(FilterType::Vowel)));
        inst.processing_chain.push(ProcessingStage::Effect(EffectSlot::new(0, EffectType::Vst(0))));

        let source_rows = inst.source_params.len().max(1);
        let vowel_rows = 4;

        let (s, local) = inst.row_info(source_rows + vowel_rows - 1);
        assert_eq!(s, InstrumentSection::Processing(0));
        assert_eq!(local, 3);
        let (s, local) = inst.row_info(source_rows + vowel_rows);
        assert_eq!(s, InstrumentSection::Processing(1));
        assert_eq!(local, 0);
        let (s, _) = inst.row_info(source_rows + vowel_rows + 1);
        assert_eq!(s, InstrumentSection::Lfo);
    }

    #[test]
    fn row_count_with_processing_chain() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let base = inst.total_editable_rows();
        inst.toggle_filter();
        let with_filter = inst.total_editable_rows();
        assert_eq!(with_filter, base - 1 + 3); // removed placeholder(1), added filter(3)
        inst.add_effect(EffectType::Delay);
        let delay_rows = 1 + EffectType::Delay.default_params().len();
        assert_eq!(inst.total_editable_rows(), with_filter + delay_rows);
    }

    #[test]
    fn section_for_row_roundtrips_all() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.toggle_filter();
        inst.add_effect(EffectType::Delay);
        let total = inst.total_editable_rows();
        let mut has_source = false;
        let mut has_processing = false;
        let mut has_lfo = false;
        let mut has_envelope = false;
        for i in 0..total {
            match inst.section_for_row(i) {
                InstrumentSection::Source => has_source = true,
                InstrumentSection::Processing(_) => has_processing = true,
                InstrumentSection::Lfo => has_lfo = true,
                InstrumentSection::Envelope => has_envelope = true,
            }
        }
        assert!(has_source);
        assert!(has_processing);
        assert!(has_lfo);
        assert!(has_envelope);
    }

    // --- Index query tests ---

    #[test]
    fn chain_index_queries() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id = inst.add_effect(EffectType::Delay);
        inst.processing_chain.insert(0, ProcessingStage::Filter(FilterConfig::new(FilterType::Lpf)));
        inst.processing_chain.insert(1, ProcessingStage::Eq(EqConfig::default()));
        assert_eq!(inst.filter_chain_index(), Some(0));
        assert_eq!(inst.eq_chain_index(), Some(1));
        assert_eq!(inst.effect_chain_index(id), Some(2));
    }

    #[test]
    fn recalculate_next_effect_id() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        inst.add_effect(EffectType::Delay);
        inst.add_effect(EffectType::Reverb);
        inst.next_effect_id = 0;
        inst.recalculate_next_effect_id();
        assert_eq!(inst.next_effect_id, 2);
    }
}
