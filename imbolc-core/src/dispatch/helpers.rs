use crate::action::{AudioEffect, DispatchResult};
use crate::state::automation::AutomationTarget;
use crate::state::AppState;

use super::automation::record_automation_point;

/// Record automation point if currently recording and playing.
/// Returns true if a point was recorded (for pushing AudioEffect::UpdateAutomation).
pub fn maybe_record_automation(
    state: &mut AppState,
    result: &mut DispatchResult,
    target: AutomationTarget,
    value: f32,
) {
    if state.recording.automation_recording && state.audio.playing {
        record_automation_point(state, target, value);
        result.audio_effects.push(AudioEffect::UpdateAutomation);
    }
}

/// Set bus mixer params directly if audio is running.
pub fn apply_bus_update(audio: &mut imbolc_audio::AudioHandle, update: Option<(imbolc_types::BusId, f32, bool, f32)>) {
    if let Some((bus_id, level, mute, pan)) = update {
        if audio.is_running() {
            let _ = audio.set_bus_mixer_params(bus_id, level, mute, pan);
        }
    }
}

/// Set layer group mixer params directly if audio is running.
pub fn apply_layer_group_update(audio: &mut imbolc_audio::AudioHandle, update: Option<(u32, f32, bool, f32)>) {
    if let Some((group_id, level, mute, pan)) = update {
        if audio.is_running() {
            let _ = audio.set_layer_group_mixer_params(group_id, level, mute, pan);
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
