use crate::audio::AudioHandle;
use crate::state::AppState;
use crate::state::automation::AutomationTarget;
use crate::action::DispatchResult;

use super::super::automation::record_automation_point;

pub(super) fn handle_set_eq_param(
    state: &mut AppState,
    audio: &mut AudioHandle,
    instrument_id: crate::state::InstrumentId,
    band_idx: usize,
    param_name: &str,
    value: f32,
) -> DispatchResult {
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(instrument_id) {
        if let Some(ref mut eq) = instrument.eq {
            if let Some(band) = eq.bands.get_mut(band_idx) {
                match param_name {
                    "freq" => band.freq = value.clamp(20.0, 20000.0),
                    "gain" => band.gain = value.clamp(-24.0, 24.0),
                    "q" => band.q = value.clamp(0.1, 10.0),
                    "on" => band.enabled = value > 0.5,
                    _ => {}
                }
                if state.recording.automation_recording && state.session.piano_roll.playing {
                    let param_idx = match param_name {
                        "freq" => Some(0),
                        "gain" => Some(1),
                        "q" => Some(2),
                        _ => None,
                    };
                    if let Some(pi) = param_idx {
                        let target = AutomationTarget::EqBandParam(instrument.id, band_idx, pi);
                        record_target = Some((target.clone(), target.normalize_value(value)));
                    }
                }
            }
        }
    }

    // Send real-time param update to audio engine
    if audio.is_running() {
        let sc_param = format!("b{}_{}", band_idx, param_name);
        let sc_value = if param_name == "q" { 1.0 / value } else { value };
        let _ = audio.set_eq_param(instrument_id, &sc_param, sc_value);
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
        if instrument.eq.is_some() {
            instrument.eq = None;
        } else {
            instrument.eq = Some(crate::state::EqConfig::default());
        }
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.instruments = true;
    result.audio_dirty.routing_instrument = Some(instrument_id);
    result
}
