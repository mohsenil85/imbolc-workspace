use std::path::PathBuf;

use imbolc_types::CustomSynthDefId;
use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};
use imbolc_types::VstPluginId;
use crate::state::instrument::{SourceExtra, SourceType};
use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;
use super::{save_project, load_project, temp_db_path};

#[test]
fn round_trip_drum_sequencer() {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let kit_id = instruments.add_instrument(SourceType::Kit);

    if let Some(inst) = instruments.instrument_mut(kit_id) {
        if let Some(seq) = inst.drum_sequencer_mut() {
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
    let seq = loaded_kit.drum_sequencer().unwrap();
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
        if let Some(config) = inst.sampler_config_mut() {
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
    let config = loaded.sampler_config().unwrap();
    assert_eq!(config.buffer_id, Some(42));
    assert_eq!(config.sample_name.as_deref(), Some("test.wav"));
    assert!(config.loop_mode);
    assert!(!config.pitch_tracking);
    assert!(!config.slices.is_empty());
    assert_eq!(config.slices.last().unwrap().name, "A");

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_vst_plugins() {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Vst(VstPluginId::new(0)));

    if let Some(inst) = instruments.instrument_mut(inst_id) {
        inst.source_extra = SourceExtra::Vst {
            param_values: vec![(0, 0.75), (1, 0.5)],
            state_path: Some(PathBuf::from("/tmp/test.vststate")),
        };
    }

    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (_, loaded_inst) = load_project(&path).expect("load");

    let loaded = loaded_inst.instruments.iter().find(|i| i.id == inst_id).unwrap();
    assert!(loaded.vst_source_params().iter().any(|&(k, v)| k == 0 && (v - 0.75).abs() < 0.01));
    assert!(loaded.vst_source_params().iter().any(|&(k, v)| k == 1 && (v - 0.5).abs() < 0.01));
    assert_eq!(loaded.vst_source_state_path().map(|p| p.as_path()), Some(std::path::Path::new("/tmp/test.vststate")));

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_custom_synthdefs() {
    let mut session = SessionState::new();
    let instruments = InstrumentState::new();

    let mut registry = CustomSynthDefRegistry::new();
    registry.add(CustomSynthDef {
        id: CustomSynthDefId::new(0),
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
fn round_trip_layer_octave_offset() {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let id1 = instruments.add_instrument(SourceType::Saw);
    let id2 = instruments.add_instrument(SourceType::Sin);

    // Set non-default offsets
    if let Some(inst) = instruments.instrument_mut(id1) {
        inst.layer.octave_offset = 3;
    }
    if let Some(inst) = instruments.instrument_mut(id2) {
        inst.layer.octave_offset = -2;
    }

    session.piano_roll.add_track(id1);
    session.piano_roll.add_track(id2);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (_, loaded_instruments) = load_project(&path).expect("load");

    assert_eq!(loaded_instruments.instrument(id1).unwrap().layer.octave_offset, 3);
    assert_eq!(loaded_instruments.instrument(id2).unwrap().layer.octave_offset, -2);

    std::fs::remove_file(&path).ok();
}
