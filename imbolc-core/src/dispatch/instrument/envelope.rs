use crate::action::{AudioEffect, DispatchResult};
use crate::dispatch::helpers::maybe_record_automation;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_types::{DomainAction, InstrumentAction, InstrumentId};

// Envelope parameter ranges
const ATTACK_MIN: f32 = 0.001;
const ATTACK_MAX: f32 = 2.0;
const DECAY_MIN: f32 = 0.001;
const DECAY_MAX: f32 = 2.0;
const RELEASE_MIN: f32 = 0.001;
const RELEASE_MAX: f32 = 5.0;

fn reduce(state: &mut AppState, action: &InstrumentAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn handle_adjust_envelope_attack(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustEnvelopeAttack(id, delta));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    if let Some(inst) = state.instruments.instrument(id) {
        let v = inst.modulation.amp_envelope.attack;
        let normalized = (v - ATTACK_MIN) / (ATTACK_MAX - ATTACK_MIN);
        maybe_record_automation(state, &mut result, AutomationTarget::attack(id), normalized);
    }
    result
}

pub(super) fn handle_adjust_envelope_decay(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustEnvelopeDecay(id, delta));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    if let Some(inst) = state.instruments.instrument(id) {
        let v = inst.modulation.amp_envelope.decay;
        let normalized = (v - DECAY_MIN) / (DECAY_MAX - DECAY_MIN);
        maybe_record_automation(state, &mut result, AutomationTarget::decay(id), normalized);
    }
    result
}

pub(super) fn handle_adjust_envelope_sustain(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustEnvelopeSustain(id, delta));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    if let Some(inst) = state.instruments.instrument(id) {
        let v = inst.modulation.amp_envelope.sustain;
        maybe_record_automation(state, &mut result, AutomationTarget::sustain(id), v);
    }
    result
}

pub(super) fn handle_adjust_envelope_release(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    reduce(state, &InstrumentAction::AdjustEnvelopeRelease(id, delta));
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(id));
    if let Some(inst) = state.instruments.instrument(id) {
        let v = inst.modulation.amp_envelope.release;
        let normalized = (v - RELEASE_MIN) / (RELEASE_MAX - RELEASE_MIN);
        maybe_record_automation(
            state,
            &mut result,
            AutomationTarget::release(id),
            normalized,
        );
    }
    result
}
