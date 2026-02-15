//! Generative music engine tick — runs on the audio thread after arpeggiator tick.
//!
//! Generates note events from Euclidean, Markov, and L-System algorithms,
//! spawning voices via the audio engine. Optionally sends events back to the
//! main thread for piano roll capture.

use std::sync::mpsc::Sender;
use std::time::Duration;

use super::engine::AudioEngine;
use super::generative_state::{GenerativePlayState, VoicePlayState};
use super::snapshot::{InstrumentSnapshot, SessionSnapshot};
use imbolc_types::state::drum_sequencer::euclidean_rhythm;
use imbolc_types::state::generative::*;
use imbolc_types::state::music::{Key, Scale};
use imbolc_types::AudioFeedback;

/// Main tick function for the generative engine.
#[allow(clippy::too_many_arguments)]
pub fn tick_generative(
    instruments: &InstrumentSnapshot,
    session: &SessionSnapshot,
    bpm: f32,
    gen_states: &mut GenerativePlayState,
    engine: &mut AudioEngine,
    rng_state: &mut u64,
    elapsed: Duration,
    feedback_tx: &Sender<AudioFeedback>,
) {
    let gen = &session.generative;
    if !gen.enabled || gen.voices.is_empty() {
        return;
    }

    // Collect voice configs to avoid borrow conflicts
    let voice_configs: Vec<_> = gen
        .voices
        .iter()
        .filter(|v| v.enabled && !v.muted && v.target_instrument.is_some())
        .map(|v| (v.id, v.target_instrument.unwrap(), v.algorithm.clone(), v.velocity_min, v.velocity_max, v.octave_min, v.octave_max))
        .collect();

    let tpb = 480u32; // ticks per beat (standard)
    let any_solo = instruments.any_instrument_solo();

    for (voice_id, target_id, algorithm, vel_min, vel_max, oct_min, oct_max) in &voice_configs {
        // Validate target instrument exists and is playable
        let target_inst = match instruments.instrument(*target_id) {
            Some(inst) => inst,
            None => continue,
        };
        let skip = !target_inst.mixer.active
            || if any_solo {
                !target_inst.mixer.solo
            } else {
                target_inst.mixer.mute
            };
        if skip {
            continue;
        }

        let state = gen_states.entry(*voice_id).or_default();
        let rate = algorithm.rate();
        let ticks_per_step = rate.ticks_per_step(tpb);
        let steps_per_second = (bpm as f64 / 60.0) * (tpb as f64 / ticks_per_step);

        state.accumulator += elapsed.as_secs_f64() * steps_per_second;

        let step_duration_secs = if steps_per_second > 0.0 {
            1.0 / steps_per_second
        } else {
            0.0
        };
        let mut step_offset: f64 = engine.schedule_lookahead_secs;

        while state.accumulator >= 1.0 {
            state.accumulator -= 1.0;

            // Release previous note
            if let Some(pitch) = state.current_pitch.take() {
                if engine.is_running() {
                    let _ = engine.release_voice(*target_id, pitch, step_offset, instruments);
                }
            }

            // Generate event based on algorithm
            let event = match algorithm {
                GenerativeAlgorithm::Euclidean(cfg) => {
                    generate_euclidean(cfg, state, &session.key, &session.scale, &gen.constraints, rng_state)
                }
                GenerativeAlgorithm::Markov(cfg) => {
                    generate_markov(cfg, state, &session.key, &session.scale, &gen.constraints, rng_state, *oct_min, *oct_max)
                }
                GenerativeAlgorithm::LSystem(cfg) => {
                    generate_lsystem(cfg, state, &gen.constraints)
                }
            };

            if let Some((pitch, velocity_raw)) = event {
                // Clamp pitch to constraint range
                let pitch = pitch.clamp(gen.constraints.pitch_min, gen.constraints.pitch_max);

                // Apply scale lock
                let pitch = if gen.constraints.scale_lock {
                    snap_to_scale(pitch, &session.key, &session.scale)
                } else {
                    pitch
                };

                // Compute velocity within voice range
                let vel_range = (*vel_max as f32 - *vel_min as f32).max(0.0);
                let velocity = *vel_min as f32 + vel_range * (velocity_raw as f32 / 127.0);

                // Humanize velocity
                let velocity = if gen.constraints.humanize_velocity > 0.0 {
                    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let noise = ((*rng_state >> 33) as f32 / (u32::MAX as f32) - 0.5) * 2.0;
                    (velocity + noise * gen.constraints.humanize_velocity * 20.0).clamp(1.0, 127.0)
                } else {
                    velocity
                };

                let vel_f = velocity / 127.0;

                // Spawn voice
                if engine.is_running() {
                    let _ = engine.spawn_voice(
                        *target_id,
                        pitch,
                        vel_f,
                        step_offset,
                        instruments,
                        session,
                    );
                }
                state.current_pitch = Some(pitch);

                // Send capture feedback if enabled
                if gen.capture_enabled {
                    let duration_ticks = ticks_per_step as u32;
                    let _ = feedback_tx.send(AudioFeedback::GenerativeEvent {
                        instrument_id: *target_id,
                        pitch,
                        velocity: velocity as u8,
                        duration_ticks,
                        tick: 0, // Will be filled by playhead position if playing
                    });
                }
            }

            state.step_index += 1;
            step_offset += step_duration_secs;
        }
    }

    // Cleanup: remove states for voices that no longer exist
    let active_ids: Vec<GenVoiceId> = gen.voices.iter().map(|v| v.id).collect();
    gen_states.retain(|id, state| {
        if !active_ids.contains(id) {
            if let Some(pitch) = state.current_pitch {
                // Find the target instrument for this voice to release
                // Since the voice is gone, we can't know the target. Just let it expire.
                let _ = pitch;
            }
            false
        } else {
            true
        }
    });
}

/// Generate event from Euclidean rhythm.
fn generate_euclidean(
    cfg: &EuclideanConfig,
    state: &mut VoicePlayState,
    key: &Key,
    scale: &Scale,
    constraints: &GenerativeConstraints,
    rng_state: &mut u64,
) -> Option<(u8, u8)> {
    // Cache or regenerate Euclidean pattern
    let pattern = state.euclidean_pattern.get_or_insert_with(|| {
        euclidean_rhythm(cfg.pulses as usize, cfg.steps as usize, cfg.rotation as usize)
    });

    let step = state.step_index % pattern.len().max(1);
    if !pattern.get(step).copied().unwrap_or(false) {
        return None; // Rest step
    }

    // Determine pitch based on mode
    let pitch = match &cfg.pitch_mode {
        EuclideanPitchMode::Fixed(p) => *p,
        EuclideanPitchMode::ScaleWalk => {
            let intervals = scale.intervals();
            if intervals.is_empty() {
                60
            } else {
                let root = key.semitone();
                let scale_len = intervals.len();
                let degree = state.step_index % scale_len;
                let octave = 4 + (state.step_index / scale_len) as i32 % 3;
                let midi = (octave * 12 + root + intervals[degree]).clamp(0, 127);
                midi as u8
            }
        }
        EuclideanPitchMode::RandomInScale => {
            let intervals = scale.intervals();
            if intervals.is_empty() {
                60
            } else {
                *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let root = key.semitone();
                let degree = ((*rng_state >> 33) as usize) % intervals.len();
                *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let min_oct = (constraints.pitch_min / 12) as i32;
                let max_oct = (constraints.pitch_max / 12) as i32;
                let oct_range = (max_oct - min_oct).max(1);
                let octave = min_oct + ((*rng_state >> 33) as i32 % oct_range);
                let midi = (octave * 12 + root + intervals[degree]).clamp(0, 127);
                midi as u8
            }
        }
    };

    Some((pitch, 100)) // Default velocity (will be scaled by voice range)
}

/// Generate event from Markov chain.
fn generate_markov(
    cfg: &MarkovConfig,
    state: &mut VoicePlayState,
    key: &Key,
    _scale: &Scale,
    _constraints: &GenerativeConstraints,
    rng_state: &mut u64,
    oct_min: i8,
    oct_max: i8,
) -> Option<(u8, u8)> {
    // Check for rest
    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let rest_roll = (*rng_state >> 33) as f32 / (u32::MAX as f32);
    if rest_roll < cfg.rest_probability {
        return None;
    }

    // Transition to next pitch class
    let from = state.markov_current_pc as usize % 12;
    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let roll = (*rng_state >> 33) as f32 / (u32::MAX as f32);

    let mut cumulative = 0.0;
    let mut next_pc = from;
    for (i, &weight) in cfg.transition_matrix[from].iter().enumerate() {
        cumulative += weight;
        if roll < cumulative {
            next_pc = i;
            break;
        }
    }

    state.markov_current_pc = next_pc as u8;

    // Map pitch class to MIDI note in a random octave within range
    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let oct_range = (oct_max as i16 - oct_min as i16).max(1);
    let octave = oct_min as i16 + ((*rng_state >> 33) as i16 % oct_range);
    let root = key.semitone() as i16;
    let pitch = ((octave * 12) + root + next_pc as i16).clamp(0, 127) as u8;

    // Velocity: moderate with some variation
    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let vel = 80 + ((*rng_state >> 33) as u8 % 48);

    Some((pitch, vel))
}

/// Generate event from L-System expanded string.
fn generate_lsystem(
    cfg: &LSystemConfig,
    state: &mut VoicePlayState,
    constraints: &GenerativeConstraints,
) -> Option<(u8, u8)> {
    // Cache or regenerate expanded string
    let expanded = state
        .lsystem_expanded
        .get_or_insert_with(|| cfg.expand());

    if expanded.is_empty() {
        return None;
    }

    // Walk the expanded string
    let chars: Vec<char> = expanded.chars().collect();
    let mut note_event = None;

    // Process one symbol at a time
    loop {
        if state.lsystem_cursor >= chars.len() {
            state.lsystem_cursor = 0; // Loop
        }

        let ch = chars[state.lsystem_cursor];
        state.lsystem_cursor += 1;

        match ch {
            'F' | 'G' => {
                // Play note at current pitch
                let pitch = state.lsystem_current_pitch.clamp(
                    constraints.pitch_min as i16,
                    constraints.pitch_max as i16,
                ) as u8;
                note_event = Some((pitch, cfg.velocity));
                break;
            }
            '+' => {
                state.lsystem_current_pitch += cfg.step_interval as i16;
            }
            '-' => {
                state.lsystem_current_pitch -= cfg.step_interval as i16;
            }
            '[' => {
                state.lsystem_pitch_stack.push(state.lsystem_current_pitch);
            }
            ']' => {
                if let Some(p) = state.lsystem_pitch_stack.pop() {
                    state.lsystem_current_pitch = p;
                }
            }
            _ => {
                // Unknown symbol: skip
            }
        }

        // Safety: don't loop infinitely if no F/G in the string
        if state.lsystem_cursor >= chars.len() {
            break;
        }
    }

    note_event
}

/// Snap a MIDI pitch to the nearest note in the given key/scale.
fn snap_to_scale(pitch: u8, key: &Key, scale: &Scale) -> u8 {
    let intervals = scale.intervals();
    if intervals.is_empty() || intervals.len() == 12 {
        return pitch; // Chromatic = no snapping needed
    }

    let root = key.semitone();
    let pc = ((pitch as i32 - root) % 12 + 12) % 12;
    let octave = pitch as i32 / 12;

    // Find nearest scale degree
    let mut best = intervals[0];
    let mut best_dist = 12;
    for &interval in intervals {
        let dist = ((pc - interval) % 12 + 12) % 12;
        let dist = dist.min(12 - dist);
        if dist < best_dist {
            best_dist = dist;
            best = interval;
        }
    }

    let result = (octave * 12 + root + best).clamp(0, 127);
    result as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_to_scale_c_major() {
        // C4 = 60, should stay
        assert_eq!(snap_to_scale(60, &Key::C, &Scale::Major), 60);
        // C#4 = 61, should snap to C=60 or D=62
        let snapped = snap_to_scale(61, &Key::C, &Scale::Major);
        assert!(snapped == 60 || snapped == 62);
    }

    #[test]
    fn snap_to_scale_chromatic_no_change() {
        for pitch in 0..=127 {
            assert_eq!(snap_to_scale(pitch, &Key::C, &Scale::Chromatic), pitch);
        }
    }

    #[test]
    fn euclidean_generates_notes_on_pulse() {
        let cfg = EuclideanConfig {
            pulses: 4,
            steps: 4,
            rotation: 0,
            rate: GenRate::Eighth,
            pitch_mode: EuclideanPitchMode::Fixed(60),
        };
        let mut state = VoicePlayState::default();
        let mut rng = 12345u64;
        let constraints = GenerativeConstraints::default();

        // All 4 steps should produce notes since pulses == steps
        for i in 0..4 {
            state.step_index = i;
            let event = generate_euclidean(&cfg, &mut state, &Key::C, &Scale::Major, &constraints, &mut rng);
            assert!(event.is_some(), "Step {} should produce a note", i);
        }
    }

    #[test]
    fn euclidean_rest_steps() {
        let cfg = EuclideanConfig {
            pulses: 1,
            steps: 4,
            rotation: 0,
            rate: GenRate::Eighth,
            pitch_mode: EuclideanPitchMode::Fixed(60),
        };
        let mut state = VoicePlayState::default();
        let mut rng = 12345u64;
        let constraints = GenerativeConstraints::default();

        let mut note_count = 0;
        for i in 0..4 {
            state.step_index = i;
            state.euclidean_pattern = None; // Clear cache each time for test
            if generate_euclidean(&cfg, &mut state, &Key::C, &Scale::Major, &constraints, &mut rng).is_some() {
                note_count += 1;
            }
        }
        assert_eq!(note_count, 1); // Only 1 pulse in 4 steps
    }

    #[test]
    fn markov_generates_some_notes() {
        let cfg = MarkovConfig {
            rest_probability: 0.0, // No rests
            ..Default::default()
        };
        let mut state = VoicePlayState::default();
        let mut rng = 12345u64;
        let constraints = GenerativeConstraints::default();

        let event = generate_markov(&cfg, &mut state, &Key::C, &Scale::Major, &constraints, &mut rng, 3, 6);
        assert!(event.is_some());
    }

    #[test]
    fn markov_rest_probability() {
        let cfg = MarkovConfig {
            rest_probability: 1.0, // Always rest
            ..Default::default()
        };
        let mut state = VoicePlayState::default();
        let mut rng = 12345u64;
        let constraints = GenerativeConstraints::default();

        let event = generate_markov(&cfg, &mut state, &Key::C, &Scale::Major, &constraints, &mut rng, 3, 6);
        assert!(event.is_none());
    }

    #[test]
    fn lsystem_plays_notes_on_f() {
        let cfg = LSystemConfig {
            axiom: "F".to_string(),
            rules: vec![],
            iterations: 0,
            step_interval: 2,
            note_duration_steps: 1,
            velocity: 100,
            rate: GenRate::Eighth,
        };
        let mut state = VoicePlayState::default();
        let constraints = GenerativeConstraints::default();

        let event = generate_lsystem(&cfg, &mut state, &constraints);
        assert!(event.is_some());
        let (pitch, vel) = event.unwrap();
        assert_eq!(pitch, 60); // Default starting pitch
        assert_eq!(vel, 100);
    }

    #[test]
    fn lsystem_pitch_movement() {
        let cfg = LSystemConfig {
            axiom: "+F".to_string(),
            rules: vec![],
            iterations: 0,
            step_interval: 3,
            note_duration_steps: 1,
            velocity: 100,
            rate: GenRate::Eighth,
        };
        let mut state = VoicePlayState::default();
        let constraints = GenerativeConstraints::default();

        let event = generate_lsystem(&cfg, &mut state, &constraints);
        assert!(event.is_some());
        let (pitch, _) = event.unwrap();
        assert_eq!(pitch, 63); // 60 + 3
    }

    #[test]
    fn lsystem_push_pop() {
        let cfg = LSystemConfig {
            axiom: "+[+F]-F".to_string(),
            rules: vec![],
            iterations: 0,
            step_interval: 5,
            note_duration_steps: 1,
            velocity: 100,
            rate: GenRate::Eighth,
        };
        let mut state = VoicePlayState::default();
        let constraints = GenerativeConstraints::default();

        // First call: processes +, [, +, F → pitch = 60+5+5 = 70
        let event1 = generate_lsystem(&cfg, &mut state, &constraints);
        assert_eq!(event1.unwrap().0, 70);

        // Second call: processes ], -, F → pop to 65, then -5 → 60
        let event2 = generate_lsystem(&cfg, &mut state, &constraints);
        assert_eq!(event2.unwrap().0, 60);
    }
}
