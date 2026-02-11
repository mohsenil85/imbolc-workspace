use crate::action::{AudioEffect, DispatchResult, EqParamKind};
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_audio::AudioHandle;
use imbolc_types::{DomainAction, InstrumentAction};

use super::super::automation::record_automation_point;

pub(super) fn handle_set_eq_param(
    state: &mut AppState,
    audio: &mut AudioHandle,
    instrument_id: crate::state::InstrumentId,
    band_idx: usize,
    param: EqParamKind,
    value: f32,
) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(InstrumentAction::SetEqParam(
            instrument_id,
            band_idx,
            param,
            value,
        )),
        &mut state.instruments,
        &mut state.session,
    );

    // Send real-time param update to audio engine
    if audio.is_running() {
        let sc_param = format!("b{}_{}", band_idx, param.as_str());
        let sc_value = if param == EqParamKind::Q {
            1.0 / value
        } else {
            value
        };
        let _ = audio.set_eq_param(instrument_id, &sc_param, sc_value);
    }

    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);

    // Automation recording
    if state.recording.automation_recording && state.audio.playing {
        let target = match param {
            EqParamKind::Freq => Some(AutomationTarget::eq_band_freq(instrument_id, band_idx)),
            EqParamKind::Gain => Some(AutomationTarget::eq_band_gain(instrument_id, band_idx)),
            EqParamKind::Q => Some(AutomationTarget::eq_band_q(instrument_id, band_idx)),
            EqParamKind::Enabled => None,
        };
        if let Some(t) = target {
            let normalized = t.normalize_value(value);
            record_automation_point(state, t, normalized);
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
    }
    result
}

pub(super) fn handle_toggle_eq(
    state: &mut AppState,
    instrument_id: crate::state::InstrumentId,
) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(InstrumentAction::ToggleEq(instrument_id)),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(instrument_id));
    result
}
