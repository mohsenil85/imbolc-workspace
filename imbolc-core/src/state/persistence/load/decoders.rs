pub(crate) fn decode_key(s: &str) -> crate::state::music::Key {
    use crate::state::music::Key;
    match s {
        "C" => Key::C,
        "Cs" => Key::Cs, "D" => Key::D, "Ds" => Key::Ds, "E" => Key::E,
        "F" => Key::F, "Fs" => Key::Fs, "G" => Key::G, "Gs" => Key::Gs,
        "A" => Key::A, "As" => Key::As, "B" => Key::B,
        other => {
            eprintln!("[imbolc] persistence: unknown Key '{}', using C", other);
            Key::C
        }
    }
}

pub(crate) fn decode_scale(s: &str) -> crate::state::music::Scale {
    use crate::state::music::Scale;
    match s {
        "Major" => Scale::Major,
        "Minor" => Scale::Minor, "Dorian" => Scale::Dorian, "Phrygian" => Scale::Phrygian,
        "Lydian" => Scale::Lydian, "Mixolydian" => Scale::Mixolydian, "Aeolian" => Scale::Aeolian,
        "Locrian" => Scale::Locrian, "Pentatonic" => Scale::Pentatonic, "Blues" => Scale::Blues,
        "Chromatic" => Scale::Chromatic,
        other => {
            eprintln!("[imbolc] persistence: unknown Scale '{}', using Major", other);
            Scale::Major
        }
    }
}

pub(crate) fn decode_source_type(s: &str) -> crate::state::instrument::SourceType {
    use crate::state::instrument::SourceType;
    use imbolc_types::{CustomSynthDefId, VstPluginId};
    if let Some(rest) = s.strip_prefix("Custom:") {
        if let Ok(id) = rest.parse::<u32>() {
            return SourceType::Custom(CustomSynthDefId::new(id));
        }
    }
    if let Some(rest) = s.strip_prefix("Vst:") {
        if let Ok(id) = rest.parse::<u32>() {
            return SourceType::Vst(VstPluginId::new(id));
        }
    }
    match s {
        "Saw" => SourceType::Saw, "Sin" => SourceType::Sin, "Sqr" => SourceType::Sqr,
        "Tri" => SourceType::Tri, "Noise" => SourceType::Noise, "Pulse" => SourceType::Pulse,
        "SuperSaw" => SourceType::SuperSaw, "Sync" => SourceType::Sync,
        "Ring" => SourceType::Ring, "FBSin" => SourceType::FBSin,
        "FM" => SourceType::FM, "PhaseMod" => SourceType::PhaseMod,
        "FMBell" => SourceType::FMBell, "FMBrass" => SourceType::FMBrass,
        "Pluck" => SourceType::Pluck, "Formant" => SourceType::Formant,
        "Bowed" => SourceType::Bowed, "Blown" => SourceType::Blown, "Membrane" => SourceType::Membrane,
        "Marimba" => SourceType::Marimba, "Vibes" => SourceType::Vibes,
        "Kalimba" => SourceType::Kalimba, "SteelDrum" => SourceType::SteelDrum,
        "TubularBell" => SourceType::TubularBell, "Glockenspiel" => SourceType::Glockenspiel,
        "Guitar" => SourceType::Guitar, "BassGuitar" => SourceType::BassGuitar,
        "Harp" => SourceType::Harp, "Koto" => SourceType::Koto,
        "Kick" => SourceType::Kick, "Snare" => SourceType::Snare,
        "HihatClosed" => SourceType::HihatClosed, "HihatOpen" => SourceType::HihatOpen,
        "Clap" => SourceType::Clap, "Cowbell" => SourceType::Cowbell,
        "Rim" => SourceType::Rim, "Tom" => SourceType::Tom,
        "Clave" => SourceType::Clave, "Conga" => SourceType::Conga,
        "Choir" => SourceType::Choir, "EPiano" => SourceType::EPiano,
        "Organ" => SourceType::Organ, "BrassStab" => SourceType::BrassStab,
        "Strings" => SourceType::Strings, "Acid" => SourceType::Acid,
        "Gendy" => SourceType::Gendy, "Chaos" => SourceType::Chaos,
        "Additive" => SourceType::Additive, "Wavetable" => SourceType::Wavetable,
        "Granular" => SourceType::Granular,
        "AudioIn" => SourceType::AudioIn, "BusIn" => SourceType::BusIn,
        "PitchedSampler" => SourceType::PitchedSampler, "TimeStretch" => SourceType::TimeStretch,
        "Kit" => SourceType::Kit,
        other => {
            eprintln!("[imbolc] persistence: unknown SourceType '{}', using Saw", other);
            SourceType::Saw
        }
    }
}

pub(crate) fn decode_effect_type(s: &str) -> crate::state::instrument::EffectType {
    use crate::state::instrument::EffectType;
    use imbolc_types::VstPluginId;
    if let Some(rest) = s.strip_prefix("Vst:") {
        if let Ok(id) = rest.parse::<u32>() {
            return EffectType::Vst(VstPluginId::new(id));
        }
    }
    match s {
        "Delay" => EffectType::Delay, "Reverb" => EffectType::Reverb,
        "Gate" => EffectType::Gate, "TapeComp" => EffectType::TapeComp,
        "SidechainComp" => EffectType::SidechainComp,
        "Chorus" => EffectType::Chorus, "Flanger" => EffectType::Flanger,
        "Phaser" => EffectType::Phaser, "Tremolo" => EffectType::Tremolo,
        "Distortion" => EffectType::Distortion, "Bitcrusher" => EffectType::Bitcrusher,
        "Wavefolder" => EffectType::Wavefolder, "Saturator" => EffectType::Saturator,
        "TiltEq" => EffectType::TiltEq,
        "StereoWidener" => EffectType::StereoWidener, "FreqShifter" => EffectType::FreqShifter,
        "Limiter" => EffectType::Limiter, "PitchShifter" => EffectType::PitchShifter,
        "Vinyl" => EffectType::Vinyl, "Cabinet" => EffectType::Cabinet,
        "GranularDelay" => EffectType::GranularDelay, "GranularFreeze" => EffectType::GranularFreeze,
        "ConvolutionReverb" => EffectType::ConvolutionReverb,
        "Vocoder" => EffectType::Vocoder, "RingMod" => EffectType::RingMod,
        "Autopan" => EffectType::Autopan, "Resonator" => EffectType::Resonator,
        "MultibandComp" => EffectType::MultibandComp, "ParaEq" => EffectType::ParaEq,
        "SpectralFreeze" => EffectType::SpectralFreeze, "Glitch" => EffectType::Glitch,
        "Leslie" => EffectType::Leslie, "SpringReverb" => EffectType::SpringReverb,
        "EnvFollower" => EffectType::EnvFollower,
        "MidSide" => EffectType::MidSide, "Crossfader" => EffectType::Crossfader,
        "Denoise" => EffectType::Denoise, "Autotune" => EffectType::Autotune,
        "WahPedal" => EffectType::WahPedal,
        other => {
            eprintln!("[imbolc] persistence: unknown EffectType '{}', using Delay", other);
            EffectType::Delay
        }
    }
}

pub(crate) fn decode_filter_type(s: &str) -> crate::state::instrument::FilterType {
    use crate::state::instrument::FilterType;
    match s {
        "Lpf" => FilterType::Lpf,
        "Hpf" => FilterType::Hpf, "Bpf" => FilterType::Bpf, "Notch" => FilterType::Notch,
        "Comb" => FilterType::Comb, "Allpass" => FilterType::Allpass,
        "Vowel" => FilterType::Vowel, "ResDrive" => FilterType::ResDrive,
        other => {
            eprintln!("[imbolc] persistence: unknown FilterType '{}', using Lpf", other);
            FilterType::Lpf
        }
    }
}

pub(crate) fn decode_eq_band_type(s: &str) -> crate::state::instrument::EqBandType {
    use crate::state::instrument::EqBandType;
    match s {
        "Peaking" => EqBandType::Peaking,
        "LowShelf" => EqBandType::LowShelf, "HighShelf" => EqBandType::HighShelf,
        other => {
            eprintln!("[imbolc] persistence: unknown EqBandType '{}', using Peaking", other);
            EqBandType::Peaking
        }
    }
}

pub(crate) fn decode_lfo_shape(s: &str) -> crate::state::instrument::LfoShape {
    use crate::state::instrument::LfoShape;
    match s {
        "Sine" => LfoShape::Sine,
        "Square" => LfoShape::Square, "Saw" => LfoShape::Saw, "Triangle" => LfoShape::Triangle,
        other => {
            eprintln!("[imbolc] persistence: unknown LfoShape '{}', using Sine", other);
            LfoShape::Sine
        }
    }
}

pub(crate) fn decode_parameter_target(s: &str) -> crate::state::instrument::ParameterTarget {
    use crate::state::instrument::ParameterTarget;

    if let Some(rest) = s.strip_prefix("SendLevel:bus:") {
        // New format: "SendLevel:bus:N"
        if let Ok(id) = rest.parse::<u8>() {
            return ParameterTarget::SendLevel(imbolc_types::BusId::new(id));
        }
    }
    if let Some(rest) = s.strip_prefix("SendLevel:") {
        // Legacy format: "SendLevel:N" where N was a Vec index (0-based); bus ids are 1-based
        if let Ok(idx) = rest.parse::<usize>() {
            return ParameterTarget::SendLevel(imbolc_types::BusId::new((idx + 1) as u8));
        }
    }
    if let Some(rest) = s.strip_prefix("EffectParam:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let (Ok(eid), Ok(pidx)) = (parts[0].parse::<u32>(), parts[1].parse::<usize>()) {
                return ParameterTarget::EffectParam(imbolc_types::EffectId::new(eid), pidx);
            }
        }
    }
    if let Some(rest) = s.strip_prefix("EffectBypass:") {
        if let Ok(eid) = rest.parse::<u32>() { return ParameterTarget::EffectBypass(imbolc_types::EffectId::new(eid)); }
    }
    if let Some(rest) = s.strip_prefix("EqBandFreq:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandFreq(idx); }
    }
    if let Some(rest) = s.strip_prefix("EqBandGain:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandGain(idx); }
    }
    if let Some(rest) = s.strip_prefix("EqBandQ:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandQ(idx); }
    }
    if let Some(rest) = s.strip_prefix("VstParam:") {
        if let Ok(idx) = rest.parse::<u32>() { return ParameterTarget::VstParam(idx); }
    }

    match s {
        "Level" => ParameterTarget::Level, "Pan" => ParameterTarget::Pan,
        "FilterCutoff" => ParameterTarget::FilterCutoff,
        "FilterResonance" => ParameterTarget::FilterResonance,
        "FilterBypass" => ParameterTarget::FilterBypass,
        "Attack" => ParameterTarget::Attack, "Decay" => ParameterTarget::Decay,
        "Sustain" => ParameterTarget::Sustain, "Release" => ParameterTarget::Release,
        "Pitch" => ParameterTarget::Pitch, "PulseWidth" => ParameterTarget::PulseWidth,
        "Detune" => ParameterTarget::Detune, "FmIndex" => ParameterTarget::FmIndex,
        "WavetablePosition" => ParameterTarget::WavetablePosition,
        "FormantFreq" => ParameterTarget::FormantFreq, "SyncRatio" => ParameterTarget::SyncRatio,
        "Pressure" => ParameterTarget::Pressure, "Embouchure" => ParameterTarget::Embouchure,
        "GrainSize" => ParameterTarget::GrainSize, "GrainDensity" => ParameterTarget::GrainDensity,
        "FbFeedback" => ParameterTarget::FbFeedback, "RingModDepth" => ParameterTarget::RingModDepth,
        "ChaosParam" => ParameterTarget::ChaosParam,
        "AdditiveRolloff" => ParameterTarget::AdditiveRolloff,
        "MembraneTension" => ParameterTarget::MembraneTension,
        "SampleRate" => ParameterTarget::SampleRate, "SampleAmp" => ParameterTarget::SampleAmp,
        "StretchRatio" => ParameterTarget::StretchRatio, "PitchShift" => ParameterTarget::PitchShift,
        "DelayTime" => ParameterTarget::DelayTime, "DelayFeedback" => ParameterTarget::DelayFeedback,
        "ReverbMix" => ParameterTarget::ReverbMix, "GateRate" => ParameterTarget::GateRate,
        "LfoRate" => ParameterTarget::LfoRate, "LfoDepth" => ParameterTarget::LfoDepth,
        "Swing" => ParameterTarget::Swing,
        "HumanizeVelocity" => ParameterTarget::HumanizeVelocity,
        "HumanizeTiming" => ParameterTarget::HumanizeTiming,
        "TimingOffset" => ParameterTarget::TimingOffset,
        "TimeSignature" => ParameterTarget::TimeSignature,
        other => {
            eprintln!("[imbolc] persistence: unknown ParameterTarget '{}', using Level", other);
            ParameterTarget::Level
        }
    }
}

pub(crate) fn decode_output_target(s: &str) -> crate::state::instrument::OutputTarget {
    use crate::state::instrument::OutputTarget;
    use imbolc_types::BusId;
    if let Some(rest) = s.strip_prefix("Bus:") {
        if let Ok(id) = rest.parse::<u8>() {
            return OutputTarget::Bus(BusId::new(id));
        }
    }
    OutputTarget::Master
}

pub(crate) fn decode_tap_point(s: &str) -> crate::state::instrument::SendTapPoint {
    use crate::state::instrument::SendTapPoint;
    match s {
        "PostInsert" => SendTapPoint::PostInsert,
        "PreInsert" => SendTapPoint::PreInsert,
        other => {
            eprintln!("[imbolc] persistence: unknown SendTapPoint '{}', using PostInsert", other);
            SendTapPoint::PostInsert
        }
    }
}

pub(crate) fn decode_channel_config(s: &str) -> imbolc_types::ChannelConfig {
    use imbolc_types::ChannelConfig;
    match s {
        "Stereo" => ChannelConfig::Stereo,
        "Mono" => ChannelConfig::Mono,
        other => {
            eprintln!("[imbolc] persistence: unknown ChannelConfig '{}', using Stereo", other);
            ChannelConfig::Stereo
        }
    }
}

pub(crate) fn decode_curve_type(s: &str) -> crate::state::automation::CurveType {
    use crate::state::automation::CurveType;
    match s {
        "Linear" => CurveType::Linear,
        "Exponential" => CurveType::Exponential,
        "Step" => CurveType::Step,
        "SCurve" => CurveType::SCurve,
        other => {
            eprintln!("[imbolc] persistence: unknown CurveType '{}', using Linear", other);
            CurveType::Linear
        }
    }
}

pub(crate) fn decode_play_mode(s: &str) -> crate::state::arrangement::PlayMode {
    use crate::state::arrangement::PlayMode;
    match s {
        "Pattern" => PlayMode::Pattern,
        "Song" => PlayMode::Song,
        other => {
            eprintln!("[imbolc] persistence: unknown PlayMode '{}', using Pattern", other);
            PlayMode::Pattern
        }
    }
}

pub(crate) fn decode_arp_direction(s: &str) -> crate::state::arpeggiator::ArpDirection {
    use crate::state::arpeggiator::ArpDirection;
    match s {
        "Up" => ArpDirection::Up,
        "Down" => ArpDirection::Down, "UpDown" => ArpDirection::UpDown,
        "Random" => ArpDirection::Random,
        other => {
            eprintln!("[imbolc] persistence: unknown ArpDirection '{}', using Up", other);
            ArpDirection::Up
        }
    }
}

pub(crate) fn decode_arp_rate(s: &str) -> crate::state::arpeggiator::ArpRate {
    use crate::state::arpeggiator::ArpRate;
    match s {
        "Eighth" => ArpRate::Eighth,
        "Quarter" => ArpRate::Quarter, "Sixteenth" => ArpRate::Sixteenth,
        "ThirtySecond" => ArpRate::ThirtySecond,
        other => {
            eprintln!("[imbolc] persistence: unknown ArpRate '{}', using Eighth", other);
            ArpRate::Eighth
        }
    }
}

pub(crate) fn decode_chord_shape(s: &str) -> crate::state::arpeggiator::ChordShape {
    use crate::state::arpeggiator::ChordShape;
    match s {
        "Major" => ChordShape::Major,
        "Minor" => ChordShape::Minor, "Seventh" => ChordShape::Seventh,
        "MinorSeventh" => ChordShape::MinorSeventh, "Sus2" => ChordShape::Sus2,
        "Sus4" => ChordShape::Sus4, "PowerChord" => ChordShape::PowerChord,
        "Octave" => ChordShape::Octave,
        other => {
            eprintln!("[imbolc] persistence: unknown ChordShape '{}', using Major", other);
            ChordShape::Major
        }
    }
}

pub(crate) fn decode_swing_grid(s: &str) -> imbolc_types::state::groove::SwingGrid {
    use imbolc_types::state::groove::SwingGrid;
    match s {
        "Eighths" => SwingGrid::Eighths,
        "Sixteenths" => SwingGrid::Sixteenths, "Both" => SwingGrid::Both,
        other => {
            eprintln!("[imbolc] persistence: unknown SwingGrid '{}', using Eighths", other);
            SwingGrid::Eighths
        }
    }
}

pub(crate) fn decode_step_resolution(s: &str) -> imbolc_types::StepResolution {
    use imbolc_types::StepResolution;
    match s {
        "Sixteenth" => StepResolution::Sixteenth,
        "Quarter" => StepResolution::Quarter, "Eighth" => StepResolution::Eighth,
        "ThirtySecond" => StepResolution::ThirtySecond,
        other => {
            eprintln!("[imbolc] persistence: unknown StepResolution '{}', using Sixteenth", other);
            StepResolution::Sixteenth
        }
    }
}

pub(crate) fn decode_automation_target(
    target_type: &str,
    target_inst_id: Option<i64>,
    target_bus_id: Option<i64>,
    _target_extra: Option<&str>,
) -> crate::state::AutomationTarget {
    use imbolc_types::{AutomationTarget, BusId, BusParameter, GlobalParameter, InstrumentParameter};

    match target_type {
        "BusLevel" => {
            AutomationTarget::Bus(BusId::new(target_bus_id.unwrap_or(1) as u8), BusParameter::Level)
        }
        "GlobalBpm" => AutomationTarget::Global(GlobalParameter::Bpm),
        "GlobalTimeSignature" => AutomationTarget::Global(GlobalParameter::TimeSignature),
        _ => {
            // It's an instrument parameter target
            let inst_id = imbolc_types::InstrumentId::new(target_inst_id.unwrap_or(0) as u32);
            let param_target = decode_parameter_target(target_type);
            AutomationTarget::Instrument(inst_id, InstrumentParameter::Standard(param_target))
        }
    }
}
