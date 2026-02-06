use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::{DispatchResult, FilterParamKind};

use super::super::automation::record_automation_point;

pub(super) fn handle_set_filter(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    filter_type: Option<crate::state::FilterType>,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.filter = filter_type.map(crate::state::FilterConfig::new);
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_toggle_filter(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if instrument.filter.is_some() {
            instrument.filter = None;
        } else {
            instrument.filter = Some(crate::state::FilterConfig::new(crate::state::FilterType::Lpf));
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}

pub(super) fn handle_cycle_filter_type(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if let Some(ref mut filter) = instrument.filter {
            filter.filter_type = match filter.filter_type {
                crate::state::FilterType::Lpf => crate::state::FilterType::Hpf,
                crate::state::FilterType::Hpf => crate::state::FilterType::Bpf,
                crate::state::FilterType::Bpf => crate::state::FilterType::Notch,
                crate::state::FilterType::Notch => crate::state::FilterType::Comb,
                crate::state::FilterType::Comb => crate::state::FilterType::Allpass,
                crate::state::FilterType::Allpass => crate::state::FilterType::Vowel,
                crate::state::FilterType::Vowel => crate::state::FilterType::ResDrive,
                crate::state::FilterType::ResDrive => crate::state::FilterType::Lpf,
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
    id: crate::state::InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;
    let mut new_cutoff: Option<f32> = None;
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if let Some(ref mut filter) = instrument.filter {
            filter.cutoff.value = (filter.cutoff.value + delta * filter.cutoff.max * 0.02)
                .clamp(filter.cutoff.min, filter.cutoff.max);
            new_cutoff = Some(filter.cutoff.value);
            if state.recording.automation_recording && state.session.piano_roll.playing {
                let target = AutomationTarget::FilterCutoff(instrument.id);
                record_target = Some((target.clone(), target.normalize_value(filter.cutoff.value)));
            }
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    // Targeted param update: send /n_set directly to filter node
    if let Some(cutoff) = new_cutoff {
        result.audio_dirty.filter_param = Some((id, FilterParamKind::Cutoff, cutoff));
    }
    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }
    result
}

pub(super) fn handle_adjust_filter_resonance(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;
    let mut new_resonance: Option<f32> = None;
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        if let Some(ref mut filter) = instrument.filter {
            filter.resonance.value = (filter.resonance.value + delta * 0.05)
                .clamp(filter.resonance.min, filter.resonance.max);
            new_resonance = Some(filter.resonance.value);
            if state.recording.automation_recording && state.session.piano_roll.playing {
                let target = AutomationTarget::FilterResonance(instrument.id);
                record_target = Some((target.clone(), target.normalize_value(filter.resonance.value)));
            }
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    // Targeted param update: send /n_set directly to filter node
    if let Some(resonance) = new_resonance {
        result.audio_dirty.filter_param = Some((id, FilterParamKind::Resonance, resonance));
    }
    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }
    result
}
