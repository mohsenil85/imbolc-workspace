use crate::action::{AudioEffect, DispatchResult, FilterParamKind};
use crate::dispatch::helpers::maybe_record_automation;
use crate::state::automation::AutomationTarget;
use crate::state::{AppState, InstrumentId};
use imbolc_types::{DomainAction, InstrumentAction};

fn reduce(state: &mut AppState, action: &InstrumentAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn handle_set_filter(
    state: &mut AppState,
    id: InstrumentId,
    filter_type: Option<crate::state::FilterType>,
) -> DispatchResult {
    reduce(state, &InstrumentAction::SetFilter(id, filter_type));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result.audio_effects.push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_toggle_filter(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    reduce(state, &InstrumentAction::ToggleFilter(id));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result.audio_effects.push(AudioEffect::RebuildRoutingForInstrument(id));
    result
}

pub(super) fn handle_cycle_filter_type(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    reduce(state, &InstrumentAction::CycleFilterType(id));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_filter_cutoff(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustFilterCutoff(id, delta));

    let mut result = DispatchResult::none();
    // Read post-mutation value for automation recording and targeted param
    if let Some(instrument) = state.instruments.instrument(id) {
        if let Some(filter) = instrument.filter() {
            let cutoff = filter.cutoff.value;
            let target = AutomationTarget::filter_cutoff(id);
            let normalized = target.normalize_value(cutoff);
            maybe_record_automation(state, &mut result, AutomationTarget::filter_cutoff(id), normalized);
            result.audio_effects.push(AudioEffect::SetFilterParam(id, FilterParamKind::Cutoff, cutoff));
        }
    }

    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_filter_resonance(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustFilterResonance(id, delta));

    let mut result = DispatchResult::none();
    // Read post-mutation value for automation recording and targeted param
    if let Some(instrument) = state.instruments.instrument(id) {
        if let Some(filter) = instrument.filter() {
            let resonance = filter.resonance.value;
            let target = AutomationTarget::filter_resonance(id);
            let normalized = target.normalize_value(resonance);
            maybe_record_automation(state, &mut result, AutomationTarget::filter_resonance(id), normalized);
            result.audio_effects.push(AudioEffect::SetFilterParam(id, FilterParamKind::Resonance, resonance));
        }
    }

    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}
