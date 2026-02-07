use crate::action::{DispatchResult, LfoParamKind};
use crate::dispatch::helpers::maybe_record_automation;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_types::{InstrumentId, LfoShape, ParameterTarget};

// LFO parameter ranges
const LFO_RATE_MIN: f32 = 0.1;
const LFO_RATE_MAX: f32 = 20.0;

pub(super) fn handle_toggle_lfo(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.lfo.enabled = !instrument.lfo.enabled;
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_adjust_lfo_rate(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut result = DispatchResult::none();
    let mut automation_data: Option<(InstrumentId, f32)> = None;
    let mut new_rate: Option<f32> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let old_rate = instrument.lfo.rate;
        instrument.lfo.rate = (old_rate + delta * 0.5).clamp(LFO_RATE_MIN, LFO_RATE_MAX);
        new_rate = Some(instrument.lfo.rate);

        let normalized = (instrument.lfo.rate - LFO_RATE_MIN) / (LFO_RATE_MAX - LFO_RATE_MIN);
        automation_data = Some((instrument.id, normalized));
    }

    if let Some((inst_id, normalized)) = automation_data {
        maybe_record_automation(state, &mut result, AutomationTarget::lfo_rate(inst_id), normalized);
    }

    result.audio_dirty.instruments = true;
    if let Some(rate) = new_rate {
        result.audio_dirty.lfo_param = Some((id, LfoParamKind::Rate, rate));
    }

    result
}

pub(super) fn handle_adjust_lfo_depth(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut result = DispatchResult::none();
    let mut automation_data: Option<(InstrumentId, f32)> = None;
    let mut new_depth: Option<f32> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.lfo.depth = (instrument.lfo.depth + delta * 0.05).clamp(0.0, 1.0);
        new_depth = Some(instrument.lfo.depth);
        automation_data = Some((instrument.id, instrument.lfo.depth));
    }

    if let Some((inst_id, depth)) = automation_data {
        maybe_record_automation(state, &mut result, AutomationTarget::lfo_depth(inst_id), depth);
    }

    result.audio_dirty.instruments = true;
    if let Some(depth) = new_depth {
        result.audio_dirty.lfo_param = Some((id, LfoParamKind::Depth, depth));
    }

    result
}

pub(super) fn handle_set_lfo_shape(
    state: &mut AppState,
    id: InstrumentId,
    shape: LfoShape,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.lfo.shape = shape;
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_set_lfo_target(
    state: &mut AppState,
    id: InstrumentId,
    target: ParameterTarget,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.lfo.target = target;
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}
