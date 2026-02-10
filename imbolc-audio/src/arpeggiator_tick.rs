use std::collections::HashMap;
use std::time::Duration;

use super::engine::AudioEngine;
use imbolc_types::ArpDirection;
use crate::arp_state::ArpPlayState;
use super::snapshot::{InstrumentSnapshot, SessionSnapshot};

pub fn tick_arpeggiator(
    instruments: &InstrumentSnapshot,
    session: &SessionSnapshot,
    bpm: f32,
    arp_states: &mut HashMap<u32, ArpPlayState>,
    engine: &mut AudioEngine,
    rng_state: &mut u64,
    elapsed: Duration,
) {
    // Collect instrument ids and arp configs to avoid borrow conflicts
    let arp_instruments: Vec<(u32, imbolc_types::ArpeggiatorConfig)> = instruments
        .instruments
        .iter()
        .filter(|inst| inst.arpeggiator.enabled)
        .map(|inst| (inst.id, inst.arpeggiator.clone()))
        .collect();

    for (instrument_id, config) in arp_instruments {
        let arp = arp_states.entry(instrument_id).or_insert_with(ArpPlayState::default);

        if arp.held_notes.is_empty() {
            // Release any currently sounding note
            if let Some(pitch) = arp.current_pitch.take() {
                if engine.is_running() {
                    let targets = instruments.layer_group_members(instrument_id);
                    for &target_id in &targets {
                        let release_pitch = instruments.instrument(target_id)
                            .map_or(pitch, |i| i.offset_pitch(pitch));
                        let _ = engine.release_voice(target_id, release_pitch, 0.0, instruments);
                    }
                }
            }
            continue;
        }

        // Build the note sequence: held notes across octaves
        let mut sequence: Vec<u8> = Vec::new();
        for octave in 0..config.octaves {
            for &note in &arp.held_notes {
                let pitched = note as i16 + (octave as i16 * 12);
                if (0..=127).contains(&pitched) {
                    sequence.push(pitched as u8);
                }
            }
        }
        if sequence.is_empty() {
            continue;
        }

        // Advance accumulator
        let steps_per_second = (bpm as f64 / 60.0) * config.rate.steps_per_beat() as f64;
        arp.accumulator += elapsed.as_secs_f64() * steps_per_second;

        let step_duration_secs = if steps_per_second > 0.0 { 1.0 / steps_per_second } else { 0.0 };
        let mut step_offset: f64 = engine.schedule_lookahead_secs;

        while arp.accumulator >= 1.0 {
            arp.accumulator -= 1.0;

            // Release previous note
            if let Some(pitch) = arp.current_pitch.take() {
                if engine.is_running() {
                    let targets = instruments.layer_group_members(instrument_id);
                    for &target_id in &targets {
                        let release_pitch = instruments.instrument(target_id)
                            .map_or(pitch, |i| i.offset_pitch(pitch));
                        let _ = engine.release_voice(target_id, release_pitch, step_offset, instruments);
                    }
                }
            }

            // Select next note based on direction
            let seq_len = sequence.len();
            let pitch = match config.direction {
                ArpDirection::Up => {
                    arp.step_index = (arp.step_index + 1) % seq_len;
                    sequence[arp.step_index]
                }
                ArpDirection::Down => {
                    if arp.step_index == 0 {
                        arp.step_index = seq_len - 1;
                    } else {
                        arp.step_index -= 1;
                    }
                    sequence[arp.step_index]
                }
                ArpDirection::UpDown => {
                    if seq_len <= 1 {
                        sequence[0]
                    } else {
                        if arp.ascending {
                            arp.step_index += 1;
                            if arp.step_index >= seq_len {
                                arp.step_index = seq_len - 2;
                                arp.ascending = false;
                            }
                        } else {
                            if arp.step_index == 0 {
                                arp.step_index = 1;
                                arp.ascending = true;
                            } else {
                                arp.step_index -= 1;
                            }
                        }
                        sequence[arp.step_index.min(seq_len - 1)]
                    }
                }
                ArpDirection::Random => {
                    // Use inline RNG to pick random index
                    *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let r = ((*rng_state >> 33) as usize) % seq_len;
                    sequence[r]
                }
            };

            // Spawn the new note (fan-out to layer group siblings)
            if engine.is_running() {
                let vel_f = 0.8; // Default velocity for arp notes
                let any_solo = instruments.any_instrument_solo();
                let targets = instruments.layer_group_members(instrument_id);
                for &target_id in &targets {
                    let inst = instruments.instrument(target_id);
                    let skip = inst.map_or(true, |inst| {
                        !inst.active || if any_solo { !inst.solo } else { inst.mute }
                    });
                    if skip { continue; }
                    let target_pitch = inst.map_or(pitch, |i| i.offset_pitch(pitch));
                    let _ = engine.spawn_voice(target_id, target_pitch, vel_f, step_offset, instruments, session);
                }
            }
            arp.current_pitch = Some(pitch);
            step_offset += step_duration_secs;
        }
    }

    // Clean up arp states for instruments that no longer have arp enabled
    let active_ids: Vec<u32> = instruments
        .instruments
        .iter()
        .filter(|inst| inst.arpeggiator.enabled)
        .map(|inst| inst.id)
        .collect();
    arp_states.retain(|id, state| {
        if !active_ids.contains(id) {
            // Release any sounding note before removing
            if let Some(pitch) = state.current_pitch {
                if engine.is_running() {
                    let targets = instruments.layer_group_members(*id);
                    for &target_id in &targets {
                        let release_pitch = instruments.instrument(target_id)
                            .map_or(pitch, |i| i.offset_pitch(pitch));
                        let _ = engine.release_voice(target_id, release_pitch, 0.0, instruments);
                    }
                }
            }
            false
        } else {
            true
        }
    });
}
