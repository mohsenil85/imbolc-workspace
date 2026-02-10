//! Unified parameter targets for modulation and automation.
//!
//! `ParameterTarget` is the single source of truth for all modulatable parameters.
//! Both LFO modulation and automation lanes reference this enum.

use serde::{Deserialize, Serialize};

use crate::{BusId, EffectId, ParamIndex};

/// Core parameter target - shared between LFO and Automation.
///
/// This enum defines all parameters that can be modulated by an LFO or
/// automated via automation lanes. Using a single enum ensures:
/// - Consistent naming across systems
/// - Single source of truth for modulatable parameters
/// - Easy extension when adding new parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterTarget {
    // === Mixer/Routing ===
    /// Output level (0.0-1.0)
    Level,
    /// Pan position (-1.0 to 1.0)
    Pan,
    /// Send level to a bus (bus_id, 0.0-1.0)
    SendLevel(BusId),

    // === Filter ===
    /// Filter cutoff frequency (20-20000 Hz)
    FilterCutoff,
    /// Filter resonance (0.0-1.0)
    FilterResonance,
    /// Filter bypass toggle (discrete)
    FilterBypass,

    // === Envelope ===
    /// Attack time (0.001-2.0 s)
    Attack,
    /// Decay time (0.001-2.0 s)
    Decay,
    /// Sustain level (0.0-1.0)
    Sustain,
    /// Release time (0.001-5.0 s)
    Release,

    // === Voice Synthesis ===
    /// Pitch modulation (semitones)
    Pitch,
    /// Pulse width for pulse/square oscillators (0.0-1.0)
    PulseWidth,
    /// Detune amount for unison/spread (cents)
    Detune,
    /// FM modulation index
    FmIndex,
    /// Wavetable position (0.0-1.0)
    WavetablePosition,
    /// Formant frequency for vocal/formant synthesis
    FormantFreq,
    /// Hard sync oscillator ratio
    SyncRatio,
    /// Breath pressure for physical models
    Pressure,
    /// Embouchure for wind instruments
    Embouchure,
    /// Grain size for granular synthesis
    GrainSize,
    /// Grain density/rate for granular synthesis
    GrainDensity,
    /// Feedback amount for feedback oscillators
    FbFeedback,
    /// Ring modulation depth
    RingModDepth,
    /// Chaos parameter for chaotic oscillators
    ChaosParam,
    /// Harmonic rolloff for additive synthesis
    AdditiveRolloff,
    /// Membrane tension for drum/percussion models
    MembraneTension,

    // === Sample/TimeStretch ===
    /// Sample playback rate (-2.0 to 2.0)
    SampleRate,
    /// Sample amplitude (0.0-1.0)
    SampleAmp,
    /// Time stretch ratio for granular time stretch
    StretchRatio,
    /// Pitch shift for granular time stretch (semitones)
    PitchShift,

    // === Effects ===
    /// Delay time
    DelayTime,
    /// Delay feedback amount
    DelayFeedback,
    /// Reverb wet/dry mix
    ReverbMix,
    /// Gate rate
    GateRate,
    /// Generic effect parameter (effect_id, param_index)
    EffectParam(EffectId, ParamIndex),
    /// Effect bypass toggle (discrete)
    EffectBypass(EffectId),

    // === EQ ===
    /// EQ band frequency (band_index)
    EqBandFreq(usize),
    /// EQ band gain (band_index)
    EqBandGain(usize),
    /// EQ band Q (band_index)
    EqBandQ(usize),

    // === LFO (meta-modulation) ===
    /// LFO rate (0.1-32.0 Hz)
    LfoRate,
    /// LFO depth (0.0-1.0)
    LfoDepth,

    // === Groove ===
    /// Swing amount (0.0-1.0)
    Swing,
    /// Velocity humanization amount (0.0-1.0)
    HumanizeVelocity,
    /// Timing humanization amount (0.0-1.0)
    HumanizeTiming,
    /// Timing offset in ms (-50.0 to +50.0)
    TimingOffset,

    // === VST ===
    /// VST plugin parameter (param_index)
    VstParam(u32),

    // === Session ===
    /// Time signature (discrete)
    TimeSignature,
}

impl ParameterTarget {
    /// Get a short display name for the target (suitable for compact UI).
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Level => "Level",
            Self::Pan => "Pan",
            Self::SendLevel(_) => "Send",
            Self::FilterCutoff => "FltCt",
            Self::FilterResonance => "FltRs",
            Self::FilterBypass => "FltBp",
            Self::Attack => "Atk",
            Self::Decay => "Dec",
            Self::Sustain => "Sus",
            Self::Release => "Rel",
            Self::Pitch => "Pitch",
            Self::PulseWidth => "PW",
            Self::Detune => "Detune",
            Self::FmIndex => "FMIdx",
            Self::WavetablePosition => "WTPos",
            Self::FormantFreq => "Frmnt",
            Self::SyncRatio => "SyncR",
            Self::Pressure => "Press",
            Self::Embouchure => "Emb",
            Self::GrainSize => "GrnSz",
            Self::GrainDensity => "GrnDns",
            Self::FbFeedback => "FBFbk",
            Self::RingModDepth => "RngDp",
            Self::ChaosParam => "Chaos",
            Self::AdditiveRolloff => "Rllff",
            Self::MembraneTension => "Tensn",
            Self::SampleRate => "SRate",
            Self::SampleAmp => "SAmp",
            Self::StretchRatio => "Strch",
            Self::PitchShift => "PtchSh",
            Self::DelayTime => "DlyTm",
            Self::DelayFeedback => "DlyFb",
            Self::ReverbMix => "RevMx",
            Self::GateRate => "GateRt",
            Self::EffectParam(_, _) => "FX",
            Self::EffectBypass(_) => "FXByp",
            Self::EqBandFreq(_) => "EqFrq",
            Self::EqBandGain(_) => "EqGn",
            Self::EqBandQ(_) => "EqQ",
            Self::LfoRate => "LfoRt",
            Self::LfoDepth => "LfoDp",
            Self::Swing => "Swing",
            Self::HumanizeVelocity => "HumVl",
            Self::HumanizeTiming => "HumTm",
            Self::TimingOffset => "TmOfs",
            Self::VstParam(_) => "VstP",
            Self::TimeSignature => "TSig",
        }
    }

    /// Get a human-readable name for the target.
    pub fn name(&self) -> String {
        match self {
            Self::Level => "Level".to_string(),
            Self::Pan => "Pan".to_string(),
            Self::SendLevel(bus_id) => format!("Send {}", bus_id),
            Self::FilterCutoff => "Filter Cutoff".to_string(),
            Self::FilterResonance => "Filter Resonance".to_string(),
            Self::FilterBypass => "Filter Bypass".to_string(),
            Self::Attack => "Attack".to_string(),
            Self::Decay => "Decay".to_string(),
            Self::Sustain => "Sustain".to_string(),
            Self::Release => "Release".to_string(),
            Self::Pitch => "Pitch".to_string(),
            Self::PulseWidth => "Pulse Width".to_string(),
            Self::Detune => "Detune".to_string(),
            Self::FmIndex => "FM Index".to_string(),
            Self::WavetablePosition => "Wavetable Position".to_string(),
            Self::FormantFreq => "Formant Freq".to_string(),
            Self::SyncRatio => "Sync Ratio".to_string(),
            Self::Pressure => "Pressure".to_string(),
            Self::Embouchure => "Embouchure".to_string(),
            Self::GrainSize => "Grain Size".to_string(),
            Self::GrainDensity => "Grain Density".to_string(),
            Self::FbFeedback => "FB Feedback".to_string(),
            Self::RingModDepth => "Ring Mod Depth".to_string(),
            Self::ChaosParam => "Chaos".to_string(),
            Self::AdditiveRolloff => "Additive Rolloff".to_string(),
            Self::MembraneTension => "Membrane Tension".to_string(),
            Self::SampleRate => "Sample Rate".to_string(),
            Self::SampleAmp => "Sample Amp".to_string(),
            Self::StretchRatio => "Stretch Ratio".to_string(),
            Self::PitchShift => "Pitch Shift".to_string(),
            Self::DelayTime => "Delay Time".to_string(),
            Self::DelayFeedback => "Delay Feedback".to_string(),
            Self::ReverbMix => "Reverb Mix".to_string(),
            Self::GateRate => "Gate Rate".to_string(),
            Self::EffectParam(fx_id, param_idx) => format!("FX{} Param{}", fx_id.get() + 1, param_idx.get() + 1),
            Self::EffectBypass(fx_id) => format!("FX{} Bypass", fx_id.get() + 1),
            Self::EqBandFreq(band) => format!("EQ B{} Freq", band + 1),
            Self::EqBandGain(band) => format!("EQ B{} Gain", band + 1),
            Self::EqBandQ(band) => format!("EQ B{} Q", band + 1),
            Self::LfoRate => "LFO Rate".to_string(),
            Self::LfoDepth => "LFO Depth".to_string(),
            Self::Swing => "Swing".to_string(),
            Self::HumanizeVelocity => "Humanize Velocity".to_string(),
            Self::HumanizeTiming => "Humanize Timing".to_string(),
            Self::TimingOffset => "Timing Offset".to_string(),
            Self::VstParam(idx) => format!("VST P{}", idx),
            Self::TimeSignature => "Time Signature".to_string(),
        }
    }

    /// Get the default min/max range for this parameter.
    pub fn default_range(&self) -> (f32, f32) {
        match self {
            Self::Level | Self::FilterResonance | Self::Sustain | Self::LfoDepth
            | Self::Swing | Self::HumanizeVelocity | Self::HumanizeTiming
            | Self::SampleAmp | Self::PulseWidth | Self::WavetablePosition
            | Self::RingModDepth | Self::GrainDensity | Self::FbFeedback
            | Self::SendLevel(_) | Self::EffectParam(_, _) | Self::VstParam(_) => (0.0, 1.0),

            Self::Pan => (-1.0, 1.0),

            Self::FilterCutoff | Self::EqBandFreq(_) => (20.0, 20000.0),

            Self::Attack | Self::Decay => (0.001, 2.0),
            Self::Release => (0.001, 5.0),

            Self::SampleRate => (-2.0, 2.0),

            Self::LfoRate => (0.1, 32.0),

            Self::TimingOffset => (-50.0, 50.0),

            Self::EqBandGain(_) => (-24.0, 24.0),
            Self::EqBandQ(_) => (0.1, 10.0),

            // Discrete/toggle parameters use 0-1 normalized
            Self::FilterBypass | Self::EffectBypass(_) | Self::TimeSignature => (0.0, 1.0),

            // Voice synthesis params - ranges vary by implementation
            Self::Pitch => (-24.0, 24.0),           // semitones
            Self::Detune => (-100.0, 100.0),        // cents
            Self::FmIndex => (0.0, 20.0),
            Self::FormantFreq => (100.0, 5000.0),
            Self::SyncRatio => (1.0, 8.0),
            Self::Pressure => (0.0, 1.0),
            Self::Embouchure => (0.0, 1.0),
            Self::GrainSize => (0.01, 0.5),
            Self::ChaosParam => (0.0, 1.0),
            Self::AdditiveRolloff => (0.0, 1.0),
            Self::MembraneTension => (0.0, 1.0),
            Self::StretchRatio => (0.25, 4.0),
            Self::PitchShift => (-24.0, 24.0),

            // Effect parameters
            Self::DelayTime => (0.0, 2.0),
            Self::DelayFeedback => (0.0, 1.0),
            Self::ReverbMix => (0.0, 1.0),
            Self::GateRate => (0.1, 32.0),
        }
    }

    /// Get all common targets suitable for LFO modulation.
    /// Returns targets that make sense for per-voice modulation.
    pub fn lfo_targets() -> Vec<ParameterTarget> {
        vec![
            Self::FilterCutoff,
            Self::FilterResonance,
            Self::Level,
            Self::Pitch,
            Self::Pan,
            Self::PulseWidth,
            Self::SampleRate,
            Self::DelayTime,
            Self::DelayFeedback,
            Self::ReverbMix,
            Self::GateRate,
            Self::SendLevel(BusId::new(1)),
            Self::Detune,
            Self::Attack,
            Self::Decay,
            Self::Sustain,
            Self::Release,
            Self::FmIndex,
            Self::WavetablePosition,
            Self::FormantFreq,
            Self::SyncRatio,
            Self::Pressure,
            Self::Embouchure,
            Self::GrainSize,
            Self::GrainDensity,
            Self::FbFeedback,
            Self::RingModDepth,
            Self::ChaosParam,
            Self::AdditiveRolloff,
            Self::MembraneTension,
            Self::StretchRatio,
            Self::PitchShift,
        ]
    }

    /// Cycle to the next LFO target in the standard list.
    pub fn next_lfo_target(&self) -> ParameterTarget {
        let targets = Self::lfo_targets();
        let current_idx = targets.iter().position(|t| t == self).unwrap_or(0);
        let next_idx = (current_idx + 1) % targets.len();
        targets[next_idx]
    }

    /// Parse a short name back to a ParameterTarget.
    /// Note: This only handles simple targets without indices.
    pub fn from_short_name(name: &str) -> Option<ParameterTarget> {
        match name {
            "Level" => Some(Self::Level),
            "Pan" => Some(Self::Pan),
            "FltCt" => Some(Self::FilterCutoff),
            "FltRs" => Some(Self::FilterResonance),
            "FltBp" => Some(Self::FilterBypass),
            "Atk" => Some(Self::Attack),
            "Dec" => Some(Self::Decay),
            "Sus" => Some(Self::Sustain),
            "Rel" => Some(Self::Release),
            "Pitch" => Some(Self::Pitch),
            "PW" => Some(Self::PulseWidth),
            "Detune" => Some(Self::Detune),
            "FMIdx" => Some(Self::FmIndex),
            "WTPos" => Some(Self::WavetablePosition),
            "Frmnt" => Some(Self::FormantFreq),
            "SyncR" => Some(Self::SyncRatio),
            "Press" => Some(Self::Pressure),
            "Emb" => Some(Self::Embouchure),
            "GrnSz" => Some(Self::GrainSize),
            "GrnDns" => Some(Self::GrainDensity),
            "FBFbk" => Some(Self::FbFeedback),
            "RngDp" => Some(Self::RingModDepth),
            "Chaos" => Some(Self::ChaosParam),
            "Rllff" => Some(Self::AdditiveRolloff),
            "Tensn" => Some(Self::MembraneTension),
            "SRate" => Some(Self::SampleRate),
            "SAmp" => Some(Self::SampleAmp),
            "Strch" => Some(Self::StretchRatio),
            "PtchSh" => Some(Self::PitchShift),
            "DlyTm" => Some(Self::DelayTime),
            "DlyFb" => Some(Self::DelayFeedback),
            "RevMx" => Some(Self::ReverbMix),
            "GateRt" => Some(Self::GateRate),
            "LfoRt" => Some(Self::LfoRate),
            "LfoDp" => Some(Self::LfoDepth),
            "Swing" => Some(Self::Swing),
            "HumVl" => Some(Self::HumanizeVelocity),
            "HumTm" => Some(Self::HumanizeTiming),
            "TmOfs" => Some(Self::TimingOffset),
            "TSig" => Some(Self::TimeSignature),
            _ => None,
        }
    }
}

impl Default for ParameterTarget {
    fn default() -> Self {
        Self::FilterCutoff
    }
}
