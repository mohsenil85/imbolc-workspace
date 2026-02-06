use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoShape {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl LfoShape {
    pub fn name(&self) -> &'static str {
        match self {
            LfoShape::Sine => "Sine",
            LfoShape::Square => "Square",
            LfoShape::Saw => "Saw",
            LfoShape::Triangle => "Triangle",
        }
    }

    pub fn index(&self) -> i32 {
        match self {
            LfoShape::Sine => 0,
            LfoShape::Square => 1,
            LfoShape::Saw => 2,
            LfoShape::Triangle => 3,
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<LfoShape> {
        vec![LfoShape::Sine, LfoShape::Square, LfoShape::Saw, LfoShape::Triangle]
    }

    pub fn next(&self) -> LfoShape {
        match self {
            LfoShape::Sine => LfoShape::Square,
            LfoShape::Square => LfoShape::Saw,
            LfoShape::Saw => LfoShape::Triangle,
            LfoShape::Triangle => LfoShape::Sine,
        }
    }

    pub fn from_name(name: &str) -> Option<LfoShape> {
        match name {
            "Sine" => Some(LfoShape::Sine),
            "Square" => Some(LfoShape::Square),
            "Saw" => Some(LfoShape::Saw),
            "Triangle" => Some(LfoShape::Triangle),
            _ => None,
        }
    }
}

// All LFO targets are wired: each target has a corresponding *_mod_in param
// in the relevant SynthDef, connected via routing.rs (routing-level targets)
// or voices.rs (voice-level targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoTarget {
    FilterCutoff,
    FilterResonance,
    Amplitude,
    Pitch,
    Pan,
    PulseWidth,
    SampleRate,
    DelayTime,
    DelayFeedback,
    ReverbMix,
    GateRate,
    SendLevel,
    Detune,
    Attack,
    Release,
    FmIndex,
    WavetablePosition,
    FormantFreq,
    SyncRatio,
    Pressure,
    Embouchure,
    GrainSize,
    GrainDensity,
    FbFeedback,
    RingModDepth,
    ChaosParam,
    AdditiveRolloff,
    MembraneTension,
    Decay,
    Sustain,
    StretchRatio,
    PitchShift,
}

impl LfoTarget {
    pub fn name(&self) -> &'static str {
        match self {
            LfoTarget::FilterCutoff => "Flt Cut",
            LfoTarget::FilterResonance => "Flt Res",
            LfoTarget::Amplitude => "Amp",
            LfoTarget::Pitch => "Pitch",
            LfoTarget::Pan => "Pan",
            LfoTarget::PulseWidth => "PW",
            LfoTarget::SampleRate => "SmpRate",
            LfoTarget::DelayTime => "DlyTime",
            LfoTarget::DelayFeedback => "DlyFdbk",
            LfoTarget::ReverbMix => "RevMix",
            LfoTarget::GateRate => "GateRt",
            LfoTarget::SendLevel => "Send",
            LfoTarget::Detune => "Detune",
            LfoTarget::Attack => "Attack",
            LfoTarget::Release => "Release",
            LfoTarget::FmIndex => "FM Idx",
            LfoTarget::WavetablePosition => "WTPos",
            LfoTarget::FormantFreq => "Frmnt",
            LfoTarget::SyncRatio => "SyncR",
            LfoTarget::Pressure => "Press",
            LfoTarget::Embouchure => "Emb",
            LfoTarget::GrainSize => "GrnSz",
            LfoTarget::GrainDensity => "GrnDns",
            LfoTarget::FbFeedback => "FBFbk",
            LfoTarget::RingModDepth => "RngDp",
            LfoTarget::ChaosParam => "Chaos",
            LfoTarget::AdditiveRolloff => "Rllff",
            LfoTarget::MembraneTension => "Tensn",
            LfoTarget::Decay => "Decay",
            LfoTarget::Sustain => "Sustn",
            LfoTarget::StretchRatio => "Strch",
            LfoTarget::PitchShift => "PtchSh",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<LfoTarget> {
        vec![
            LfoTarget::FilterCutoff,
            LfoTarget::FilterResonance,
            LfoTarget::Amplitude,
            LfoTarget::Pitch,
            LfoTarget::Pan,
            LfoTarget::PulseWidth,
            LfoTarget::SampleRate,
            LfoTarget::DelayTime,
            LfoTarget::DelayFeedback,
            LfoTarget::ReverbMix,
            LfoTarget::GateRate,
            LfoTarget::SendLevel,
            LfoTarget::Detune,
            LfoTarget::Attack,
            LfoTarget::Release,
            LfoTarget::FmIndex,
            LfoTarget::WavetablePosition,
            LfoTarget::FormantFreq,
            LfoTarget::SyncRatio,
            LfoTarget::Pressure,
            LfoTarget::Embouchure,
            LfoTarget::GrainSize,
            LfoTarget::GrainDensity,
            LfoTarget::FbFeedback,
            LfoTarget::RingModDepth,
            LfoTarget::ChaosParam,
            LfoTarget::AdditiveRolloff,
            LfoTarget::MembraneTension,
            LfoTarget::Decay,
            LfoTarget::Sustain,
            LfoTarget::StretchRatio,
            LfoTarget::PitchShift,
        ]
    }

    pub fn next(&self) -> LfoTarget {
        match self {
            LfoTarget::FilterCutoff => LfoTarget::FilterResonance,
            LfoTarget::FilterResonance => LfoTarget::Amplitude,
            LfoTarget::Amplitude => LfoTarget::Pitch,
            LfoTarget::Pitch => LfoTarget::Pan,
            LfoTarget::Pan => LfoTarget::PulseWidth,
            LfoTarget::PulseWidth => LfoTarget::SampleRate,
            LfoTarget::SampleRate => LfoTarget::DelayTime,
            LfoTarget::DelayTime => LfoTarget::DelayFeedback,
            LfoTarget::DelayFeedback => LfoTarget::ReverbMix,
            LfoTarget::ReverbMix => LfoTarget::GateRate,
            LfoTarget::GateRate => LfoTarget::SendLevel,
            LfoTarget::SendLevel => LfoTarget::Detune,
            LfoTarget::Detune => LfoTarget::Attack,
            LfoTarget::Attack => LfoTarget::Release,
            LfoTarget::Release => LfoTarget::FmIndex,
            LfoTarget::FmIndex => LfoTarget::WavetablePosition,
            LfoTarget::WavetablePosition => LfoTarget::FormantFreq,
            LfoTarget::FormantFreq => LfoTarget::SyncRatio,
            LfoTarget::SyncRatio => LfoTarget::Pressure,
            LfoTarget::Pressure => LfoTarget::Embouchure,
            LfoTarget::Embouchure => LfoTarget::GrainSize,
            LfoTarget::GrainSize => LfoTarget::GrainDensity,
            LfoTarget::GrainDensity => LfoTarget::FbFeedback,
            LfoTarget::FbFeedback => LfoTarget::RingModDepth,
            LfoTarget::RingModDepth => LfoTarget::ChaosParam,
            LfoTarget::ChaosParam => LfoTarget::AdditiveRolloff,
            LfoTarget::AdditiveRolloff => LfoTarget::MembraneTension,
            LfoTarget::MembraneTension => LfoTarget::Decay,
            LfoTarget::Decay => LfoTarget::Sustain,
            LfoTarget::Sustain => LfoTarget::StretchRatio,
            LfoTarget::StretchRatio => LfoTarget::PitchShift,
            LfoTarget::PitchShift => LfoTarget::FilterCutoff,
        }
    }

    pub fn from_name(name: &str) -> Option<LfoTarget> {
        match name {
            "Flt Cut" => Some(LfoTarget::FilterCutoff),
            "Flt Res" => Some(LfoTarget::FilterResonance),
            "Amp" => Some(LfoTarget::Amplitude),
            "Pitch" => Some(LfoTarget::Pitch),
            "Pan" => Some(LfoTarget::Pan),
            "PW" => Some(LfoTarget::PulseWidth),
            "SmpRate" => Some(LfoTarget::SampleRate),
            "DlyTime" => Some(LfoTarget::DelayTime),
            "DlyFdbk" => Some(LfoTarget::DelayFeedback),
            "RevMix" => Some(LfoTarget::ReverbMix),
            "GateRt" => Some(LfoTarget::GateRate),
            "Send" => Some(LfoTarget::SendLevel),
            "Detune" => Some(LfoTarget::Detune),
            "Attack" => Some(LfoTarget::Attack),
            "Release" => Some(LfoTarget::Release),
            "FM Idx" => Some(LfoTarget::FmIndex),
            "WTPos" => Some(LfoTarget::WavetablePosition),
            "Frmnt" => Some(LfoTarget::FormantFreq),
            "SyncR" => Some(LfoTarget::SyncRatio),
            "Press" => Some(LfoTarget::Pressure),
            "Emb" => Some(LfoTarget::Embouchure),
            "GrnSz" => Some(LfoTarget::GrainSize),
            "GrnDns" => Some(LfoTarget::GrainDensity),
            "FBFbk" => Some(LfoTarget::FbFeedback),
            "RngDp" => Some(LfoTarget::RingModDepth),
            "Chaos" => Some(LfoTarget::ChaosParam),
            "Rllff" => Some(LfoTarget::AdditiveRolloff),
            "Tensn" => Some(LfoTarget::MembraneTension),
            "Decay" => Some(LfoTarget::Decay),
            "Sustn" => Some(LfoTarget::Sustain),
            "Strch" => Some(LfoTarget::StretchRatio),
            "PtchSh" => Some(LfoTarget::PitchShift),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfoConfig {
    pub enabled: bool,
    pub rate: f32,
    pub depth: f32,
    pub shape: LfoShape,
    pub target: LfoTarget,
}

impl Default for LfoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate: 2.0,
            depth: 0.5,
            shape: LfoShape::Sine,
            target: LfoTarget::FilterCutoff,
        }
    }
}
