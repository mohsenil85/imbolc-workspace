use crate::{Note, PianoRollAction, SessionState};

pub(super) fn reduce(action: &PianoRollAction, session: &mut SessionState) -> bool {
    match action {
        PianoRollAction::ToggleNote {
            pitch,
            tick,
            duration,
            velocity,
            track,
        } => {
            session
                .piano_roll
                .toggle_note(*track, *pitch, *tick, *duration, *velocity);
            true
        }
        PianoRollAction::PlayStop => {
            let pr = &mut session.piano_roll;
            pr.playing = !pr.playing;
            if !pr.playing {
                pr.recording = false;
            }
            true
        }
        PianoRollAction::PlayStopRecord => {
            let is_playing = session.piano_roll.playing;
            if !is_playing {
                session.piano_roll.playing = true;
                session.piano_roll.recording = true;
            } else {
                session.piano_roll.playing = false;
                session.piano_roll.recording = false;
            }
            true
        }
        PianoRollAction::ToggleLoop => {
            session.piano_roll.looping = !session.piano_roll.looping;
            true
        }
        PianoRollAction::SetLoopStart(tick) => {
            session.piano_roll.loop_start = *tick;
            true
        }
        PianoRollAction::SetLoopEnd(tick) => {
            session.piano_roll.loop_end = *tick;
            true
        }
        PianoRollAction::CycleTimeSig => {
            let new_ts = match session.time_signature {
                (4, 4) => (3, 4),
                (3, 4) => (6, 8),
                (6, 8) => (5, 4),
                (5, 4) => (7, 8),
                _ => (4, 4),
            };
            session.time_signature = new_ts;
            session.piano_roll.time_signature = new_ts;
            true
        }
        PianoRollAction::TogglePolyMode(track_idx) => {
            if let Some(track) = session.piano_roll.track_at_mut(*track_idx) {
                track.polyphonic = !track.polyphonic;
            }
            true
        }
        PianoRollAction::AdjustSwing(delta) => {
            let pr = &mut session.piano_roll;
            pr.swing_amount = (pr.swing_amount + delta).clamp(0.0, 1.0);
            true
        }
        PianoRollAction::DeleteNotesInRegion {
            track,
            start_tick,
            end_tick,
            start_pitch,
            end_pitch,
        } => {
            if let Some(t) = session.piano_roll.track_at_mut(*track) {
                t.notes.retain(|n| {
                    !(n.pitch >= *start_pitch
                        && n.pitch <= *end_pitch
                        && n.tick >= *start_tick
                        && n.tick < *end_tick)
                });
            }
            true
        }
        PianoRollAction::PasteNotes {
            track,
            anchor_tick,
            anchor_pitch,
            notes,
        } => {
            if let Some(t) = session.piano_roll.track_at_mut(*track) {
                for cn in notes {
                    let tick = *anchor_tick + cn.tick_offset;
                    let pitch_i16 = *anchor_pitch as i16 + cn.pitch_offset;
                    if !(0..=127).contains(&pitch_i16) {
                        continue;
                    }
                    let pitch = pitch_i16 as u8;
                    if !t.notes.iter().any(|n| n.pitch == pitch && n.tick == tick) {
                        let pos = t.notes.partition_point(|n| n.tick < tick);
                        t.notes.insert(
                            pos,
                            Note {
                                tick,
                                duration: cn.duration,
                                pitch,
                                velocity: cn.velocity,
                                probability: cn.probability,
                            },
                        );
                    }
                }
            }
            true
        }
        // PlayNote/PlayNotes: voice spawning only
        PianoRollAction::PlayNote { .. } | PianoRollAction::PlayNotes { .. } => true,
        // ReleaseNote/ReleaseNotes: audio side effect only
        PianoRollAction::ReleaseNote { .. } | PianoRollAction::ReleaseNotes { .. } => true,
        // CopyNotes: clipboard only
        PianoRollAction::CopyNotes { .. } => true,
        // Render/Export: file I/O
        PianoRollAction::RenderToWav(_)
        | PianoRollAction::BounceToWav
        | PianoRollAction::ExportStems
        | PianoRollAction::CancelExport => false,
    }
}
