use crate::action::{BusAction, DispatchResult, LayerGroupAction, NavIntent};
use imbolc_audio::AudioHandle;
use crate::dispatch::side_effects::AudioSideEffect;
use crate::state::{AppState, OutputTarget};

/// Dispatch bus management actions
pub fn dispatch_bus(action: &BusAction, state: &mut AppState) -> DispatchResult {
    let mut result = DispatchResult::none();

    match action {
        BusAction::Add => {
            if let Some(_new_id) = state.session.add_bus() {
                // Sync all instruments with the new bus
                let bus_ids: Vec<u8> = state.session.bus_ids().collect();
                for inst in &mut state.instruments.instruments {
                    inst.sync_sends_with_buses(&bus_ids);
                }
                // Sync layer group mixers with the new bus
                for gm in &mut state.session.mixer.layer_group_mixers {
                    gm.sync_sends_with_buses(&bus_ids);
                }
                result.audio_dirty.routing = true;
                result.audio_dirty.session = true;
            }
        }

        BusAction::Remove(bus_id) => {
            let bus_id = *bus_id;

            // Check if bus exists
            if state.session.bus(bus_id).is_none() {
                return result;
            }

            // Reset instruments that output to this bus
            for inst in &mut state.instruments.instruments {
                if inst.output_target == OutputTarget::Bus(bus_id) {
                    inst.output_target = OutputTarget::Master;
                }
                // Disable sends to this bus
                inst.disable_send_for_bus(bus_id);
            }

            // Reset layer group mixers that output to this bus and disable their sends
            for gm in &mut state.session.mixer.layer_group_mixers {
                if gm.output_target == OutputTarget::Bus(bus_id) {
                    gm.output_target = OutputTarget::Master;
                }
                gm.disable_send_for_bus(bus_id);
            }

            // Remove automation lanes for this bus
            state.session.automation.remove_lanes_for_bus(bus_id);

            // Remove the bus
            state.session.remove_bus(bus_id);

            // Update mixer selection if it was pointing to the removed bus
            if let crate::state::MixerSelection::Bus(id) = state.session.mixer.selection {
                if id == bus_id {
                    // Select first remaining bus, or Master if none
                    let first_bus = state.session.bus_ids().next();
                    state.session.mixer.selection = first_bus
                        .map(crate::state::MixerSelection::Bus)
                        .unwrap_or(crate::state::MixerSelection::Master);
                }
            }

            result.audio_dirty.routing = true;
            result.audio_dirty.session = true;
        }

        BusAction::Rename(bus_id, name) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                bus.name = name.clone();
            }
        }

        BusAction::AddEffect(bus_id, effect_type) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                bus.add_effect(*effect_type);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
            result.nav.push(NavIntent::Pop);
        }

        BusAction::RemoveEffect(bus_id, effect_id) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                bus.remove_effect(*effect_id);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        BusAction::MoveEffect(bus_id, effect_id, direction) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                bus.move_effect(*effect_id, *direction);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        BusAction::ToggleEffectBypass(bus_id, effect_id) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                if let Some(effect) = bus.effect_by_id_mut(*effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        BusAction::AdjustEffectParam(bus_id, effect_id, param_idx, delta) => {
            let mut targeted_value: Option<f32> = None;
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                if let Some(effect) = bus.effect_by_id_mut(*effect_id) {
                    if let Some(param) = effect.params.get_mut(*param_idx) {
                        let range = param.max - param.min;
                        match &mut param.value {
                            crate::state::ParamValue::Float(v) => {
                                *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                                targeted_value = Some(*v);
                            }
                            crate::state::ParamValue::Int(v) => {
                                *v = (*v + (delta * range * 0.02) as i32).clamp(param.min as i32, param.max as i32);
                            }
                            crate::state::ParamValue::Bool(b) => {
                                *b = !*b;
                            }
                        }
                    }
                }
            }
            result.audio_dirty.session = true;
            if let Some(value) = targeted_value {
                result.audio_dirty.bus_effect_param = Some((*bus_id, *effect_id, *param_idx, value));
            }
        }
    }

    result
}

/// Dispatch layer group actions
pub fn dispatch_layer_group(
    action: &LayerGroupAction,
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
) -> DispatchResult {
    let mut result = DispatchResult::none();

    match action {
        LayerGroupAction::AddEffect(group_id, effect_type) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                gm.add_effect(*effect_type);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
            result.nav.push(NavIntent::Pop);
        }

        LayerGroupAction::RemoveEffect(group_id, effect_id) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                gm.remove_effect(*effect_id);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        LayerGroupAction::MoveEffect(group_id, effect_id, direction) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                gm.move_effect(*effect_id, *direction);
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        LayerGroupAction::ToggleEffectBypass(group_id, effect_id) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(effect) = gm.effect_by_id_mut(*effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        LayerGroupAction::AdjustEffectParam(group_id, effect_id, param_idx, delta) => {
            let mut targeted_value: Option<f32> = None;
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(effect) = gm.effect_by_id_mut(*effect_id) {
                    if let Some(param) = effect.params.get_mut(*param_idx) {
                        let range = param.max - param.min;
                        match &mut param.value {
                            crate::state::ParamValue::Float(v) => {
                                *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                                targeted_value = Some(*v);
                            }
                            crate::state::ParamValue::Int(v) => {
                                *v = (*v + (delta * range * 0.02) as i32).clamp(param.min as i32, param.max as i32);
                            }
                            crate::state::ParamValue::Bool(b) => {
                                *b = !*b;
                            }
                        }
                    }
                }
            }
            result.audio_dirty.session = true;
            if let Some(value) = targeted_value {
                result.audio_dirty.layer_group_effect_param = Some((*group_id, *effect_id, *param_idx, value));
            }
        }

        LayerGroupAction::ToggleEq(group_id) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                gm.toggle_eq();
            }
            result.audio_dirty.routing_bus_processing = true;
            result.audio_dirty.session = true;
        }

        LayerGroupAction::SetEqParam(group_id, band_idx, param_name, value) => {
            if let Some(gm) = state.session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(ref mut eq) = gm.eq {
                    if let Some(band) = eq.bands.get_mut(*band_idx) {
                        match param_name.as_str() {
                            "freq" => band.freq = value.clamp(20.0, 20000.0),
                            "gain" => band.gain = value.clamp(-24.0, 24.0),
                            "q" => band.q = value.clamp(0.1, 10.0),
                            "on" => band.enabled = *value > 0.5,
                            _ => {}
                        }
                    }
                }
            }
            // Send real-time param update to audio engine
            if audio.is_running() {
                let sc_param = format!("b{}_{}", band_idx, param_name);
                let sc_value = if param_name == "q" { 1.0 / value } else { *value };
                effects.push(AudioSideEffect::SetLayerGroupEqParam {
                    group_id: *group_id,
                    param: sc_param,
                    value: sc_value,
                });
            }
            result.audio_dirty.session = true;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SourceType;
    use crate::state::automation::AutomationTarget;

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
    fn add_bus_syncs_instrument_sends() {
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        let initial_sends = state.instruments.instruments[0].sends.len();

        dispatch_bus(&BusAction::Add, &mut state);

        assert_eq!(state.instruments.instruments[0].sends.len(), initial_sends + 1);
    }

    #[test]
    fn remove_bus_resets_instrument_output() {
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        state.instruments.instruments[0].output_target = OutputTarget::Bus(3);

        dispatch_bus(&BusAction::Remove(3), &mut state);

        assert_eq!(state.instruments.instruments[0].output_target, OutputTarget::Master);
    }

    #[test]
    fn remove_bus_disables_sends() {
        let mut state = setup();
        state.add_instrument(SourceType::Saw);
        // Enable send to bus 3
        if let Some(send) = state.instruments.instruments[0].sends.iter_mut().find(|s| s.bus_id == 3) {
            send.enabled = true;
            send.level = 0.5;
        }

        dispatch_bus(&BusAction::Remove(3), &mut state);

        // Send should be disabled but still exist
        let send = state.instruments.instruments[0].sends.iter().find(|s| s.bus_id == 3);
        assert!(send.is_some());
        assert!(!send.unwrap().enabled);
    }

    #[test]
    fn remove_bus_clears_automation() {
        let mut state = setup();
        state.session.automation.add_lane(AutomationTarget::bus_level(3));
        assert!(!state.session.automation.lanes.is_empty());

        dispatch_bus(&BusAction::Remove(3), &mut state);

        assert!(state.session.automation.lanes.is_empty());
    }

    #[test]
    fn rename_bus() {
        let mut state = setup();

        dispatch_bus(&BusAction::Rename(1, "Drums".to_string()), &mut state);

        assert_eq!(state.session.bus(1).unwrap().name, "Drums");
    }

    // ========================================================================
    // Bus effect dispatch tests
    // ========================================================================

    #[test]
    fn bus_add_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        let result = dispatch_bus(&BusAction::AddEffect(1, EffectType::Reverb), &mut state);
        let bus = state.session.bus(1).unwrap();
        assert_eq!(bus.effects.len(), 1);
        assert_eq!(bus.effects[0].effect_type, EffectType::Reverb);
        assert!(result.audio_dirty.routing_bus_processing);
    }

    #[test]
    fn bus_remove_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(&BusAction::AddEffect(1, EffectType::Reverb), &mut state);
        let effect_id = state.session.bus(1).unwrap().effects[0].id;

        let result = dispatch_bus(&BusAction::RemoveEffect(1, effect_id), &mut state);
        assert!(state.session.bus(1).unwrap().effects.is_empty());
        assert!(result.audio_dirty.routing_bus_processing);
    }

    #[test]
    fn bus_move_effect_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(&BusAction::AddEffect(1, EffectType::Reverb), &mut state);
        dispatch_bus(&BusAction::AddEffect(1, EffectType::Delay), &mut state);
        let id0 = state.session.bus(1).unwrap().effects[0].id;

        dispatch_bus(&BusAction::MoveEffect(1, id0, 1), &mut state);
        let bus = state.session.bus(1).unwrap();
        assert_eq!(bus.effects[1].id, id0);
    }

    #[test]
    fn bus_toggle_effect_bypass_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(&BusAction::AddEffect(1, EffectType::Reverb), &mut state);
        let effect_id = state.session.bus(1).unwrap().effects[0].id;
        assert!(state.session.bus(1).unwrap().effects[0].enabled);

        dispatch_bus(&BusAction::ToggleEffectBypass(1, effect_id), &mut state);
        assert!(!state.session.bus(1).unwrap().effects[0].enabled);
    }

    #[test]
    fn bus_adjust_effect_param_dispatch() {
        use crate::state::EffectType;
        let mut state = setup();
        dispatch_bus(&BusAction::AddEffect(1, EffectType::Reverb), &mut state);
        let effect_id = state.session.bus(1).unwrap().effects[0].id;
        let initial_val = match &state.session.bus(1).unwrap().effects[0].params[0].value {
            crate::state::ParamValue::Float(v) => *v,
            _ => panic!("expected float"),
        };

        let result = dispatch_bus(&BusAction::AdjustEffectParam(1, effect_id, 0, 1.0), &mut state);
        let new_val = match &state.session.bus(1).unwrap().effects[0].params[0].value {
            crate::state::ParamValue::Float(v) => *v,
            _ => panic!("expected float"),
        };
        assert_ne!(initial_val, new_val);
        assert!(result.audio_dirty.bus_effect_param.is_some());
    }

    // ========================================================================
    // LayerGroup effect dispatch tests
    // ========================================================================

    use imbolc_audio::AudioHandle;

    fn setup_with_audio() -> (AppState, AudioHandle, Vec<AudioSideEffect>) {
        (AppState::new(), AudioHandle::new(), Vec::new())
    }

    #[test]
    fn layer_group_add_effect_dispatch() {
        use crate::state::EffectType;
        let (mut state, audio, mut effects) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[1, 2]);

        let result = dispatch_layer_group(&LayerGroupAction::AddEffect(1, EffectType::TapeComp), &mut state, &audio, &mut effects);
        let gm = state.session.mixer.layer_group_mixer(1).unwrap();
        assert_eq!(gm.effects.len(), 1);
        assert_eq!(gm.effects[0].effect_type, EffectType::TapeComp);
        assert!(result.audio_dirty.routing_bus_processing);
    }

    #[test]
    fn layer_group_remove_effect_dispatch() {
        use crate::state::EffectType;
        let (mut state, audio, mut effects) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state.session.mixer.layer_group_mixer_mut(1).unwrap().add_effect(EffectType::Limiter);
        let effect_id = state.session.mixer.layer_group_mixer(1).unwrap().effects[0].id;

        dispatch_layer_group(&LayerGroupAction::RemoveEffect(1, effect_id), &mut state, &audio, &mut effects);
        assert!(state.session.mixer.layer_group_mixer(1).unwrap().effects.is_empty());
    }

    #[test]
    fn layer_group_toggle_bypass_dispatch() {
        use crate::state::EffectType;
        let (mut state, audio, mut effects) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state.session.mixer.layer_group_mixer_mut(1).unwrap().add_effect(EffectType::Reverb);
        let effect_id = state.session.mixer.layer_group_mixer(1).unwrap().effects[0].id;

        dispatch_layer_group(&LayerGroupAction::ToggleEffectBypass(1, effect_id), &mut state, &audio, &mut effects);
        assert!(!state.session.mixer.layer_group_mixer(1).unwrap().effects[0].enabled);
    }

    #[test]
    fn layer_group_toggle_eq_dispatch() {
        let (mut state, audio, mut effects) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        assert!(state.session.mixer.layer_group_mixer(1).unwrap().eq().is_some());

        let result = dispatch_layer_group(&LayerGroupAction::ToggleEq(1), &mut state, &audio, &mut effects);
        assert!(state.session.mixer.layer_group_mixer(1).unwrap().eq().is_none());
        assert!(result.audio_dirty.routing_bus_processing);
        assert!(result.audio_dirty.session);

        dispatch_layer_group(&LayerGroupAction::ToggleEq(1), &mut state, &audio, &mut effects);
        assert!(state.session.mixer.layer_group_mixer(1).unwrap().eq().is_some());
    }

    #[test]
    fn layer_group_set_eq_param_dispatch() {
        let (mut state, audio, mut effects) = setup_with_audio();
        state.session.mixer.add_layer_group_mixer(1, &[]);

        let result = dispatch_layer_group(
            &LayerGroupAction::SetEqParam(1, 0, "gain".to_string(), 6.0),
            &mut state, &audio, &mut effects,
        );
        let eq = state.session.mixer.layer_group_mixer(1).unwrap().eq().unwrap();
        assert_eq!(eq.bands[0].gain, 6.0);
        assert!(result.audio_dirty.session);
    }
}
