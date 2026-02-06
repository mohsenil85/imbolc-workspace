use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::DispatchResult;
use imbolc_types::InstrumentId;

use super::super::automation::record_automation_point;

pub(super) fn handle_adjust_envelope_attack(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // Attack range: 0.001 to 2.0 seconds
        let old_attack = instrument.amp_envelope.attack;
        instrument.amp_envelope.attack = (old_attack + delta * 0.1).clamp(0.001, 2.0);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::EnvelopeAttack(instrument.id);
            // Normalize to 0-1 range for automation
            let normalized = (instrument.amp_envelope.attack - 0.001) / (2.0 - 0.001);
            record_target = Some((target, normalized));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }

    result
}

pub(super) fn handle_adjust_envelope_decay(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // Decay range: 0.001 to 2.0 seconds
        let old_decay = instrument.amp_envelope.decay;
        instrument.amp_envelope.decay = (old_decay + delta * 0.1).clamp(0.001, 2.0);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::EnvelopeDecay(instrument.id);
            let normalized = (instrument.amp_envelope.decay - 0.001) / (2.0 - 0.001);
            record_target = Some((target, normalized));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }

    result
}

pub(super) fn handle_adjust_envelope_sustain(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // Sustain range: 0.0 to 1.0
        instrument.amp_envelope.sustain = (instrument.amp_envelope.sustain + delta * 0.05).clamp(0.0, 1.0);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::EnvelopeSustain(instrument.id);
            record_target = Some((target, instrument.amp_envelope.sustain));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }

    result
}

pub(super) fn handle_adjust_envelope_release(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        // Release range: 0.001 to 5.0 seconds
        let old_release = instrument.amp_envelope.release;
        instrument.amp_envelope.release = (old_release + delta * 0.2).clamp(0.001, 5.0);

        if state.recording.automation_recording && state.session.piano_roll.playing {
            let target = AutomationTarget::EnvelopeRelease(instrument.id);
            let normalized = (instrument.amp_envelope.release - 0.001) / (5.0 - 0.001);
            record_target = Some((target, normalized));
        }
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(id);

    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }

    result
}
