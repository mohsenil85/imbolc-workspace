use crate::action::DispatchResult;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_types::InstrumentId;

use super::automation::record_automation_point;
use super::side_effects::AudioSideEffect;

/// Record automation point if currently recording and playing.
/// Returns true if a point was recorded (for setting audio_dirty.automation).
pub fn maybe_record_automation(
    state: &mut AppState,
    result: &mut DispatchResult,
    target: AutomationTarget,
    value: f32,
) {
    if state.recording.automation_recording && state.audio.playing {
        record_automation_point(state, target, value);
        result.audio_dirty.automation = true;
    }
}

/// Adjust an instrument parameter with clamping and optional automation recording.
/// Generic helper that reduces boilerplate in envelope, LFO, and filter dispatch handlers.
pub fn adjust_instrument_param<F, G>(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
    scale: f32,
    min: f32,
    max: f32,
    get_value: F,
    set_value: G,
    make_target: impl FnOnce(InstrumentId) -> AutomationTarget,
    normalize: impl FnOnce(f32) -> f32,
) -> DispatchResult
where
    F: FnOnce(&crate::state::Instrument) -> f32,
    G: FnOnce(&mut crate::state::Instrument, f32),
{
    let mut record_target: Option<(AutomationTarget, f32)> = None;

    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let old_value = get_value(instrument);
        let new_value = (old_value + delta * scale).clamp(min, max);
        set_value(instrument, new_value);

        if state.recording.automation_recording && state.audio.playing {
            let target = make_target(instrument.id);
            record_target = Some((target, normalize(new_value)));
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

/// Adjust a groove parameter (swing, humanize_velocity, humanize_timing).
/// Falls back to the global session value if no per-instrument override exists.
pub fn adjust_groove_param<F, G>(
    state: &mut AppState,
    id: InstrumentId,
    delta: f32,
    get_override: F,
    set_override: G,
    get_default: impl FnOnce(&crate::state::SessionState) -> f32,
) -> DispatchResult
where
    F: FnOnce(&crate::state::Instrument) -> Option<f32>,
    G: FnOnce(&mut crate::state::Instrument, Option<f32>),
{
    if let Some(instrument) = state.instruments.instrument_mut(id) {
        let current = get_override(instrument).unwrap_or_else(|| get_default(&state.session));
        let new_value = (current + delta).clamp(0.0, 1.0);
        set_override(instrument, Some(new_value));
    }
    DispatchResult::none()
}

/// Push bus mixer params effect if audio is running.
pub fn apply_bus_update(audio: &crate::audio::AudioHandle, effects: &mut Vec<AudioSideEffect>, update: Option<(u8, f32, bool, f32)>) {
    if let Some((bus_id, level, mute, pan)) = update {
        if audio.is_running() {
            effects.push(AudioSideEffect::SetBusMixerParams { bus_id, level, mute, pan });
        }
    }
}

/// Push layer group mixer params effect if audio is running.
pub fn apply_layer_group_update(audio: &crate::audio::AudioHandle, effects: &mut Vec<AudioSideEffect>, update: Option<(u32, f32, bool, f32)>) {
    if let Some((group_id, level, mute, pan)) = update {
        if audio.is_running() {
            effects.push(AudioSideEffect::SetLayerGroupMixerParams { group_id, level, mute, pan });
        }
    }
}

/// Compute waveform peaks from a WAV file for display
pub fn compute_waveform_peaks(path: &str) -> (Vec<f32>, f32) {
    let reader = match hound::WavReader::open(path) {
        Ok(r) => r,
        Err(_) => return (Vec::new(), 0.0),
    };
    let spec = reader.spec();
    let num_channels = spec.channels as usize;
    let sample_rate = spec.sample_rate;
    let num_samples = reader.len() as usize;
    let duration_secs = num_samples as f32 / (sample_rate as f32 * num_channels as f32);

    let target_peaks = 512;
    let samples_per_peak = (num_samples / target_peaks).max(1);

    let mut peaks = Vec::with_capacity(target_peaks);
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader.into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
    };

    for chunk in samples.chunks(samples_per_peak) {
        let peak = chunk.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        peaks.push(peak);
    }

    (peaks, duration_secs)
}
