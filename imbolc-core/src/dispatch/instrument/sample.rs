use imbolc_audio::AudioHandle;
use crate::state::AppState;
use crate::action::{DispatchResult, NavIntent};
use crate::dispatch::side_effects::AudioSideEffect;

pub(super) fn handle_load_sample_result(
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    instrument_id: crate::state::InstrumentId,
    path: &std::path::Path,
) -> DispatchResult {
    let path_str = path.to_string_lossy().to_string();
    let sample_name = path.file_stem()
        .map(|s| s.to_string_lossy().to_string());

    let buffer_id = state.instruments.next_sampler_buffer_id;
    state.instruments.next_sampler_buffer_id += 1;

    if audio.is_running() {
        effects.push(AudioSideEffect::LoadSample { buffer_id, path: path_str });
    }

    if let Some(instrument) = state.instruments.instrument_mut(instrument_id) {
        if let Some(ref mut config) = instrument.sampler_config_mut() {
            config.buffer_id = Some(buffer_id);
            config.sample_name = sample_name;
        }
    }

    let mut result = DispatchResult::with_nav(NavIntent::Pop);
    result.audio_dirty.instruments = true;
    result
}
