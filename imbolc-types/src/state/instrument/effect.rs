use std::path::PathBuf;

use serde::{Serialize, Deserialize};

use crate::{Param, ParamValue, VstPluginId, EffectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    Delay,
    Reverb,
    Gate,
    TapeComp,
    SidechainComp,
    // Modulation
    Chorus,
    Flanger,
    Phaser,
    Tremolo,
    // Distortion
    Distortion,
    Bitcrusher,
    Wavefolder,
    Saturator,
    // EQ
    TiltEq,
    // Stereo
    StereoWidener,
    FreqShifter,
    // Utility
    Limiter,
    PitchShifter,
    // Lo-fi
    Vinyl,
    Cabinet,
    // Granular
    GranularDelay,
    GranularFreeze,
    // Convolution
    ConvolutionReverb,
    // New effects
    Vocoder,
    RingMod,
    Autopan,
    Resonator,
    MultibandComp,
    ParaEq,
    SpectralFreeze,
    Glitch,
    Leslie,
    SpringReverb,
    EnvFollower,
    MidSide,
    Crossfader,
    Denoise,
    Autotune,
    WahPedal,
    Vst(VstPluginId),
}

impl EffectType {
    pub fn name(&self) -> &'static str {
        match self {
            EffectType::Delay => "Delay",
            EffectType::Reverb => "Reverb",
            EffectType::Gate => "Gate",
            EffectType::TapeComp => "Tape Comp",
            EffectType::SidechainComp => "SC Comp",
            EffectType::Chorus => "Chorus",
            EffectType::Flanger => "Flanger",
            EffectType::Phaser => "Phaser",
            EffectType::Tremolo => "Tremolo",
            EffectType::Distortion => "Distortion",
            EffectType::Bitcrusher => "Bitcrusher",
            EffectType::Wavefolder => "Wavefolder",
            EffectType::Saturator => "Saturator",
            EffectType::TiltEq => "Tilt EQ",
            EffectType::StereoWidener => "Stereo Widener",
            EffectType::FreqShifter => "Freq Shifter",
            EffectType::Limiter => "Limiter",
            EffectType::PitchShifter => "Pitch Shifter",
            EffectType::Vinyl => "Vinyl",
            EffectType::Cabinet => "Cabinet",
            EffectType::GranularDelay => "Granular Delay",
            EffectType::GranularFreeze => "Granular Freeze",
            EffectType::ConvolutionReverb => "Conv Reverb",
            EffectType::Vocoder => "Vocoder",
            EffectType::RingMod => "Ring Mod",
            EffectType::Autopan => "Autopan",
            EffectType::Resonator => "Resonator",
            EffectType::MultibandComp => "MB Comp",
            EffectType::ParaEq => "Para EQ",
            EffectType::SpectralFreeze => "Spectral Freeze",
            EffectType::Glitch => "Glitch",
            EffectType::Leslie => "Leslie",
            EffectType::SpringReverb => "Spring Reverb",
            EffectType::EnvFollower => "Env Follower",
            EffectType::MidSide => "Mid/Side",
            EffectType::Crossfader => "Crossfader",
            EffectType::Denoise => "Denoise",
            EffectType::Autotune => "Autotune",
            EffectType::WahPedal => "Wah Pedal",
            EffectType::Vst(_) => "VST",
        }
    }

    pub fn synth_def_name(&self) -> &'static str {
        match self {
            EffectType::Delay => "imbolc_delay",
            EffectType::Reverb => "imbolc_reverb",
            EffectType::Gate => "imbolc_gate",
            EffectType::TapeComp => "imbolc_tape_comp",
            EffectType::SidechainComp => "imbolc_sc_comp",
            EffectType::Chorus => "imbolc_chorus",
            EffectType::Flanger => "imbolc_flanger",
            EffectType::Phaser => "imbolc_phaser",
            EffectType::Tremolo => "imbolc_tremolo",
            EffectType::Distortion => "imbolc_distortion",
            EffectType::Bitcrusher => "imbolc_bitcrusher",
            EffectType::Wavefolder => "imbolc_wavefolder",
            EffectType::Saturator => "imbolc_saturator",
            EffectType::TiltEq => "imbolc_tilt_eq",
            EffectType::StereoWidener => "imbolc_stereo_widener",
            EffectType::FreqShifter => "imbolc_freq_shifter",
            EffectType::Limiter => "imbolc_limiter",
            EffectType::PitchShifter => "imbolc_pitch_shifter",
            EffectType::Vinyl => "imbolc_vinyl",
            EffectType::Cabinet => "imbolc_cabinet",
            EffectType::GranularDelay => "imbolc_granular_delay",
            EffectType::GranularFreeze => "imbolc_granular_freeze",
            EffectType::ConvolutionReverb => "imbolc_convolution_reverb",
            EffectType::Vocoder => "imbolc_vocoder",
            EffectType::RingMod => "imbolc_ringmod",
            EffectType::Autopan => "imbolc_autopan",
            EffectType::Resonator => "imbolc_resonator",
            EffectType::MultibandComp => "imbolc_multiband_comp",
            EffectType::ParaEq => "imbolc_para_eq",
            EffectType::SpectralFreeze => "imbolc_spectral_freeze",
            EffectType::Glitch => "imbolc_glitch",
            EffectType::Leslie => "imbolc_leslie",
            EffectType::SpringReverb => "imbolc_spring_reverb",
            EffectType::EnvFollower => "imbolc_env_follower",
            EffectType::MidSide => "imbolc_midside",
            EffectType::Crossfader => "imbolc_crossfader",
            EffectType::Denoise => "imbolc_denoise",
            EffectType::Autotune => "imbolc_autotune",
            EffectType::WahPedal => "imbolc_wah_pedal",
            EffectType::Vst(_) => "imbolc_vst_effect",
        }
    }

    /// Returns true if this effect has a mono variant SynthDef.
    /// Effects that are inherently stereo (StereoWidener, granular effects, etc.) return false.
    pub fn has_mono_variant(&self) -> bool {
        !matches!(
            self,
            EffectType::StereoWidener
                | EffectType::GranularDelay
                | EffectType::GranularFreeze
                | EffectType::MidSide
                | EffectType::Autopan
                | EffectType::Leslie
                | EffectType::ConvolutionReverb
                | EffectType::Vocoder
                | EffectType::RingMod
                | EffectType::Resonator
                | EffectType::MultibandComp
                | EffectType::ParaEq
                | EffectType::SpectralFreeze
                | EffectType::Glitch
                | EffectType::SpringReverb
                | EffectType::EnvFollower
                | EffectType::Crossfader
                | EffectType::Denoise
                | EffectType::Vst(_)
        )
    }

    pub fn synth_def_name_mono(&self) -> &'static str {
        match self {
            EffectType::Delay => "imbolc_delay_mono",
            EffectType::Reverb => "imbolc_reverb_mono",
            EffectType::Gate => "imbolc_gate_mono",
            EffectType::TapeComp => "imbolc_tape_comp_mono",
            EffectType::SidechainComp => "imbolc_sc_comp_mono",
            EffectType::Chorus => "imbolc_chorus_mono",
            EffectType::Flanger => "imbolc_flanger_mono",
            EffectType::Phaser => "imbolc_phaser_mono",
            EffectType::Tremolo => "imbolc_tremolo_mono",
            EffectType::Distortion => "imbolc_distortion_mono",
            EffectType::Bitcrusher => "imbolc_bitcrusher_mono",
            EffectType::Wavefolder => "imbolc_wavefolder_mono",
            EffectType::Saturator => "imbolc_saturator_mono",
            EffectType::TiltEq => "imbolc_tilt_eq_mono",
            EffectType::FreqShifter => "imbolc_freq_shifter_mono",
            EffectType::Limiter => "imbolc_limiter_mono",
            EffectType::PitchShifter => "imbolc_pitch_shifter_mono",
            EffectType::Vinyl => "imbolc_vinyl_mono",
            EffectType::Cabinet => "imbolc_cabinet_mono",
            // Inherently stereo effects - fallback to stereo
            EffectType::StereoWidener => "imbolc_stereo_widener",
            EffectType::GranularDelay => "imbolc_granular_delay",
            EffectType::GranularFreeze => "imbolc_granular_freeze",
            EffectType::ConvolutionReverb => "imbolc_convolution_reverb",
            EffectType::Vocoder => "imbolc_vocoder",
            EffectType::RingMod => "imbolc_ringmod",
            EffectType::Autopan => "imbolc_autopan",
            EffectType::Resonator => "imbolc_resonator",
            EffectType::MultibandComp => "imbolc_multiband_comp",
            EffectType::ParaEq => "imbolc_para_eq",
            EffectType::SpectralFreeze => "imbolc_spectral_freeze",
            EffectType::Glitch => "imbolc_glitch",
            EffectType::Leslie => "imbolc_leslie",
            EffectType::SpringReverb => "imbolc_spring_reverb",
            EffectType::EnvFollower => "imbolc_env_follower",
            EffectType::MidSide => "imbolc_midside",
            EffectType::Crossfader => "imbolc_crossfader",
            EffectType::Denoise => "imbolc_denoise",
            EffectType::Autotune => "imbolc_autotune",
            EffectType::WahPedal => "imbolc_wah_pedal",
            EffectType::Vst(_) => "imbolc_vst_effect",
        }
    }

    pub fn is_vst(&self) -> bool {
        matches!(self, EffectType::Vst(_))
    }

    #[allow(dead_code)]
    pub fn vst_id(&self) -> Option<VstPluginId> {
        match self {
            EffectType::Vst(id) => Some(*id),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<EffectType> {
        vec![
            EffectType::Delay, EffectType::Reverb, EffectType::Gate,
            EffectType::TapeComp, EffectType::SidechainComp,
            EffectType::Chorus, EffectType::Flanger, EffectType::Phaser, EffectType::Tremolo,
            EffectType::Distortion, EffectType::Bitcrusher, EffectType::Wavefolder, EffectType::Saturator,
            EffectType::TiltEq,
            EffectType::StereoWidener, EffectType::FreqShifter,
            EffectType::Limiter, EffectType::PitchShifter,
            EffectType::Vinyl, EffectType::Cabinet,
            EffectType::GranularDelay, EffectType::GranularFreeze,
            EffectType::ConvolutionReverb,
            EffectType::Vocoder, EffectType::RingMod, EffectType::Autopan,
            EffectType::Resonator, EffectType::MultibandComp, EffectType::ParaEq,
            EffectType::SpectralFreeze, EffectType::Glitch, EffectType::Leslie,
            EffectType::SpringReverb, EffectType::EnvFollower, EffectType::MidSide,
            EffectType::Crossfader, EffectType::Denoise,
            EffectType::Autotune, EffectType::WahPedal,
        ]
    }
}

// Note: display_name(&self, vst_registry) stays in imbolc-core - requires VstPluginRegistry

impl EffectType {
    pub fn default_params(&self) -> Vec<Param> {
        match self {
            EffectType::Delay => vec![
                Param { name: "time".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 2.0 },
                Param { name: "feedback".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
            ],
            EffectType::Reverb => vec![
                Param { name: "room".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "damp".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
            ],
            EffectType::Gate => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(4.0), min: 0.1, max: 32.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
                Param { name: "shape".to_string(), value: ParamValue::Int(1), min: 0.0, max: 2.0 },
            ],
            EffectType::TapeComp => vec![
                Param { name: "drive".to_string(), value: ParamValue::Float(1.5), min: 1.0, max: 8.0 },
                Param { name: "threshold".to_string(), value: ParamValue::Float(0.5), min: 0.01, max: 1.0 },
                Param { name: "ratio".to_string(), value: ParamValue::Float(3.0), min: 1.0, max: 20.0 },
                Param { name: "makeup".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 4.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::SidechainComp => vec![
                Param { name: "sc_bus".to_string(), value: ParamValue::Int(0), min: 0.0, max: 8.0 },
                Param { name: "threshold".to_string(), value: ParamValue::Float(0.3), min: 0.01, max: 1.0 },
                Param { name: "ratio".to_string(), value: ParamValue::Float(4.0), min: 1.0, max: 20.0 },
                Param { name: "attack".to_string(), value: ParamValue::Float(0.01), min: 0.001, max: 0.5 },
                Param { name: "release".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 2.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::Chorus => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(0.5), min: 0.1, max: 10.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.005), min: 0.001, max: 0.05 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Flanger => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(0.3), min: 0.05, max: 10.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.003), min: 0.0005, max: 0.01 },
                Param { name: "feedback".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 0.99 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Phaser => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(0.5), min: 0.05, max: 10.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "stages".to_string(), value: ParamValue::Int(4), min: 2.0, max: 12.0 },
                Param { name: "feedback".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 0.9 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Tremolo => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(4.0), min: 0.1, max: 32.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "shape".to_string(), value: ParamValue::Int(0), min: 0.0, max: 2.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::Distortion => vec![
                Param { name: "drive".to_string(), value: ParamValue::Float(2.0), min: 1.0, max: 20.0 },
                Param { name: "mode".to_string(), value: ParamValue::Int(0), min: 0.0, max: 2.0 },
                Param { name: "tone".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Bitcrusher => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(8000.0), min: 100.0, max: 44100.0 },
                Param { name: "bits".to_string(), value: ParamValue::Int(8), min: 1.0, max: 16.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Wavefolder => vec![
                Param { name: "drive".to_string(), value: ParamValue::Float(1.0), min: 0.1, max: 10.0 },
                Param { name: "symmetry".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Saturator => vec![
                Param { name: "drive".to_string(), value: ParamValue::Float(1.5), min: 1.0, max: 8.0 },
                Param { name: "color".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::TiltEq => vec![
                Param { name: "tilt".to_string(), value: ParamValue::Float(0.0), min: -1.0, max: 1.0 },
                Param { name: "frequency".to_string(), value: ParamValue::Float(1000.0), min: 100.0, max: 10000.0 },
            ],
            EffectType::StereoWidener => vec![
                Param { name: "width".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 2.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::FreqShifter => vec![
                Param { name: "shift_hz".to_string(), value: ParamValue::Float(0.0), min: -2000.0, max: 2000.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Limiter => vec![
                Param { name: "threshold".to_string(), value: ParamValue::Float(0.9), min: 0.1, max: 1.0 },
                Param { name: "release".to_string(), value: ParamValue::Float(0.01), min: 0.001, max: 1.0 },
                Param { name: "ceiling".to_string(), value: ParamValue::Float(1.0), min: 0.1, max: 1.0 },
            ],
            EffectType::PitchShifter => vec![
                Param { name: "shift".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "window".to_string(), value: ParamValue::Float(0.2), min: 0.01, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Vinyl => vec![
                Param { name: "wow".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "flutter".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "noise".to_string(), value: ParamValue::Float(0.1), min: 0.0, max: 1.0 },
                Param { name: "hiss".to_string(), value: ParamValue::Float(0.05), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Cabinet => vec![
                Param { name: "type".to_string(), value: ParamValue::Int(0), min: 0.0, max: 3.0 },
                Param { name: "tone".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::GranularDelay => vec![
                Param { name: "time".to_string(), value: ParamValue::Float(0.3), min: 0.01, max: 2.0 },
                Param { name: "grain_size".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 0.5 },
                Param { name: "density".to_string(), value: ParamValue::Float(10.0), min: 1.0, max: 40.0 },
                Param { name: "pitch".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::GranularFreeze => vec![
                Param { name: "grain_size".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 0.5 },
                Param { name: "density".to_string(), value: ParamValue::Float(10.0), min: 1.0, max: 40.0 },
                Param { name: "pitch".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "spread".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::ConvolutionReverb => vec![
                Param { name: "ir_buffer".to_string(), value: ParamValue::Int(-1), min: -1.0, max: 65536.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "predelay".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 0.5 },
            ],
            EffectType::Vocoder => vec![
                Param { name: "bands".to_string(), value: ParamValue::Int(16), min: 4.0, max: 32.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::RingMod => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 5000.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Autopan => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(2.0), min: 0.1, max: 20.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "shape".to_string(), value: ParamValue::Int(0), min: 0.0, max: 2.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::Resonator => vec![
                Param { name: "freq".to_string(), value: ParamValue::Float(440.0), min: 20.0, max: 5000.0 },
                Param { name: "decay".to_string(), value: ParamValue::Float(1.0), min: 0.01, max: 5.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::MultibandComp => vec![
                Param { name: "low_thresh".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "mid_thresh".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "hi_thresh".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::ParaEq => vec![
                Param { name: "lo_freq".to_string(), value: ParamValue::Float(200.0), min: 20.0, max: 20000.0 },
                Param { name: "lo_gain".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "mid_freq".to_string(), value: ParamValue::Float(1000.0), min: 20.0, max: 20000.0 },
                Param { name: "mid_gain".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "hi_freq".to_string(), value: ParamValue::Float(5000.0), min: 20.0, max: 20000.0 },
                Param { name: "hi_gain".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
            ],
            EffectType::SpectralFreeze => vec![
                Param { name: "freeze".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
                Param { name: "blur".to_string(), value: ParamValue::Float(0.0), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Glitch => vec![
                Param { name: "rate".to_string(), value: ParamValue::Float(4.0), min: 0.5, max: 32.0 },
                Param { name: "size".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 0.5 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::Leslie => vec![
                Param { name: "speed".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 2.0 },
                Param { name: "horn_depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
            ],
            EffectType::SpringReverb => vec![
                Param { name: "decay".to_string(), value: ParamValue::Float(2.0), min: 0.1, max: 6.0 },
                Param { name: "tone".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
            ],
            EffectType::EnvFollower => vec![
                Param { name: "attack".to_string(), value: ParamValue::Float(0.01), min: 0.001, max: 0.5 },
                Param { name: "release".to_string(), value: ParamValue::Float(0.1), min: 0.01, max: 1.0 },
                Param { name: "depth".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::MidSide => vec![
                Param { name: "width".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 3.0 },
                Param { name: "mid_gain".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "side_gain".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
            ],
            EffectType::Crossfader => vec![
                Param { name: "crossfade".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "bus_b".to_string(), value: ParamValue::Int(0), min: 0.0, max: 8.0 },
            ],
            EffectType::Denoise => vec![
                Param { name: "threshold".to_string(), value: ParamValue::Float(0.3), min: 0.0, max: 1.0 },
                Param { name: "hp_freq".to_string(), value: ParamValue::Float(80.0), min: 20.0, max: 500.0 },
                Param { name: "smoothing".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::Autotune => vec![
                Param { name: "speed".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "shift".to_string(), value: ParamValue::Float(0.0), min: -12.0, max: 12.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::WahPedal => vec![
                Param { name: "position".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "resonance".to_string(), value: ParamValue::Float(3.0), min: 0.5, max: 10.0 },
                Param { name: "mode".to_string(), value: ParamValue::Int(0), min: 0.0, max: 1.0 },
                Param { name: "sensitivity".to_string(), value: ParamValue::Float(0.5), min: 0.0, max: 1.0 },
                Param { name: "mix".to_string(), value: ParamValue::Float(1.0), min: 0.0, max: 1.0 },
            ],
            EffectType::Vst(_) => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectSlot {
    pub id: EffectId,
    pub effect_type: EffectType,
    pub params: Vec<Param>,
    pub enabled: bool,
    pub vst_param_values: Vec<(u32, f32)>,
    pub vst_state_path: Option<PathBuf>,
}

impl EffectSlot {
    pub fn new(id: EffectId, effect_type: EffectType) -> Self {
        Self {
            id,
            params: effect_type.default_params(),
            effect_type,
            enabled: true,
            vst_param_values: Vec::new(),
            vst_state_path: None,
        }
    }
}

/// Shared effect chain used by MixerBus and LayerGroupMixer.
/// Instruments use ProcessingStage instead (flexible filter/EQ/effects ordering).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectChain {
    #[serde(default)]
    pub effects: Vec<EffectSlot>,
    #[serde(default)]
    pub next_effect_id: EffectId,
}

impl EffectChain {
    /// Add an effect and return its stable EffectId.
    pub fn add_effect(&mut self, effect_type: EffectType) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id = EffectId::new(self.next_effect_id.get() + 1);
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
            .map(|e| e.id.get())
            .max()
            .map_or(EffectId::new(0), |m| EffectId::new(m + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_type_name_coverage() {
        for et in EffectType::all() {
            assert!(!et.name().is_empty(), "{:?} has empty name", et);
        }
    }

    #[test]
    fn effect_type_synth_def_name_coverage() {
        for et in EffectType::all() {
            assert!(et.synth_def_name().starts_with("imbolc_"), "{:?}", et);
        }
    }

    #[test]
    fn effect_type_has_mono_variant() {
        assert!(!EffectType::StereoWidener.has_mono_variant());
        assert!(EffectType::Delay.has_mono_variant());
        assert!(!EffectType::Vst(VstPluginId::new(0)).has_mono_variant());
    }

    #[test]
    fn effect_type_mono_name_stereo_only_falls_back() {
        // StereoWidener has no mono variant, so mono name should equal stereo name
        assert_eq!(
            EffectType::StereoWidener.synth_def_name_mono(),
            EffectType::StereoWidener.synth_def_name()
        );
    }

    #[test]
    fn effect_type_all_count() {
        let all = EffectType::all();
        // Should not contain Vst
        assert!(all.iter().all(|e| !e.is_vst()));
        assert!(!all.is_empty());
    }

    #[test]
    fn effect_type_is_vst() {
        assert!(EffectType::Vst(VstPluginId::new(0)).is_vst());
        assert!(!EffectType::Delay.is_vst());
    }

    #[test]
    fn effect_type_vst_id() {
        assert_eq!(EffectType::Vst(VstPluginId::new(42)).vst_id(), Some(VstPluginId::new(42)));
        assert_eq!(EffectType::Delay.vst_id(), None);
    }

    #[test]
    fn effect_type_default_params_non_empty() {
        assert!(!EffectType::Delay.default_params().is_empty());
        assert!(!EffectType::Reverb.default_params().is_empty());
    }

    #[test]
    fn effect_type_default_params_vst_empty() {
        assert!(EffectType::Vst(VstPluginId::new(0)).default_params().is_empty());
    }

    #[test]
    fn effect_slot_new() {
        let slot = EffectSlot::new(EffectId::new(5), EffectType::Delay);
        assert_eq!(slot.id, EffectId::new(5));
        assert!(slot.enabled);
        assert_eq!(slot.effect_type, EffectType::Delay);
        assert!(!slot.params.is_empty());
    }

    // --- EffectChain tests ---

    #[test]
    fn effect_chain_default_empty() {
        let chain = EffectChain::default();
        assert!(chain.effects.is_empty());
        assert_eq!(chain.next_effect_id, EffectId::new(0));
    }

    #[test]
    fn effect_chain_add_effect() {
        let mut chain = EffectChain::default();
        let id = chain.add_effect(EffectType::Reverb);
        assert_eq!(id, EffectId::new(0));
        assert_eq!(chain.effects.len(), 1);
        assert_eq!(chain.effects[0].effect_type, EffectType::Reverb);
        assert_eq!(chain.next_effect_id, EffectId::new(1));
    }

    #[test]
    fn effect_chain_add_multiple() {
        let mut chain = EffectChain::default();
        let id0 = chain.add_effect(EffectType::Reverb);
        let id1 = chain.add_effect(EffectType::Delay);
        assert_eq!(id0, EffectId::new(0));
        assert_eq!(id1, EffectId::new(1));
        assert_eq!(chain.effects.len(), 2);
        assert_eq!(chain.next_effect_id, EffectId::new(2));
    }

    #[test]
    fn effect_chain_lookup_by_id() {
        let mut chain = EffectChain::default();
        let id = chain.add_effect(EffectType::Reverb);
        assert!(chain.effect_by_id(id).is_some());
        assert_eq!(chain.effect_by_id(id).unwrap().effect_type, EffectType::Reverb);
        assert!(chain.effect_by_id(EffectId::new(999)).is_none());
    }

    #[test]
    fn effect_chain_remove() {
        let mut chain = EffectChain::default();
        let id = chain.add_effect(EffectType::Reverb);
        assert!(chain.remove_effect(id));
        assert!(chain.effects.is_empty());
        assert!(!chain.remove_effect(id));
    }

    #[test]
    fn effect_chain_move_effect() {
        let mut chain = EffectChain::default();
        let id0 = chain.add_effect(EffectType::Reverb);
        let id1 = chain.add_effect(EffectType::Delay);
        assert!(chain.move_effect(id0, 1));
        assert_eq!(chain.effects[0].id, id1);
        assert_eq!(chain.effects[1].id, id0);
        assert!(!chain.move_effect(id0, 1)); // already at end
    }

    #[test]
    fn effect_chain_recalculate() {
        let mut chain = EffectChain::default();
        chain.add_effect(EffectType::Reverb);
        chain.add_effect(EffectType::Delay);
        chain.next_effect_id = EffectId::new(0);
        chain.recalculate_next_effect_id();
        assert_eq!(chain.next_effect_id, EffectId::new(2));
    }

    #[test]
    fn effect_chain_recalculate_empty() {
        let mut chain = EffectChain::default();
        chain.next_effect_id = EffectId::new(5);
        chain.recalculate_next_effect_id();
        assert_eq!(chain.next_effect_id, EffectId::new(0));
    }
}

use crate::state::vst::VstPluginRegistry;

/// Extension trait for EffectType methods that require registry access
pub trait EffectTypeExt {
    /// Get display name with VST plugin registry lookup
    fn display_name(&self, vst_registry: &VstPluginRegistry) -> String;
}

impl EffectTypeExt for EffectType {
    fn display_name(&self, vst_registry: &VstPluginRegistry) -> String {
        match self {
            EffectType::Vst(id) => vst_registry
                .get(*id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "VST".to_string()),
            _ => self.name().to_string(),
        }
    }
}
