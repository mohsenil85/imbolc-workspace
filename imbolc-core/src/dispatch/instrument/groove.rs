//! Dispatch handlers for per-track groove settings.

use crate::action::DispatchResult;
use crate::state::AppState;
use imbolc_types::{InstrumentId, SwingGrid};

pub fn handle_set_track_swing(
    state: &mut AppState,
    instrument_id: InstrumentId,
    value: Option<f32>,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.swing_amount = value.map(|v| v.clamp(0.0, 1.0));
    }
    DispatchResult::none()
}

pub fn handle_set_track_swing_grid(
    state: &mut AppState,
    instrument_id: InstrumentId,
    grid: Option<SwingGrid>,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.swing_grid = grid;
    }
    DispatchResult::none()
}

pub fn handle_adjust_track_swing(
    state: &mut AppState,
    instrument_id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        // Get current value (override or fall back to global piano roll swing)
        let current = inst.groove.swing_amount
            .unwrap_or(state.session.piano_roll.swing_amount);
        let new_value = (current + delta).clamp(0.0, 1.0);
        inst.groove.swing_amount = Some(new_value);
    }
    DispatchResult::none()
}

pub fn handle_set_track_humanize_velocity(
    state: &mut AppState,
    instrument_id: InstrumentId,
    value: Option<f32>,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.humanize_velocity = value.map(|v| v.clamp(0.0, 1.0));
    }
    DispatchResult::none()
}

pub fn handle_adjust_track_humanize_velocity(
    state: &mut AppState,
    instrument_id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        let current = inst.groove.humanize_velocity
            .unwrap_or(state.session.humanize.velocity);
        let new_value = (current + delta).clamp(0.0, 1.0);
        inst.groove.humanize_velocity = Some(new_value);
    }
    DispatchResult::none()
}

pub fn handle_set_track_humanize_timing(
    state: &mut AppState,
    instrument_id: InstrumentId,
    value: Option<f32>,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.humanize_timing = value.map(|v| v.clamp(0.0, 1.0));
    }
    DispatchResult::none()
}

pub fn handle_adjust_track_humanize_timing(
    state: &mut AppState,
    instrument_id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        let current = inst.groove.humanize_timing
            .unwrap_or(state.session.humanize.timing);
        let new_value = (current + delta).clamp(0.0, 1.0);
        inst.groove.humanize_timing = Some(new_value);
    }
    DispatchResult::none()
}

pub fn handle_set_track_timing_offset(
    state: &mut AppState,
    instrument_id: InstrumentId,
    value: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.timing_offset_ms = value.clamp(-50.0, 50.0);
    }
    DispatchResult::none()
}

pub fn handle_adjust_track_timing_offset(
    state: &mut AppState,
    instrument_id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        let new_value = (inst.groove.timing_offset_ms + delta).clamp(-50.0, 50.0);
        inst.groove.timing_offset_ms = new_value;
    }
    DispatchResult::none()
}

pub fn handle_reset_track_groove(
    state: &mut AppState,
    instrument_id: InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(instrument_id) {
        inst.groove.reset();
    }
    DispatchResult::none()
}
