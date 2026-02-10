use serde::{Serialize, Deserialize};

use crate::{CustomSynthDefId, Param, ParamValue, VstPluginId};
use super::envelope::EnvConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    // Basic Oscillators
    Saw,
    Sin,
    Sqr,
    Tri,
    Noise,
    Pulse,
    SuperSaw,
    Sync,
    // Modulation
    Ring,
    FBSin,
    FM,
    PhaseMod,
    FMBell,
    FMBrass,
    // Physical Models
    Pluck,
    Formant,
    Bowed,
    Blown,
    Membrane,
    // Mallet Percussion
    Marimba,
    Vibes,
    Kalimba,
    SteelDrum,
    TubularBell,
    Glockenspiel,
    // Plucked Strings
    Guitar,
    BassGuitar,
    Harp,
    Koto,
    // Drums
    Kick,
    Snare,
    HihatClosed,
    HihatOpen,
    Clap,
    Cowbell,
    Rim,
    Tom,
    Clave,
    Conga,
    // Classic Synths
    Choir,
    EPiano,
    Organ,
    BrassStab,
    Strings,
    Acid,
    // Experimental
    Gendy,
    Chaos,
    // Synthesis
    Additive,
    Wavetable,
    Granular,
    // Routing
    AudioIn,
    BusIn,
    // Samplers
    PitchedSampler,
    TimeStretch,
    Kit,
    // External
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
            SourceType::FMBell => "FM Bell",
            SourceType::FMBrass => "FM Brass",
            SourceType::Pluck => "Pluck",
            SourceType::Formant => "Formant",
            SourceType::Bowed => "Bowed",
            SourceType::Blown => "Blown",
            SourceType::Membrane => "Membrane",
            SourceType::Marimba => "Marimba",
            SourceType::Vibes => "Vibraphone",
            SourceType::Kalimba => "Kalimba",
            SourceType::SteelDrum => "Steel Drum",
            SourceType::TubularBell => "Tubular Bell",
            SourceType::Glockenspiel => "Glockenspiel",
            SourceType::Guitar => "Guitar",
            SourceType::BassGuitar => "Bass Guitar",
            SourceType::Harp => "Harp",
            SourceType::Koto => "Koto",
            SourceType::Kick => "Kick",
            SourceType::Snare => "Snare",
            SourceType::HihatClosed => "Hi-Hat Closed",
            SourceType::HihatOpen => "Hi-Hat Open",
            SourceType::Clap => "Clap",
            SourceType::Cowbell => "Cowbell",
            SourceType::Rim => "Rim",
            SourceType::Tom => "Tom",
            SourceType::Clave => "Clave",
            SourceType::Conga => "Conga",
            SourceType::Choir => "Choir",
            SourceType::EPiano => "Electric Piano",
            SourceType::Organ => "Organ",
            SourceType::BrassStab => "Brass Stab",
            SourceType::Strings => "Strings",
            SourceType::Acid => "Acid",
            SourceType::Gendy => "Gendy",
            SourceType::Chaos => "Chaos",
            SourceType::Additive => "Additive",
            SourceType::Wavetable => "Wavetable",
            SourceType::Granular => "Granular",
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
            SourceType::FMBell => "fmbell",
            SourceType::FMBrass => "fmbrass",
            SourceType::Pluck => "pluck",
            SourceType::Formant => "formant",
            SourceType::Bowed => "bowed",
            SourceType::Blown => "blown",
            SourceType::Membrane => "membrane",
            SourceType::Marimba => "marimba",
            SourceType::Vibes => "vibes",
            SourceType::Kalimba => "kalimba",
            SourceType::SteelDrum => "steeldrum",
            SourceType::TubularBell => "tubular",
            SourceType::Glockenspiel => "glock",
            SourceType::Guitar => "guitar",
            SourceType::BassGuitar => "bass",
            SourceType::Harp => "harp",
            SourceType::Koto => "koto",
            SourceType::Kick => "kick",
            SourceType::Snare => "snare",
            SourceType::HihatClosed => "hh_cl",
            SourceType::HihatOpen => "hh_op",
            SourceType::Clap => "clap",
            SourceType::Cowbell => "cowbell",
            SourceType::Rim => "rim",
            SourceType::Tom => "tom",
            SourceType::Clave => "clave",
            SourceType::Conga => "conga",
            SourceType::Choir => "choir",
            SourceType::EPiano => "epiano",
            SourceType::Organ => "organ",
            SourceType::BrassStab => "brass",
            SourceType::Strings => "strings",
            SourceType::Acid => "acid",
            SourceType::Gendy => "gendy",
            SourceType::Chaos => "chaos",
            SourceType::Additive => "additive",
            SourceType::Wavetable => "wavetable",
            SourceType::Granular => "granular",
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
            SourceType::FMBell => "imbolc_fm_bell",
            SourceType::FMBrass => "imbolc_fm_brass",
            SourceType::Pluck => "imbolc_pluck",
            SourceType::Formant => "imbolc_formant",
            SourceType::Bowed => "imbolc_bowed",
            SourceType::Blown => "imbolc_blown",
            SourceType::Membrane => "imbolc_membrane",
            SourceType::Marimba => "imbolc_marimba",
            SourceType::Vibes => "imbolc_vibes",
            SourceType::Kalimba => "imbolc_kalimba",
            SourceType::SteelDrum => "imbolc_steel_drum",
            SourceType::TubularBell => "imbolc_tubular_bell",
            SourceType::Glockenspiel => "imbolc_glockenspiel",
            SourceType::Guitar => "imbolc_guitar",
            SourceType::BassGuitar => "imbolc_bass_guitar",
            SourceType::Harp => "imbolc_harp",
            SourceType::Koto => "imbolc_koto",
            SourceType::Kick => "imbolc_kick",
            SourceType::Snare => "imbolc_snare",
            SourceType::HihatClosed => "imbolc_hihat_closed",
            SourceType::HihatOpen => "imbolc_hihat_open",
            SourceType::Clap => "imbolc_clap",
            SourceType::Cowbell => "imbolc_cowbell",
            SourceType::Rim => "imbolc_rim",
            SourceType::Tom => "imbolc_tom",
            SourceType::Clave => "imbolc_clave",
            SourceType::Conga => "imbolc_conga",
            SourceType::Choir => "imbolc_choir",
            SourceType::EPiano => "imbolc_epiano",
            SourceType::Organ => "imbolc_organ",
            SourceType::BrassStab => "imbolc_brass_stab",
            SourceType::Strings => "imbolc_strings",
            SourceType::Acid => "imbolc_acid",
            SourceType::Gendy => "imbolc_gendy",
            SourceType::Chaos => "imbolc_chaos",
            SourceType::Additive => "imbolc_additive",
            SourceType::Wavetable => "imbolc_wavetable",
            SourceType::Granular => "imbolc_granular",
            SourceType::AudioIn => "imbolc_audio_in",
            SourceType::BusIn => "imbolc_bus_in",
            SourceType::PitchedSampler => "imbolc_sampler",
            SourceType::TimeStretch => "imbolc_timestretch",
            SourceType::Kit => "imbolc_sampler_oneshot",
            SourceType::Custom(_) => "imbolc_saw", // Fallback, use synth_def_name_with_registry instead
            SourceType::Vst(_) => "imbolc_vst_instrument",
        }
    }

    /// Returns true if this source type has a mono variant SynthDef.
    pub fn has_mono_variant(&self) -> bool {
        matches!(
            self,
            SourceType::Saw
                | SourceType::Sin
                | SourceType::Sqr
                | SourceType::Tri
                | SourceType::Noise
                | SourceType::Pulse
        )
    }

    /// Get the mono SuperCollider synthdef name (falls back to stereo if no mono variant)
    pub fn synth_def_name_mono(&self) -> &'static str {
        match self {
            SourceType::Saw => "imbolc_saw_mono",
            SourceType::Sin => "imbolc_sin_mono",
            SourceType::Sqr => "imbolc_sqr_mono",
            SourceType::Tri => "imbolc_tri_mono",
            SourceType::Noise => "imbolc_noise_mono",
            SourceType::Pulse => "imbolc_pulse_mono",
            // No mono variants for these - fall back to stereo
            _ => self.synth_def_name(),
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
            // Basic Oscillators
            SourceType::Saw, SourceType::Sin, SourceType::Sqr, SourceType::Tri,
            SourceType::Noise, SourceType::Pulse, SourceType::SuperSaw, SourceType::Sync,
            // Modulation
            SourceType::Ring, SourceType::FBSin, SourceType::FM, SourceType::PhaseMod,
            SourceType::FMBell, SourceType::FMBrass,
            // Physical Models
            SourceType::Pluck, SourceType::Formant, SourceType::Bowed, SourceType::Blown, SourceType::Membrane,
            // Mallet Percussion
            SourceType::Marimba, SourceType::Vibes, SourceType::Kalimba, SourceType::SteelDrum,
            SourceType::TubularBell, SourceType::Glockenspiel,
            // Plucked Strings
            SourceType::Guitar, SourceType::BassGuitar, SourceType::Harp, SourceType::Koto,
            // Drums
            SourceType::Kick, SourceType::Snare, SourceType::HihatClosed, SourceType::HihatOpen,
            SourceType::Clap, SourceType::Cowbell, SourceType::Rim, SourceType::Tom,
            SourceType::Clave, SourceType::Conga,
            // Classic Synths
            SourceType::Choir, SourceType::EPiano, SourceType::Organ, SourceType::BrassStab,
            SourceType::Strings, SourceType::Acid,
            // Experimental
            SourceType::Gendy, SourceType::Chaos,
            // Synthesis
            SourceType::Additive, SourceType::Wavetable, SourceType::Granular,
            // Routing
            SourceType::AudioIn, SourceType::BusIn,
            // Samplers
            SourceType::PitchedSampler, SourceType::TimeStretch, SourceType::Kit,
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

    /// Returns the default ADSR envelope for this source type.
    /// Values are tuned to the acoustic characteristics of each category.
    pub fn default_envelope(&self) -> EnvConfig {
        match self {
            // Percussive: short, punchy
            SourceType::Kick | SourceType::Snare | SourceType::HihatClosed |
            SourceType::HihatOpen | SourceType::Clap | SourceType::Cowbell |
            SourceType::Rim | SourceType::Tom | SourceType::Clave | SourceType::Conga =>
                EnvConfig { attack: 0.001, decay: 0.3, sustain: 0.0, release: 0.1 },

            // Plucked strings: natural decay
            SourceType::Guitar | SourceType::BassGuitar | SourceType::Harp |
            SourceType::Koto | SourceType::Pluck =>
                EnvConfig { attack: 0.001, decay: 2.5, sustain: 0.0, release: 0.1 },

            // Mallet percussion: resonant decay
            SourceType::Marimba | SourceType::Vibes | SourceType::Kalimba |
            SourceType::SteelDrum | SourceType::Glockenspiel =>
                EnvConfig { attack: 0.001, decay: 1.5, sustain: 0.0, release: 0.2 },

            // FM/Bell: bell-like decay with longer release
            SourceType::FMBell | SourceType::TubularBell =>
                EnvConfig { attack: 0.001, decay: 2.0, sustain: 0.0, release: 0.5 },

            // Sustained: full sustain for held notes
            SourceType::Organ | SourceType::Strings | SourceType::Choir |
            SourceType::Bowed | SourceType::Blown =>
                EnvConfig { attack: 0.05, decay: 0.1, sustain: 0.8, release: 0.3 },

            // EPiano/Brass: moderate sustain
            SourceType::EPiano | SourceType::BrassStab | SourceType::FMBrass =>
                EnvConfig { attack: 0.01, decay: 0.3, sustain: 0.5, release: 0.2 },

            // Membrane: moderate decay
            SourceType::Membrane =>
                EnvConfig { attack: 0.001, decay: 0.8, sustain: 0.0, release: 0.15 },

            // Synth oscillators and experimental: flexible default
            SourceType::Saw | SourceType::Sin | SourceType::Sqr | SourceType::Tri |
            SourceType::Noise | SourceType::Pulse | SourceType::SuperSaw | SourceType::Sync |
            SourceType::Ring | SourceType::FBSin | SourceType::FM | SourceType::PhaseMod |
            SourceType::Formant | SourceType::Gendy | SourceType::Chaos |
            SourceType::Additive | SourceType::Wavetable | SourceType::Granular | SourceType::Acid =>
                EnvConfig { attack: 0.01, decay: 0.1, sustain: 0.7, release: 0.3 },

            // Routing/Samplers: pass-through or sample-driven
            SourceType::AudioIn | SourceType::BusIn | SourceType::PitchedSampler |
            SourceType::TimeStretch | SourceType::Kit =>
                EnvConfig { attack: 0.001, decay: 0.1, sustain: 1.0, release: 0.1 },

            // External: generic default
            SourceType::Custom(_) | SourceType::Vst(_) =>
                EnvConfig { attack: 0.01, decay: 0.1, sustain: 0.7, release: 0.3 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_type_name_non_empty() {
        for st in SourceType::all() {
            assert!(!st.name().is_empty(), "{:?} has empty name", st);
        }
    }

    #[test]
    fn source_type_synth_def_name_prefixed() {
        for st in SourceType::all() {
            assert!(st.synth_def_name().starts_with("imbolc_"), "{:?}", st);
        }
    }

    #[test]
    fn source_type_all_excludes_custom_vst() {
        let all = SourceType::all();
        assert!(all.iter().all(|s| !s.is_custom()));
        assert!(all.iter().all(|s| !s.is_vst()));
    }

    #[test]
    fn source_type_is_sample_checks() {
        assert!(SourceType::PitchedSampler.is_sample());
        assert!(!SourceType::Saw.is_sample());
    }

    #[test]
    fn source_type_is_kit() {
        assert!(SourceType::Kit.is_kit());
        assert!(!SourceType::Saw.is_kit());
    }

    #[test]
    fn source_type_custom_id() {
        assert_eq!(SourceType::Custom(CustomSynthDefId::new(5)).custom_id(), Some(CustomSynthDefId::new(5)));
        assert_eq!(SourceType::Saw.custom_id(), None);
    }

    #[test]
    fn source_type_vst_id() {
        assert_eq!(SourceType::Vst(VstPluginId::new(3)).vst_id(), Some(VstPluginId::new(3)));
        assert_eq!(SourceType::Saw.vst_id(), None);
    }

    #[test]
    fn source_type_default_params_non_empty() {
        // Basic oscillators should have params
        assert!(!SourceType::Saw.default_params().is_empty());
        assert!(!SourceType::FM.default_params().is_empty());
    }
}

use crate::state::custom_synthdef::CustomSynthDefRegistry;
use crate::state::vst::VstPluginRegistry;

/// Extension trait for SourceType methods that require registry access
pub trait SourceTypeExt {
    /// Get display name, with custom synthdef name lookup
    fn display_name(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get display name with VST plugin registry lookup
    fn display_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String;
    /// Get short name with custom synthdef lookup
    fn short_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get short name with VST plugin registry lookup
    fn short_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String;
    /// Get the SuperCollider synthdef name with custom synthdef lookup
    fn synth_def_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String;
    /// Get default params with custom synthdef lookup
    fn default_params_with_registry(&self, registry: &CustomSynthDefRegistry) -> Vec<Param>;
    /// All source types including custom ones from registry
    fn all_with_custom(registry: &CustomSynthDefRegistry) -> Vec<SourceType>;
}

impl SourceTypeExt for SourceType {
    fn display_name(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Custom".to_string()),
            _ => self.name().to_string(),
        }
    }

    fn display_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String {
        match self {
            SourceType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "VST".to_string()),
            _ => self.display_name(custom_registry),
        }
    }

    fn short_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.synthdef_name.clone())
                .unwrap_or_else(|| "custom".to_string()),
            _ => self.short_name().to_string(),
        }
    }

    fn short_name_vst(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> String {
        match self {
            SourceType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.to_lowercase())
                .unwrap_or_else(|| "vst".to_string()),
            _ => self.short_name_with_registry(custom_registry),
        }
    }

    fn synth_def_name_with_registry(&self, registry: &CustomSynthDefRegistry) -> String {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| s.synthdef_name.clone())
                .unwrap_or_else(|| "imbolc_saw".to_string()),
            SourceType::Vst(_) => "imbolc_vst_instrument".to_string(),
            _ => self.synth_def_name().to_string(),
        }
    }

    fn default_params_with_registry(&self, registry: &CustomSynthDefRegistry) -> Vec<Param> {
        match self {
            SourceType::Custom(id) => registry
                .get(*id)
                .map(|s| {
                    s.params
                        .iter()
                        .map(|p| Param {
                            name: p.name.clone(),
                            value: ParamValue::Float(p.default),
                            min: p.min,
                            max: p.max,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => self.default_params(),
        }
    }

    fn all_with_custom(registry: &CustomSynthDefRegistry) -> Vec<SourceType> {
        let mut types = SourceType::all();
        for synthdef in &registry.synthdefs {
            types.push(SourceType::Custom(synthdef.id));
        }
        types
    }
}
