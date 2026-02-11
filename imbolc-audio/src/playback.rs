use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::commands::AudioFeedback;
use super::engine::AudioEngine;
use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use crate::arp_state::ArpPlayState;
use imbolc_types::SwingGrid;
use imbolc_types::{AutomationTarget, InstrumentId};

fn next_random(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) as f32) / (u32::MAX as f32)
}

#[allow(clippy::too_many_arguments)]
pub fn tick_playback(
    piano_roll: &mut PianoRollSnapshot,
    instruments: &mut InstrumentSnapshot,
    session: &SessionSnapshot,
    automation_lanes: &AutomationSnapshot,
    engine: &mut AudioEngine,
    active_notes: &mut Vec<(InstrumentId, u8, u32)>,
    arp_states: &mut HashMap<InstrumentId, ArpPlayState>,
    rng_state: &mut u64,
    feedback_tx: &Sender<AudioFeedback>,
    elapsed: Duration,
    tick_accumulator: &mut f64,
    last_scheduled_tick: &mut Option<u32>,
) {
    // (instrument_id, pitch, velocity, duration, note_tick, probability, ticks_from_old_playhead)
    #[allow(clippy::type_complexity)]
    let mut playback_data: Option<(
        Vec<(InstrumentId, u8, u8, u32, u32, f32, f64)>,
        u32,
        u32,
        u32,
        f64,
        u32, // scan_end for updating last_scheduled_tick
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

            let secs_per_tick = 60.0 / (piano_roll.bpm as f64 * piano_roll.ticks_per_beat as f64);

            // Compute lookahead in ticks for pre-scheduling
            let lookahead_ticks = ((engine.schedule_lookahead_secs * piano_roll.bpm as f64 / 60.0)
                * piano_roll.ticks_per_beat as f64) as u32;

            // Determine scan window using high-water mark
            // scan_start: resume from where we left off, or old_playhead if no prior scheduling
            let scan_start = last_scheduled_tick.unwrap_or(old_playhead);
            // scan_end: schedule up to new_playhead + lookahead_ticks
            let scan_end_raw = new_playhead.wrapping_add(lookahead_ticks);

            // Build scan ranges: handle loop wrapping
            // Notes in [scan_start, scan_end) need to be scheduled.
            // base_ticks is relative to old_playhead for offset calculation.
            let wrapped_playhead = new_playhead < old_playhead;
            let _loop_len = if piano_roll.loop_end > piano_roll.loop_start {
                piano_roll.loop_end - piano_roll.loop_start
            } else {
                1 // avoid division by zero
            };

            let scan_ranges: Vec<(u32, u32, f64)>;
            let effective_scan_end: u32;

            if wrapped_playhead {
                // Playhead already wrapped during advance.
                // scan_start is before loop_end; new_playhead is after loop_start.
                // We need: [scan_start, loop_end) and [loop_start, new_playhead + lookahead)
                let after_wrap_end = (new_playhead + lookahead_ticks).min(piano_roll.loop_end);
                let _pre_wrap_base = 0.0_f64; // ticks from old_playhead to scan_start
                let post_wrap_base = (piano_roll.loop_end - old_playhead) as f64;

                scan_ranges = vec![
                    (
                        scan_start,
                        piano_roll.loop_end,
                        (scan_start as f64 - old_playhead as f64),
                    ),
                    (piano_roll.loop_start, after_wrap_end, post_wrap_base),
                ];
                effective_scan_end = after_wrap_end;
            } else if scan_end_raw > piano_roll.loop_end
                && piano_roll.loop_end > piano_roll.loop_start
            {
                // Pre-scheduling crosses the loop boundary but playhead hasn't wrapped yet.
                // Scan [scan_start, loop_end) and [loop_start, loop_start + overflow)
                let overflow = scan_end_raw - piano_roll.loop_end;
                let after_wrap_end = (piano_roll.loop_start + overflow).min(piano_roll.loop_end);
                let post_wrap_base = (piano_roll.loop_end - old_playhead) as f64;

                scan_ranges = vec![
                    (
                        scan_start,
                        piano_roll.loop_end,
                        (scan_start as f64 - old_playhead as f64),
                    ),
                    (piano_roll.loop_start, after_wrap_end, post_wrap_base),
                ];
                effective_scan_end = after_wrap_end;
            } else {
                // No wrap — simple linear scan
                let clamped_end = scan_end_raw.min(piano_roll.loop_end);
                scan_ranges = vec![(
                    scan_start,
                    clamped_end,
                    (scan_start as f64 - old_playhead as f64),
                )];
                effective_scan_end = clamped_end;
            };

            let mut note_ons: Vec<(InstrumentId, u8, u8, u32, u32, f32, f64)> = Vec::new();
            let any_solo = instruments.any_instrument_solo();
            for &instrument_id in &piano_roll.track_order {
                if let Some(track) = piano_roll.tracks.get(&instrument_id) {
                    // Expand layer group: collect all target IDs for this instrument
                    let targets = instruments.layer_group_members(instrument_id);

                    for &(range_start, range_end, base_ticks) in &scan_ranges {
                        if range_start >= range_end {
                            continue;
                        }
                        // Binary search for efficiency
                        // Notes are expected to be sorted by tick
                        let start_idx = track.notes.partition_point(|n| n.tick < range_start);
                        let end_idx = track.notes.partition_point(|n| n.tick < range_end);

                        for note in &track.notes[start_idx..end_idx] {
                            let ticks_from_old = base_ticks + (note.tick - range_start) as f64;
                            for &target_id in &targets {
                                // Skip muted/inactive siblings
                                let skip = instruments.instrument(target_id).is_none_or(|inst| {
                                    !inst.mixer.active
                                        || if any_solo {
                                            !inst.mixer.solo
                                        } else {
                                            inst.mixer.mute
                                        }
                                });
                                if skip {
                                    continue;
                                }
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

            playback_data = Some((
                note_ons,
                old_playhead,
                new_playhead,
                tick_delta,
                secs_per_tick,
                effective_scan_end,
            ));
        }
    }

    if let Some((
        note_ons,
        _old_playhead,
        new_playhead,
        tick_delta,
        secs_per_tick,
        effective_scan_end,
    )) = playback_data
    {
        // Update the high-water mark
        *last_scheduled_tick = Some(effective_scan_end);

        if engine.is_running() {
            // Global settings used as fallback
            let global_swing = piano_roll.swing_amount;
            let global_humanize_vel = session.humanize.velocity;
            let global_humanize_time = session.humanize.timing;

            for &(
                instrument_id,
                pitch,
                velocity,
                duration,
                note_tick,
                probability,
                ticks_from_old,
            ) in &note_ons
            {
                // Probability check: skip note if random exceeds probability
                if probability < 1.0 && next_random(rng_state) > probability {
                    continue;
                }

                // Get per-track groove settings, falling back to global
                let groove = instruments.instrument(instrument_id).map(|i| &i.groove);
                let effective_swing = groove.and_then(|g| g.swing_amount).unwrap_or(global_swing);
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
                let arp_enabled = instruments
                    .instruments
                    .iter()
                    .find(|inst| inst.id == instrument_id)
                    .map(|inst| inst.note_input.arpeggiator.enabled)
                    .unwrap_or(false);

                if arp_enabled {
                    // Buffer note for arpeggiator instead of spawning directly
                    let arp = arp_states.entry(instrument_id).or_default();
                    if !arp.held_notes.contains(&pitch) {
                        arp.held_notes.push(pitch);
                        arp.held_notes.sort();
                    }
                    // Track as active note so note-off removes from held_notes
                    active_notes.push((instrument_id, pitch, duration));
                    continue;
                }

                let mut offset = ticks_from_old * secs_per_tick + engine.schedule_lookahead_secs;

                // Apply timing offset (rush/drag) - negative = rush, positive = drag
                offset += (timing_offset_ms / 1000.0) as f64;

                // Apply swing: delay notes on offbeat positions based on swing grid
                if effective_swing > 0.0 {
                    let tpb = piano_roll.ticks_per_beat as f64;
                    let eighth = tpb / 2.0;
                    let sixteenth = tpb / 4.0;
                    let pos_in_beat = (note_tick as f64) % tpb;

                    let apply_eighth_swing =
                        matches!(effective_swing_grid, SwingGrid::Eighths | SwingGrid::Both)
                            && (pos_in_beat - eighth).abs() < 1.0;

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
                    let jitter =
                        (next_random(rng_state) - 0.5) * 2.0 * effective_humanize_time * 0.02;
                    offset = (offset + jitter as f64).max(0.0);
                }

                // Apply velocity humanization: jitter velocity by up to +/- 30
                let mut vel_f = velocity as f32 / 127.0;
                if effective_humanize_vel > 0.0 {
                    let jitter = (next_random(rng_state) - 0.5)
                        * 2.0
                        * effective_humanize_vel
                        * (30.0 / 127.0);
                    vel_f = (vel_f + jitter).clamp(0.01, 1.0);
                }

                let pitch = instruments
                    .instrument(instrument_id)
                    .map_or(pitch, |inst| inst.offset_pitch(pitch));
                // Evict stale entry for same instrument+pitch (voice was already stolen by spawn_voice)
                active_notes.retain(|n| !(n.0 == instrument_id && n.1 == pitch));
                let _ =
                    engine.spawn_voice(instrument_id, pitch, vel_f, offset, instruments, session);
                active_notes.push((instrument_id, pitch, duration));
            }

            // Collect automation updates into a single bundle
            let mut automation_msgs = Vec::new();
            for lane in automation_lanes {
                if !lane.enabled {
                    continue;
                }
                if let Some(value) = lane.value_at(new_playhead) {
                    if matches!(
                        lane.target,
                        AutomationTarget::Global(imbolc_types::GlobalParameter::Bpm)
                    ) {
                        if (piano_roll.bpm - value).abs() > f32::EPSILON {
                            piano_roll.bpm = value;
                            let _ = feedback_tx.send(AudioFeedback::BpmUpdate(value));
                        }
                    } else {
                        automation_msgs.extend(engine.collect_automation_messages(
                            &lane.target,
                            value,
                            instruments,
                            session,
                        ));
                    }
                }
            }
            let _ = engine.send_automation_bundle(automation_msgs, engine.schedule_lookahead_secs);
        }

        let mut note_offs: Vec<(InstrumentId, u8, u32)> = Vec::new();
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
                let offset = *remaining as f64 * secs_per_tick + engine.schedule_lookahead_secs;
                let _ = engine.release_voice(*instrument_id, *pitch, offset, instruments);
            }
        }

        let _ = feedback_tx.send(AudioFeedback::PlayheadPosition(new_playhead));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imbolc_types::PianoRollState;
    use imbolc_types::SourceType;
    use imbolc_types::{InstrumentState, SessionState};
    use std::sync::mpsc;

    /// Helper: create minimal fixtures for tick_playback tests.
    /// Returns (piano_roll, instruments, session, engine, feedback_rx) with one instrument and track.
    fn make_fixtures() -> (
        PianoRollState,
        InstrumentState,
        SessionState,
        AudioEngine,
        mpsc::Receiver<AudioFeedback>,
        mpsc::Sender<AudioFeedback>,
    ) {
        let mut instruments = InstrumentState::new();
        let inst_id = instruments.add_instrument(SourceType::Saw);

        let mut piano_roll = PianoRollState::new();
        piano_roll.playing = true;
        piano_roll.add_track(inst_id);

        let session = SessionState::new();
        let engine = AudioEngine::new();
        let (tx, rx) = mpsc::channel();

        (piano_roll, instruments, session, engine, rx, tx)
    }

    /// Helper: call tick_playback with standard fixtures.
    #[allow(clippy::too_many_arguments)]
    fn do_tick(
        piano_roll: &mut PianoRollState,
        instruments: &mut InstrumentState,
        session: &SessionState,
        engine: &mut AudioEngine,
        feedback_tx: &mpsc::Sender<AudioFeedback>,
        elapsed: Duration,
        tick_accumulator: &mut f64,
        last_scheduled_tick: &mut Option<u32>,
    ) -> Vec<(InstrumentId, u8, u32)> {
        let mut active_notes = Vec::new();
        let mut arp_states = HashMap::new();
        let mut rng_state = 0u64;
        let automation_lanes: AutomationSnapshot = Vec::new();

        tick_playback(
            piano_roll,
            instruments,
            session,
            &automation_lanes,
            engine,
            &mut active_notes,
            &mut arp_states,
            &mut rng_state,
            feedback_tx,
            elapsed,
            tick_accumulator,
            last_scheduled_tick,
        );

        active_notes
    }

    #[test]
    fn last_scheduled_tick_advances_beyond_playhead() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();
        // Place a note at tick 0
        pr.toggle_note(0, 60, 0, 480, 100);
        pr.playhead = 0;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        // At 120 BPM, 480 tpb: 1 tick = 1/960 sec ≈ 1.04ms
        // With default 15ms lookahead: lookahead_ticks ≈ 14
        // Advance enough to get tick_delta > 0
        let elapsed = Duration::from_millis(10); // ~9.6 ticks

        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            elapsed,
            &mut tick_acc,
            &mut last_sched,
        );

        // last_scheduled_tick should be set and ahead of playhead by lookahead_ticks
        assert!(last_sched.is_some(), "last_scheduled_tick should be set");
        let sched = last_sched.unwrap();
        assert!(
            sched > pr.playhead,
            "scheduled tick {} should be ahead of playhead {}",
            sched,
            pr.playhead
        );

        // lookahead_ticks = (0.015 * 120 / 60) * 480 = 14.4 → 14
        let expected_lookahead_ticks =
            ((engine.schedule_lookahead_secs * 120.0 / 60.0) * 480.0) as u32;
        assert_eq!(sched, pr.playhead + expected_lookahead_ticks);
    }

    #[test]
    fn no_double_scheduling_on_consecutive_ticks() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();

        pr.toggle_note(0, 60, 0, 480, 100);
        pr.playhead = 0;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        // First tick: advance ~9 ticks
        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        let first_sched = last_sched.unwrap();

        // Second tick: advance another ~9 ticks
        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        let second_sched = last_sched.unwrap();

        // The high-water mark should advance: second scan starts at first_sched
        // so scan window is [first_sched, new_playhead+lookahead)
        // No overlap with [0, first_sched)
        assert!(
            second_sched > first_sched,
            "second scheduled ({}) should be > first ({})",
            second_sched,
            first_sched
        );

        // Verify the gap is consistent: second_sched = playhead_2 + lookahead_ticks
        let lookahead_ticks = ((engine.schedule_lookahead_secs * 120.0 / 60.0) * 480.0) as u32;
        assert_eq!(second_sched, pr.playhead + lookahead_ticks);
    }

    #[test]
    fn reset_last_scheduled_tick_rescans_from_playhead() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();

        pr.toggle_note(0, 60, 5, 480, 100);
        pr.playhead = 0;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        // First tick
        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );
        assert!(last_sched.is_some());

        // Reset last_scheduled_tick (simulates state invalidation from PianoRollUpdate)
        last_sched = None;

        // Second tick: scan_start falls back to old_playhead (= playhead_after_1)
        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        // last_sched should be set again
        assert!(
            last_sched.is_some(),
            "last_scheduled_tick should be restored after reset"
        );

        // After reset, scan started from old_playhead instead of the high-water mark
        // So last_sched = new_playhead + lookahead_ticks (standard formula)
        let lookahead_ticks = ((engine.schedule_lookahead_secs * 120.0 / 60.0) * 480.0) as u32;
        assert_eq!(last_sched.unwrap(), pr.playhead + lookahead_ticks);
    }

    #[test]
    fn pre_schedule_crosses_loop_boundary() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();

        // Set a short loop: [0, 100)
        pr.loop_start = 0;
        pr.loop_end = 100;

        pr.toggle_note(0, 60, 2, 10, 100);
        pr.toggle_note(0, 64, 95, 10, 100);

        // Position playhead near the loop end so pre-scheduling crosses the boundary
        // At 120 BPM, 480 tpb, 15ms lookahead = ~14 ticks ahead
        // playhead at 85, advance ~9 ticks → new_playhead ~94
        // scan_end = 94 + 14 = 108, which is > loop_end (100)
        // So should split: [85, 100) and [0, 8)
        pr.playhead = 85;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        let elapsed = Duration::from_millis(10);
        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            elapsed,
            &mut tick_acc,
            &mut last_sched,
        );

        // effective_scan_end should be in the wrapped region [loop_start, loop_start + overflow)
        let sched = last_sched.unwrap();
        // The overflow wraps to loop_start + (scan_end_raw - loop_end)
        // Since playhead didn't wrap (94 < 100), we're in the "pre-scheduling crosses loop" branch
        assert!(
            sched < pr.loop_end,
            "effective_scan_end ({}) should be in wrapped region (< loop_end {})",
            sched,
            pr.loop_end
        );
        assert!(
            sched >= pr.loop_start,
            "effective_scan_end ({}) should be >= loop_start ({})",
            sched,
            pr.loop_start
        );
    }

    #[test]
    fn not_playing_leaves_last_scheduled_tick_unchanged() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();
        pr.playing = false;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = Some(42);

        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        assert_eq!(
            last_sched,
            Some(42),
            "last_scheduled_tick should not change when not playing"
        );
    }

    #[test]
    fn same_pitch_retrigger_evicts_stale_active_note() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();
        engine.is_running = true;

        // Two notes at the same pitch (60), different ticks, both within the scan window
        pr.toggle_note(0, 60, 2, 480, 100);
        pr.toggle_note(0, 60, 5, 480, 100);
        pr.playhead = 0;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        let active = do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        // Should have exactly one active_notes entry for pitch 60 (the second note evicts the first)
        let pitch_60_count = active.iter().filter(|n| n.1 == 60).count();
        assert_eq!(
            pitch_60_count, 1,
            "expected 1 active_notes entry for pitch 60 after retrigger, got {}",
            pitch_60_count
        );
    }

    #[test]
    fn lookahead_ticks_scale_with_bpm() {
        let (mut pr, mut inst, session, mut engine, _rx, tx) = make_fixtures();

        pr.toggle_note(0, 60, 0, 480, 100);
        pr.playhead = 0;

        // Test at 200 BPM — lookahead_ticks should be larger
        pr.bpm = 200.0;

        let mut tick_acc = 0.0;
        let mut last_sched: Option<u32> = None;

        do_tick(
            &mut pr,
            &mut inst,
            &session,
            &mut engine,
            &tx,
            Duration::from_millis(10),
            &mut tick_acc,
            &mut last_sched,
        );

        let sched = last_sched.unwrap();
        // At 200 BPM: lookahead_ticks = (0.015 * 200/60) * 480 = 24
        let expected_lt = ((engine.schedule_lookahead_secs * 200.0 / 60.0) * 480.0) as u32;
        assert_eq!(sched, pr.playhead + expected_lt);
        assert!(
            expected_lt > 14,
            "At 200 BPM, lookahead_ticks ({}) should be > 14 (120 BPM value)",
            expected_lt
        );
    }
}
