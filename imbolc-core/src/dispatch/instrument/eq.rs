use imbolc_audio::AudioHandle;
use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::{DispatchResult, EqParamKind};
use crate::dispatch::side_effects::AudioSideEffect;

use super::super::automation::record_automation_point;

pub(super) fn handle_set_eq_param(
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    instrument_id: crate::state::InstrumentId,
    band_idx: usize,
    param: EqParamKind,
    value: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(instrument_id) {
        if let Some(eq) = instrument.eq_mut() {
            if let Some(band) = eq.bands.get_mut(band_idx) {
                match param {
                    EqParamKind::Freq => band.freq = value.clamp(20.0, 20000.0),
                    EqParamKind::Gain => band.gain = value.clamp(-24.0, 24.0),
                    EqParamKind::Q => band.q = value.clamp(0.1, 10.0),
                    EqParamKind::Enabled => band.enabled = value > 0.5,
                }
                if state.recording.automation_recording && state.audio.playing {
                    let target = match param {
                        EqParamKind::Freq => Some(AutomationTarget::eq_band_freq(instrument.id, band_idx)),
                        EqParamKind::Gain => Some(AutomationTarget::eq_band_gain(instrument.id, band_idx)),
                        EqParamKind::Q => Some(AutomationTarget::eq_band_q(instrument.id, band_idx)),
                        EqParamKind::Enabled => None,
                    };
                    if let Some(t) = target {
                        record_target = Some((t.clone(), t.normalize_value(value)));
                    }
                }
            }
        }
    }

    // Send real-time param update to audio engine
    if audio.is_running() {
        let sc_param = format!("b{}_{}", band_idx, param.as_str());
        let sc_value = if param == EqParamKind::Q { 1.0 / value } else { value };
        effects.push(AudioSideEffect::SetEqParam {
            instrument_id,
            param: sc_param,
            value: sc_value,
        });
    }

    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    if let Some((target, value)) = record_target {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }
    result
}

pub(super) fn handle_toggle_eq(
    state: &mut AppState,
    instrument_id: crate::state::InstrumentId,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.instrument_mut(instrument_id) {
        instrument.toggle_eq();
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.set_routing_instrument(instrument_id);
    result
}
