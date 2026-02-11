use crate::action::{AudioEffect, DispatchResult, NavIntent, VstTarget};
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_audio::AudioHandle;
use imbolc_types::{DomainAction, InstrumentAction, ParamValue};

use super::super::automation::record_automation_point;

fn reduce(state: &mut AppState, action: &InstrumentAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn handle_add_effect(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_type: crate::state::EffectType,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AddEffect(id, effect_type));
    let mut result = DispatchResult::with_nav(NavIntent::Pop);
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_remove_effect(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    reduce(state, &InstrumentAction::RemoveEffect(id, effect_id));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_toggle_effect_bypass(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    reduce(state, &InstrumentAction::ToggleEffectBypass(id, effect_id));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_effect_param(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
    param_idx: imbolc_types::ParamIndex,
    delta: f32,
) -> DispatchResult {
    reduce(
        state,
        &InstrumentAction::AdjustEffectParam(id, effect_id, param_idx, delta),
    );

    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);

    // Read post-mutation value for targeted param + automation recording
    // Extract value first to avoid borrow conflict with record_automation_point
    let param_value = state
        .instruments
        .instrument(id)
        .and_then(|inst| inst.effects().find(|e| e.id == effect_id))
        .and_then(|effect| effect.params.get(param_idx.get()))
        .and_then(|param| match param.value {
            ParamValue::Float(v) => Some(v),
            _ => None,
        });

    if let Some(value) = param_value {
        result
            .audio_effects
            .push(AudioEffect::SetEffectParam(id, effect_id, param_idx, value));
        if state.recording.automation_recording && state.audio.playing {
            let target = AutomationTarget::effect_param(id, effect_id, param_idx);
            let normalized = target.normalize_value(value);
            record_automation_point(state, target, normalized);
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
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
    // Load sample into audio engine before reducer increments the buffer_id
    let buffer_id = state.instruments.next_sampler_buffer_id;
    if audio.is_running() {
        let _ = audio.load_sample(buffer_id, &path.to_string_lossy());
    }

    reduce(
        state,
        &InstrumentAction::LoadIRResult(instrument_id, effect_id, path.to_path_buf()),
    );

    let mut result = DispatchResult::with_nav(NavIntent::Pop);
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(instrument_id));
    result
}

pub(super) fn handle_open_vst_effect_params(
    instrument_id: crate::state::InstrumentId,
    effect_id: crate::state::EffectId,
) -> DispatchResult {
    DispatchResult::with_nav(NavIntent::OpenVstParams(
        instrument_id,
        VstTarget::Effect(effect_id),
    ))
}
