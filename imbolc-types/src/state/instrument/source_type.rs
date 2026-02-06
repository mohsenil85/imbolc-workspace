use serde::{Serialize, Deserialize};

use crate::{CustomSynthDefId, Param, ParamValue, VstPluginId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    Saw,
    Sin,
    Sqr,
    Tri,
    Noise,
    Pulse,
    SuperSaw,
    Sync,
    Ring,
    FBSin,
    FM,
    PhaseMod,
    Pluck,
    Formant,
    Gendy,
    Chaos,
    Additive,
    Wavetable,
    Granular,
    Bowed,
    Blown,
    Membrane,
    AudioIn,
    BusIn,
    PitchedSampler,
    TimeStretch,
    Kit,
    Custom(CustomSynthDefId),
    Vst(VstPluginId),
}

impl SourceType {
    pub fn name(&self) -> &'static str {
        match self {
            SourceType::Saw => "Saw",
            SourceType::Sin => "Sine",
            SourceType::Sqr => "Square",
            SourceType::Tri => "Triangle",
            SourceType::Noise => "Noise",
            SourceType::Pulse => "Pulse",
            SourceType::SuperSaw => "SuperSaw",
            SourceType::Sync => "Sync",
            SourceType::Ring => "Ring Mod",
            SourceType::FBSin => "FB Sine",
            SourceType::FM => "FM",
            SourceType::PhaseMod => "Phase Mod",
            SourceType::Pluck => "Pluck",
            SourceType::Formant => "Formant",
            SourceType::Gendy => "Gendy",
            SourceType::Chaos => "Chaos",
            SourceType::Additive => "Additive",
            SourceType::Wavetable => "Wavetable",
            SourceType::Granular => "Granular",
            SourceType::Bowed => "Bowed",
            SourceType::Blown => "Blown",
            SourceType::Membrane => "Membrane",
            SourceType::AudioIn => "Audio In",
            SourceType::BusIn => "Bus In",
            SourceType::PitchedSampler => "Pitched Sampler",
            SourceType::TimeStretch => "Time Stretch",
            SourceType::Kit => "Kit",
            SourceType::Custom(_) => "Custom",
            SourceType::Vst(_) => "VST",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            SourceType::Saw => "saw",
            SourceType::Sin => "sin",
            SourceType::Sqr => "sqr",
            SourceType::Tri => "tri",
            SourceType::Noise => "noise",
            SourceType::Pulse => "pulse",
            SourceType::SuperSaw => "supersaw",
            SourceType::Sync => "sync",
            SourceType::Ring => "ring",
            SourceType::FBSin => "fbsin",
            SourceType::FM => "fm",
            SourceType::PhaseMod => "phasemod",
            SourceType::Pluck => "pluck",
            SourceType::Formant => "formant",
            SourceType::Gendy => "gendy",
            SourceType::Chaos => "chaos",
            SourceType::Additive => "additive",
            SourceType::Wavetable => "wavetable",
            SourceType::Granular => "granular",
            SourceType::Bowed => "bowed",
            SourceType::Blown => "blown",
            SourceType::Membrane => "membrane",
            SourceType::AudioIn => "audio_in",
            SourceType::BusIn => "bus_in",
            SourceType::PitchedSampler => "sample",
            SourceType::TimeStretch => "stretch",
            SourceType::Kit => "kit",
            SourceType::Custom(_) => "custom",
            SourceType::Vst(_) => "vst",
        }
    }

    /// Get the SuperCollider synthdef name (static for built-ins)
    pub fn synth_def_name(&self) -> &'static str {
        match self {
            SourceType::Saw => "imbolc_saw",
            SourceType::Sin => "imbolc_sin",
            SourceType::Sqr => "imbolc_sqr",
            SourceType::Tri => "imbolc_tri",
            SourceType::Noise => "imbolc_noise",
            SourceType::Pulse => "imbolc_pulse",
            SourceType::SuperSaw => "imbolc_supersaw",
            SourceType::Sync => "imbolc_sync",
            SourceType::Ring => "imbolc_ring",
            SourceType::FBSin => "imbolc_fbsin",
            SourceType::FM => "imbolc_fm",
            SourceType::PhaseMod => "imbolc_phasemod",
            SourceType::Pluck => "imbolc_pluck",
            SourceType::Formant => "imbolc_formant",
            SourceType::Gendy => "imbolc_gendy",
            SourceType::Chaos => "imbolc_chaos",
            SourceType::Additive => "imbolc_additive",
            SourceType::Wavetable => "imbolc_wavetable",
            SourceType::Granular => "imbolc_granular",
            SourceType::Bowed => "imbolc_bowed",
            SourceType::Blown => "imbolc_blown",
            SourceType::Membrane => "imbolc_membrane",
            SourceType::AudioIn => "imbolc_audio_in",
            SourceType::BusIn => "imbolc_bus_in",
            SourceType::PitchedSampler => "imbolc_sampler",
            SourceType::TimeStretch => "imbolc_timestretch",
            SourceType::Kit => "imbolc_sampler_oneshot",
            SourceType::Custom(_) => "imbolc_saw", // Fallback, use synth_def_name_with_registry instead
            SourceType::Vst(_) => "imbolc_vst_instrument",
        }
    }

    pub fn is_audio_input(&self) -> bool {
        matches!(self, SourceType::AudioIn)
    }

    pub fn is_sample(&self) -> bool {
        matches!(self, SourceType::PitchedSampler)
    }

    pub fn is_kit(&self) -> bool {
        matches!(self, SourceType::Kit)
    }

    pub fn is_time_stretch(&self) -> bool {
        matches!(self, SourceType::TimeStretch)
    }

    pub fn is_bus_in(&self) -> bool {
        matches!(self, SourceType::BusIn)
    }

    #[allow(dead_code)]
    pub fn is_custom(&self) -> bool {
        matches!(self, SourceType::Custom(_))
    }

    #[allow(dead_code)]
    pub fn custom_id(&self) -> Option<CustomSynthDefId> {
        match self {
            SourceType::Custom(id) => Some(*id),
            _ => None,
        }
    }

    pub fn is_vst(&self) -> bool {
        matches!(self, SourceType::Vst(_))
    }

    #[allow(dead_code)]
    pub fn vst_id(&self) -> Option<VstPluginId> {
        match self {
            SourceType::Vst(id) => Some(*id),
            _ => None,
        }
    }

    /// Built-in source types (excluding custom)
    pub fn all() -> Vec<SourceType> {
        vec![
            SourceType::Saw, SourceType::Sin, SourceType::Sqr, SourceType::Tri,
            SourceType::Noise, SourceType::Pulse, SourceType::SuperSaw, SourceType::Sync,
            SourceType::Ring, SourceType::FBSin, SourceType::FM, SourceType::PhaseMod,
            SourceType::Pluck, SourceType::Formant, SourceType::Gendy, SourceType::Chaos,
            SourceType::Additive, SourceType::Wavetable, SourceType::Granular,
            SourceType::Bowed, SourceType::Blown, SourceType::Membrane,
            SourceType::AudioIn, SourceType::BusIn, SourceType::PitchedSampler, SourceType::TimeStretch, SourceType::Kit,
        ]
    }

    pub fn default_params(&self) -> Vec<Param> {
        match self {
            SourceType::Noise => vec![
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "color".to_string(), value: ParamValue::Int(0), min: 0.0, max: 4.0 },
                Param { name: "density".to_string(), value: ParamValue::Float(20.0), min: 1.0, max: 100.0 },
            ],
            SourceType::Pulse => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "width".to_string(), value: ParamValue::Float(0.5), min: 0.01, max: 0.99 },
            ],
            SourceType::SuperSaw => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "detune".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            SourceType::Sync => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "sync_ratio".to_string(), value: ParamValue::Float(2.0), min: 1.0, max: 8.0 },
            ],
            SourceType::Ring => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mod_ratio".to_string(), value: ParamValue::Float(2.0), min: 0.1, max: 16.0 },
                Param { name: "mod_depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            SourceType::FBSin => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "feedback".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 3.0 },
            ],
            SourceType::FM => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "ratio".to_string(), value: ParamValue::Float(2.0), min: 0.25, max: 16.0 },
                Param { name: "index".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 20.0 },
            ],
            SourceType::PhaseMod => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "ratio".to_string(), value: ParamValue::Float(2.0), min: 0.25, max: 16.0 },
                Param { name: "index".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 10.0 },
            ],
            SourceType::Pluck => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "decay".to_string(), value: ParamValue::Float(2.0), min: 0.1, max: 10.0 },
                Param { name: "coef".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
            ],
            SourceType::Formant => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "formant".to_string(), value: ParamValue::Float(800.0), min: 100.0, max: 5000.0 },
                Param { name: "bw".to_string(), value: ParamValue::Float(200.0), min: 10.0, max: 1000.0 },
            ],
            SourceType::Gendy => vec![
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "ampdist".to_string(), value: ParamValue::Int(1), min: 0.0, max: 6.0 },
                Param { name: "durdist".to_string(), value: ParamValue::Int(1), min: 0.0, max: 6.0 },
                Param { name: "minfreq".to_string(), value: ParamValue::Float(100.0), min: 20.0, max: 1000.0 },
                Param { name: "maxfreq".to_string(), value: ParamValue::Float(1000.0), min: 100.0, max: 10000.0 },
            ],
            SourceType::Chaos => vec![
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "model".to_string(), value: ParamValue::Int(0), min: 0.0, max: 1.0 },
                Param { name: "chaos_freq".to_string(), value: ParamValue::Float(8000.0), min: 20.0, max: 20000.0 },
                Param { name: "chaos_param".to_string(), value: ParamValue::Float(1.3), min: 0.0, max: 2.0 },
            ],
            SourceType::Additive => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "harmonics".to_string(), value: ParamValue::Int(8), min: 1.0, max: 32.0 },
                Param { name: "rolloff".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 2.0 },
            ],
            SourceType::Wavetable => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "position".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
            ],
            SourceType::Granular => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "grain_size".to_string(), value: ParamValue::Float(0.05), min: 0.001, max: 0.5 },
                Param { name: "density".to_string(), value: ParamValue::Float(20.0), min: 1.0, max: 100.0 },
                Param { name: "spread".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
                Param { name: "pitch_rnd".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
            ],
            SourceType::Bowed => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "pressure".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "bow_pos".to_string(), value: ParamValue::Float(0.12), min: 0.01, max: 0.5 },
            ],
            SourceType::Blown => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "pressure".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "embouchure".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            SourceType::Membrane => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "tension".to_string(), value: ParamValue::Float(0.05), min: 0.01, max: 0.2 },
                Param { name: "loss".to_string(), value: ParamValue::Float(0.99), min: 0.9, max: 1.0 },
            ],
            SourceType::AudioIn => vec![
                Param { name: "gain".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 4.0 },
                Param { name: "channel".to_string(), value: ParamValue::Int(0), min: 0.0, max: 7.0 },
                Param { name: "test_tone".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
                Param { name: "test_freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 2000.0 },
            ],
            SourceType::BusIn => vec![
                Param { name: "bus".to_string(), value: ParamValue::Int(1), min: 1.0, max: 8.0 },
                Param { name: "gain".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 4.0 },
            ],
            SourceType::PitchedSampler => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(1.0), min: -2.0, max: 2.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.8), min: 0.0, max: 1.0 },
                Param { name: "loop".to_string(), value: ParamValue::Bool(false), min: 0.0, max: 1.0 },
            ],
            SourceType::TimeStretch => vec![
                Param { name: "stretch".to_string(), value: ParamValue::Float(1.0), min: 0.25, max: 4.0 },
                Param { name: "pitch".to_string(), value: ParamValue::Float(0.0), min: -24.0, max: 24.0 },
                Param { name: "grain_size".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 0.5 },
                Param { name: "overlap".to_string(), value: ParamValue::Int(4), min: 1.0, max: 8.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.8), min: 0.0, max: 1.0 },
            ],
            SourceType::Kit => vec![],
            SourceType::Custom(_) => vec![],
            SourceType::Vst(_) => vec![],
            _ => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 20000.0 },
                Param { name: "amp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
        }
    }
}

// Note: The following methods stay in imbolc-core because they require registry lookups:
// - display_name(&self, registry: &CustomSynthDefRegistry)
// - display_name_vst(&self, custom_registry, vst_registry)
// - short_name_with_registry(&self, registry)
// - short_name_vst(&self, custom_registry, vst_registry)
// - synth_def_name_with_registry(&self, registry)
// - default_params_with_registry(&self, registry)
// - all_with_custom(registry)
