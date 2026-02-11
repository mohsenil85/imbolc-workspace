//! Projection parity tests: verify that `project_action()` in imbolc-audio
//! produces identical state mutations to `dispatch_action()` in imbolc-core.
//!
//! Each test clones state before dispatch, runs dispatch on the original,
//! runs projection on the clone, and asserts the session + instruments match.

use imbolc_audio::action_projection::project_action;
use imbolc_audio::AudioHandle;

use crate::action::{
    Action, AutomationAction, BusAction, ClickAction, EqParamKind, InstrumentAction,
    InstrumentUpdate, IoFeedback, LayerGroupAction, MixerAction, PianoRollAction, SessionAction,
    VstParamAction, VstTarget,
};
use crate::dispatch::dispatch_action;
use crate::state::AppState;

use imbolc_types::{
    AutomationTarget, BusId, CurveType, EffectId, EffectType, FilterType, InstrumentId, LfoShape,
    MixerSelection, MixerSend, MusicalSettings, OutputTarget, ParamIndex, ParameterTarget,
    SourceType,
};

// ============================================================================
// Core comparison infrastructure
// ============================================================================

/// Compare dispatch path vs projection path for a given action.
///
/// Clones state before dispatch, runs dispatch on the original, runs projection
/// on the clone, then compares session + instruments via serde_json::to_value.
fn assert_parity(state: &mut AppState, action: &Action) {
    // Clone state for projection path BEFORE dispatch mutates it
    let mut proj_session = state.session.clone();
    let mut proj_instruments = state.instruments.clone();

    // Dispatch path (mutates state.session + state.instruments, plus undo/dirty/etc)
    let audio = AudioHandle::new();
    let (io_tx, _io_rx) = std::sync::mpsc::channel::<IoFeedback>();
    dispatch_action(action, state, &audio, &mut vec![], &io_tx);

    // Projection path (mutates cloned copies)
    let projected = project_action(action, &mut proj_instruments, &mut proj_session);
    assert!(projected, "action should be projectable: {:?}", action);

    // Compare via serde (handles types without PartialEq; #[serde(skip)] fields
    // like id_index are excluded — which is correct for this comparison)
    let ds = serde_json::to_value(&state.session).unwrap();
    let ps = serde_json::to_value(&proj_session).unwrap();
    assert_eq!(ds, ps, "SessionState diverged for {:?}", action);

    let di = serde_json::to_value(&state.instruments).unwrap();
    let pi = serde_json::to_value(&proj_instruments).unwrap();
    assert_eq!(di, pi, "InstrumentState diverged for {:?}", action);
}

/// Assert that an action is NOT projectable (returns false from project_action).
fn assert_not_projectable(action: &Action) {
    let mut session = imbolc_types::SessionState::new();
    let mut instruments = imbolc_types::InstrumentState::new();
    let projected = project_action(action, &mut instruments, &mut session);
    assert!(!projected, "action should NOT be projectable: {:?}", action);
}

macro_rules! parity_test {
    ($name:ident, $fixture:expr, $action:expr) => {
        #[test]
        fn $name() {
            let mut state = $fixture;
            let action = $action;
            assert_parity(&mut state, &action);
        }
    };
}

// ============================================================================
// Fixture builders
// ============================================================================

/// Minimal fixture: fresh AppState with automation recording disabled.
fn minimal() -> AppState {
    let mut s = AppState::new();
    s.recording.automation_recording = false;
    s
}

/// One-instrument fixture: Saw instrument with Lpf filter, Reverb effect, EQ enabled.
fn one_instrument() -> AppState {
    let mut s = minimal();
    let id = s.add_instrument(SourceType::Saw);
    if let Some(inst) = s.instruments.instrument_mut(id) {
        inst.set_filter(Some(FilterType::Lpf));
        inst.add_effect(EffectType::Reverb);
        // EQ is enabled by default via toggle_eq pattern — ensure it's on
        if inst.eq().is_none() {
            inst.toggle_eq();
        }
    }
    s.instruments.selected = Some(0);
    s
}

/// Helper: get the first instrument ID from fixture state.
fn first_id(s: &AppState) -> InstrumentId {
    s.instruments.instruments[0].id
}

/// Helper: get the first effect ID from the first instrument.
fn first_effect_id(s: &AppState) -> EffectId {
    s.instruments.instruments[0]
        .effects()
        .next()
        .expect("fixture should have an effect")
        .id
}

/// Rich fixture: two instruments, bus with Delay effect, piano roll note, automation lane.
fn rich() -> AppState {
    let mut s = minimal();
    let id1 = s.add_instrument(SourceType::Saw);
    let _id2 = s.add_instrument(SourceType::Sin);
    s.instruments.selected = Some(0);
    s.session.mixer.selection = MixerSelection::Instrument(0);

    // Add a note to the piano roll
    s.session.piano_roll.toggle_note(0, 60, 0, 480, 100);

    // Add automation lane
    s.session
        .automation
        .add_lane(AutomationTarget::level(id1));

    // Bus 1 gets a Delay effect
    if let Some(bus) = s.session.bus_mut(BusId::new(1)) {
        bus.effect_chain.add_effect(EffectType::Delay);
    }

    s
}

/// Bus-effect fixture: state with a bus that has a Delay effect (for bus param tests).
fn bus_effect() -> AppState {
    let mut s = minimal();
    s.add_instrument(SourceType::Saw);
    if let Some(bus) = s.session.bus_mut(BusId::new(1)) {
        bus.effect_chain.add_effect(EffectType::Delay);
    }
    s
}

/// Layer-group fixture: two instruments linked into a layer group.
fn layer_group() -> AppState {
    let mut s = minimal();
    let id1 = s.add_instrument(SourceType::Saw);
    let id2 = s.add_instrument(SourceType::Sin);
    // Link them
    let action = Action::Instrument(InstrumentAction::LinkLayer(id1, id2));
    let audio = AudioHandle::new();
    let (io_tx, _) = std::sync::mpsc::channel::<IoFeedback>();
    dispatch_action(&action, &mut s, &audio, &mut vec![], &io_tx);
    s.recording.automation_recording = false;
    s
}

/// Helper: get the layer group ID from the first instrument.
fn group_id(s: &AppState) -> u32 {
    s.instruments.instruments[0]
        .layer.group
        .expect("fixture should have layer group")
}

// ============================================================================
// Filter tests
// ============================================================================

parity_test!(
    parity_set_filter,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetFilter(
        first_id(&one_instrument()),
        Some(FilterType::Hpf)
    ))
);

parity_test!(
    parity_toggle_filter,
    one_instrument(),
    Action::Instrument(InstrumentAction::ToggleFilter(first_id(&one_instrument())))
);

parity_test!(
    parity_cycle_filter_type,
    one_instrument(),
    Action::Instrument(InstrumentAction::CycleFilterType(first_id(
        &one_instrument()
    )))
);

parity_test!(
    parity_adjust_filter_cutoff,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustFilterCutoff(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_adjust_filter_resonance,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustFilterResonance(
        first_id(&one_instrument()),
        1.0
    ))
);

// ============================================================================
// LFO tests
// ============================================================================

parity_test!(
    parity_toggle_lfo,
    one_instrument(),
    Action::Instrument(InstrumentAction::ToggleLfo(first_id(&one_instrument())))
);

parity_test!(
    parity_adjust_lfo_rate,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustLfoRate(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_adjust_lfo_depth,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustLfoDepth(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_set_lfo_shape,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetLfoShape(
        first_id(&one_instrument()),
        LfoShape::Square
    ))
);

parity_test!(
    parity_set_lfo_target,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetLfoTarget(
        first_id(&one_instrument()),
        ParameterTarget::Pan
    ))
);

// ============================================================================
// Envelope tests
// ============================================================================

parity_test!(
    parity_adjust_attack,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustEnvelopeAttack(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_adjust_decay,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustEnvelopeDecay(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_adjust_sustain,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustEnvelopeSustain(
        first_id(&one_instrument()),
        1.0
    ))
);

parity_test!(
    parity_adjust_release,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustEnvelopeRelease(
        first_id(&one_instrument()),
        1.0
    ))
);

// ============================================================================
// Effect tests
// ============================================================================

#[test]
fn parity_add_effect() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::AddEffect(id, EffectType::Delay)),
    );
}

#[test]
fn parity_remove_effect() {
    let mut s = one_instrument();
    let id = first_id(&s);
    let eid = first_effect_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::RemoveEffect(id, eid)),
    );
}

#[test]
fn parity_toggle_effect_bypass() {
    let mut s = one_instrument();
    let id = first_id(&s);
    let eid = first_effect_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::ToggleEffectBypass(id, eid)),
    );
}

#[test]
fn parity_adjust_effect_param() {
    let mut s = one_instrument();
    let id = first_id(&s);
    let eid = first_effect_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::AdjustEffectParam(id, eid, ParamIndex::new(0), 1.0)),
    );
}

#[test]
fn parity_move_stage() {
    let mut s = one_instrument();
    let id = first_id(&s);
    // Add a second effect so we can move
    {
        let inst = s.instruments.instrument_mut(id).unwrap();
        inst.add_effect(EffectType::Delay);
    }
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::MoveStage(id, 0, 1)),
    );
}

// ============================================================================
// EQ tests
// ============================================================================

#[test]
fn parity_set_eq_param() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::SetEqParam(
            id,
            0,
            EqParamKind::Gain,
            6.0,
        )),
    );
}

#[test]
fn parity_toggle_eq() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::ToggleEq(id)),
    );
}

// ============================================================================
// Arpeggiator tests
// ============================================================================

parity_test!(
    parity_toggle_arp,
    one_instrument(),
    Action::Instrument(InstrumentAction::ToggleArp(first_id(&one_instrument())))
);

parity_test!(
    parity_cycle_arp_direction,
    one_instrument(),
    Action::Instrument(InstrumentAction::CycleArpDirection(first_id(
        &one_instrument()
    )))
);

parity_test!(
    parity_cycle_arp_rate,
    one_instrument(),
    Action::Instrument(InstrumentAction::CycleArpRate(first_id(
        &one_instrument()
    )))
);

parity_test!(
    parity_adjust_arp_octaves,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustArpOctaves(
        first_id(&one_instrument()),
        1
    ))
);

parity_test!(
    parity_adjust_arp_gate,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustArpGate(
        first_id(&one_instrument()),
        0.1
    ))
);

parity_test!(
    parity_cycle_chord_shape,
    one_instrument(),
    Action::Instrument(InstrumentAction::CycleChordShape(first_id(
        &one_instrument()
    )))
);

parity_test!(
    parity_clear_chord_shape,
    one_instrument(),
    Action::Instrument(InstrumentAction::ClearChordShape(first_id(
        &one_instrument()
    )))
);

// ============================================================================
// Groove tests
// ============================================================================

parity_test!(
    parity_set_track_swing,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetTrackSwing(
        first_id(&one_instrument()),
        Some(0.6)
    ))
);

parity_test!(
    parity_adjust_track_swing,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustTrackSwing(
        first_id(&one_instrument()),
        0.1
    ))
);

parity_test!(
    parity_set_track_humanize_velocity,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetTrackHumanizeVelocity(
        first_id(&one_instrument()),
        Some(0.3)
    ))
);

parity_test!(
    parity_adjust_track_humanize_velocity,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustTrackHumanizeVelocity(
        first_id(&one_instrument()),
        0.1
    ))
);

parity_test!(
    parity_set_track_humanize_timing,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetTrackHumanizeTiming(
        first_id(&one_instrument()),
        Some(0.2)
    ))
);

parity_test!(
    parity_adjust_track_humanize_timing,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustTrackHumanizeTiming(
        first_id(&one_instrument()),
        0.1
    ))
);

parity_test!(
    parity_set_track_timing_offset,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetTrackTimingOffset(
        first_id(&one_instrument()),
        5.0
    ))
);

parity_test!(
    parity_adjust_track_timing_offset,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustTrackTimingOffset(
        first_id(&one_instrument()),
        2.0
    ))
);

parity_test!(
    parity_reset_track_groove,
    one_instrument(),
    Action::Instrument(InstrumentAction::ResetTrackGroove(first_id(
        &one_instrument()
    )))
);

parity_test!(
    parity_set_track_time_signature,
    one_instrument(),
    Action::Instrument(InstrumentAction::SetTrackTimeSignature(
        first_id(&one_instrument()),
        Some((3, 4))
    ))
);

parity_test!(
    parity_cycle_track_time_signature,
    one_instrument(),
    Action::Instrument(InstrumentAction::CycleTrackTimeSignature(first_id(
        &one_instrument()
    )))
);

// ============================================================================
// Channel config test
// ============================================================================

parity_test!(
    parity_toggle_channel_config,
    one_instrument(),
    Action::Instrument(InstrumentAction::ToggleChannelConfig(first_id(
        &one_instrument()
    )))
);

// ============================================================================
// Layer octave offset test
// ============================================================================

parity_test!(
    parity_adjust_layer_octave_offset,
    one_instrument(),
    Action::Instrument(InstrumentAction::AdjustLayerOctaveOffset(
        first_id(&one_instrument()),
        1
    ))
);

// ============================================================================
// Mixer tests
// ============================================================================

#[test]
fn parity_mixer_move() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::Move(1)));
}

#[test]
fn parity_mixer_jump() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::Jump(-1)));
}

#[test]
fn parity_mixer_select_at() {
    let mut s = rich();
    assert_parity(
        &mut s,
        &Action::Mixer(MixerAction::SelectAt(MixerSelection::Instrument(1))),
    );
}

#[test]
fn parity_mixer_adjust_level_instrument() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustLevel(0.1)));
}

#[test]
fn parity_mixer_adjust_level_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustLevel(0.1)));
}

#[test]
fn parity_mixer_adjust_level_master() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Master;
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustLevel(0.1)));
}

#[test]
fn parity_mixer_adjust_level_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustLevel(0.1)));
}

#[test]
fn parity_mixer_toggle_mute_instrument() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleMute));
}

#[test]
fn parity_mixer_toggle_mute_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleMute));
}

#[test]
fn parity_mixer_toggle_mute_master() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Master;
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleMute));
}

#[test]
fn parity_mixer_toggle_mute_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleMute));
}

#[test]
fn parity_mixer_toggle_solo_instrument() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleSolo));
}

#[test]
fn parity_mixer_toggle_solo_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleSolo));
}

#[test]
fn parity_mixer_toggle_solo_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleSolo));
}

#[test]
fn parity_mixer_cycle_section() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::CycleSection));
}

#[test]
fn parity_mixer_cycle_output() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::CycleOutput));
}

#[test]
fn parity_mixer_cycle_output_reverse() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::CycleOutputReverse));
}

#[test]
fn parity_mixer_adjust_send() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    // Enable the send first so there's something to adjust
    s.instruments.instruments[0].mixer.sends.insert(
        BusId::new(1),
        MixerSend { bus_id: BusId::new(1), level: 0.5, enabled: true, tap_point: Default::default() },
    );
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustSend(BusId::new(1), 0.1)));
}

#[test]
fn parity_mixer_toggle_send() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::ToggleSend(BusId::new(1))));
}

#[test]
fn parity_mixer_cycle_send_tap_point() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    s.instruments.instruments[0].mixer.sends.insert(
        BusId::new(1),
        MixerSend { bus_id: BusId::new(1), level: 0.5, enabled: true, tap_point: Default::default() },
    );
    assert_parity(
        &mut s,
        &Action::Mixer(MixerAction::CycleSendTapPoint(BusId::new(1))),
    );
}

#[test]
fn parity_mixer_adjust_pan_instrument() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustPan(0.1)));
}

#[test]
fn parity_mixer_adjust_pan_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustPan(0.1)));
}

#[test]
fn parity_mixer_adjust_pan_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::AdjustPan(0.1)));
}

#[test]
fn parity_mixer_move_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::Move(1)));
}

#[test]
fn parity_mixer_jump_bus() {
    let mut s = rich();
    s.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Mixer(MixerAction::Jump(-1)));
}

// ============================================================================
// Bus tests
// ============================================================================

#[test]
fn parity_bus_add() {
    let mut s = one_instrument();
    assert_parity(&mut s, &Action::Bus(BusAction::Add));
}

#[test]
fn parity_bus_remove() {
    let mut s = one_instrument();
    // Set output to bus 1 to test reset behavior
    s.instruments.instruments[0].mixer.output_target = OutputTarget::Bus(BusId::new(1));
    assert_parity(&mut s, &Action::Bus(BusAction::Remove(BusId::new(1))));
}

#[test]
fn parity_bus_rename() {
    let mut s = minimal();
    assert_parity(
        &mut s,
        &Action::Bus(BusAction::Rename(BusId::new(1), "Drums".to_string())),
    );
}

#[test]
fn parity_bus_add_effect() {
    let mut s = minimal();
    assert_parity(
        &mut s,
        &Action::Bus(BusAction::AddEffect(BusId::new(1), EffectType::Delay)),
    );
}

#[test]
fn parity_bus_remove_effect() {
    let mut s = bus_effect();
    let eid = s.session.bus(BusId::new(1)).unwrap().effect_chain.effects[0].id;
    assert_parity(&mut s, &Action::Bus(BusAction::RemoveEffect(BusId::new(1), eid)));
}

#[test]
fn parity_bus_move_effect() {
    let mut s = bus_effect();
    // Add second effect so we can move
    s.session.bus_mut(BusId::new(1)).unwrap().effect_chain.add_effect(EffectType::Reverb);
    let eid = s.session.bus(BusId::new(1)).unwrap().effect_chain.effects[0].id;
    assert_parity(&mut s, &Action::Bus(BusAction::MoveEffect(BusId::new(1), eid, 1)));
}

#[test]
fn parity_bus_toggle_effect_bypass() {
    let mut s = bus_effect();
    let eid = s.session.bus(BusId::new(1)).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::Bus(BusAction::ToggleEffectBypass(BusId::new(1), eid)),
    );
}

#[test]
fn parity_bus_adjust_effect_param() {
    let mut s = bus_effect();
    let eid = s.session.bus(BusId::new(1)).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::Bus(BusAction::AdjustEffectParam(BusId::new(1), eid, ParamIndex::new(0), 1.0)),
    );
}

// ============================================================================
// Layer group effect tests
// ============================================================================

#[test]
fn parity_layer_group_add_effect() {
    let mut s = layer_group();
    let gid = group_id(&s);
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::AddEffect(gid, EffectType::TapeComp)),
    );
}

#[test]
fn parity_layer_group_remove_effect() {
    let mut s = layer_group();
    let gid = group_id(&s);
    // Add an effect first
    s.session
        .mixer
        .layer_group_mixer_mut(gid)
        .unwrap()
        .effect_chain.add_effect(EffectType::Reverb);
    let eid = s.session.mixer.layer_group_mixer(gid).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::RemoveEffect(gid, eid)),
    );
}

#[test]
fn parity_layer_group_move_effect() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session
        .mixer
        .layer_group_mixer_mut(gid)
        .unwrap()
        .effect_chain.add_effect(EffectType::Reverb);
    s.session
        .mixer
        .layer_group_mixer_mut(gid)
        .unwrap()
        .effect_chain.add_effect(EffectType::Delay);
    let eid = s.session.mixer.layer_group_mixer(gid).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::MoveEffect(gid, eid, 1)),
    );
}

#[test]
fn parity_layer_group_toggle_effect_bypass() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session
        .mixer
        .layer_group_mixer_mut(gid)
        .unwrap()
        .effect_chain.add_effect(EffectType::Limiter);
    let eid = s.session.mixer.layer_group_mixer(gid).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::ToggleEffectBypass(gid, eid)),
    );
}

#[test]
fn parity_layer_group_adjust_effect_param() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session
        .mixer
        .layer_group_mixer_mut(gid)
        .unwrap()
        .effect_chain.add_effect(EffectType::Reverb);
    let eid = s.session.mixer.layer_group_mixer(gid).unwrap().effect_chain.effects[0].id;
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::AdjustEffectParam(gid, eid, ParamIndex::new(0), 1.0)),
    );
}

#[test]
fn parity_layer_group_toggle_eq() {
    let mut s = layer_group();
    let gid = group_id(&s);
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::ToggleEq(gid)),
    );
}

#[test]
fn parity_layer_group_set_eq_param() {
    let mut s = layer_group();
    let gid = group_id(&s);
    assert_parity(
        &mut s,
        &Action::LayerGroup(LayerGroupAction::SetEqParam(gid, 0, EqParamKind::Gain, 6.0)),
    );
}

// ============================================================================
// CRUD tests
// ============================================================================

#[test]
fn parity_add() {
    let mut s = minimal();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Add(SourceType::Saw)),
    );
}

#[test]
fn parity_delete() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Delete(id)),
    );
}

#[test]
fn parity_edit() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Edit(id)),
    );
}

#[test]
fn parity_update() {
    let mut s = one_instrument();
    let id = first_id(&s);
    let inst = s.instruments.instrument(id).unwrap();
    let update = InstrumentUpdate {
        id,
        source: inst.source,
        source_params: inst.source_params.clone(),
        processing_chain: inst.processing_chain.clone(),
        lfo: inst.modulation.lfo.clone(),
        amp_envelope: inst.modulation.amp_envelope.clone(),
        polyphonic: !inst.polyphonic,
        active: inst.mixer.active,
    };
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Update(Box::new(update))),
    );
}

// ============================================================================
// Selection tests
// ============================================================================

#[test]
fn parity_select() {
    let mut s = rich();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Select(1)),
    );
}

parity_test!(
    parity_select_next,
    rich(),
    Action::Instrument(InstrumentAction::SelectNext)
);

parity_test!(
    parity_select_prev,
    rich(),
    Action::Instrument(InstrumentAction::SelectPrev)
);

parity_test!(
    parity_select_first,
    rich(),
    Action::Instrument(InstrumentAction::SelectFirst)
);

parity_test!(
    parity_select_last,
    rich(),
    Action::Instrument(InstrumentAction::SelectLast)
);

// ============================================================================
// Piano roll tests
// ============================================================================

parity_test!(
    parity_piano_roll_toggle_note,
    rich(),
    Action::PianoRoll(PianoRollAction::ToggleNote {
        pitch: 64,
        tick: 480,
        duration: 480,
        velocity: 100,
        track: 0
    })
);

parity_test!(
    parity_piano_roll_toggle_loop,
    rich(),
    Action::PianoRoll(PianoRollAction::ToggleLoop)
);

parity_test!(
    parity_piano_roll_set_loop_start,
    rich(),
    Action::PianoRoll(PianoRollAction::SetLoopStart(960))
);

parity_test!(
    parity_piano_roll_set_loop_end,
    rich(),
    Action::PianoRoll(PianoRollAction::SetLoopEnd(3840))
);

parity_test!(
    parity_piano_roll_cycle_time_sig,
    rich(),
    Action::PianoRoll(PianoRollAction::CycleTimeSig)
);

parity_test!(
    parity_piano_roll_toggle_poly_mode,
    rich(),
    Action::PianoRoll(PianoRollAction::TogglePolyMode(0))
);

parity_test!(
    parity_piano_roll_adjust_swing,
    rich(),
    Action::PianoRoll(PianoRollAction::AdjustSwing(0.1))
);

#[test]
fn parity_piano_roll_delete_notes_in_region() {
    let mut s = rich();
    // The rich fixture has a note at pitch 60, tick 0
    assert_parity(
        &mut s,
        &Action::PianoRoll(PianoRollAction::DeleteNotesInRegion {
            track: 0,
            start_tick: 0,
            end_tick: 960,
            start_pitch: 0,
            end_pitch: 127,
        }),
    );
}

#[test]
fn parity_piano_roll_paste_notes() {
    let mut s = rich();
    let cn = imbolc_types::ClipboardNote {
        tick_offset: 0,
        pitch_offset: 0,
        duration: 480,
        velocity: 100,
        probability: 1.0,
    };
    assert_parity(
        &mut s,
        &Action::PianoRoll(PianoRollAction::PasteNotes {
            track: 0,
            anchor_tick: 1920,
            anchor_pitch: 72,
            notes: vec![cn],
        }),
    );
}

#[test]
fn parity_piano_roll_play_stop() {
    let mut s = rich();
    // Ensure IO state won't block: piano_roll PlayStop in dispatch checks state.io
    // but projection skips that guard. We need the io state to allow play.
    assert_parity(&mut s, &Action::PianoRoll(PianoRollAction::PlayStop));
}

#[test]
fn parity_piano_roll_play_stop_record() {
    let mut s = rich();
    assert_parity(
        &mut s,
        &Action::PianoRoll(PianoRollAction::PlayStopRecord),
    );
}

// ============================================================================
// Automation tests
// ============================================================================

#[test]
fn parity_automation_add_lane() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::AddLane(AutomationTarget::level(id))),
    );
}

#[test]
fn parity_automation_remove_lane() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::RemoveLane(lane_id)),
    );
}

#[test]
fn parity_automation_toggle_lane_enabled() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::ToggleLaneEnabled(lane_id)),
    );
}

#[test]
fn parity_automation_add_point() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::AddPoint(lane_id, 480, 0.75)),
    );
}

#[test]
fn parity_automation_remove_point() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    // Add a point first
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(480, 0.5);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::RemovePoint(lane_id, 480)),
    );
}

#[test]
fn parity_automation_move_point() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(480, 0.5);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::MovePoint(lane_id, 480, 960, 0.8)),
    );
}

#[test]
fn parity_automation_set_curve_type() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(480, 0.5);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::SetCurveType(
            lane_id,
            480,
            CurveType::Exponential,
        )),
    );
}

#[test]
fn parity_automation_select_lane() {
    let mut s = rich();
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::SelectLane(1)),
    );
}

#[test]
fn parity_automation_clear_lane() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(480, 0.5);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::ClearLane(lane_id)),
    );
}

#[test]
fn parity_automation_toggle_lane_arm() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::ToggleLaneArm(lane_id)),
    );
}

parity_test!(
    parity_automation_arm_all_lanes,
    rich(),
    Action::Automation(AutomationAction::ArmAllLanes)
);

parity_test!(
    parity_automation_disarm_all_lanes,
    rich(),
    Action::Automation(AutomationAction::DisarmAllLanes)
);

#[test]
fn parity_automation_delete_points_in_range() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(100, 0.3);
    s.session
        .automation
        .lane_mut(lane_id)
        .unwrap()
        .add_point(200, 0.6);
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::DeletePointsInRange(lane_id, 0, 300)),
    );
}

#[test]
fn parity_automation_paste_points() {
    let mut s = rich();
    let lane_id = s.session.automation.lanes[0].id;
    assert_parity(
        &mut s,
        &Action::Automation(AutomationAction::PastePoints(
            lane_id,
            0,
            vec![(100, 0.5), (200, 0.8)],
        )),
    );
}

// ============================================================================
// Session tests
// ============================================================================

#[test]
fn parity_session_update() {
    let mut s = minimal();
    let settings = MusicalSettings {
        bpm: 140,
        ..MusicalSettings::default()
    };
    assert_parity(
        &mut s,
        &Action::Session(SessionAction::UpdateSession(settings)),
    );
}

#[test]
fn parity_session_update_live() {
    let mut s = minimal();
    let settings = MusicalSettings {
        bpm: 160,
        ..MusicalSettings::default()
    };
    assert_parity(
        &mut s,
        &Action::Session(SessionAction::UpdateSessionLive(settings)),
    );
}

parity_test!(
    parity_session_adjust_humanize_velocity,
    minimal(),
    Action::Session(SessionAction::AdjustHumanizeVelocity(0.1))
);

parity_test!(
    parity_session_adjust_humanize_timing,
    minimal(),
    Action::Session(SessionAction::AdjustHumanizeTiming(0.1))
);

parity_test!(
    parity_session_toggle_master_mute,
    minimal(),
    Action::Session(SessionAction::ToggleMasterMute)
);

parity_test!(
    parity_session_cycle_theme,
    minimal(),
    Action::Session(SessionAction::CycleTheme)
);

// ============================================================================
// Click track tests
// ============================================================================

parity_test!(
    parity_click_toggle,
    minimal(),
    Action::Click(ClickAction::Toggle)
);

parity_test!(
    parity_click_toggle_mute,
    minimal(),
    Action::Click(ClickAction::ToggleMute)
);

parity_test!(
    parity_click_adjust_volume,
    minimal(),
    Action::Click(ClickAction::AdjustVolume(0.1))
);

parity_test!(
    parity_click_set_volume,
    minimal(),
    Action::Click(ClickAction::SetVolume(0.7))
);

// ============================================================================
// Layer link/unlink tests
// ============================================================================

#[test]
fn parity_link_layer() {
    let mut s = rich();
    let id1 = s.instruments.instruments[0].id;
    let id2 = s.instruments.instruments[1].id;
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::LinkLayer(id1, id2)),
    );
}

#[test]
fn parity_unlink_layer() {
    let mut s = layer_group();
    let id1 = s.instruments.instruments[0].id;
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::UnlinkLayer(id1)),
    );
}

// ============================================================================
// Mixer layer group send tests
// ============================================================================

#[test]
fn parity_mixer_adjust_send_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    // Enable a send
    if let Some(gm) = s.session.mixer.layer_group_mixer_mut(gid) {
        if let Some(send) = gm.sends.values_mut().next() {
            send.enabled = true;
            send.level = 0.5;
        }
    }
    let bus_id = s.session.mixer.layer_group_mixer(gid)
        .and_then(|gm| gm.sends.values().next())
        .map(|s| s.bus_id)
        .unwrap_or(BusId::new(1));
    assert_parity(
        &mut s,
        &Action::Mixer(MixerAction::AdjustSend(bus_id, 0.1)),
    );
}

#[test]
fn parity_mixer_toggle_send_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    let bus_id = s.session.mixer.layer_group_mixer(gid)
        .and_then(|gm| gm.sends.values().next())
        .map(|s| s.bus_id)
        .unwrap_or(BusId::new(1));
    assert_parity(
        &mut s,
        &Action::Mixer(MixerAction::ToggleSend(bus_id)),
    );
}

#[test]
fn parity_mixer_cycle_send_tap_point_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    if let Some(gm) = s.session.mixer.layer_group_mixer_mut(gid) {
        if let Some(send) = gm.sends.values_mut().next() {
            send.enabled = true;
        }
    }
    let bus_id = s.session.mixer.layer_group_mixer(gid)
        .and_then(|gm| gm.sends.values().next())
        .map(|s| s.bus_id)
        .unwrap_or(BusId::new(1));
    assert_parity(
        &mut s,
        &Action::Mixer(MixerAction::CycleSendTapPoint(bus_id)),
    );
}

// ============================================================================
// Mixer cycle output for layer group
// ============================================================================

#[test]
fn parity_mixer_cycle_output_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::CycleOutput));
}

#[test]
fn parity_mixer_cycle_output_reverse_layer_group() {
    let mut s = layer_group();
    let gid = group_id(&s);
    s.session.mixer.selection = MixerSelection::LayerGroup(gid);
    assert_parity(&mut s, &Action::Mixer(MixerAction::CycleOutputReverse));
}

// ============================================================================
// No-op / passthrough action tests
// ============================================================================

parity_test!(parity_none, minimal(), Action::None);

#[test]
fn parity_play_note_noop() {
    // PlayNote produces audio side effects only, no state mutation
    let mut s = one_instrument();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::PlayNote(60, 100)),
    );
}

#[test]
fn parity_play_notes_noop() {
    let mut s = one_instrument();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::PlayNotes(vec![60, 64, 67], 100)),
    );
}

// ============================================================================
// Non-projectable action tests
// ============================================================================

#[test]
fn not_projectable_undo() {
    assert_not_projectable(&Action::Undo);
}

#[test]
fn not_projectable_redo() {
    assert_not_projectable(&Action::Redo);
}

#[test]
fn not_projectable_arrangement() {
    assert_not_projectable(&Action::Arrangement(
        imbolc_types::ArrangementAction::TogglePlayMode,
    ));
}

#[test]
fn not_projectable_sequencer() {
    assert_not_projectable(&Action::Sequencer(
        imbolc_types::SequencerAction::PlayStop,
    ));
}

#[test]
fn not_projectable_session_save() {
    assert_not_projectable(&Action::Session(SessionAction::Save));
}

#[test]
fn not_projectable_session_load() {
    assert_not_projectable(&Action::Session(SessionAction::Load));
}

#[test]
fn not_projectable_session_new() {
    assert_not_projectable(&Action::Session(SessionAction::NewProject));
}

#[test]
fn not_projectable_automation_toggle_recording() {
    assert_not_projectable(&Action::Automation(AutomationAction::ToggleRecording));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn parity_action_on_nonexistent_instrument() {
    // Both paths should no-op when targeting a nonexistent instrument
    let mut s = minimal();
    let bogus_id = InstrumentId::new(99999);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::ToggleFilter(bogus_id)),
    );
}

#[test]
fn parity_filter_cutoff_extreme_delta() {
    // Boundary clamping should match
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::AdjustFilterCutoff(id, 99999.0)),
    );
}

#[test]
fn parity_filter_resonance_extreme_delta() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::AdjustFilterResonance(id, -99999.0)),
    );
}

#[test]
fn parity_select_out_of_bounds() {
    // Selecting beyond instruments count — both should no-op/clamp identically
    let mut s = one_instrument();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Select(999)),
    );
}

#[test]
fn parity_bus_remove_nonexistent() {
    let mut s = minimal();
    // Bus 200 doesn't exist — both paths should no-op
    assert_parity(&mut s, &Action::Bus(BusAction::Remove(BusId::new(200))));
}

#[test]
fn parity_mixer_move_with_empty_instruments() {
    let mut s = minimal();
    s.session.mixer.selection = MixerSelection::Instrument(0);
    assert_parity(&mut s, &Action::Mixer(MixerAction::Move(1)));
}

#[test]
fn parity_add_two_instruments() {
    // Ensure sequential adds produce matching state
    let mut s = minimal();
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Add(SourceType::Saw)),
    );
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::Add(SourceType::Sin)),
    );
}

#[test]
fn parity_swing_grid() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::Instrument(InstrumentAction::SetTrackSwingGrid(
            id,
            Some(imbolc_types::SwingGrid::Eighths),
        )),
    );
}

// ============================================================================
// VstParam tests (non-file-IO variants)
// ============================================================================

#[test]
fn parity_vst_param_set() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::VstParam(VstParamAction::SetParam(id, VstTarget::Source, 0, 0.5)),
    );
}

#[test]
fn parity_vst_param_adjust() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::VstParam(VstParamAction::AdjustParam(id, VstTarget::Source, 0, 0.1)),
    );
}

#[test]
fn parity_vst_param_reset() {
    let mut s = one_instrument();
    let id = first_id(&s);
    assert_parity(
        &mut s,
        &Action::VstParam(VstParamAction::ResetParam(id, VstTarget::Source, 0)),
    );
}

#[test]
fn not_projectable_vst_discover() {
    assert_not_projectable(&Action::VstParam(VstParamAction::DiscoverParams(
        InstrumentId::new(1),
        VstTarget::Source,
    )));
}

#[test]
fn not_projectable_vst_save_state() {
    assert_not_projectable(&Action::VstParam(VstParamAction::SaveState(
        InstrumentId::new(1),
        VstTarget::Source,
    )));
}
