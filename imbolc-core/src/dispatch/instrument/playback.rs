use imbolc_audio::AudioHandle;
use crate::state::AppState;
use crate::action::DispatchResult;
use crate::dispatch::side_effects::AudioSideEffect;

pub(super) fn handle_play_note(
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    pitch: u8,
    velocity: u8,
) -> DispatchResult {
    let instrument_id = state.instruments.selected_instrument().map(|s| s.id);

    if let Some(instrument_id) = instrument_id {
        let targets = state.instruments.layer_group_members(instrument_id);
        if audio.is_running() {
            let vel_f = velocity as f32 / 127.0;
            for &target_id in &targets {
                if let Some(inst) = state.instruments.instrument(target_id) {
                    if state.effective_instrument_mute(inst) { continue; }
                    let pitches = match inst.note_input.chord_shape {
                        Some(shape) => shape.expand(pitch),
                        None => vec![pitch],
                    };
                    for p in &pitches {
                        let p = inst.offset_pitch(*p);
                        effects.push(AudioSideEffect::SpawnVoice {
                            instrument_id: target_id,
                            pitch: p,
                            velocity: vel_f,
                            offset_secs: 0.0,
                        });
                        effects.push(AudioSideEffect::PushActiveNote {
                            instrument_id: target_id,
                            pitch: p,
                            duration_ticks: 240,
                        });
                    }
                }
            }
        }
    }
    DispatchResult::none()
}

pub(super) fn handle_play_notes(
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    pitches: &[u8],
    velocity: u8,
) -> DispatchResult {
    let instrument_id = state.instruments.selected_instrument().map(|s| s.id);

    if let Some(instrument_id) = instrument_id {
        let targets = state.instruments.layer_group_members(instrument_id);
        if audio.is_running() {
            let vel_f = velocity as f32 / 127.0;
            for &target_id in &targets {
                if let Some(inst) = state.instruments.instrument(target_id) {
                    if state.effective_instrument_mute(inst) { continue; }
                    for &pitch in pitches {
                        let expanded = match inst.note_input.chord_shape {
                            Some(shape) => shape.expand(pitch),
                            None => vec![pitch],
                        };
                        for p in &expanded {
                            let p = inst.offset_pitch(*p);
                            effects.push(AudioSideEffect::SpawnVoice {
                                instrument_id: target_id,
                                pitch: p,
                                velocity: vel_f,
                                offset_secs: 0.0,
                            });
                            effects.push(AudioSideEffect::PushActiveNote {
                                instrument_id: target_id,
                                pitch: p,
                                duration_ticks: 240,
                            });
                        }
                    }
                }
            }
        }
    }
    DispatchResult::none()
}

pub(super) fn handle_play_drum_pad(
    state: &AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
    pad_idx: usize,
) -> DispatchResult {
    if let Some(instrument) = state.instruments.selected_instrument() {
        if let Some(seq) = &instrument.drum_sequencer {
            if let Some(pad) = seq.pads.get(pad_idx) {
                if let (Some(buffer_id), instrument_id) = (pad.buffer_id, instrument.id) {
                    let amp = pad.level;
                    let pitch_rate = 2.0_f32.powf(pad.pitch as f32 / 12.0);
                    let rate = if pad.reverse { -pitch_rate } else { pitch_rate };
                    if audio.is_running() {
                        effects.push(AudioSideEffect::PlayDrumHit {
                            buffer_id,
                            amp,
                            instrument_id,
                            slice_start: pad.slice_start,
                            slice_end: pad.slice_end,
                            rate,
                            offset_secs: 0.0,
                        });
                    }
                }
            }
        }
    }
    DispatchResult::none()
}
