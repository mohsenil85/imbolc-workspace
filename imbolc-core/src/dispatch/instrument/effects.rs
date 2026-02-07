use crate::audio::AudioHandle;
use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::{DispatchResult, NavIntent, VstTarget};

use super::super::automation::record_automation_point;

pub(super) fn handle_add_effect(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_type: crate::state::EffectType,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.add_effect(effect_type);
    }
    let mut result = DispatchResult::with_nav(NavIntent::Pop);
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_remove_effect(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.remove_effect(effect_id);
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_move_effect(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
    direction: i8,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.move_effect(effect_id, direction);
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_toggle_effect_bypass(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if let Some(effect) = instrument.effect_by_id_mut(effect_id) {
            effect.enabled = !effect.enabled;
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result
}

pub(super) fn handle_adjust_effect_param(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
    param_idx: usize,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;
    let mut targeted_value: Option<f32> = None;
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let inst_id = instrument.id;
        if let Some(effect) = instrument.effect_by_id_mut(effect_id) {
            if let Some(param) = effect.params.get_mut(param_idx) {
                let range = param.max - param.min;
                match &mut param.value {
                    crate::state::ParamValue::Float(v) => {
                        *v = (*v + delta * range * 0.02).clamp(param.min, param.max);
                        // Targeted param update for float params
                        targeted_value = Some(*v);
                        if state.recording.automation_recording && state.session.piano_roll.playing {
                            let target = AutomationTarget::effect_param(inst_id, effect_id, param_idx);
                            record_target = Some((target.clone(), target.normalize_value(*v)));
                        }
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
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    // Targeted param update: send /n_set directly to effect node
    if let Some(value) = targeted_value {
        result.audio_dirty.effect_param = Some((id, effect_id, param_idx, value));
    }
    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }
    result
}

pub(super) fn handle_load_ir_result(
    state: &mut AppState,
    audio: &mut AudioHandle,
    instrument_id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
    path: &std::path::Path,
) -> DispatchResult {
    let path_str = path.to_string_lossy().to_string();

    let buffer_id = state.instruments.next_sampler_buffer_id;
    state.instruments.next_sampler_buffer_id += 1;

    if audio.is_running() {
        let _ = audio.load_sample(buffer_id, &path_str);
    }

    if let Some(instrument) = state.instruments.instrument_mut(instrument_id) {
        // Update the ir_buffer param on the convolution reverb effect
        if let Some(effect) = instrument.effect_by_id_mut(effect_id) {
            if effect.effect_type == crate::state::EffectType::ConvolutionReverb {
                for p in &mut effect.params {
                    if p.name == "ir_buffer" {
                        p.value = crate::state::param::ParamValue::Int(buffer_id as i32);
                    }
                }
            }
        }
        instrument.convolution_ir_path = Some(path_str);
    }

    let mut result = DispatchResult::with_nav(NavIntent::Pop);
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(instrument_id);
    result
}

pub(super) fn handle_open_vst_effect_params(
    instrument_id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    DispatchResult::with_nav(NavIntent::OpenVstParams(instrument_id, VstTarget::Effect(effect_id)))
}
