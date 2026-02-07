use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::commands::AudioFeedback;
use super::engine::{AudioEngine, SCHEDULE_LOOKAHEAD_SECS};
use crate::state::arpeggiator::ArpPlayState;
use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use crate::state::automation::AutomationTarget;
use imbolc_types::SwingGrid;

fn next_random(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*state >> 33) as f32) / (u32::MAX as f32)
}

pub fn tick_playback(
    piano_roll: &mut PianoRollSnapshot,
    instruments: &mut InstrumentSnapshot,
    session: &SessionSnapshot,
    automation_lanes: &AutomationSnapshot,
    engine: &mut AudioEngine,
    active_notes: &mut Vec<(u32, u8, u32)>,
    arp_states: &mut HashMap<u32, ArpPlayState>,
    rng_state: &mut u64,
    feedback_tx: &Sender<AudioFeedback>,
    elapsed: Duration,
    tick_accumulator: &mut f64,
) {
    // (instrument_id, pitch, velocity, duration, note_tick, probability, ticks_from_old_playhead)
    let mut playback_data: Option<(
        Vec<(u32, u8, u8, u32, u32, f32, f64)>,
        u32,
        u32,
        u32,
        f64,
    )> = None;

    if piano_roll.playing {
        *tick_accumulator += elapsed.as_secs_f64()
            * (piano_roll.bpm as f64 / 60.0)
            * piano_roll.ticks_per_beat as f64;
        let tick_delta = *tick_accumulator as u32;
        *tick_accumulator -= tick_delta as f64;

        if tick_delta > 0 {
            let old_playhead = piano_roll.playhead;
            piano_roll.advance(tick_delta);
            let new_playhead = piano_roll.playhead;

            // Build scan ranges: (start, end, base_ticks_from_old_playhead)
            // On wrap, scan both [old_playhead, loop_end) and [loop_start, new_playhead)
            let wrapped = new_playhead < old_playhead;
            let scan_ranges: Vec<(u32, u32, f64)> = if wrapped {
                vec![
                    (old_playhead, piano_roll.loop_end, 0.0),
                    (piano_roll.loop_start, new_playhead, (piano_roll.loop_end - old_playhead) as f64),
                ]
            } else {
                vec![(old_playhead, new_playhead, 0.0)]
            };

            let secs_per_tick = 60.0 / (piano_roll.bpm as f64 * piano_roll.ticks_per_beat as f64);

            let mut note_ons: Vec<(u32, u8, u8, u32, u32, f32, f64)> = Vec::new();
            let any_solo = instruments.any_instrument_solo();
            for &instrument_id in &piano_roll.track_order {
                if let Some(track) = piano_roll.tracks.get(&instrument_id) {
                    // Expand layer group: collect all target IDs for this instrument
                    let targets = instruments.layer_group_members(instrument_id);

                    for &(scan_start, scan_end, base_ticks) in &scan_ranges {
                        // Binary search for efficiency
                        // Notes are expected to be sorted by tick
                        let start_idx = track.notes.partition_point(|n| n.tick < scan_start);
                        let end_idx = track.notes.partition_point(|n| n.tick < scan_end);

                        for note in &track.notes[start_idx..end_idx] {
                            let ticks_from_old = base_ticks + (note.tick - scan_start) as f64;
                            for &target_id in &targets {
                                // Skip muted/inactive siblings
                                let skip = instruments.instrument(target_id).map_or(true, |inst| {
                                    !inst.active || if any_solo { !inst.solo } else { inst.mute }
                                });
                                if skip { continue; }
                                note_ons.push((
                                    target_id,
                                    note.pitch,
                                    note.velocity,
                                    note.duration,
                                    note.tick,
                                    note.probability,
                                    ticks_from_old,
                                ));
                            }
                        }
                    }
                }
            }

            playback_data = Some((note_ons, old_playhead, new_playhead, tick_delta, secs_per_tick));
        }
    }

    if let Some((note_ons, _old_playhead, new_playhead, tick_delta, secs_per_tick)) = playback_data {
        if engine.is_running() {
            // Global settings used as fallback
            let global_swing = piano_roll.swing_amount;
            let global_humanize_vel = session.humanize.velocity;
            let global_humanize_time = session.humanize.timing;

            for &(instrument_id, pitch, velocity, duration, note_tick, probability, ticks_from_old) in &note_ons {
                // Probability check: skip note if random exceeds probability
                if probability < 1.0 && next_random(rng_state) > probability {
                    continue;
                }

                // Get per-track groove settings, falling back to global
                let groove = instruments.instrument(instrument_id).map(|i| &i.groove);
                let effective_swing = groove
                    .and_then(|g| g.swing_amount)
                    .unwrap_or(global_swing);
                let effective_swing_grid = groove
                    .and_then(|g| g.swing_grid)
                    .unwrap_or(SwingGrid::Eighths);
                let effective_humanize_vel = groove
                    .and_then(|g| g.humanize_velocity)
                    .unwrap_or(global_humanize_vel);
                let effective_humanize_time = groove
                    .and_then(|g| g.humanize_timing)
                    .unwrap_or(global_humanize_time);
                let timing_offset_ms = groove.map(|g| g.timing_offset_ms).unwrap_or(0.0);

                // Check if this instrument has arpeggiator enabled
                let arp_enabled = instruments.instruments.iter()
                    .find(|inst| inst.id == instrument_id)
                    .map(|inst| inst.arpeggiator.enabled)
                    .unwrap_or(false);

                if arp_enabled {
                    // Buffer note for arpeggiator instead of spawning directly
                    let arp = arp_states.entry(instrument_id)
                        .or_insert_with(ArpPlayState::default);
                    if !arp.held_notes.contains(&pitch) {
                        arp.held_notes.push(pitch);
                        arp.held_notes.sort();
                    }
                    // Track as active note so note-off removes from held_notes
                    active_notes.push((instrument_id, pitch, duration));
                    continue;
                }

                let mut offset = ticks_from_old * secs_per_tick + SCHEDULE_LOOKAHEAD_SECS;

                // Apply timing offset (rush/drag) - negative = rush, positive = drag
                offset += (timing_offset_ms / 1000.0) as f64;

                // Apply swing: delay notes on offbeat positions based on swing grid
                if effective_swing > 0.0 {
                    let tpb = piano_roll.ticks_per_beat as f64;
                    let eighth = tpb / 2.0;
                    let sixteenth = tpb / 4.0;
                    let pos_in_beat = (note_tick as f64) % tpb;

                    let apply_eighth_swing = matches!(
                        effective_swing_grid,
                        SwingGrid::Eighths | SwingGrid::Both
                    ) && (pos_in_beat - eighth).abs() < 1.0;

                    let apply_sixteenth_swing = matches!(
                        effective_swing_grid,
                        SwingGrid::Sixteenths | SwingGrid::Both
                    ) && ((pos_in_beat - sixteenth).abs() < 1.0
                        || (pos_in_beat - sixteenth * 3.0).abs() < 1.0);

                    if apply_eighth_swing {
                        offset += effective_swing as f64 * eighth * secs_per_tick * 0.5;
                    } else if apply_sixteenth_swing {
                        offset += effective_swing as f64 * sixteenth * secs_per_tick * 0.5;
                    }
                }

                // Apply timing humanization: jitter offset by up to +/- 20ms
                if effective_humanize_time > 0.0 {
                    let jitter = (next_random(rng_state) - 0.5) * 2.0 * effective_humanize_time * 0.02;
                    offset = (offset + jitter as f64).max(0.0);
                }

                // Apply velocity humanization: jitter velocity by up to +/- 30
                let mut vel_f = velocity as f32 / 127.0;
                if effective_humanize_vel > 0.0 {
                    let jitter = (next_random(rng_state) - 0.5) * 2.0 * effective_humanize_vel * (30.0 / 127.0);
                    vel_f = (vel_f + jitter).clamp(0.01, 1.0);
                }

                let _ = engine.spawn_voice(instrument_id, pitch, vel_f, offset, instruments, session);
                active_notes.push((instrument_id, pitch, duration));
            }

            // Collect automation updates into a single bundle
            let mut automation_msgs = Vec::new();
            for lane in automation_lanes {
                if !lane.enabled {
                    continue;
                }
                if let Some(value) = lane.value_at(new_playhead) {
                    if matches!(lane.target, AutomationTarget::Global(imbolc_types::GlobalParameter::Bpm)) {
                        if (piano_roll.bpm - value).abs() > f32::EPSILON {
                            piano_roll.bpm = value;
                            let _ = feedback_tx.send(AudioFeedback::BpmUpdate(value));
                        }
                    } else {
                        automation_msgs.extend(
                            engine.collect_automation_messages(&lane.target, value, instruments, session)
                        );
                    }
                }
            }
            let _ = engine.send_automation_bundle(automation_msgs, SCHEDULE_LOOKAHEAD_SECS);
        }

        let mut note_offs: Vec<(u32, u8, u32)> = Vec::new();
        for note in active_notes.iter_mut() {
            if note.2 <= tick_delta {
                note_offs.push((note.0, note.1, note.2));
                note.2 = 0;
            } else {
                note.2 -= tick_delta;
            }
        }
        active_notes.retain(|n| n.2 > 0);

        if engine.is_running() {
            for (instrument_id, pitch, remaining) in &note_offs {
                // For arp-enabled instruments, remove from held_notes instead of releasing
                if let Some(arp) = arp_states.get_mut(instrument_id) {
                    arp.held_notes.retain(|&p| p != *pitch);
                    continue;
                }
                let offset = *remaining as f64 * secs_per_tick + SCHEDULE_LOOKAHEAD_SECS;
                let _ = engine.release_voice(*instrument_id, *pitch, offset, instruments);
            }
        }

        let _ = feedback_tx.send(AudioFeedback::PlayheadPosition(new_playhead));
    }
}
