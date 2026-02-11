use crate::action::{AudioEffect, DispatchResult, LfoParamKind};
use crate::dispatch::helpers::maybe_record_automation;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_types::{DomainAction, InstrumentAction, InstrumentId, LfoShape, ParameterTarget};

// LFO parameter ranges
const LFO_RATE_MIN: f32 = 0.1;
const LFO_RATE_MAX: f32 = 20.0;

fn reduce(state: &mut AppState, action: &InstrumentAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn handle_toggle_lfo(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    reduce(state, &InstrumentAction::ToggleLfo(id));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_adjust_lfo_rate(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustLfoRate(id, delta));

    let mut result = DispatchResult::none();
    if let Some(instrument) = state.instruments.instrument(id) {
        let rate = instrument.modulation.lfo.rate;
        let normalized = (rate - LFO_RATE_MIN) / (LFO_RATE_MAX - LFO_RATE_MIN);
        maybe_record_automation(
            state,
            &mut result,
            AutomationTarget::lfo_rate(id),
            normalized,
        );
        result
            .audio_effects
            .push(AudioEffect::SetLfoParam(id, LfoParamKind::Rate, rate));
    }

    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_lfo_depth(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustLfoDepth(id, delta));

    let mut result = DispatchResult::none();
    if let Some(instrument) = state.instruments.instrument(id) {
        let depth = instrument.modulation.lfo.depth;
        maybe_record_automation(state, &mut result, AutomationTarget::lfo_depth(id), depth);
        result
            .audio_effects
            .push(AudioEffect::SetLfoParam(id, LfoParamKind::Depth, depth));
    }

    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_set_lfo_shape(
    state: &mut AppState,
    id: InstrumentId,
    shape: LfoShape,
) -> DispatchResult {
    reduce(state, &InstrumentAction::SetLfoShape(id, shape));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_set_lfo_target(
    state: &mut AppState,
    id: InstrumentId,
    target: ParameterTarget,
) -> DispatchResult {
    reduce(state, &InstrumentAction::SetLfoTarget(id, target));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}
