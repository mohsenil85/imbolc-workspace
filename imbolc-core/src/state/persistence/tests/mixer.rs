use std::path::PathBuf;

use imbolc_types::BusId;
use crate::state::instrument::{EffectType, FilterType, SourceType};
use crate::state::instrument_state::InstrumentState;
use crate::state::param::ParamValue;
use crate::state::session::SessionState;
use super::{save_project, load_project, temp_db_path};

#[test]
fn round_trip_bus_effects() {
    let mut session = SessionState::new();
    let instruments = InstrumentState::new();

    // session.mixer.buses should already have default buses (1 and 2)
    assert!(session.mixer.buses.len() >= 2, "expected at least 2 default buses");

    // Add effects to bus 1
    let bus = session.mixer.bus_mut(BusId::new(1)).unwrap();
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
    let loaded_bus = loaded_session.mixer.buses.iter().find(|b| b.id == BusId::new(1)).unwrap();
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
    let loaded_bus2 = loaded_session.mixer.buses.iter().find(|b| b.id == BusId::new(2)).unwrap();
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
    let bus_ids: Vec<BusId> = session.bus_ids().collect();
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

#[test]
fn round_trip_layer_group_eq() {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);

    // Assign instrument to group 1
    if let Some(inst) = instruments.instrument_mut(inst_id) {
        inst.layer_group = Some(1);
    }

    // Add layer group mixer (comes with default EQ)
    let bus_ids: Vec<BusId> = session.bus_ids().collect();
    session.mixer.add_layer_group_mixer(1, &bus_ids);

    // Verify EQ is present and modify some bands
    let gm = session.mixer.layer_group_mixer_mut(1).unwrap();
    assert!(gm.eq().is_some());
    if let Some(eq) = gm.eq_mut() {
        eq.bands[0].freq = 80.0;
        eq.bands[0].gain = -3.5;
        eq.bands[0].q = 1.2;
        eq.bands[0].enabled = false;
        eq.bands[5].freq = 2000.0;
        eq.bands[5].gain = 4.0;
        eq.bands[5].q = 0.8;
        eq.bands[11].freq = 16000.0;
        eq.bands[11].gain = -6.0;
    }

    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (loaded_session, _) = load_project(&path).expect("load");

    let loaded_gm = loaded_session.mixer.layer_group_mixers.iter().find(|g| g.group_id == 1).unwrap();
    let loaded_eq = loaded_gm.eq().expect("EQ should be present after load");
    assert_eq!(loaded_eq.bands.len(), 12);

    // Band 0
    assert!((loaded_eq.bands[0].freq - 80.0).abs() < 0.01);
    assert!((loaded_eq.bands[0].gain - -3.5).abs() < 0.01);
    assert!((loaded_eq.bands[0].q - 1.2).abs() < 0.01);
    assert!(!loaded_eq.bands[0].enabled);

    // Band 5
    assert!((loaded_eq.bands[5].freq - 2000.0).abs() < 0.01);
    assert!((loaded_eq.bands[5].gain - 4.0).abs() < 0.01);
    assert!((loaded_eq.bands[5].q - 0.8).abs() < 0.01);

    // Band 11
    assert!((loaded_eq.bands[11].freq - 16000.0).abs() < 0.01);
    assert!((loaded_eq.bands[11].gain - -6.0).abs() < 0.01);

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_layer_group_eq_disabled() {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);

    if let Some(inst) = instruments.instrument_mut(inst_id) {
        inst.layer_group = Some(1);
    }

    let bus_ids: Vec<BusId> = session.bus_ids().collect();
    session.mixer.add_layer_group_mixer(1, &bus_ids);

    // Toggle EQ off
    let gm = session.mixer.layer_group_mixer_mut(1).unwrap();
    gm.toggle_eq(); // was Some → now None
    assert!(gm.eq().is_none());

    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (loaded_session, _) = load_project(&path).expect("load");

    let loaded_gm = loaded_session.mixer.layer_group_mixers.iter().find(|g| g.group_id == 1).unwrap();
    assert!(loaded_gm.eq().is_none(), "EQ should be None after load when toggled off");

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_processing_chain_order() {
    use imbolc_types::ProcessingStage;

    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);

    if let Some(inst) = instruments.instrument_mut(inst_id) {
        // Add effect, filter, EQ → default order is Filter(0), EQ(1), Effect(2)
        let effect_id = inst.add_effect(EffectType::Delay);
        inst.set_filter(Some(FilterType::Hpf));
        inst.toggle_eq();

        // Reorder to: Effect → Filter → EQ
        // Current chain after above: [Filter, EQ, Effect(Delay)]
        // We want: [Effect(Delay), Filter, EQ]
        inst.processing_chain.clear();
        inst.processing_chain.push(ProcessingStage::Effect(
            crate::state::instrument::EffectSlot::new(effect_id, EffectType::Delay),
        ));
        inst.processing_chain.push(ProcessingStage::Filter(
            crate::state::instrument::FilterConfig::new(FilterType::Hpf),
        ));
        inst.processing_chain.push(ProcessingStage::Eq(
            crate::state::instrument::EqConfig::default(),
        ));
    }

    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (_, loaded_inst) = load_project(&path).expect("load");

    let loaded = loaded_inst.instruments.iter().find(|i| i.id == inst_id).unwrap();
    assert_eq!(loaded.processing_chain.len(), 3);
    assert!(loaded.processing_chain[0].is_effect(), "expected Effect at position 0");
    assert!(loaded.processing_chain[1].is_filter(), "expected Filter at position 1");
    assert!(loaded.processing_chain[2].is_eq(), "expected EQ at position 2");

    // Verify effect data
    if let ProcessingStage::Effect(e) = &loaded.processing_chain[0] {
        assert_eq!(e.effect_type, EffectType::Delay);
    }
    // Verify filter data
    if let ProcessingStage::Filter(f) = &loaded.processing_chain[1] {
        assert_eq!(f.filter_type, FilterType::Hpf);
    }

    std::fs::remove_file(&path).ok();
}

#[test]
fn round_trip_processing_chain_interleaved() {
    use imbolc_types::ProcessingStage;

    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();
    let inst_id = instruments.add_instrument(SourceType::Saw);

    if let Some(inst) = instruments.instrument_mut(inst_id) {
        // Build chain: Filter → Effect(Delay) → EQ → Effect(Reverb)
        let delay_id = inst.add_effect(EffectType::Delay);
        let reverb_id = inst.add_effect(EffectType::Reverb);

        inst.processing_chain.clear();
        inst.processing_chain.push(ProcessingStage::Filter(
            crate::state::instrument::FilterConfig::new(FilterType::Lpf),
        ));
        inst.processing_chain.push(ProcessingStage::Effect(
            crate::state::instrument::EffectSlot::new(delay_id, EffectType::Delay),
        ));
        inst.processing_chain.push(ProcessingStage::Eq(
            crate::state::instrument::EqConfig::default(),
        ));
        inst.processing_chain.push(ProcessingStage::Effect(
            crate::state::instrument::EffectSlot::new(reverb_id, EffectType::Reverb),
        ));
    }

    session.piano_roll.add_track(inst_id);

    let path = temp_db_path();
    save_project(&path, &session, &instruments).expect("save");
    let (_, loaded_inst) = load_project(&path).expect("load");

    let loaded = loaded_inst.instruments.iter().find(|i| i.id == inst_id).unwrap();
    assert_eq!(loaded.processing_chain.len(), 4);
    assert!(loaded.processing_chain[0].is_filter(), "expected Filter at 0");
    assert!(loaded.processing_chain[1].is_effect(), "expected Effect at 1");
    assert!(loaded.processing_chain[2].is_eq(), "expected EQ at 2");
    assert!(loaded.processing_chain[3].is_effect(), "expected Effect at 3");

    // Verify effect identities
    if let ProcessingStage::Effect(e) = &loaded.processing_chain[1] {
        assert_eq!(e.effect_type, EffectType::Delay);
    }
    if let ProcessingStage::Effect(e) = &loaded.processing_chain[3] {
        assert_eq!(e.effect_type, EffectType::Reverb);
    }
    // Verify filter type
    if let ProcessingStage::Filter(f) = &loaded.processing_chain[0] {
        assert_eq!(f.filter_type, FilterType::Lpf);
    }

    std::fs::remove_file(&path).ok();
}
