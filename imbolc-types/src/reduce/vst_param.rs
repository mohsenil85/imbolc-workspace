use crate::{
    EffectType, Instrument, InstrumentState, SessionState, SourceExtra, SourceType, VstParamAction,
    VstTarget,
};

pub(super) fn reduce(
    action: &VstParamAction,
    instruments: &mut InstrumentState,
    session: &SessionState,
) -> bool {
    match action {
        VstParamAction::SetParam(instrument_id, target, param_index, value) => {
            let value = value.clamp(0.0, 1.0);
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(values) = get_param_values_mut(instrument, *target) {
                    if let Some(entry) = values.iter_mut().find(|(idx, _)| *idx == *param_index) {
                        entry.1 = value;
                    } else {
                        values.push((*param_index, value));
                    }
                }
            }
            true
        }
        VstParamAction::AdjustParam(instrument_id, target, param_index, delta) => {
            let current = instruments
                .instrument(*instrument_id)
                .map(|inst| {
                    let values = match *target {
                        VstTarget::Source => inst.vst_source_params(),
                        VstTarget::Effect(effect_id) => inst
                            .effect_by_id(effect_id)
                            .map(|e| e.vst_param_values.as_slice())
                            .unwrap_or(&[]),
                    };
                    values
                        .iter()
                        .find(|(idx, _)| *idx == *param_index)
                        .map(|(_, v)| *v)
                        .unwrap_or_else(|| {
                            if let Some(plugin_id) = get_vst_plugin_id(inst, *target) {
                                if let Some(plugin) = session.vst_plugins.get(plugin_id) {
                                    if let Some(spec) =
                                        plugin.params.iter().find(|p| p.index == *param_index)
                                    {
                                        return spec.default;
                                    }
                                }
                            }
                            0.5
                        })
                })
                .unwrap_or(0.5);
            let new_value = (current + delta).clamp(0.0, 1.0);
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(values) = get_param_values_mut(instrument, *target) {
                    if let Some(entry) = values.iter_mut().find(|(idx, _)| *idx == *param_index) {
                        entry.1 = new_value;
                    } else {
                        values.push((*param_index, new_value));
                    }
                }
            }
            true
        }
        VstParamAction::ResetParam(instrument_id, target, param_index) => {
            let default = instruments
                .instrument(*instrument_id)
                .and_then(|inst| {
                    let plugin_id = get_vst_plugin_id(inst, *target)?;
                    session
                        .vst_plugins
                        .get(plugin_id)
                        .and_then(|plugin| plugin.params.iter().find(|p| p.index == *param_index))
                        .map(|spec| spec.default)
                })
                .unwrap_or(0.5);
            if let Some(instrument) = instruments.instrument_mut(*instrument_id) {
                if let Some(values) = get_param_values_mut(instrument, *target) {
                    if let Some(entry) = values.iter_mut().find(|(idx, _)| *idx == *param_index) {
                        entry.1 = default;
                    } else {
                        values.push((*param_index, default));
                    }
                }
            }
            true
        }
        VstParamAction::DiscoverParams(_, _) => false,
        VstParamAction::SaveState(_, _) => false,
    }
}

fn get_vst_plugin_id(instrument: &Instrument, target: VstTarget) -> Option<crate::VstPluginId> {
    match target {
        VstTarget::Source => {
            if let SourceType::Vst(id) = instrument.source {
                Some(id)
            } else {
                None
            }
        }
        VstTarget::Effect(effect_id) => instrument.effect_by_id(effect_id).and_then(|e| {
            if let EffectType::Vst(id) = e.effect_type {
                Some(id)
            } else {
                None
            }
        }),
    }
}

fn get_param_values_mut(
    instrument: &mut Instrument,
    target: VstTarget,
) -> Option<&mut Vec<(u32, f32)>> {
    match target {
        VstTarget::Source => match &mut instrument.source_extra {
            SourceExtra::Vst {
                ref mut param_values,
                ..
            } => Some(param_values),
            _ => None,
        },
        VstTarget::Effect(effect_id) => instrument
            .effect_by_id_mut(effect_id)
            .map(|e| &mut e.vst_param_values),
    }
}
