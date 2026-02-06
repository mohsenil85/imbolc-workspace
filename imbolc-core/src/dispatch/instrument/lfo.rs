use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::{DispatchResult, LfoParamKind};
use imbolc_types::{InstrumentId, LfoShape, LfoTarget};

use super::super::automation::record_automation_point;

pub(super) fn handle_toggle_lfo(
    state: &mut AppState,
    id: InstrumentId,
) -> DispatchResult {
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
    let mut record_target: Option<(AutomationTarget, f32)> = None;
    let mut new_rate: Option<f32> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // LFO rate range: 0.1 to 20.0 Hz
        let old_rate = instrument.lfo.rate;
        instrument.lfo.rate = (old_rate + delta * 0.5).clamp(0.1, 20.0);
        new_rate = Some(instrument.lfo.rate);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::LfoRate(instrument.id);
            // Normalize to 0-1 range for automation
            let normalized = (instrument.lfo.rate - 0.1) / (20.0 - 0.1);
            record_target = Some((target, normalized));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;

    // Targeted LFO param update
    if let Some(rate) = new_rate {
        result.audio_dirty.lfo_param = Some((id, LfoParamKind::Rate, rate));
    }

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }

    result
}

pub(super) fn handle_adjust_lfo_depth(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;
    let mut new_depth: Option<f32> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // LFO depth range: 0.0 to 1.0
        instrument.lfo.depth = (instrument.lfo.depth + delta * 0.05).clamp(0.0, 1.0);
        new_depth = Some(instrument.lfo.depth);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::LfoDepth(instrument.id);
            record_target = Some((target, instrument.lfo.depth));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;

    // Targeted LFO param update
    if let Some(depth) = new_depth {
        result.audio_dirty.lfo_param = Some((id, LfoParamKind::Depth, depth));
    }

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
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
    target: LfoTarget,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        instrument.lfo.target = target;
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);
    result
}
