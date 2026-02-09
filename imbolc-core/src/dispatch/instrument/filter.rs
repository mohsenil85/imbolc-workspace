use crate::action::{DispatchResult, FilterParamKind};
use crate::dispatch::helpers::maybe_record_automation;
use crate::state::automation::AutomationTarget;
use crate::state::{AppState, FilterType, InstrumentId};

pub(super) fn handle_set_filter(
    state: &mut AppState,
    id: InstrumentId,
    filter_type: Option<FilterType>,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.set_filter(filter_type);
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_toggle_filter(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.toggle_filter();
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_cycle_filter_type(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if let Some(filter) = instrument.filter_mut() {
            filter.filter_type = match filter.filter_type {
                FilterType::Lpf => FilterType::Hpf,
                FilterType::Hpf => FilterType::Bpf,
                FilterType::Bpf => FilterType::Notch,
                FilterType::Notch => FilterType::Comb,
                FilterType::Comb => FilterType::Allpass,
                FilterType::Allpass => FilterType::Vowel,
                FilterType::Vowel => FilterType::ResDrive,
                FilterType::ResDrive => FilterType::Lpf,
            };
            filter.extra_params = filter.filter_type.default_extra_params();
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result
}

pub(super) fn handle_adjust_filter_cutoff(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut result = DispatchResult::none();
    let mut new_cutoff: Option<f32> = None;
    let mut automation_data: Option<(InstrumentId, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let inst_id = instrument.id;
        if let Some(filter) = instrument.filter_mut() {
            filter.cutoff.value = (filter.cutoff.value + delta * filter.cutoff.max * 0.02)
                .clamp(filter.cutoff.min, filter.cutoff.max);
            new_cutoff = Some(filter.cutoff.value);

            let target = AutomationTarget::filter_cutoff(inst_id);
            let normalized = target.normalize_value(filter.cutoff.value);
            automation_data = Some((inst_id, normalized));
        }
    }

    if let Some((inst_id, normalized)) = automation_data {
        maybe_record_automation(
            state,
            &mut result,
            AutomationTarget::filter_cutoff(inst_id),
            normalized,
        );
    }

    result.audio_dirty.instruments = true;
    if let Some(cutoff) = new_cutoff {
        result.audio_dirty.filter_param = Some((id, FilterParamKind::Cutoff, cutoff));
    }
    result
}

pub(super) fn handle_adjust_filter_resonance(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut result = DispatchResult::none();
    let mut new_resonance: Option<f32> = None;
    let mut automation_data: Option<(InstrumentId, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let inst_id = instrument.id;
        if let Some(filter) = instrument.filter_mut() {
            filter.resonance.value = (filter.resonance.value + delta * 0.05)
                .clamp(filter.resonance.min, filter.resonance.max);
            new_resonance = Some(filter.resonance.value);

            let target = AutomationTarget::filter_resonance(inst_id);
            let normalized = target.normalize_value(filter.resonance.value);
            automation_data = Some((inst_id, normalized));
        }
    }

    if let Some((inst_id, normalized)) = automation_data {
        maybe_record_automation(
            state,
            &mut result,
            AutomationTarget::filter_resonance(inst_id),
            normalized,
        );
    }

    result.audio_dirty.instruments = true;
    if let Some(resonance) = new_resonance {
        result.audio_dirty.filter_param = Some((id, FilterParamKind::Resonance, resonance));
    }
    result
}
