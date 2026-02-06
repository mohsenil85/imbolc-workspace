use serde::{Serialize, Deserialize};

use super::ModulatedParam;
use crate::{Param, ParamValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Lpf,
    Hpf,
    Bpf,
    Notch,
    Comb,
    Allpass,
    Vowel,
    ResDrive,
}

impl FilterType {
    pub fn name(&self) -> &'static str {
        match self {
            FilterType::Lpf => "Low-Pass",
            FilterType::Hpf => "High-Pass",
            FilterType::Bpf => "Band-Pass",
            FilterType::Notch => "Notch",
            FilterType::Comb => "Comb",
            FilterType::Allpass => "Allpass",
            FilterType::Vowel => "Vowel",
            FilterType::ResDrive => "ResDrive",
        }
    }

    pub fn synth_def_name(&self) -> &'static str {
        match self {
            FilterType::Lpf => "imbolc_lpf",
            FilterType::Hpf => "imbolc_hpf",
            FilterType::Bpf => "imbolc_bpf",
            FilterType::Notch => "imbolc_notch",
            FilterType::Comb => "imbolc_comb",
            FilterType::Allpass => "imbolc_allpass",
            FilterType::Vowel => "imbolc_vowel",
            FilterType::ResDrive => "imbolc_resdrive",
        }
    }

    pub fn synth_def_name_mono(&self) -> &'static str {
        match self {
            FilterType::Lpf => "imbolc_lpf_mono",
            FilterType::Hpf => "imbolc_hpf_mono",
            FilterType::Bpf => "imbolc_bpf_mono",
            FilterType::Notch => "imbolc_notch_mono",
            FilterType::Comb => "imbolc_comb_mono",
            FilterType::Allpass => "imbolc_allpass_mono",
            FilterType::Vowel => "imbolc_vowel_mono",
            FilterType::ResDrive => "imbolc_resdrive_mono",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<FilterType> {
        vec![
            FilterType::Lpf, FilterType::Hpf, FilterType::Bpf,
            FilterType::Notch, FilterType::Comb, FilterType::Allpass,
            FilterType::Vowel, FilterType::ResDrive,
        ]
    }

    pub fn default_extra_params(&self) -> Vec<Param> {
        match self {
            FilterType::Vowel => vec![
                Param { name: "shape".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
            ],
            FilterType::ResDrive => vec![
                Param { name: "drive".to_string(), value: ParamValue::Float(1.0), min: 1.0, max: 8.0 },
            ],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub filter_type: FilterType,
    pub cutoff: ModulatedParam,
    pub resonance: ModulatedParam,
    pub extra_params: Vec<Param>,
    /// Whether the filter is enabled (bypassed when false).
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl FilterConfig {
    pub fn new(filter_type: FilterType) -> Self {
        Self {
            extra_params: filter_type.default_extra_params(),
            filter_type,
            cutoff: ModulatedParam { value: 1000.0, min: 20.0, max: 20000.0, mod_source: None },
            resonance: ModulatedParam { value: 0.5, min: 0.0, max: 1.0, mod_source: None },
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqBandType {
    LowShelf,
    Peaking,
    HighShelf,
}

impl EqBandType {
    pub fn name(&self) -> &'static str {
        match self {
            EqBandType::LowShelf => "LS",
            EqBandType::Peaking => "PK",
            EqBandType::HighShelf => "HS",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub band_type: EqBandType,
    pub freq: f32,
    pub gain: f32,
    pub q: f32,
    pub enabled: bool,
}

pub const EQ_BAND_COUNT: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqConfig {
    pub bands: [EqBand; EQ_BAND_COUNT],
    pub enabled: bool,
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            bands: [
                EqBand { band_type: EqBandType::LowShelf, freq: 40.0,    gain: 0.0, q: 0.7, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 80.0,    gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 160.0,   gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 320.0,   gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 640.0,   gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 1200.0,  gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 2500.0,  gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 5000.0,  gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 8000.0,  gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 12000.0, gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::Peaking,  freq: 16000.0, gain: 0.0, q: 1.0, enabled: true },
                EqBand { band_type: EqBandType::HighShelf, freq: 18000.0, gain: 0.0, q: 0.7, enabled: true },
            ],
            enabled: true,
        }
    }
}
