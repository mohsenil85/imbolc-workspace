use crate::state::AppState;
use crate::action::{AudioEffect, DispatchResult};

pub(super) fn handle_toggle_arp(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.arpeggiator.enabled = !inst.note_input.arpeggiator.enabled;
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_cycle_arp_direction(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.arpeggiator.direction = inst.note_input.arpeggiator.direction.next();
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_cycle_arp_rate(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.arpeggiator.rate = inst.note_input.arpeggiator.rate.next();
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_arp_octaves(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    delta: i8,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.arpeggiator.octaves = (inst.note_input.arpeggiator.octaves as i8 + delta).clamp(1, 4) as u8;
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_adjust_arp_gate(
    state: &mut AppState,
    id: crate::state::InstrumentId,
    delta: f32,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.arpeggiator.gate = (inst.note_input.arpeggiator.gate + delta).clamp(0.1, 1.0);
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_cycle_chord_shape(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.chord_shape = Some(match inst.note_input.chord_shape {
            Some(shape) => shape.next(),
            None => crate::state::arpeggiator::ChordShape::Major,
        });
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_clear_chord_shape(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.note_input.chord_shape = None;
    }
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}
