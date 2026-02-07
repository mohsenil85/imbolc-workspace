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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerSend {
    pub bus_id: u8,
    pub level: f32,
    pub enabled: bool,
}

impl MixerSend {
    pub fn new(bus_id: u8) -> Self {
        Self {
            bus_id,
            level: 0.0,
            enabled: false,
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
        }
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

/// Which section of an instrument a given editing row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentSection {
    Source,
    Filter,
    Effects,
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
    filter: &Option<FilterConfig>,
    effects: &[EffectSlot],
) -> usize {
    let sample_row = if source.is_sample() || source.is_time_stretch() {
        1
    } else {
        0
    };
    let source_rows = sample_row + source_params.len().max(1);
    let filter_rows = if let Some(ref f) = filter {
        3 + f.extra_params.len()
    } else {
        1
    };
    let effect_rows = if effects.is_empty() {
        1
    } else {
        effects.iter().map(|e| 1 + e.params.len()).sum()
    };
    let lfo_rows = 4;
    let env_rows = if source.is_vst() { 0 } else { 4 };
    source_rows + filter_rows + effect_rows + lfo_rows + env_rows
}

/// Which section a given row belongs to.
///
/// Free function variant for use with decomposed fields.
pub fn instrument_section_for_row(
    row: usize,
    source: SourceType,
    source_params: &[Param],
    filter: &Option<FilterConfig>,
    effects: &[EffectSlot],
) -> InstrumentSection {
    let sample_row = if source.is_sample() || source.is_time_stretch() {
        1
    } else {
        0
    };
    let source_rows = sample_row + source_params.len().max(1);
    let filter_rows = if let Some(ref f) = filter {
        3 + f.extra_params.len()
    } else {
        1
    };
    let effect_rows = if effects.is_empty() {
        1
    } else {
        effects.iter().map(|e| 1 + e.params.len()).sum()
    };
    let lfo_rows = 4;

    if row < source_rows {
        InstrumentSection::Source
    } else if row < source_rows + filter_rows {
        InstrumentSection::Filter
    } else if row < source_rows + filter_rows + effect_rows {
        InstrumentSection::Effects
    } else if row < source_rows + filter_rows + effect_rows + lfo_rows {
        InstrumentSection::Lfo
    } else {
        InstrumentSection::Envelope
    }
}

/// Get section and local index for a given row.
///
/// Free function variant for use with decomposed fields.
pub fn instrument_row_info(
    row: usize,
    source: SourceType,
    source_params: &[Param],
    filter: &Option<FilterConfig>,
    effects: &[EffectSlot],
) -> (InstrumentSection, usize) {
    let sample_row = if source.is_sample() || source.is_time_stretch() {
        1
    } else {
        0
    };
    let source_rows = sample_row + source_params.len().max(1);
    let filter_rows = if let Some(ref f) = filter {
        3 + f.extra_params.len()
    } else {
        1
    };
    let effect_rows = if effects.is_empty() {
        1
    } else {
        effects.iter().map(|e| 1 + e.params.len()).sum()
    };
    let lfo_rows = 4;

    if row < source_rows {
        (InstrumentSection::Source, row)
    } else if row < source_rows + filter_rows {
        (InstrumentSection::Filter, row - source_rows)
    } else if row < source_rows + filter_rows + effect_rows {
        (InstrumentSection::Effects, row - source_rows - filter_rows)
    } else if row < source_rows + filter_rows + effect_rows + lfo_rows {
        (
            InstrumentSection::Lfo,
            row - source_rows - filter_rows - effect_rows,
        )
    } else {
        (
            InstrumentSection::Envelope,
            row - source_rows - filter_rows - effect_rows - lfo_rows,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    pub source: SourceType,
    pub source_params: Vec<Param>,
    pub filter: Option<FilterConfig>,
    pub eq: Option<EqConfig>,
    pub effects: Vec<EffectSlot>,
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
            filter: None,
            eq: None,
            effects: Vec::new(),
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
            next_effect_id: 0,
            groove: GrooveConfig::default(),
        }
    }

    /// Add an effect and return its stable EffectId
    pub fn add_effect(&mut self, effect_type: EffectType) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        self.effects.push(EffectSlot::new(id, effect_type));
        id
    }

    /// Find an effect by its stable EffectId
    pub fn effect_by_id(&self, id: EffectId) -> Option<&EffectSlot> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Find a mutable effect by its stable EffectId
    pub fn effect_by_id_mut(&mut self, id: EffectId) -> Option<&mut EffectSlot> {
        self.effects.iter_mut().find(|e| e.id == id)
    }

    /// Get the position of an effect in the effects chain by EffectId
    pub fn effect_position(&self, id: EffectId) -> Option<usize> {
        self.effects.iter().position(|e| e.id == id)
    }

    /// Remove an effect by its EffectId, returns true if removed
    pub fn remove_effect(&mut self, id: EffectId) -> bool {
        if let Some(pos) = self.effect_position(id) {
            self.effects.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move an effect up or down by its EffectId
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

    /// Recalculate next_effect_id from existing effects (used after loading)
    pub fn recalculate_next_effect_id(&mut self) {
        self.next_effect_id = self
            .effects
            .iter()
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
        instrument_row_count(self.source, &self.source_params, &self.filter, &self.effects)
    }

    /// Which section a given row belongs to.
    pub fn section_for_row(&self, row: usize) -> InstrumentSection {
        instrument_section_for_row(
            row,
            self.source,
            &self.source_params,
            &self.filter,
            &self.effects,
        )
    }

    /// Get section and local index for a given row.
    pub fn row_info(&self, row: usize) -> (InstrumentSection, usize) {
        instrument_row_info(
            row,
            self.source,
            &self.source_params,
            &self.filter,
            &self.effects,
        )
    }

    /// Decode a flat cursor position over the effects chain into (EffectId, Option<param_index>).
    /// Returns None if cursor is out of range. None param_index means the effect header row.
    pub fn decode_effect_cursor(&self, cursor: usize) -> Option<(EffectId, Option<usize>)> {
        let mut pos = 0;
        for effect in &self.effects {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_count_basic() {
        let inst = Instrument::new(1, SourceType::Saw);
        let count = inst.total_editable_rows();
        // Saw: no sample row, default params + filter(disabled=1) + effects(empty=1) + lfo(4) + env(4)
        let expected = inst.source_params.len().max(1) + 1 + 1 + 4 + 4;
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
        // First row after source section should be Filter with local_idx 0
        let (section, local_idx) = inst.row_info(source_rows);
        assert_eq!(section, InstrumentSection::Filter);
        assert_eq!(local_idx, 0);
    }

    #[test]
    fn row_info_roundtrips_all_sections() {
        let inst = Instrument::new(1, SourceType::Saw);
        let total = inst.total_editable_rows();
        // Every row should resolve to a valid section
        let mut saw_sections = vec![];
        for i in 0..total {
            saw_sections.push(inst.section_for_row(i));
        }
        assert!(saw_sections.contains(&InstrumentSection::Source));
        assert!(saw_sections.contains(&InstrumentSection::Lfo));
        assert!(saw_sections.contains(&InstrumentSection::Envelope));
    }

    #[test]
    fn vst_has_no_envelope_rows() {
        // SourceType::Vst(0) is a VST instrument
        let inst = Instrument::new(1, SourceType::Vst(0));
        let total = inst.total_editable_rows();
        for i in 0..total {
            assert_ne!(inst.section_for_row(i), InstrumentSection::Envelope);
        }
    }

    #[test]
    fn decode_effect_cursor_empty() {
        let inst = Instrument::new(1, SourceType::Saw);
        assert!(inst.effects.is_empty());
        assert_eq!(inst.decode_effect_cursor(0), None);
    }

    #[test]
    fn decode_effect_cursor_with_effects() {
        let mut inst = Instrument::new(1, SourceType::Saw);
        let id1 = inst.add_effect(EffectType::Delay);
        let id2 = inst.add_effect(EffectType::Reverb);
        // Effect 1 header at pos 0
        assert_eq!(inst.decode_effect_cursor(0), Some((id1, None)));
        // Effect 1 params at pos 1..
        let params1 = inst.effects[0].params.len();
        // Effect 2 header at pos 1+params1
        assert_eq!(inst.decode_effect_cursor(1 + params1), Some((id2, None)));
    }

    #[test]
    fn timestretch_has_sampler_config_and_correct_params() {
        let inst = Instrument::new(1, SourceType::TimeStretch);
        // TimeStretch instruments should have a sampler config
        assert!(inst.sampler_config.is_some());
        // Should have the default TimeStretch params
        let param_names: Vec<&str> = inst.source_params.iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"stretch"));
        assert!(param_names.contains(&"pitch"));
        assert!(param_names.contains(&"grain_size"));
        assert!(param_names.contains(&"overlap"));
        assert!(param_names.contains(&"amp"));
        // Should not have PitchedSampler's rate param
        assert!(!param_names.contains(&"rate"));
    }

    #[test]
    fn timestretch_has_sample_row_in_count() {
        let inst = Instrument::new(1, SourceType::TimeStretch);
        let count = inst.total_editable_rows();
        // TimeStretch: 1 sample row + params + filter(disabled=1) + effects(empty=1) + lfo(4) + env(4)
        let expected = 1 + inst.source_params.len().max(1) + 1 + 1 + 4 + 4;
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
        // Move id2 backward (up)
        inst.move_effect(id2, -1);
        assert_eq!(inst.effect_position(id2), Some(0));
        assert_eq!(inst.effect_position(id1), Some(1));
    }
}
