use crate::{
    BusAction, EqParamKind, InstrumentState, LayerGroupAction,
    MixerSelection, OutputTarget, SessionState,
};

pub(super) fn reduce_bus(
    action: &BusAction,
    session: &mut SessionState,
    instruments: &mut InstrumentState,
) -> bool {
    match action {
        BusAction::Add => {
            session.add_bus();
            true
        }
        BusAction::Remove(bus_id) => {
            let bus_id = *bus_id;
            if session.bus(bus_id).is_none() {
                return true;
            }
            for inst in &mut instruments.instruments {
                if inst.mixer.output_target == OutputTarget::Bus(bus_id) {
                    inst.mixer.output_target = OutputTarget::Master;
                }
                inst.disable_send_for_bus(bus_id);
            }
            for gm in &mut session.mixer.layer_group_mixers {
                if gm.output_target == OutputTarget::Bus(bus_id) {
                    gm.output_target = OutputTarget::Master;
                }
                gm.disable_send_for_bus(bus_id);
            }
            session.automation.remove_lanes_for_bus(bus_id);
            session.remove_bus(bus_id);
            if let MixerSelection::Bus(id) = session.mixer.selection {
                if id == bus_id {
                    let first_bus = session.bus_ids().next();
                    session.mixer.selection = first_bus
                        .map(MixerSelection::Bus)
                        .unwrap_or(MixerSelection::Master);
                }
            }
            true
        }
        BusAction::Rename(bus_id, name) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                bus.name = name.clone();
            }
            true
        }
        BusAction::AddEffect(bus_id, effect_type) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                bus.effect_chain.add_effect(*effect_type);
            }
            true
        }
        BusAction::RemoveEffect(bus_id, effect_id) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                bus.effect_chain.remove_effect(*effect_id);
            }
            true
        }
        BusAction::MoveEffect(bus_id, effect_id, direction) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                bus.effect_chain.move_effect(*effect_id, *direction);
            }
            true
        }
        BusAction::ToggleEffectBypass(bus_id, effect_id) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                if let Some(effect) = bus.effect_chain.effect_by_id_mut(*effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            true
        }
        BusAction::AdjustEffectParam(bus_id, effect_id, param_idx, delta) => {
            if let Some(bus) = session.bus_mut(*bus_id) {
                if let Some(effect) = bus.effect_chain.effect_by_id_mut(*effect_id) {
                    if let Some(param) = effect.params.get_mut(param_idx.get()) {
                        let range = param.max - param.min;
                        match &mut param.value {
                            crate::ParamValue::Float(ref mut v) => {
                                *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                            }
                            crate::ParamValue::Int(ref mut v) => {
                                *v = (*v + (delta * range * 0.02) as i32).clamp(param.min as i32, param.max as i32);
                            }
                            crate::ParamValue::Bool(ref mut b) => {
                                *b = !*b;
                            }
                        }
                    }
                }
            }
            true
        }
    }
}

pub(super) fn reduce_layer_group(
    action: &LayerGroupAction,
    session: &mut SessionState,
) -> bool {
    match action {
        LayerGroupAction::AddEffect(group_id, effect_type) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                gm.effect_chain.add_effect(*effect_type);
            }
            true
        }
        LayerGroupAction::RemoveEffect(group_id, effect_id) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                gm.effect_chain.remove_effect(*effect_id);
            }
            true
        }
        LayerGroupAction::MoveEffect(group_id, effect_id, direction) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                gm.effect_chain.move_effect(*effect_id, *direction);
            }
            true
        }
        LayerGroupAction::ToggleEffectBypass(group_id, effect_id) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(effect) = gm.effect_chain.effect_by_id_mut(*effect_id) {
                    effect.enabled = !effect.enabled;
                }
            }
            true
        }
        LayerGroupAction::AdjustEffectParam(group_id, effect_id, param_idx, delta) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(effect) = gm.effect_chain.effect_by_id_mut(*effect_id) {
                    if let Some(param) = effect.params.get_mut(param_idx.get()) {
                        let range = param.max - param.min;
                        match &mut param.value {
                            crate::ParamValue::Float(ref mut v) => {
                                *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                            }
                            crate::ParamValue::Int(ref mut v) => {
                                *v = (*v + (delta * range * 0.02) as i32).clamp(param.min as i32, param.max as i32);
                            }
                            crate::ParamValue::Bool(ref mut b) => {
                                *b = !*b;
                            }
                        }
                    }
                }
            }
            true
        }
        LayerGroupAction::ToggleEq(group_id) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                gm.toggle_eq();
            }
            true
        }
        LayerGroupAction::SetEqParam(group_id, band_idx, param, value) => {
            if let Some(gm) = session.mixer.layer_group_mixer_mut(*group_id) {
                if let Some(ref mut eq) = gm.eq {
                    if let Some(band) = eq.bands.get_mut(*band_idx) {
                        match param {
                            EqParamKind::Freq => band.freq = value.clamp(20.0, 20000.0),
                            EqParamKind::Gain => band.gain = value.clamp(-24.0, 24.0),
                            EqParamKind::Q => band.q = value.clamp(0.1, 10.0),
                            EqParamKind::Enabled => band.enabled = *value > 0.5,
                        }
                    }
                }
            }
            true
        }
    }
}
