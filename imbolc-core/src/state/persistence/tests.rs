#[cfg(test)]
mod tests {
    use crate::state::persistence::{save_project, load_project};
    use crate::state::AutomationTarget;
    use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};
    use crate::state::instrument::{EffectType, FilterConfig, FilterType, LfoConfig, LfoShape, ParameterTarget, ModSource, OutputTarget, SourceType};
    use crate::state::instrument_state::InstrumentState;
    use crate::state::param::ParamValue;
    use crate::state::sampler::Slice;
    use crate::state::session::SessionState;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("imbolc_persistence_test_{}.sqlite", nanos));
        path
    }

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
        inst.filter = Some(FilterConfig::new(FilterType::Hpf));
        if let Some(filter) = inst.filter.as_mut() {
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
        assert!(matches!(
            loaded_inst.filter,
            Some(FilterConfig { filter_type: FilterType::Hpf, .. })
        ));
        if let Some(filter) = &loaded_inst.filter {
            assert!((filter.cutoff.value - 1234.0).abs() < 0.01);
            assert!((filter.resonance.value - 0.42).abs() < 0.01);
        }
        assert_eq!(loaded_inst.effects.len(), 1);
        assert_eq!(loaded_inst.effects[0].effect_type, EffectType::Delay);

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
            inst.filter = Some(FilterConfig::new(FilterType::Hpf));
            if let Some(filter) = inst.filter.as_mut() {
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
        assert!(loaded_saw.filter.is_some());
        if let Some(filter) = &loaded_saw.filter {
            assert!((filter.cutoff.value - 1234.0).abs() < 0.01);
            match &filter.cutoff.mod_source {
                Some(ModSource::Lfo(lfo)) => {
                    assert!((lfo.rate - 3.0).abs() < 0.01);
                    assert!((lfo.depth - 0.25).abs() < 0.01);
                }
                _ => panic!("Expected LFO mod source on cutoff"),
            }
        }
        assert_eq!(loaded_saw.effects.len(), 1);
        let loaded_effect = &loaded_saw.effects[0];
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

    #[test]
    fn save_and_load_round_trip_arrangement() {
        use crate::state::arrangement::PlayMode;
        use crate::state::piano_roll::Note;

        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let inst_id = instruments.add_instrument(SourceType::Saw);

        // Create clips with notes
        let clip_id = session.arrangement.add_clip("Melody".to_string(), inst_id, 480);
        if let Some(clip) = session.arrangement.clip_mut(clip_id) {
            clip.notes.push(Note { tick: 0, pitch: 60, velocity: 100, duration: 120, probability: 1.0 });
            clip.notes.push(Note { tick: 120, pitch: 64, velocity: 80, duration: 120, probability: 0.8 });
        }

        let clip_id2 = session.arrangement.add_clip("Bass".to_string(), inst_id, 960);
        if let Some(clip) = session.arrangement.clip_mut(clip_id2) {
            clip.notes.push(Note { tick: 0, pitch: 36, velocity: 127, duration: 480, probability: 1.0 });
        }

        // Place clips on timeline
        let pid1 = session.arrangement.add_placement(clip_id, inst_id, 0);
        let pid2 = session.arrangement.add_placement(clip_id, inst_id, 480);
        let pid3 = session.arrangement.add_placement(clip_id2, inst_id, 960);

        // Resize one placement
        session.arrangement.resize_placement(pid2, Some(240));

        // Set UI state
        session.arrangement.play_mode = PlayMode::Song;
        session.arrangement.view_start_tick = 100;
        session.arrangement.ticks_per_col = 60;
        session.arrangement.cursor_tick = 480;
        session.arrangement.selected_lane = 0;
        session.arrangement.selected_placement = Some(1);

        // Add piano roll track (required for persistence)
        session.piano_roll.add_track(inst_id);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save_project");
        let (loaded_session, _loaded_instruments) = load_project(&path).expect("load_project");

        let arr = &loaded_session.arrangement;

        // Clips
        assert_eq!(arr.clips.len(), 2);
        let lc1 = arr.clip(clip_id).expect("clip 1 missing");
        assert_eq!(lc1.name, "Melody");
        assert_eq!(lc1.instrument_id, inst_id);
        assert_eq!(lc1.length_ticks, 480);
        assert_eq!(lc1.notes.len(), 2);
        assert_eq!(lc1.notes[0].pitch, 60);
        assert_eq!(lc1.notes[1].pitch, 64);
        assert!((lc1.notes[1].probability - 0.8).abs() < 0.01);

        let lc2 = arr.clip(clip_id2).expect("clip 2 missing");
        assert_eq!(lc2.name, "Bass");
        assert_eq!(lc2.length_ticks, 960);
        assert_eq!(lc2.notes.len(), 1);

        // Placements
        assert_eq!(arr.placements.len(), 3);
        let lp1 = arr.placements.iter().find(|p| p.id == pid1).expect("placement 1");
        assert_eq!(lp1.start_tick, 0);
        assert_eq!(lp1.length_override, None);

        let lp2 = arr.placements.iter().find(|p| p.id == pid2).expect("placement 2");
        assert_eq!(lp2.start_tick, 480);
        assert_eq!(lp2.length_override, Some(240));

        let lp3 = arr.placements.iter().find(|p| p.id == pid3).expect("placement 3");
        assert_eq!(lp3.clip_id, clip_id2);
        assert_eq!(lp3.start_tick, 960);

        // Settings
        assert_eq!(arr.play_mode, PlayMode::Song);
        assert_eq!(arr.view_start_tick, 100);
        assert_eq!(arr.ticks_per_col, 60);
        assert_eq!(arr.cursor_tick, 480);
        assert_eq!(arr.selected_lane, 0);
        assert_eq!(arr.selected_placement, Some(1));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_automation_with_curves() {
        let mut session = SessionState::new();
        let instruments = InstrumentState::new();

        let lane_id = session.automation.add_lane(AutomationTarget::bpm());
        let lane = session.automation.lane_mut(lane_id).unwrap();
        lane.add_point(0, 0.0);
        if let Some(p) = lane.point_at_mut(0) {
            p.curve = crate::state::automation::CurveType::Exponential;
        }
        lane.add_point(480, 0.5);
        if let Some(p) = lane.point_at_mut(480) {
            p.curve = crate::state::automation::CurveType::SCurve;
        }
        lane.add_point(960, 1.0);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (loaded, _) = load_project(&path).expect("load");

        let loaded_lane = loaded.automation.lane(lane_id).expect("lane missing");
        assert_eq!(loaded_lane.points.len(), 3);
        assert_eq!(loaded_lane.points[0].curve, crate::state::automation::CurveType::Exponential);
        assert_eq!(loaded_lane.points[1].curve, crate::state::automation::CurveType::SCurve);

        // Verify interpolation still works after reload
        let val = loaded_lane.value_at(240);
        assert!(val.is_some());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_drum_sequencer() {
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let kit_id = instruments.add_instrument(SourceType::Kit);

        if let Some(inst) = instruments.instrument_mut(kit_id) {
            if let Some(seq) = inst.drum_sequencer.as_mut() {
                seq.pads[0].name = "Kick".to_string();
                seq.pads[0].level = 0.9;
                seq.pads[0].reverse = true;
                seq.pads[0].pitch = -3;
                seq.pattern_mut().steps[0][0].active = true;
                seq.pattern_mut().steps[0][0].velocity = 110;
                seq.pattern_mut().steps[0][0].probability = 0.75;
                seq.swing_amount = 0.3;
                seq.chain = vec![0, 1, 0];
                seq.chain_enabled = true;
            }
        }

        session.piano_roll.add_track(kit_id);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (_, loaded_inst) = load_project(&path).expect("load");

        let loaded_kit = loaded_inst.instruments.iter().find(|i| i.id == kit_id).unwrap();
        let seq = loaded_kit.drum_sequencer.as_ref().unwrap();
        assert_eq!(seq.pads[0].name, "Kick");
        assert!((seq.pads[0].level - 0.9).abs() < 0.001);
        assert!(seq.pads[0].reverse);
        assert_eq!(seq.pads[0].pitch, -3);
        assert!(seq.patterns[0].steps[0][0].active);
        assert_eq!(seq.patterns[0].steps[0][0].velocity, 110);
        assert!((seq.patterns[0].steps[0][0].probability - 0.75).abs() < 0.01);
        assert!((seq.swing_amount - 0.3).abs() < 0.01);
        assert_eq!(seq.chain, vec![0, 1, 0]);
        assert!(seq.chain_enabled);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_sampler_config() {
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let sampler_id = instruments.add_instrument(SourceType::PitchedSampler);

        if let Some(inst) = instruments.instrument_mut(sampler_id) {
            if let Some(config) = inst.sampler_config.as_mut() {
                config.buffer_id = Some(42);
                config.sample_name = Some("test.wav".to_string());
                config.loop_mode = true;
                config.pitch_tracking = false;
                let slice_id = config.add_slice(0.0, 0.5);
                if let Some(s) = config.slices.iter_mut().find(|s| s.id == slice_id) {
                    s.name = "A".to_string();
                    s.root_note = 64;
                }
            }
        }

        session.piano_roll.add_track(sampler_id);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (_, loaded_inst) = load_project(&path).expect("load");

        let loaded = loaded_inst.instruments.iter().find(|i| i.id == sampler_id).unwrap();
        let config = loaded.sampler_config.as_ref().unwrap();
        assert_eq!(config.buffer_id, Some(42));
        assert_eq!(config.sample_name.as_deref(), Some("test.wav"));
        assert!(config.loop_mode);
        assert!(!config.pitch_tracking);
        assert!(!config.slices.is_empty());
        assert_eq!(config.slices.last().unwrap().name, "A");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_custom_synthdefs() {
        use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};

        let mut session = SessionState::new();
        let instruments = InstrumentState::new();

        let mut registry = CustomSynthDefRegistry::new();
        registry.add(CustomSynthDef {
            id: 0,
            name: "TestSynth".to_string(),
            synthdef_name: "test_synth".to_string(),
            source_path: PathBuf::from("/tmp/test.scd"),
            params: vec![
                ParamSpec { name: "freq".to_string(), default: 440.0, min: 20.0, max: 20000.0 },
                ParamSpec { name: "amp".to_string(), default: 0.5, min: 0.0, max: 1.0 },
            ],
        });
        session.custom_synthdefs = registry;

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (loaded, _) = load_project(&path).expect("load");

        assert_eq!(loaded.custom_synthdefs.synthdefs.len(), 1);
        let synth = &loaded.custom_synthdefs.synthdefs[0];
        assert_eq!(synth.synthdef_name, "test_synth");
        assert_eq!(synth.params.len(), 2);
        assert_eq!(synth.params[0].name, "freq");
        assert!((synth.params[0].default - 440.0).abs() < 0.01);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_vst_plugins() {
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let inst_id = instruments.add_instrument(SourceType::Saw);

        if let Some(inst) = instruments.instrument_mut(inst_id) {
            inst.vst_param_values = vec![(0, 0.75), (1, 0.5)];
            inst.vst_state_path = Some(PathBuf::from("/tmp/test.vststate"));
        }

        session.piano_roll.add_track(inst_id);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (_, loaded_inst) = load_project(&path).expect("load");

        let loaded = loaded_inst.instruments.iter().find(|i| i.id == inst_id).unwrap();
        assert!(loaded.vst_param_values.iter().any(|&(k, v)| k == 0 && (v - 0.75).abs() < 0.01));
        assert!(loaded.vst_param_values.iter().any(|&(k, v)| k == 1 && (v - 0.5).abs() < 0.01));
        assert_eq!(loaded.vst_state_path.as_deref(), Some(std::path::Path::new("/tmp/test.vststate")));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_arrangement_clips() {
        use crate::state::arrangement::PlayMode;
        use crate::state::piano_roll::Note;

        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let inst_id = instruments.add_instrument(SourceType::Saw);
        session.piano_roll.add_track(inst_id);

        let clip_id = session.arrangement.add_clip("Loop".to_string(), inst_id, 960);
        if let Some(clip) = session.arrangement.clip_mut(clip_id) {
            clip.notes.push(Note { tick: 0, pitch: 48, velocity: 100, duration: 240, probability: 1.0 });
            clip.notes.push(Note { tick: 480, pitch: 52, velocity: 80, duration: 240, probability: 0.5 });
        }

        let _pid = session.arrangement.add_placement(clip_id, inst_id, 0);
        session.arrangement.add_placement(clip_id, inst_id, 960);

        session.arrangement.play_mode = PlayMode::Song;

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (loaded, _) = load_project(&path).expect("load");

        let arr = &loaded.arrangement;
        assert_eq!(arr.clips.len(), 1);
        let clip = arr.clip(clip_id).unwrap();
        assert_eq!(clip.name, "Loop");
        assert_eq!(clip.notes.len(), 2);
        assert!((clip.notes[1].probability - 0.5).abs() < 0.01);
        assert_eq!(arr.placements.len(), 2);
        assert_eq!(arr.play_mode, PlayMode::Song);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_bus_effects() {
        let mut session = SessionState::new();
        let instruments = InstrumentState::new();

        // session.mixer.buses should already have default buses (1 and 2)
        assert!(session.mixer.buses.len() >= 2, "expected at least 2 default buses");

        // Add effects to bus 1
        let bus = session.mixer.bus_mut(1).unwrap();
        let reverb_id = bus.add_effect(EffectType::Reverb);
        let delay_id = bus.add_effect(EffectType::Delay);

        // Modify a param on the reverb
        if let Some(effect) = bus.effect_by_id_mut(reverb_id) {
            if let Some(param) = effect.params.get_mut(0) {
                param.value = ParamValue::Float(0.77);
            }
        }

        // Disable the delay and set VST fields
        if let Some(effect) = bus.effect_by_id_mut(delay_id) {
            effect.enabled = false;
            effect.vst_state_path = Some(PathBuf::from("/tmp/delay.vststate"));
            effect.vst_param_values = vec![(0, 0.42), (3, 0.88)];
        }

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (loaded_session, _) = load_project(&path).expect("load");

        // Bus 1 should have 2 effects
        let loaded_bus = loaded_session.mixer.buses.iter().find(|b| b.id == 1).unwrap();
        assert_eq!(loaded_bus.effects.len(), 2);
        assert_eq!(loaded_bus.next_effect_id, 2);

        // Reverb (id=0)
        let loaded_reverb = loaded_bus.effect_by_id(reverb_id).unwrap();
        assert_eq!(loaded_reverb.effect_type, EffectType::Reverb);
        assert!(loaded_reverb.enabled);
        match loaded_reverb.params[0].value {
            ParamValue::Float(v) => assert!((v - 0.77).abs() < 0.01),
            _ => panic!("Expected float param"),
        }

        // Delay (id=1)
        let loaded_delay = loaded_bus.effect_by_id(delay_id).unwrap();
        assert_eq!(loaded_delay.effect_type, EffectType::Delay);
        assert!(!loaded_delay.enabled);
        assert_eq!(loaded_delay.vst_state_path.as_deref(), Some(std::path::Path::new("/tmp/delay.vststate")));
        assert_eq!(loaded_delay.vst_param_values.len(), 2);
        assert!(loaded_delay.vst_param_values.iter().any(|&(k, v)| k == 0 && (v - 0.42).abs() < 0.01));
        assert!(loaded_delay.vst_param_values.iter().any(|&(k, v)| k == 3 && (v - 0.88).abs() < 0.01));

        // Bus 2 should have no effects
        let loaded_bus2 = loaded_session.mixer.buses.iter().find(|b| b.id == 2).unwrap();
        assert!(loaded_bus2.effects.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trip_layer_group_effects() {
        let mut session = SessionState::new();
        let mut instruments = InstrumentState::new();
        let inst_id = instruments.add_instrument(SourceType::Saw);

        // Assign instrument to group 1
        if let Some(inst) = instruments.instrument_mut(inst_id) {
            inst.layer_group = Some(1);
        }

        // Add layer group mixer
        let bus_ids: Vec<u8> = session.bus_ids().collect();
        session.mixer.add_layer_group_mixer(1, &bus_ids);

        // Add effects to layer group mixer
        let gm = session.mixer.layer_group_mixer_mut(1).unwrap();
        let comp_id = gm.add_effect(EffectType::TapeComp);
        let lim_id = gm.add_effect(EffectType::Limiter);

        // Modify a param on the compressor
        if let Some(effect) = gm.effect_by_id_mut(comp_id) {
            if let Some(param) = effect.params.get_mut(0) {
                param.value = ParamValue::Float(0.65);
            }
        }

        session.piano_roll.add_track(inst_id);

        let path = temp_db_path();
        save_project(&path, &session, &instruments).expect("save");
        let (loaded_session, _) = load_project(&path).expect("load");

        let loaded_gm = loaded_session.mixer.layer_group_mixers.iter().find(|g| g.group_id == 1).unwrap();
        assert_eq!(loaded_gm.effects.len(), 2);
        assert_eq!(loaded_gm.next_effect_id, 2);

        let loaded_comp = loaded_gm.effect_by_id(comp_id).unwrap();
        assert_eq!(loaded_comp.effect_type, EffectType::TapeComp);
        match loaded_comp.params[0].value {
            ParamValue::Float(v) => assert!((v - 0.65).abs() < 0.01),
            _ => panic!("Expected float param"),
        }

        let loaded_lim = loaded_gm.effect_by_id(lim_id).unwrap();
        assert_eq!(loaded_lim.effect_type, EffectType::Limiter);

        std::fs::remove_file(&path).ok();
    }
}
