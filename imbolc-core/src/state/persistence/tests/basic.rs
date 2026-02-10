use std::path::PathBuf;

use crate::state::AutomationTarget;
use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};
use crate::state::instrument::{EffectType, FilterType, LfoConfig, LfoShape, ParameterTarget, ModSource, OutputTarget, SourceType};
use crate::state::instrument_state::InstrumentState;
use crate::state::param::ParamValue;
use crate::state::sampler::Slice;
use crate::state::session::SessionState;
use super::{save_project, load_project, temp_db_path};

#[test]
fn save_and_load_round_trip_basic() {
    let mut session = SessionState::new();
    session.bpm = 140;
    session.time_signature = (3, 4);
    session.key = crate::state::music::Key::D;
    session.scale = crate::state::music::Scale::Minor;
    session.tuning_a4 = 432.0;
    session.snap = true;
    session.piano_roll.bpm = session.bpm as f32;
    session.piano_roll.time_signature = session.time_signature;

    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);
    let inst = instruments.instrument_mut(inst_id).unwrap();
    inst.name = "Test".to_string();
    inst.set_filter(Some(FilterType::Hpf));
    if let Some(filter) = inst.filter_mut() {
        filter.cutoff.value = 1234.0;
        filter.resonance.value = 0.42;
    }
    inst.lfo.enabled = true;
    inst.lfo.rate = 5.0;
    inst.level = 0.42;
    inst.pan = -0.2;
    inst.output_target = OutputTarget::Bus(2);
    inst.add_effect(EffectType::Delay);

    session.piano_roll.add_track(inst_id);
    session.piano_roll.toggle_note(0, 60, 0, 480, 100);

    let lane_id = session
        .automation
        .add_lane(AutomationTarget::level(inst_id));
    let lane = session.automation.lane_mut(lane_id).unwrap();
    lane.add_point(0, 0.5);
    lane.add_point(480, 0.75);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save_project");
    let (loaded_session, loaded_instruments) = load_project(&path).expect("load_project");

    assert_eq!(loaded_session.bpm, session.bpm);
    assert_eq!(loaded_session.time_signature, session.time_signature);
    assert_eq!(loaded_session.key, session.key);
    assert_eq!(loaded_session.scale, session.scale);
    assert_eq!(loaded_session.tuning_a4, session.tuning_a4);
    assert_eq!(loaded_session.snap, session.snap);

    assert_eq!(loaded_instruments.instruments.len(), 1);
    let loaded_inst = &loaded_instruments.instruments[0];
    assert_eq!(loaded_inst.id, inst_id);
    assert_eq!(loaded_inst.name, "Test");
    assert!((loaded_inst.level - 0.42).abs() < 0.001);
    assert!((loaded_inst.pan - -0.2).abs() < 0.001);
    assert_eq!(loaded_inst.output_target, OutputTarget::Bus(2));
    assert!(loaded_inst.filter().is_some());
    assert_eq!(loaded_inst.filter().unwrap().filter_type, FilterType::Hpf);
    if let Some(filter) = loaded_inst.filter() {
        assert!((filter.cutoff.value - 1234.0).abs() < 0.01);
        assert!((filter.resonance.value - 0.42).abs() < 0.01);
    }
    assert_eq!(loaded_inst.effects().count(), 1);
    assert_eq!(loaded_inst.effects().next().unwrap().effect_type, EffectType::Delay);

    assert_eq!(loaded_session.piano_roll.track_order.len(), 1);
    assert_eq!(loaded_session.piano_roll.track_order[0], inst_id);
    assert_eq!(
        loaded_session
            .piano_roll
            .track_at(0)
            .map(|t| t.notes.len())
            .unwrap_or(0),
        1
    );

    assert_eq!(loaded_session.automation.lanes.len(), 1);
    let loaded_lane = &loaded_session.automation.lanes[0];
    assert_eq!(loaded_lane.target, AutomationTarget::level(inst_id));
    assert_eq!(loaded_lane.points.len(), 2);

    std::fs::remove_file(&path).ok();
}

#[test]
fn save_and_load_round_trip_complex() {
    let mut session = SessionState::new();
    session.bpm = 98;
    session.time_signature = (7, 8);
    session.key = crate::state::music::Key::F;
    session.scale = crate::state::music::Scale::Dorian;
    session.tuning_a4 = 442.0;
    session.snap = true;
    session.piano_roll.bpm = session.bpm as f32;
    session.piano_roll.time_signature = session.time_signature;
    session.piano_roll.looping = true;
    session.piano_roll.loop_start = 480;
    session.piano_roll.loop_end = 960;

    let mut instruments = InstrumentState::new();

    let saw_id = instruments.add_instrument(SourceType::Saw);
    let sampler_id = instruments.add_instrument(SourceType::PitchedSampler);
    let kit_id = instruments.add_instrument(SourceType::Kit);

    let mut registry = CustomSynthDefRegistry::new();
    let custom_id = registry.add(CustomSynthDef {
        id: 0,
        name: "MySynth".to_string(),
        synthdef_name: "my_synth".to_string(),
        source_path: PathBuf::from("/tmp/my_synth.scd"),
        params: vec![ParamSpec {
            name: "cutoff".to_string(),
            default: 0.5,
            min: 0.0,
            max: 1.0,
        }],
    });
    session.custom_synthdefs = registry;

    let custom_inst_id = instruments.add_instrument(SourceType::Custom(custom_id));

    // Sync sends for all instruments with session buses
    let bus_ids: Vec<u8> = session.bus_ids().collect();
    for inst in &mut instruments.instruments {
        inst.sync_sends_with_buses(&bus_ids);
    }

    // Saw instrument: filter, mod source, effect, output, send, and source param
    if let Some(inst) = instruments.instrument_mut(saw_id) {
        inst.set_filter(Some(FilterType::Hpf));
        if let Some(filter) = inst.filter_mut() {
            filter.cutoff.value = 1234.0;
            filter.cutoff.mod_source = Some(ModSource::Lfo(LfoConfig {
                enabled: true,
                rate: 3.0,
                depth: 0.25,
                shape: LfoShape::Triangle,
                target: ParameterTarget::FilterCutoff,
            }));
        }
        inst.output_target = OutputTarget::Bus(2);
        inst.level = 0.55;
        inst.pan = 0.25;
        inst.sends[0].level = 0.33;
        inst.sends[0].enabled = true;

        let delay_id = inst.add_effect(EffectType::Delay);
        if let Some(effect) = inst.effect_by_id_mut(delay_id) {
            if let Some(param) = effect.params.get_mut(0) {
                param.value = ParamValue::Float(0.75);
            }
        }

        if let Some(param) = inst.source_params.get_mut(0) {
            param.value = ParamValue::Float(880.0);
        }
    }

    // Sampler instrument: config and slices
    if let Some(inst) = instruments.instrument_mut(sampler_id) {
        if let Some(config) = inst.sampler_config.as_mut() {
            config.buffer_id = Some(77);
            config.sample_name = Some("kick.wav".to_string());
            config.loop_mode = true;
            config.pitch_tracking = false;
            let slice_id = config.add_slice(0.0, 0.5);
            if let Some(slice) = config.slices.iter_mut().find(|s| s.id == slice_id) {
                slice.name = "Half".to_string();
                slice.root_note = 64;
            }
            config.selected_slice = 1;
            config.set_next_slice_id(10);
        }
    }

    // Kit instrument: pads, steps, and chopper state
    if let Some(inst) = instruments.instrument_mut(kit_id) {
        if let Some(seq) = inst.drum_sequencer.as_mut() {
            seq.pads[0].buffer_id = Some(123);
            seq.pads[0].path = Some("/tmp/kick.wav".to_string());
            seq.pads[0].name = "Kick".to_string();
            seq.pads[0].level = 0.9;

            seq.pattern_mut().steps[0][0].active = true;
            seq.pattern_mut().steps[0][0].velocity = 110;

            seq.chopper = Some(crate::state::drum_sequencer::ChopperState {
                buffer_id: Some(55),
                path: Some("/tmp/chop.wav".to_string()),
                name: "Chop".to_string(),
                slices: vec![
                    Slice::new(0, 0.0, 0.5),
                    Slice::new(1, 0.5, 1.0),
                ],
                selected_slice: 1,
                next_slice_id: 2,
                waveform_peaks: vec![0.1, 0.2],
                duration_secs: 1.23,
            });
            if let Some(chopper) = seq.chopper.as_mut() {
                chopper.slices[0].name = "A".to_string();
                chopper.slices[0].root_note = 60;
            }
        }
    }

    // Piano roll tracks and notes
    session.piano_roll.add_track(saw_id);
    session.piano_roll.add_track(sampler_id);
    session.piano_roll.toggle_note(0, 60, 0, 480, 100);

    // Automation lane targeting effect param
    let lane_id = session
        .automation
        .add_lane(AutomationTarget::effect_param(saw_id, 0, 0));
    if let Some(lane) = session.automation.lane_mut(lane_id) {
        lane.add_point(0, 0.2);
        lane.add_point(480, 0.8);
    }

    // MIDI recording settings and mappings
    session.midi_recording.live_input_instrument = Some(saw_id);
    session.midi_recording.note_passthrough = false;
    session.midi_recording.channel_filter = Some(2);
    let mut cc = crate::state::midi_recording::MidiCcMapping::new(
        7,
        AutomationTarget::level(saw_id),
    );
    cc.channel = Some(1);
    cc.min_value = 0.1;
    cc.max_value = 0.9;
    session.midi_recording.add_cc_mapping(cc);
    session.midi_recording.add_pitch_bend_config(
        crate::state::midi_recording::PitchBendConfig {
            target: AutomationTarget::sample_rate(sampler_id),
            center_value: 1.0,
            range: 0.5,
            sensitivity: 2.0,
        },
    );

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save_project");
    let (loaded_session, loaded_instruments) = load_project(&path).expect("load_project");

    // Custom synthdefs
    assert_eq!(loaded_session.custom_synthdefs.synthdefs.len(), 1);
    let loaded_synth = &loaded_session.custom_synthdefs.synthdefs[0];
    assert_eq!(loaded_synth.synthdef_name, "my_synth");
    assert_eq!(loaded_synth.params.len(), 1);
    assert_eq!(loaded_synth.params[0].name, "cutoff");

    // Instruments + sources
    assert_eq!(loaded_instruments.instruments.len(), 4);
    let loaded_saw = loaded_instruments
        .instruments
        .iter()
        .find(|i| i.id == saw_id)
        .unwrap();
    assert!(matches!(loaded_saw.source, SourceType::Saw));
    assert_eq!(loaded_saw.output_target, OutputTarget::Bus(2));
    assert!((loaded_saw.level - 0.55).abs() < 0.001);
    assert!((loaded_saw.pan - 0.25).abs() < 0.001);
    assert!(loaded_saw.sends[0].enabled);
    assert!((loaded_saw.sends[0].level - 0.33).abs() < 0.001);
    assert!(loaded_saw.filter().is_some());
    if let Some(filter) = loaded_saw.filter() {
        assert!((filter.cutoff.value - 1234.0).abs() < 0.01);
        match &filter.cutoff.mod_source {
            Some(ModSource::Lfo(lfo)) => {
                assert!((lfo.rate - 3.0).abs() < 0.01);
                assert!((lfo.depth - 0.25).abs() < 0.01);
            }
            _ => panic!("Expected LFO mod source on cutoff"),
        }
    }
    let loaded_effects: Vec<_> = loaded_saw.effects().collect();
    assert_eq!(loaded_effects.len(), 1);
    let loaded_effect = loaded_effects[0];
    assert_eq!(loaded_effect.effect_type, EffectType::Delay);
    let effect_param_name = &loaded_effect.params[0].name;
    let effect_param = loaded_effect
        .params
        .iter()
        .find(|p| &p.name == effect_param_name)
        .unwrap();
    match effect_param.value {
        ParamValue::Float(v) => assert!((v - 0.75).abs() < 0.01),
        _ => panic!("Expected float param"),
    }

    let loaded_sampler = loaded_instruments
        .instruments
        .iter()
        .find(|i| i.id == sampler_id)
        .unwrap();
    assert!(matches!(loaded_sampler.source, SourceType::PitchedSampler));
    let config = loaded_sampler.sampler_config.as_ref().unwrap();
    assert_eq!(config.buffer_id, Some(77));
    assert_eq!(config.sample_name.as_deref(), Some("kick.wav"));
    assert!(config.loop_mode);
    assert!(!config.pitch_tracking);
    assert_eq!(config.selected_slice, 1);
    assert_eq!(config.slices.len(), 2);
    assert_eq!(config.slices[1].name, "Half");
    assert_eq!(config.slices[1].root_note, 64);
    assert_eq!(config.next_slice_id(), 10);

    let loaded_kit = loaded_instruments
        .instruments
        .iter()
        .find(|i| i.id == kit_id)
        .unwrap();
    let seq = loaded_kit.drum_sequencer.as_ref().unwrap();
    assert_eq!(seq.pads[0].buffer_id, Some(123));
    assert_eq!(seq.pads[0].name, "Kick");
    assert!((seq.pads[0].level - 0.9).abs() < 0.001);
    assert!(seq.patterns[0].steps[0][0].active);
    assert_eq!(seq.patterns[0].steps[0][0].velocity, 110);
    let chopper = seq.chopper.as_ref().unwrap();
    assert_eq!(chopper.buffer_id, Some(55));
    assert_eq!(chopper.name, "Chop");
    assert_eq!(chopper.slices.len(), 2);
    assert_eq!(chopper.slices[0].name, "A");

    let loaded_custom = loaded_instruments
        .instruments
        .iter()
        .find(|i| i.id == custom_inst_id)
        .unwrap();
    assert!(matches!(loaded_custom.source, SourceType::Custom(id) if id == custom_id));

    // MIDI recording
    assert_eq!(loaded_session.midi_recording.live_input_instrument, Some(saw_id));
    assert!(!loaded_session.midi_recording.note_passthrough);
    assert_eq!(loaded_session.midi_recording.channel_filter, Some(2));
    assert_eq!(loaded_session.midi_recording.cc_mappings.len(), 1);
    let loaded_cc = &loaded_session.midi_recording.cc_mappings[0];
    assert_eq!(loaded_cc.cc_number, 7);
    assert_eq!(loaded_cc.channel, Some(1));
    assert_eq!(loaded_cc.min_value, 0.1);
    assert_eq!(loaded_cc.max_value, 0.9);
    assert!(loaded_session.midi_recording.pitch_bend_configs.len() == 1);
    let loaded_pb = &loaded_session.midi_recording.pitch_bend_configs[0];
    assert_eq!(loaded_pb.center_value, 1.0);
    assert_eq!(loaded_pb.range, 0.5);
    assert_eq!(loaded_pb.sensitivity, 2.0);

    std::fs::remove_file(&path).ok();
}
