use crate::state::persistence::load::decoders;
use imbolc_types::{CustomSynthDefId, VstPluginId};

#[test]
fn roundtrip_key() {
    use crate::state::music::Key::*;
    let all = [C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B];
    for &k in &all {
        match k {
            C | Cs | D | Ds | E | F | Fs | G | Gs | A | As | B => {}
        }
        let encoded = format!("{:?}", k);
        let decoded = decoders::decode_key(&encoded);
        assert_eq!(decoded, k, "Key roundtrip failed for {:?}", k);
    }
}

#[test]
fn roundtrip_scale() {
    use crate::state::music::Scale::*;
    let all = [
        Major, Minor, Dorian, Phrygian, Lydian, Mixolydian, Aeolian, Locrian, Pentatonic, Blues,
        Chromatic,
    ];
    for &s in &all {
        match s {
            Major | Minor | Dorian | Phrygian | Lydian | Mixolydian | Aeolian | Locrian
            | Pentatonic | Blues | Chromatic => {}
        }
        let encoded = format!("{:?}", s);
        let decoded = decoders::decode_scale(&encoded);
        assert_eq!(decoded, s, "Scale roundtrip failed for {:?}", s);
    }
}

#[test]
fn roundtrip_source_type() {
    use crate::state::instrument::SourceType::*;
    let all = [
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
        FMBell,
        FMBrass,
        Pluck,
        Formant,
        Bowed,
        Blown,
        Membrane,
        Marimba,
        Vibes,
        Kalimba,
        SteelDrum,
        TubularBell,
        Glockenspiel,
        Guitar,
        BassGuitar,
        Harp,
        Koto,
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
        Choir,
        EPiano,
        Organ,
        BrassStab,
        Strings,
        Acid,
        Gendy,
        Chaos,
        Additive,
        Wavetable,
        Granular,
        AudioIn,
        BusIn,
        PitchedSampler,
        TimeStretch,
        Kit,
        Custom(CustomSynthDefId::new(42)),
        Vst(VstPluginId::new(99)),
    ];
    for s in &all {
        match s {
            Saw | Sin | Sqr | Tri | Noise | Pulse | SuperSaw | Sync | Ring | FBSin | FM
            | PhaseMod | FMBell | FMBrass | Pluck | Formant | Bowed | Blown | Membrane
            | Marimba | Vibes | Kalimba | SteelDrum | TubularBell | Glockenspiel | Guitar
            | BassGuitar | Harp | Koto | Kick | Snare | HihatClosed | HihatOpen | Clap
            | Cowbell | Rim | Tom | Clave | Conga | Choir | EPiano | Organ | BrassStab
            | Strings | Acid | Gendy | Chaos | Additive | Wavetable | Granular | AudioIn
            | BusIn | PitchedSampler | TimeStretch | Kit | Custom(_) | Vst(_) => {}
        }
        let encoded = format!("{:?}", s);
        // Custom and Vst use "Custom:42" / "Vst:99" encoding in save
        let save_str = match s {
            Custom(id) => format!("Custom:{}", id.get()),
            Vst(id) => format!("Vst:{}", id.get()),
            _ => encoded,
        };
        let decoded = decoders::decode_source_type(&save_str);
        assert_eq!(&decoded, s, "SourceType roundtrip failed for {:?}", s);
    }
}

#[test]
fn roundtrip_effect_type() {
    use crate::state::instrument::EffectType::*;
    let all = [
        Delay,
        Reverb,
        Gate,
        TapeComp,
        SidechainComp,
        Chorus,
        Flanger,
        Phaser,
        Tremolo,
        Distortion,
        Bitcrusher,
        Wavefolder,
        Saturator,
        TiltEq,
        StereoWidener,
        FreqShifter,
        Limiter,
        PitchShifter,
        Vinyl,
        Cabinet,
        GranularDelay,
        GranularFreeze,
        ConvolutionReverb,
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
        Vst(VstPluginId::new(42)),
    ];
    for e in &all {
        match e {
            Delay | Reverb | Gate | TapeComp | SidechainComp | Chorus | Flanger | Phaser
            | Tremolo | Distortion | Bitcrusher | Wavefolder | Saturator | TiltEq
            | StereoWidener | FreqShifter | Limiter | PitchShifter | Vinyl | Cabinet
            | GranularDelay | GranularFreeze | ConvolutionReverb | Vocoder | RingMod | Autopan
            | Resonator | MultibandComp | ParaEq | SpectralFreeze | Glitch | Leslie
            | SpringReverb | EnvFollower | MidSide | Crossfader | Denoise | Autotune | WahPedal
            | Vst(_) => {}
        }
        let save_str = match e {
            Vst(id) => format!("Vst:{}", id.get()),
            _ => format!("{:?}", e),
        };
        let decoded = decoders::decode_effect_type(&save_str);
        assert_eq!(&decoded, e, "EffectType roundtrip failed for {:?}", e);
    }
}

#[test]
fn roundtrip_filter_type() {
    use crate::state::instrument::FilterType::*;
    let all = [Lpf, Hpf, Bpf, Notch, Comb, Allpass, Vowel, ResDrive];
    for &f in &all {
        match f {
            Lpf | Hpf | Bpf | Notch | Comb | Allpass | Vowel | ResDrive => {}
        }
        let encoded = format!("{:?}", f);
        let decoded = decoders::decode_filter_type(&encoded);
        assert_eq!(decoded, f, "FilterType roundtrip failed for {:?}", f);
    }
}

#[test]
fn roundtrip_eq_band_type() {
    use crate::state::instrument::EqBandType::*;
    let all = [LowShelf, Peaking, HighShelf];
    for &b in &all {
        match b {
            LowShelf | Peaking | HighShelf => {}
        }
        let encoded = format!("{:?}", b);
        let decoded = decoders::decode_eq_band_type(&encoded);
        assert_eq!(decoded, b, "EqBandType roundtrip failed for {:?}", b);
    }
}

#[test]
fn roundtrip_lfo_shape() {
    use crate::state::instrument::LfoShape::*;
    let all = [Sine, Square, Saw, Triangle];
    for &s in &all {
        match s {
            Sine | Square | Saw | Triangle => {}
        }
        let encoded = format!("{:?}", s);
        let decoded = decoders::decode_lfo_shape(&encoded);
        assert_eq!(decoded, s, "LfoShape roundtrip failed for {:?}", s);
    }
}

#[test]
fn roundtrip_parameter_target() {
    use crate::state::instrument::ParameterTarget::*;
    use imbolc_types::{BusId, EffectId, ParamIndex};
    let all = [
        Level,
        Pan,
        FilterCutoff,
        FilterResonance,
        FilterBypass,
        Attack,
        Decay,
        Sustain,
        Release,
        Pitch,
        PulseWidth,
        Detune,
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
        SampleRate,
        SampleAmp,
        StretchRatio,
        PitchShift,
        DelayTime,
        DelayFeedback,
        ReverbMix,
        GateRate,
        LfoRate,
        LfoDepth,
        Swing,
        HumanizeVelocity,
        HumanizeTiming,
        TimingOffset,
        TimeSignature,
        // Data variants
        SendLevel(BusId::new(1)),
        SendLevel(BusId::new(3)),
        EffectParam(EffectId::new(1), ParamIndex::new(2)),
        EffectBypass(EffectId::new(5)),
        EqBandFreq(0),
        EqBandGain(1),
        EqBandQ(2),
        VstParam(7),
    ];
    for pt in &all {
        match pt {
            Level
            | Pan
            | FilterCutoff
            | FilterResonance
            | FilterBypass
            | Attack
            | Decay
            | Sustain
            | Release
            | Pitch
            | PulseWidth
            | Detune
            | FmIndex
            | WavetablePosition
            | FormantFreq
            | SyncRatio
            | Pressure
            | Embouchure
            | GrainSize
            | GrainDensity
            | FbFeedback
            | RingModDepth
            | ChaosParam
            | AdditiveRolloff
            | MembraneTension
            | SampleRate
            | SampleAmp
            | StretchRatio
            | PitchShift
            | DelayTime
            | DelayFeedback
            | ReverbMix
            | GateRate
            | LfoRate
            | LfoDepth
            | Swing
            | HumanizeVelocity
            | HumanizeTiming
            | TimingOffset
            | TimeSignature
            | SendLevel(_)
            | EffectParam(_, _)
            | EffectBypass(_)
            | EqBandFreq(_)
            | EqBandGain(_)
            | EqBandQ(_)
            | VstParam(_) => {}
        }
        // Use the same encoding as save.rs: {:?} for simple variants,
        // "SendLevel:N" / "EffectParam:E:P" / "EffectBypass:E" / etc for data variants
        let save_str = match pt {
            SendLevel(bus_id) => format!("SendLevel:bus:{}", bus_id.get()),
            EffectParam(eid, pidx) => format!("EffectParam:{}:{}", eid, pidx),
            EffectBypass(eid) => format!("EffectBypass:{}", eid),
            EqBandFreq(idx) => format!("EqBandFreq:{}", idx),
            EqBandGain(idx) => format!("EqBandGain:{}", idx),
            EqBandQ(idx) => format!("EqBandQ:{}", idx),
            VstParam(idx) => format!("VstParam:{}", idx),
            _ => format!("{:?}", pt),
        };
        let decoded = decoders::decode_parameter_target(&save_str);
        assert_eq!(
            &decoded, pt,
            "ParameterTarget roundtrip failed for {:?}",
            pt
        );
    }
}

#[test]
fn roundtrip_output_target() {
    use crate::state::instrument::OutputTarget::*;
    use imbolc_types::BusId;
    let all = [Master, Bus(BusId::new(1)), Bus(BusId::new(5))];
    for ot in &all {
        match ot {
            Master | Bus(_) => {}
        }
        let save_str = match ot {
            Master => "Master".to_string(),
            Bus(id) => format!("Bus:{}", id),
        };
        let decoded = decoders::decode_output_target(&save_str);
        assert_eq!(&decoded, ot, "OutputTarget roundtrip failed for {:?}", ot);
    }
}

#[test]
fn roundtrip_tap_point() {
    use crate::state::instrument::SendTapPoint::*;
    let all = [PreInsert, PostInsert];
    for &tp in &all {
        match tp {
            PreInsert | PostInsert => {}
        }
        let encoded = format!("{:?}", tp);
        let decoded = decoders::decode_tap_point(&encoded);
        assert_eq!(decoded, tp, "SendTapPoint roundtrip failed for {:?}", tp);
    }
}

#[test]
fn roundtrip_channel_config() {
    use imbolc_types::ChannelConfig::*;
    let all = [Mono, Stereo];
    for &cc in &all {
        match cc {
            Mono | Stereo => {}
        }
        let encoded = format!("{:?}", cc);
        let decoded = decoders::decode_channel_config(&encoded);
        assert_eq!(decoded, cc, "ChannelConfig roundtrip failed for {:?}", cc);
    }
}

#[test]
fn roundtrip_curve_type() {
    use crate::state::automation::CurveType::*;
    let all = [Linear, Exponential, Step, SCurve];
    for &ct in &all {
        match ct {
            Linear | Exponential | Step | SCurve => {}
        }
        let encoded = format!("{:?}", ct);
        let decoded = decoders::decode_curve_type(&encoded);
        assert_eq!(decoded, ct, "CurveType roundtrip failed for {:?}", ct);
    }
}

#[test]
fn roundtrip_play_mode() {
    use crate::state::arrangement::PlayMode::*;
    let all = [Pattern, Song];
    for &pm in &all {
        match pm {
            Pattern | Song => {}
        }
        let encoded = format!("{:?}", pm);
        let decoded = decoders::decode_play_mode(&encoded);
        assert_eq!(decoded, pm, "PlayMode roundtrip failed for {:?}", pm);
    }
}

#[test]
fn roundtrip_arp_direction() {
    use crate::state::arpeggiator::ArpDirection::*;
    let all = [Up, Down, UpDown, Random];
    for &d in &all {
        match d {
            Up | Down | UpDown | Random => {}
        }
        let encoded = format!("{:?}", d);
        let decoded = decoders::decode_arp_direction(&encoded);
        assert_eq!(decoded, d, "ArpDirection roundtrip failed for {:?}", d);
    }
}

#[test]
fn roundtrip_arp_rate() {
    use crate::state::arpeggiator::ArpRate::*;
    let all = [Quarter, Eighth, Sixteenth, ThirtySecond];
    for &r in &all {
        match r {
            Quarter | Eighth | Sixteenth | ThirtySecond => {}
        }
        let encoded = format!("{:?}", r);
        let decoded = decoders::decode_arp_rate(&encoded);
        assert_eq!(decoded, r, "ArpRate roundtrip failed for {:?}", r);
    }
}

#[test]
fn roundtrip_chord_shape() {
    use crate::state::arpeggiator::ChordShape::*;
    let all = [
        Major,
        Minor,
        Seventh,
        MinorSeventh,
        Sus2,
        Sus4,
        PowerChord,
        Octave,
    ];
    for &cs in &all {
        match cs {
            Major | Minor | Seventh | MinorSeventh | Sus2 | Sus4 | PowerChord | Octave => {}
        }
        let encoded = format!("{:?}", cs);
        let decoded = decoders::decode_chord_shape(&encoded);
        assert_eq!(decoded, cs, "ChordShape roundtrip failed for {:?}", cs);
    }
}

#[test]
fn roundtrip_swing_grid() {
    use imbolc_types::state::groove::SwingGrid::*;
    let all = [Eighths, Sixteenths, Both];
    for &sg in &all {
        match sg {
            Eighths | Sixteenths | Both => {}
        }
        let encoded = format!("{:?}", sg);
        let decoded = decoders::decode_swing_grid(&encoded);
        assert_eq!(decoded, sg, "SwingGrid roundtrip failed for {:?}", sg);
    }
}

#[test]
fn roundtrip_step_resolution() {
    use imbolc_types::StepResolution::*;
    let all = [Quarter, Eighth, Sixteenth, ThirtySecond];
    for &sr in &all {
        match sr {
            Quarter | Eighth | Sixteenth | ThirtySecond => {}
        }
        let encoded = format!("{:?}", sr);
        let decoded = decoders::decode_step_resolution(&encoded);
        assert_eq!(decoded, sr, "StepResolution roundtrip failed for {:?}", sr);
    }
}
