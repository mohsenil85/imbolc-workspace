use std::sync::mpsc::Sender;
use std::time::Duration;

use super::commands::AudioFeedback;
use super::engine::AudioEngine;
use super::snapshot::{InstrumentSnapshot, SessionSnapshot};
use imbolc_types::InstrumentId;

pub fn tick_drum_sequencer(
    instruments: &mut InstrumentSnapshot,
    session: &SessionSnapshot,
    bpm: f32,
    engine: &mut AudioEngine,
    rng_state: &mut u64,
    feedback_tx: &Sender<AudioFeedback>,
    elapsed: Duration,
) {
    // Collect instrument triggers to execute after the main loop
    // (target_instrument_id, freq, velocity, offset_secs)
    let mut instrument_triggers: Vec<(InstrumentId, f32, f32, f64)> = Vec::new();

    for instrument in &mut instruments.instruments {
        let seq = match &mut instrument.drum_sequencer {
            Some(s) => s,
            None => continue,
        };
        if !seq.playing {
            seq.last_played_step = None;
            continue;
        }

        let pattern_length = seq.pattern().length;
        let steps_per_beat = seq.step_resolution.steps_per_beat();
        let steps_per_second = (bpm as f64 / 60.0) * steps_per_beat;
        if steps_per_second <= 0.0 {
            continue;
        }
        let secs_per_step_unit = 1.0 / steps_per_second;

        let old_accum = seq.step_accumulator;
        seq.step_accumulator += elapsed.as_secs_f64() * steps_per_second;

        // Collect all steps that should fire in this tick with their precise offsets.
        // Each entry: (step_index, pattern_index, offset_secs)
        let mut steps_to_play: Vec<(usize, usize, f64)> = Vec::new();
        let mut threshold_consumed: f64 = 0.0;

        loop {
            // Swing threshold depends on which step boundary we're crossing
            let next_step = (seq.current_step + 1) % pattern_length;
            let swing_threshold: f64 = if seq.swing_amount > 0.0 && next_step % 2 == 1 {
                1.0 + seq.swing_amount as f64 * 0.5
            } else if seq.swing_amount > 0.0 && seq.current_step % 2 == 1 {
                1.0 - seq.swing_amount as f64 * 0.5
            } else {
                1.0
            };

            if seq.step_accumulator < swing_threshold {
                break;
            }

            seq.step_accumulator -= swing_threshold;
            threshold_consumed += swing_threshold;

            // Advance step
            let next = seq.current_step + 1;
            if next >= pattern_length {
                // Pattern wrapped — advance chain if enabled
                if seq.chain_enabled && !seq.chain.is_empty() {
                    seq.chain_position = (seq.chain_position + 1) % seq.chain.len();
                    let next_pattern = seq.chain[seq.chain_position];
                    if next_pattern < seq.patterns.len() {
                        seq.current_pattern = next_pattern;
                    }
                }
                seq.current_step = 0;
            } else {
                seq.current_step = next;
            }

            // Precise offset: time from tick start to this step crossing
            let offset_secs = ((threshold_consumed - old_accum) * secs_per_step_unit).max(0.0)
                + engine.schedule_lookahead_secs;

            steps_to_play.push((seq.current_step, seq.current_pattern, offset_secs));
        }

        // Handle initial step when sequencer first starts (no threshold crossed yet)
        if steps_to_play.is_empty() && seq.last_played_step != Some(seq.current_step) {
            steps_to_play.push((seq.current_step, seq.current_pattern, engine.schedule_lookahead_secs));
        }

        // Play each step with its precise offset
        for &(step, pattern_idx, offset_secs) in &steps_to_play {
            if engine.is_running() && !instrument.mute {
                let pattern = &seq.patterns[pattern_idx];
                for (pad_idx, pad) in seq.pads.iter().enumerate() {
                    if let Some(step_data) = pattern
                        .steps
                        .get(pad_idx)
                        .and_then(|s| s.get(step))
                    {
                        if !step_data.active {
                            continue;
                        }

                        // Probability check: skip hit if random exceeds probability
                        if step_data.probability < 1.0 {
                            *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                            let r = ((*rng_state >> 33) as f32) / (u32::MAX as f32);
                            if r > step_data.probability { continue; }
                        }

                        // Per-track groove settings
                        let effective_humanize_vel = instrument.groove
                            .humanize_velocity
                            .unwrap_or(session.humanize.velocity);
                        let effective_humanize_time = instrument.groove
                            .humanize_timing
                            .unwrap_or(session.humanize.timing);
                        let timing_offset_ms = instrument.groove.timing_offset_ms;

                        // Calculate final offset with timing offset (rush/drag)
                        let mut final_offset = offset_secs + (timing_offset_ms / 1000.0) as f64;

                        // Timing humanization: jitter offset by up to +/- 20ms
                        if effective_humanize_time > 0.0 {
                            *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                            let r = ((*rng_state >> 33) as f32) / (u32::MAX as f32);
                            let jitter = (r - 0.5) * 2.0 * effective_humanize_time * 0.02;
                            final_offset = (final_offset + jitter as f64).max(0.0);
                        }

                        let mut amp = (step_data.velocity as f32 / 127.0) * pad.level;
                        // Velocity humanization using per-track setting
                        if effective_humanize_vel > 0.0 {
                            *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                            let r = ((*rng_state >> 33) as f32) / (u32::MAX as f32);
                            let jitter = (r - 0.5) * 2.0 * effective_humanize_vel * (30.0 / 127.0);
                            amp = (amp + jitter).clamp(0.01, 1.0);
                        }

                        // Calculate pitch offset (used for both samples and instruments)
                        let total_pitch = pad.pitch as i16 + step_data.pitch_offset as i16;

                        // Check if this pad triggers an instrument (one-shot synth)
                        if let Some(target_instrument_id) = pad.instrument_id {
                            // Instrument trigger mode: collect for execution after loop
                            let freq = pad.trigger_freq * 2.0_f32.powf(total_pitch as f32 / 12.0);
                            instrument_triggers.push((target_instrument_id, freq, amp, final_offset));
                        } else if let Some(buffer_id) = pad.buffer_id {
                            // Sample mode: play one-shot sample
                            let pitch_rate = 2.0_f32.powf(total_pitch as f32 / 12.0);
                            let rate = if pad.reverse { -pitch_rate } else { pitch_rate };
                            let _ = engine.play_drum_hit_to_instrument(
                                buffer_id, amp, instrument.id,
                                pad.slice_start, pad.slice_end, rate, final_offset,
                            );
                        }
                    }
                }
            }
            let _ = feedback_tx.send(AudioFeedback::DrumSequencerStep {
                instrument_id: instrument.id,
                step,
            });
            seq.last_played_step = Some(step);
        }
    }

    // Execute collected instrument triggers (needs immutable borrow of instruments)
    for (target_id, freq, amp, offset) in instrument_triggers {
        let _ = engine.trigger_instrument_oneshot(target_id, freq, amp, offset, instruments, session);
    }
}
