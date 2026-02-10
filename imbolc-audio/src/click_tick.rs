//! Click track (metronome) tick logic.
//!
//! Generates synthesized click sounds on beat boundaries during playback.

use std::time::Duration;

use super::engine::AudioEngine;
use super::snapshot::{PianoRollSnapshot, SessionSnapshot};
use imbolc_types::ClickTrackState;

/// Tick the click track, spawning click voices on beat boundaries.
///
/// # Arguments
/// * `engine` - Audio engine for spawning clicks
/// * `click_state` - Current click track state (enabled, volume, muted)
/// * `session` - Session snapshot (for time signature)
/// * `piano_roll` - Piano roll snapshot (for playhead, playing state, BPM)
/// * `elapsed` - Time since last tick
/// * `click_accumulator` - Accumulator tracking fractional beats
pub fn tick_click(
    engine: &mut AudioEngine,
    click_state: &ClickTrackState,
    session: &SessionSnapshot,
    piano_roll: &PianoRollSnapshot,
    elapsed: Duration,
    click_accumulator: &mut f64,
) {
    // Skip if click is disabled, muted, or not playing
    if !click_state.enabled || click_state.muted || !piano_roll.playing {
        return;
    }

    let ticks_per_beat = piano_roll.ticks_per_beat as f64;
    let bpm = piano_roll.bpm as f64;

    if bpm <= 0.0 || ticks_per_beat <= 0.0 {
        return;
    }

    // Calculate beats per second
    let beats_per_second = bpm / 60.0;
    let secs_per_beat = 1.0 / beats_per_second;

    // Accumulate time
    let old_accum = *click_accumulator;
    *click_accumulator += elapsed.as_secs_f64() * beats_per_second;

    // Pre-calculate bar/beat constants
    let beats_per_bar = session.time_signature.0 as u32;
    let ticks_per_beat_u32 = piano_roll.ticks_per_beat;
    let ticks_per_bar = beats_per_bar * ticks_per_beat_u32;

    if ticks_per_bar == 0 {
        return;
    }

    // Calculate the base tick position before any beats in this tick
    // This is where we were at the start of the tick minus accumulated fractional beats
    let base_tick = (piano_roll.playhead as f64) - (*click_accumulator * ticks_per_beat);

    // Track how many beat boundaries we've crossed
    let mut beat_count: u32 = 0;

    // Process all beat boundaries crossed in this tick
    while *click_accumulator >= 1.0 {
        *click_accumulator -= 1.0;
        beat_count += 1;

        // Calculate the tick position for THIS beat boundary
        let beat_tick = (base_tick + (beat_count as f64 * ticks_per_beat)) as u32;

        // Determine beat number within bar for this specific beat
        let beat_in_bar = (beat_tick % ticks_per_bar) / ticks_per_beat_u32;
        let is_downbeat = beat_in_bar == 0;

        // Calculate precise offset from tick start
        let offset_secs = ((beat_count as f64 - old_accum) * secs_per_beat).max(0.0)
            + engine.schedule_lookahead_secs;

        // Spawn the click sound
        if engine.is_running() {
            let _ = engine.spawn_click(is_downbeat, click_state.volume, offset_secs);
        }
    }
}
