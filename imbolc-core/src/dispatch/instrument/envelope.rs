use crate::action::DispatchResult;
use crate::dispatch::helpers::adjust_instrument_param;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_types::InstrumentId;

// Envelope parameter ranges
const ATTACK_MIN: f32 = 0.001;
const ATTACK_MAX: f32 = 2.0;
const DECAY_MIN: f32 = 0.001;
const DECAY_MAX: f32 = 2.0;
const SUSTAIN_MIN: f32 = 0.0;
const SUSTAIN_MAX: f32 = 1.0;
const RELEASE_MIN: f32 = 0.001;
const RELEASE_MAX: f32 = 5.0;

pub(super) fn handle_adjust_envelope_attack(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    adjust_instrument_param(
        state,
        id,
        delta,
        0.1,
        ATTACK_MIN,
        ATTACK_MAX,
        |inst| inst.modulation.amp_envelope.attack,
        |inst, v| inst.modulation.amp_envelope.attack = v,
        AutomationTarget::attack,
        |v| (v - ATTACK_MIN) / (ATTACK_MAX - ATTACK_MIN),
    )
}

pub(super) fn handle_adjust_envelope_decay(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    adjust_instrument_param(
        state,
        id,
        delta,
        0.1,
        DECAY_MIN,
        DECAY_MAX,
        |inst| inst.modulation.amp_envelope.decay,
        |inst, v| inst.modulation.amp_envelope.decay = v,
        AutomationTarget::decay,
        |v| (v - DECAY_MIN) / (DECAY_MAX - DECAY_MIN),
    )
}

pub(super) fn handle_adjust_envelope_sustain(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    adjust_instrument_param(
        state,
        id,
        delta,
        0.05,
        SUSTAIN_MIN,
        SUSTAIN_MAX,
        |inst| inst.modulation.amp_envelope.sustain,
        |inst, v| inst.modulation.amp_envelope.sustain = v,
        AutomationTarget::sustain,
        |v| v, // Already 0-1 range
    )
}

pub(super) fn handle_adjust_envelope_release(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
) -> DispatchResult {
    adjust_instrument_param(
        state,
        id,
        delta,
        0.2,
        RELEASE_MIN,
        RELEASE_MAX,
        |inst| inst.modulation.amp_envelope.release,
        |inst, v| inst.modulation.amp_envelope.release = v,
        AutomationTarget::release,
        |v| (v - RELEASE_MIN) / (RELEASE_MAX - RELEASE_MIN),
    )
}
