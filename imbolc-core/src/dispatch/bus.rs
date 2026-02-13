use crate::action::{
    AudioEffect, BusAction, DispatchResult, EqParamKind, LayerGroupAction, NavIntent,
};
use crate::state::AppState;
use imbolc_audio::AudioHandle;
use imbolc_types::DomainAction;

fn reduce(state: &mut AppState, action: &BusAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Bus(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

/// Dispatch bus management actions
pub fn dispatch_bus(action: &BusAction, state: &mut AppState) -> DispatchResult {
    // Check bus existence before reducer for Remove (to skip audio effects on no-op)
    let bus_exists = match action {
        BusAction::Remove(bus_id) => state.session.bus(*bus_id).is_some(),
        _ => true,
    };

    // Delegate pure state mutation to the shared reducer
    reduce(state, action);

    // Orchestration: AudioEffects and navigation
    let mut result = DispatchResult::none();

    match action {
        BusAction::Add => {
            result.audio_effects.push(AudioEffect::RebuildRouting);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        BusAction::Remove(_) => {
            if bus_exists {
                result.audio_effects.push(AudioEffect::RebuildRouting);
                result.audio_effects.push(AudioEffect::RebuildSession);
            }
        }

        BusAction::Rename(_, _) => {}

        BusAction::AddEffect(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
            result.nav.push(NavIntent::Pop);
        }

        BusAction::RemoveEffect(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        BusAction::MoveEffect(_, _, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        BusAction::ToggleEffectBypass(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        BusAction::AdjustEffectParam(bus_id, effect_id, param_idx, _delta) => {
            result.audio_effects.push(AudioEffect::RebuildSession);
            // Read back the param value after reducer mutation for targeted audio update
            let targeted_value = state
                .session
                .bus(*bus_id)
                .and_then(|bus| bus.effect_chain.effect_by_id(*effect_id))
                .and_then(|effect| effect.params.get(param_idx.get()))
                .map(|param| param.value.to_f32());
            if let Some(value) = targeted_value {
                result.audio_effects.push(AudioEffect::SetBusEffectParam(
                    *bus_id, *effect_id, *param_idx, value,
                ));
            }
        }
    }

    result
}

fn reduce_lg(state: &mut AppState, action: &LayerGroupAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::LayerGroup(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

/// Dispatch layer group actions
pub fn dispatch_layer_group(
    action: &LayerGroupAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    // Delegate pure state mutation to the shared reducer
    reduce_lg(state, action);

    // Orchestration: AudioEffects, real-time audio updates, navigation
    let mut result = DispatchResult::none();

    match action {
        LayerGroupAction::AddEffect(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
            result.nav.push(NavIntent::Pop);
        }

        LayerGroupAction::RemoveEffect(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        LayerGroupAction::MoveEffect(_, _, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        LayerGroupAction::ToggleEffectBypass(_, _) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        LayerGroupAction::AdjustEffectParam(group_id, effect_id, param_idx, _delta) => {
            result.audio_effects.push(AudioEffect::RebuildSession);
            // Read back the param value after reducer mutation for targeted audio update
            let targeted_value = state
                .session
                .mixer
                .layer_group_mixer(*group_id)
                .and_then(|gm| gm.effect_chain.effect_by_id(*effect_id))
                .and_then(|effect| effect.params.get(param_idx.get()))
                .map(|param| param.value.to_f32());
            if let Some(value) = targeted_value {
                result
                    .audio_effects
                    .push(AudioEffect::SetLayerGroupEffectParam(
                        *group_id, *effect_id, *param_idx, value,
                    ));
            }
        }

        LayerGroupAction::ToggleEq(_) => {
            result.audio_effects.push(AudioEffect::RebuildBusProcessing);
            result.audio_effects.push(AudioEffect::RebuildSession);
        }

        LayerGroupAction::SetEqParam(group_id, band_idx, param, value) => {
            // Send real-time param update to audio engine
            if audio.is_running() {
                let sc_param = format!("b{}_{}", band_idx, param.as_str());
                let sc_value = if *param == EqParamKind::Q {
                    1.0 / value
                } else {
                    *value
                };
                let _ = audio.set_layer_group_eq_param(*group_id, &sc_param, sc_value);
            }
            result.audio_effects.push(AudioEffect::RebuildSession);
        }
    }

    result
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use crate::state::automation::AutomationTarget;
    use crate::state::SourceType;
    use imbolc_types::{BusId, OutputTarget, ParamIndex};

    fn setup() -> AppState {
        AppState::new()
    }

    #[test]
    fn add_bus() {
        let mut state = setup();
        let initial_count = state.session.mixer.buses.len();

        dispatch_bus(&BusAction::Add, &mut state);

        assert_eq!(state.session.mixer.buses.len(), initial_count + 1);
    }

    #[test]
    fn add_bus_creates_bus() {
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        let initial_bus_count = state.session.mixer.buses.len();

        dispatch_bus(&BusAction::Add, &mut state);

        assert_eq!(state.session.mixer.buses.len(), initial_bus_count + 1);
        // Sends are lazily created now, so instrument sends remain empty
        assert!(state.instruments.instruments[0].mixer.sends.is_empty());
    }

    #[test]
    fn remove_bus_resets_instrument_output() {
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        state.instruments.instruments[0].mixer.output_target = OutputTarget::Bus(BusId::new(3));

        dispatch_bus(&BusAction::Remove(BusId::new(3)), &mut state);

        assert_eq!(
            state.instruments.instruments[0].mixer.output_target,
            OutputTarget::Master
        );
    }

    #[test]
    fn remove_bus_disables_sends() {
        use imbolc_types::MixerSend;
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        // Insert and enable a send to bus 3
        state.instruments.instruments[0].mixer.sends.insert(
            BusId::new(3),
            MixerSend {
                bus_id: BusId::new(3),
                level: 0.5,
                enabled: true,
                tap_point: Default::default(),
            },
        );

        dispatch_bus(&BusAction::Remove(BusId::new(3)), &mut state);

        // Send should be disabled but still exist
        let send = state.instruments.instruments[0]
            .mixer
            .sends
            .get(&BusId::new(3));
        assert!(send.is_some());
        assert!(!send.unwrap().enabled);
    }

    #[test]
    fn remove_bus_clears_automation() {
        let mut state = setup();
        state
            .session
            .automation
            .add_lane(AutomationTarget::bus_level(BusId::new(3)));
        assert!(!state.session.automation.lanes.is_empty());

        dispatch_bus(&BusAction::Remove(BusId::new(3)), &mut state);

        assert!(state.session.automation.lanes.is_empty());
    }

    #[test]
    fn rename_bus() {
        let mut state = setup();

        dispatch_bus(
            &BusAction::Rename(BusId::new(1), "Drums".to_string()),
            &mut state,
        );

        assert_eq!(state.session.bus(BusId::new(1)).unwrap().name, "Drums");
    }

    // ========================================================================
    // Bus effect dispatch tests
    // ========================================================================

    #[test]
    fn bus_add_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        let result = dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Reverb),
            &mut state,
        );
        let bus = state.session.bus(BusId::new(1)).unwrap();
        assert_eq!(bus.effect_chain.effects.len(), 1);
        assert_eq!(bus.effect_chain.effects[0].effect_type, EffectType::Reverb);
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildBusProcessing));
    }

    #[test]
    fn bus_remove_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Reverb),
            &mut state,
        );
        let effect_id = state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .id;

        let result = dispatch_bus(
            &BusAction::RemoveEffect(BusId::new(1), effect_id),
            &mut state,
        );
        assert!(state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects
            .is_empty());
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildBusProcessing));
    }

    #[test]
    fn bus_move_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Reverb),
            &mut state,
        );
        dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Delay),
            &mut state,
        );
        let id0 = state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .id;

        dispatch_bus(&BusAction::MoveEffect(BusId::new(1), id0, 1), &mut state);
        let bus = state.session.bus(BusId::new(1)).unwrap();
        assert_eq!(bus.effect_chain.effects[1].id, id0);
    }

    #[test]
    fn bus_toggle_effect_bypass_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Reverb),
            &mut state,
        );
        let effect_id = state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .id;
        assert!(
            state
                .session
                .bus(BusId::new(1))
                .unwrap()
                .effect_chain
                .effects[0]
                .enabled
        );

        dispatch_bus(
            &BusAction::ToggleEffectBypass(BusId::new(1), effect_id),
            &mut state,
        );
        assert!(
            !state
                .session
                .bus(BusId::new(1))
                .unwrap()
                .effect_chain
                .effects[0]
                .enabled
        );
    }

    #[test]
    fn bus_adjust_effect_param_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(
            &BusAction::AddEffect(BusId::new(1), EffectType::Reverb),
            &mut state,
        );
        let effect_id = state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .id;
        let initial_val = match &state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .params[0]
            .value
        {
            crate::state::ParamValue::Float(v) => *v,
            _ => panic!("expected float"),
        };

        let result = dispatch_bus(
            &BusAction::AdjustEffectParam(BusId::new(1), effect_id, ParamIndex::new(0), 1.0),
            &mut state,
        );
        let new_val = match &state
            .session
            .bus(BusId::new(1))
            .unwrap()
            .effect_chain
            .effects[0]
            .params[0]
            .value
        {
            crate::state::ParamValue::Float(v) => *v,
            _ => panic!("expected float"),
        };
        assert_ne!(initial_val, new_val);
        assert!(result
            .audio_effects
            .iter()
            .any(|e| matches!(e, AudioEffect::SetBusEffectParam(..))));
    }

    // ========================================================================
    // LayerGroup effect dispatch tests
    // ========================================================================

    use imbolc_audio::AudioHandle;

    fn setup_with_audio() -> (AppState, AudioHandle) {
        (AppState::new(), AudioHandle::new())
    }

    #[test]
    fn layer_group_add_effect_dispatch() {
        use crate::state::EffectType;
        let (mut state, mut audio) = setup_with_audio();
        state
            .session
            .mixer
            .add_layer_group_mixer(1, &[BusId::new(1), BusId::new(2)]);

        let result = dispatch_layer_group(
            &LayerGroupAction::AddEffect(1, EffectType::TapeComp),
            &mut state,
            &mut audio,
        );
        let gm = state.session.mixer.layer_group_mixer(1).unwrap();
        assert_eq!(gm.effect_chain.effects.len(), 1);
        assert_eq!(gm.effect_chain.effects[0].effect_type, EffectType::TapeComp);
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildBusProcessing));
    }

    #[test]
    fn layer_group_remove_effect_dispatch() {
        use crate::state::EffectType;
        let (mut state, mut audio) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state
            .session
            .mixer
            .layer_group_mixer_mut(1)
            .unwrap()
            .effect_chain
            .add_effect(EffectType::Limiter);
        let effect_id = state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .effect_chain
            .effects[0]
            .id;

        dispatch_layer_group(
            &LayerGroupAction::RemoveEffect(1, effect_id),
            &mut state,
            &mut audio,
        );
        assert!(state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .effect_chain
            .effects
            .is_empty());
    }

    #[test]
    fn layer_group_toggle_bypass_dispatch() {
        use crate::state::EffectType;
        let (mut state, mut audio) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state
            .session
            .mixer
            .layer_group_mixer_mut(1)
            .unwrap()
            .effect_chain
            .add_effect(EffectType::Reverb);
        let effect_id = state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .effect_chain
            .effects[0]
            .id;

        dispatch_layer_group(
            &LayerGroupAction::ToggleEffectBypass(1, effect_id),
            &mut state,
            &mut audio,
        );
        assert!(
            !state
                .session
                .mixer
                .layer_group_mixer(1)
                .unwrap()
                .effect_chain
                .effects[0]
                .enabled
        );
    }

    #[test]
    fn layer_group_toggle_eq_dispatch() {
        let (mut state, mut audio) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        assert!(state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .eq()
            .is_some());

        let result = dispatch_layer_group(&LayerGroupAction::ToggleEq(1), &mut state, &mut audio);
        assert!(state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .eq()
            .is_none());
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildBusProcessing));
        assert!(result.audio_effects.contains(&AudioEffect::RebuildSession));

        dispatch_layer_group(&LayerGroupAction::ToggleEq(1), &mut state, &mut audio);
        assert!(state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .eq()
            .is_some());
    }

    #[test]
    fn layer_group_set_eq_param_dispatch() {
        let (mut state, mut audio) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);

        let result = dispatch_layer_group(
            &LayerGroupAction::SetEqParam(1, 0, EqParamKind::Gain, 6.0),
            &mut state,
            &mut audio,
        );
        let eq = state
            .session
            .mixer
            .layer_group_mixer(1)
            .unwrap()
            .eq()
            .unwrap();
        assert_eq!(eq.bands[0].gain, 6.0);
        assert!(result.audio_effects.contains(&AudioEffect::RebuildSession));
    }
}
