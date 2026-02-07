use std::path::PathBuf;

use crate::audio::AudioHandle;
use crate::audio::commands::AudioCmd;
use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::{DispatchResult, VstParamAction, VstTarget};
use crate::state::instrument::Instrument;
use crate::state::vst_plugin::VstPluginId;
use crate::dispatch::automation::record_automation_point;

/// Compute VST state file path for an instrument source
fn vst_state_path(instrument_id: u32, plugin_name: &str) -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    let sanitized: String = plugin_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    config_dir
        .join("imbolc")
        .join("vst_states")
        .join(format!("instrument_{}_{}.fxp", instrument_id, sanitized))
}

/// Compute VST state file path for an effect slot
fn vst_effect_state_path(instrument_id: u32, effect_id: u32, plugin_name: &str) -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    let sanitized: String = plugin_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    config_dir
        .join("imbolc")
        .join("vst_states")
        .join(format!("instrument_{}_fx_{}_{}.fxp", instrument_id, effect_id, sanitized))
}

/// Get the VstPluginId for a given instrument and target
fn get_vst_plugin_id(instrument: &Instrument, target: VstTarget) -> Option<VstPluginId> {
    match target {
        VstTarget::Source => {
            if let crate::state::SourceType::Vst(id) = instrument.source {
                Some(id)
            } else {
                None
            }
        }
        VstTarget::Effect(effect_id) => {
            instrument.effect_by_id(effect_id).and_then(|e| {
                if let crate::state::EffectType::Vst(id) = e.effect_type {
                    Some(id)
                } else {
                    None
                }
            })
        }
    }
}

/// Get param values slice for a given target
fn get_param_values(instrument: &Instrument, target: VstTarget) -> &[(u32, f32)] {
    match target {
        VstTarget::Source => &instrument.vst_param_values,
        VstTarget::Effect(effect_id) => {
            instrument.effect_by_id(effect_id)
                .map(|e| e.vst_param_values.as_slice())
                .unwrap_or(&[])
        }
    }
}

/// Get mutable param values for a given target
fn get_param_values_mut(instrument: &mut Instrument, target: VstTarget) -> Option<&mut Vec<(u32, f32)>> {
    match target {
        VstTarget::Source => Some(&mut instrument.vst_param_values),
        VstTarget::Effect(effect_id) => {
            instrument.effect_by_id_mut(effect_id)
                .map(|e| &mut e.vst_param_values)
        }
    }
}

pub(super) fn dispatch_vst_param(
    action: &VstParamAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    match action {
        VstParamAction::SetParam(instrument_id, target, param_index, value) => {
            let value = value.clamp(0.0, 1.0);
            if let Some(instrument) = state.instruments.instrument_mut(*instrument_id) {
                if let Some(values) = get_param_values_mut(instrument, *target) {
                    if let Some(entry) = values.iter_mut().find(|(idx, _)| *idx == *param_index) {
                        entry.1 = value;
                    } else {
                        values.push((*param_index, value));
                    }
                }
            }
            if audio.is_running() {
                if let Err(e) = audio.send_cmd(AudioCmd::SetVstParam {
                    instrument_id: *instrument_id,
                    target: *target,
                    param_index: *param_index,
                    value,
                }) {
                    log::warn!(target: "dispatch::vst", "SetVstParam dropped: {}", e);
                }
            }
            // Record automation when recording + playing
            if state.recording.automation_recording && state.session.piano_roll.playing {
                record_automation_point(
                    state,
                    AutomationTarget::vst_param(*instrument_id, *param_index),
                    value,
                );
            }
            DispatchResult::none()
        }
        VstParamAction::AdjustParam(instrument_id, target, param_index, delta) => {
            let current = state.instruments.instrument(*instrument_id)
                .map(|inst| {
                    let values = get_param_values(inst, *target);
                    values.iter().find(|(idx, _)| *idx == *param_index)
                        .map(|(_, v)| *v)
                        .unwrap_or_else(|| {
                            // Look up default from VST plugin registry
                            if let Some(plugin_id) = get_vst_plugin_id(inst, *target) {
                                if let Some(plugin) = state.session.vst_plugins.get(plugin_id) {
                                    if let Some(spec) = plugin.params.iter().find(|p| p.index == *param_index) {
                                        return spec.default;
                                    }
                                }
                            }
                            0.5
                        })
                })
                .unwrap_or(0.5);
            let new_value = (current + delta).clamp(0.0, 1.0);
            dispatch_vst_param(
                &VstParamAction::SetParam(*instrument_id, *target, *param_index, new_value),
                state,
                audio,
            )
        }
        VstParamAction::ResetParam(instrument_id, target, param_index) => {
            let default = state.instruments.instrument(*instrument_id)
                .and_then(|inst| {
                    let plugin_id = get_vst_plugin_id(inst, *target)?;
                    state.session.vst_plugins.get(plugin_id)
                        .and_then(|plugin| plugin.params.iter().find(|p| p.index == *param_index))
                        .map(|spec| spec.default)
                })
                .unwrap_or(0.5);
            dispatch_vst_param(
                &VstParamAction::SetParam(*instrument_id, *target, *param_index, default),
                state,
                audio,
            )
        }
        VstParamAction::DiscoverParams(instrument_id, target) => {
            // Try VST3 probe first — direct binary probing gives real param names
            let probed = state.instruments.instrument(*instrument_id)
                .and_then(|inst| {
                    let plugin_id = get_vst_plugin_id(inst, *target)?;
                    let plugin = state.session.vst_plugins.get(plugin_id)?;
                    let path = &plugin.plugin_path;
                    if path.extension().and_then(|e| e.to_str()) == Some("vst3") {
                        match crate::vst3_probe::probe_vst3_params(path) {
                            Ok(params) if !params.is_empty() => Some((plugin_id, params)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                });

            if let Some((plugin_id, probed_params)) = probed {
                // Update the plugin registry with probed params
                use crate::state::vst_plugin::VstParamSpec;
                if let Some(plugin) = state.session.vst_plugins.get_mut(plugin_id) {
                    plugin.params = probed_params.iter().map(|p| VstParamSpec {
                        index: p.index as u32,
                        name: p.name.clone(),
                        default: p.default_normalized as f32,
                        label: if p.units.is_empty() { None } else { Some(p.units.clone()) },
                    }).collect();
                }
            } else {
                // Fall back to OSC discovery
                if audio.is_running() {
                    if let Err(e) = audio.send_cmd(AudioCmd::QueryVstParams {
                        instrument_id: *instrument_id,
                        target: *target,
                    }) {
                        log::warn!(target: "dispatch::vst", "QueryVstParams dropped: {}", e);
                    }
                }
            }
            DispatchResult::none()
        }
        VstParamAction::SaveState(instrument_id, target) => {
            if let Some(instrument) = state.instruments.instrument(*instrument_id) {
                let plugin_id = get_vst_plugin_id(instrument, *target);
                let plugin_name = plugin_id
                    .and_then(|id| state.session.vst_plugins.get(id))
                    .map(|p| p.name.clone());
                let plugin_name = match plugin_name {
                    Some(name) => name,
                    None => return DispatchResult::none(),
                };
                let path = match *target {
                    VstTarget::Source => vst_state_path(*instrument_id, &plugin_name),
                    VstTarget::Effect(effect_id) => vst_effect_state_path(*instrument_id, effect_id, &plugin_name),
                };
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                // Store the path in state
                if let Some(instrument) = state.instruments.instrument_mut(*instrument_id) {
                    match *target {
                        VstTarget::Source => {
                            instrument.vst_state_path = Some(path.clone());
                        }
                        VstTarget::Effect(effect_id) => {
                            if let Some(effect) = instrument.effect_by_id_mut(effect_id) {
                                effect.vst_state_path = Some(path.clone());
                            }
                        }
                    }
                }
                if audio.is_running() {
                    if let Err(e) = audio.send_cmd(AudioCmd::SaveVstState {
                        instrument_id: *instrument_id,
                        target: *target,
                        path,
                    }) {
                        log::warn!(target: "dispatch::vst", "SaveVstState dropped: {}", e);
                    }
                }
            }
            DispatchResult::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::instrument::SourceType;

    fn setup() -> (AppState, AudioHandle) {
        let state = AppState::new();
        let audio = AudioHandle::new();
        (state, audio)
    }

    #[test]
    fn set_param_records_when_recording() {
        let (mut state, mut audio) = setup();
        let id = state.instruments.add_instrument(SourceType::Saw);
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playhead = 100;

        dispatch_vst_param(
            &VstParamAction::SetParam(id, VstTarget::Source, 0, 0.7),
            &mut state,
            &mut audio,
        );

        let target = AutomationTarget::vst_param(id, 0);
        let lane = state.session.automation.lane_for_target(&target);
        assert!(lane.is_some(), "VstParam lane should be created");
        assert_eq!(lane.unwrap().points.len(), 1);
    }

    #[test]
    fn set_param_no_record_when_not_recording() {
        let (mut state, mut audio) = setup();
        let id = state.instruments.add_instrument(SourceType::Saw);
        state.recording.automation_recording = false;
        state.session.piano_roll.playing = true;

        dispatch_vst_param(
            &VstParamAction::SetParam(id, VstTarget::Source, 0, 0.7),
            &mut state,
            &mut audio,
        );

        let target = AutomationTarget::vst_param(id, 0);
        assert!(state.session.automation.lane_for_target(&target).is_none());
    }

    #[test]
    fn set_param_updates_state_regardless() {
        let (mut state, mut audio) = setup();
        let id = state.instruments.add_instrument(SourceType::Saw);
        state.recording.automation_recording = false;

        dispatch_vst_param(
            &VstParamAction::SetParam(id, VstTarget::Source, 0, 0.7),
            &mut state,
            &mut audio,
        );

        // State should be updated even without recording
        let inst = state.instruments.instrument(id).unwrap();
        let val = inst.vst_param_values.iter().find(|(idx, _)| *idx == 0);
        assert!(val.is_some());
        assert!((val.unwrap().1 - 0.7).abs() < f32::EPSILON);
    }
}
