use imbolc_audio::AudioHandle;
use imbolc_types::{DomainAction, PianoRollAction};
use crate::state::AppState;
use crate::state::{ClipboardContents, ClipboardNote};
use crate::action::{AudioEffect, DispatchResult};

fn reduce(action: &PianoRollAction, state: &mut AppState) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::PianoRoll(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn dispatch_piano_roll(
    action: &PianoRollAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    match action {
        PianoRollAction::ToggleNote { .. } => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::PlayStop => {
            // Ignore play/stop while exporting — user must cancel first
            if state.io.pending_export.is_some() || state.io.pending_render.is_some() {
                return DispatchResult::none();
            }
            reduce(action, state);
            let playing = state.session.piano_roll.playing;
            state.audio.playing = playing;
            audio.set_playing(playing);
            if !playing {
                state.audio.playhead = 0;
                audio.reset_playhead();
                if audio.is_running() {
                    audio.release_all_voices();
                }
                audio.clear_active_notes();
            }
            // Clear recording unconditionally via normal play/stop
            state.session.piano_roll.recording = false;
            DispatchResult::none()
        }
        PianoRollAction::PlayStopRecord => {
            let was_playing = state.audio.playing;
            reduce(action, state);

            if !was_playing {
                // Started playing + recording
                state.audio.playing = true;
                audio.set_playing(true);
            } else {
                // Stopped playing + recording
                state.audio.playing = false;
                state.audio.playhead = 0;
                audio.set_playing(false);
                audio.reset_playhead();
                if audio.is_running() {
                    audio.release_all_voices();
                }
                audio.clear_active_notes();
            }
            DispatchResult::none()
        }
        PianoRollAction::ToggleLoop => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::SetLoopStart(_) => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::SetLoopEnd(_) => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::CycleTimeSig => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result.audio_effects.push(AudioEffect::RebuildSession);
            result
        }
        PianoRollAction::TogglePolyMode(_) => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::PlayNote { pitch, velocity, instrument_id, track } => {
            let pitch = *pitch;
            let velocity = *velocity;
            let instrument_id = *instrument_id;
            let track = *track;

            // Fan-out to layer group members
            let targets = state.instruments.layer_group_members(instrument_id);

            if audio.is_running() {
                let vel_f = velocity as f32 / 127.0;
                for &target_id in &targets {
                    if let Some(inst) = state.instruments.instrument(target_id) {
                        if state.effective_instrument_mute(inst) { continue; }
                        let expanded: Vec<u8> = match inst.note_input.chord_shape {
                            Some(shape) => shape.expand(pitch),
                            None => vec![pitch],
                        };
                        for &p in &expanded {
                            let p = inst.offset_pitch(p);
                            let _ = audio.spawn_voice(target_id, p, vel_f, 0.0);
                            audio.push_active_note(target_id, p, 240);
                        }
                    }
                }
            } else if !state.session.piano_roll.recording {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "Audio engine not running");
            }

            // Record note only on the original track (not siblings)
            if state.session.piano_roll.recording {
                let chord_shape = state.instruments.instrument(instrument_id)
                    .and_then(|inst| inst.note_input.chord_shape);
                let record_pitches: Vec<u8> = match chord_shape {
                    Some(shape) => shape.expand(pitch),
                    None => vec![pitch],
                };
                let playhead = state.audio.playhead;
                let duration = 480; // One beat for live recording
                for &p in &record_pitches {
                    state.session.piano_roll.toggle_note(track, p, playhead, duration, velocity);
                }
                let mut result = DispatchResult::none();
                result.audio_effects.push(AudioEffect::UpdatePianoRoll);
                return result;
            }
            DispatchResult::none()
        }
        PianoRollAction::PlayNotes { pitches, velocity, instrument_id, track } => {
            let velocity = *velocity;
            let instrument_id = *instrument_id;
            let track = *track;

            // Fan-out to layer group members
            let targets = state.instruments.layer_group_members(instrument_id);

            if audio.is_running() {
                let vel_f = velocity as f32 / 127.0;
                for &target_id in &targets {
                    if let Some(inst) = state.instruments.instrument(target_id) {
                        if state.effective_instrument_mute(inst) { continue; }
                        for &pitch in pitches {
                            let expanded: Vec<u8> = match inst.note_input.chord_shape {
                                Some(shape) => shape.expand(pitch),
                                None => vec![pitch],
                            };
                            for &p in &expanded {
                                let p = inst.offset_pitch(p);
                                let _ = audio.spawn_voice(target_id, p, vel_f, 0.0);
                                audio.push_active_note(target_id, p, 240);
                            }
                        }
                    }
                }
            } else if !state.session.piano_roll.recording {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "Audio engine not running");
            }

            // Record chord notes only on the original track (not siblings)
            if state.session.piano_roll.recording {
                let chord_shape = state.instruments.instrument(instrument_id)
                    .and_then(|inst| inst.note_input.chord_shape);
                let all_pitches: Vec<u8> = pitches.iter().flat_map(|&pitch| {
                    match chord_shape {
                        Some(shape) => shape.expand(pitch),
                        None => vec![pitch],
                    }
                }).collect();
                let playhead = state.audio.playhead;
                let duration = 480; // One beat for live recording
                for &p in &all_pitches {
                    state.session.piano_roll.toggle_note(track, p, playhead, duration, velocity);
                }
                let mut result = DispatchResult::none();
                result.audio_effects.push(AudioEffect::UpdatePianoRoll);
                return result;
            }
            DispatchResult::none()
        }
        PianoRollAction::AdjustSwing(_) => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::RenderToWav(instrument_id) => {
            let instrument_id = *instrument_id;
            if state.io.pending_render.is_some() || state.io.pending_export.is_some() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Running, "Already rendering or exporting");
            }
            if !audio.is_running() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "Audio engine not running");
            }

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let render_dir = std::path::Path::new(&home).join(".config/imbolc/renders");
            let _ = std::fs::create_dir_all(&render_dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let path = render_dir.join(format!("render_{}_{}.wav", instrument_id, timestamp));

            let pr = &mut state.session.piano_roll;
            state.io.pending_render = Some(crate::state::PendingRender {
                instrument_id,
                path: path.clone(),
                was_looping: pr.looping,
            });

            pr.playhead = pr.loop_start;
            pr.playing = true;
            state.audio.playing = true;
            pr.looping = false;

            let _ = audio.start_instrument_render(instrument_id, &path);

            let mut result = DispatchResult::with_status(imbolc_audio::ServerStatus::Running, "Rendering...");
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::DeleteNotesInRegion { .. } => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::PasteNotes { .. } => {
            reduce(action, state);
            let mut result = DispatchResult::none();
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::BounceToWav => {
            if state.io.pending_render.is_some() || state.io.pending_export.is_some() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Running, "Already rendering or exporting");
            }
            if !audio.is_running() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "Audio engine not running");
            }
            if state.instruments.instruments.is_empty() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "No instruments");
            }

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let export_dir = std::path::Path::new(&home).join(".config/imbolc/exports");
            let _ = std::fs::create_dir_all(&export_dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let path = export_dir.join(format!("bounce_{}.wav", timestamp));

            let pr = &mut state.session.piano_roll;
            state.io.pending_export = Some(crate::state::PendingExport {
                kind: imbolc_audio::commands::ExportKind::MasterBounce,
                was_looping: pr.looping,
                paths: vec![path.clone()],
            });

            pr.playhead = pr.loop_start;
            pr.playing = true;
            state.audio.playing = true;
            pr.looping = false;

            let _ = audio.start_master_bounce(&path);

            let mut result = DispatchResult::with_status(
                imbolc_audio::ServerStatus::Running,
                "Bouncing to WAV...",
            );
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::ExportStems => {
            if state.io.pending_render.is_some() || state.io.pending_export.is_some() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Running, "Already rendering or exporting");
            }
            if !audio.is_running() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "Audio engine not running");
            }
            if state.instruments.instruments.is_empty() {
                return DispatchResult::with_status(imbolc_audio::ServerStatus::Stopped, "No instruments");
            }

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let export_dir = std::path::Path::new(&home).join(".config/imbolc/exports");
            let _ = std::fs::create_dir_all(&export_dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let mut stems = Vec::new();
            let mut paths = Vec::new();
            for inst in &state.instruments.instruments {
                let safe_name: String = inst
                    .name
                    .replace(' ', "_")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                let path = export_dir.join(format!("stem_{}_{}.wav", safe_name, timestamp));
                stems.push((inst.id, path.clone()));
                paths.push(path);
            }

            let pr = &mut state.session.piano_roll;
            state.io.pending_export = Some(crate::state::PendingExport {
                kind: imbolc_audio::commands::ExportKind::StemExport,
                was_looping: pr.looping,
                paths,
            });

            pr.playhead = pr.loop_start;
            pr.playing = true;
            state.audio.playing = true;
            pr.looping = false;

            let _ = audio.start_stem_export(&stems);

            let mut result = DispatchResult::with_status(
                imbolc_audio::ServerStatus::Running,
                "Exporting stems...".to_string(),
            );
            result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            result
        }
        PianoRollAction::CancelExport => {
            if state.io.pending_export.is_some() {
                let _ = audio.cancel_export();
                let pr = &mut state.session.piano_roll;
                if let Some(export) = state.io.pending_export.take() {
                    pr.looping = export.was_looping;
                }
                pr.playing = false;
                state.audio.playing = false;
                state.audio.playhead = 0;
                state.io.export_progress = 0.0;
                audio.reset_playhead();
                let mut result = DispatchResult::with_status(
                    imbolc_audio::ServerStatus::Running,
                    "Export cancelled",
                );
                result.audio_effects.push(AudioEffect::UpdatePianoRoll);
                return result;
            }
            DispatchResult::none()
        }
        PianoRollAction::CopyNotes { track, start_tick, end_tick, start_pitch, end_pitch } => {
            if let Some(t) = state.session.piano_roll.track_at(*track) {
                let mut notes = Vec::new();
                for note in &t.notes {
                    if note.tick >= *start_tick && note.tick < *end_tick
                        && note.pitch >= *start_pitch && note.pitch <= *end_pitch
                    {
                        notes.push(ClipboardNote {
                            tick_offset: note.tick - start_tick,
                            pitch_offset: note.pitch as i16 - *start_pitch as i16,
                            duration: note.duration,
                            velocity: note.velocity,
                            probability: note.probability,
                        });
                    }
                }
                if !notes.is_empty() {
                    state.clipboard.contents = Some(ClipboardContents::PianoRollNotes(notes));
                }
            }
            DispatchResult::none()
        }
        PianoRollAction::ReleaseNote { pitch, instrument_id } => {
            // Fan-out to layer group members
            let targets = state.instruments.layer_group_members(*instrument_id);

            if audio.is_running() {
                for &target_id in &targets {
                    if let Some(inst) = state.instruments.instrument(target_id) {
                        let _ = audio.release_voice(target_id, inst.offset_pitch(*pitch), 0.0);
                    }
                }
            }
            DispatchResult::none()
        }
        PianoRollAction::ReleaseNotes { pitches, instrument_id } => {
            // Fan-out to layer group members
            let targets = state.instruments.layer_group_members(*instrument_id);

            if audio.is_running() {
                for &target_id in &targets {
                    if let Some(inst) = state.instruments.instrument(target_id) {
                        for &pitch in pitches {
                            let _ = audio.release_voice(target_id, inst.offset_pitch(pitch), 0.0);
                        }
                    }
                }
            }
            DispatchResult::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imbolc_audio::AudioHandle;
    use crate::state::ClipboardContents;

    fn setup() -> (AppState, AudioHandle) {
        let state = AppState::new();
        let audio = AudioHandle::new();
        (state, audio)
    }

    #[test]
    fn toggle_note_adds_note_and_sets_dirty() {
        let (mut state, mut audio) = setup();
        let _id = state.add_instrument(crate::state::SourceType::Saw);
        let action = PianoRollAction::ToggleNote {
            pitch: 60,
            tick: 0,
            duration: 480,
            velocity: 100,
            track: 0,
        };
        let result = dispatch_piano_roll(&action, &mut state, &mut audio);
        assert!(result.audio_effects.contains(&AudioEffect::UpdatePianoRoll));
        assert_eq!(state.session.piano_roll.track_at(0).unwrap().notes.len(), 1);

        // Toggle again removes
        let result = dispatch_piano_roll(&action, &mut state, &mut audio);
        assert!(result.audio_effects.contains(&AudioEffect::UpdatePianoRoll));
        assert!(state.session.piano_roll.track_at(0).unwrap().notes.is_empty());
    }

    #[test]
    fn play_stop_toggles_playing_and_clears_recording() {
        let (mut state, mut audio) = setup();
        state.session.piano_roll.recording = true;

        let action = PianoRollAction::PlayStop;
        dispatch_piano_roll(&action, &mut state, &mut audio);
        assert!(state.session.piano_roll.playing);

        dispatch_piano_roll(&action, &mut state, &mut audio);
        assert!(!state.session.piano_roll.playing);
        assert!(!state.session.piano_roll.recording);
    }

    #[test]
    fn play_stop_noop_while_exporting() {
        let (mut state, mut audio) = setup();
        state.io.pending_export = Some(crate::state::PendingExport {
            kind: imbolc_audio::commands::ExportKind::MasterBounce,
            was_looping: false,
            paths: vec![],
        });
        let action = PianoRollAction::PlayStop;
        dispatch_piano_roll(&action, &mut state, &mut audio);
        assert!(!state.session.piano_roll.playing);
    }

    #[test]
    fn toggle_loop_flips() {
        let (mut state, mut audio) = setup();
        // Default is looping=true
        assert!(state.session.piano_roll.looping);
        dispatch_piano_roll(&PianoRollAction::ToggleLoop, &mut state, &mut audio);
        assert!(!state.session.piano_roll.looping);
        dispatch_piano_roll(&PianoRollAction::ToggleLoop, &mut state, &mut audio);
        assert!(state.session.piano_roll.looping);
    }

    #[test]
    fn cycle_time_sig() {
        let (mut state, mut audio) = setup();
        let expected = vec![(3, 4), (6, 8), (5, 4), (7, 8), (4, 4)];
        for ts in expected {
            dispatch_piano_roll(&PianoRollAction::CycleTimeSig, &mut state, &mut audio);
            assert_eq!(state.session.time_signature, ts);
            assert_eq!(state.session.piano_roll.time_signature, ts);
        }
    }

    #[test]
    fn adjust_swing_clamps() {
        let (mut state, mut audio) = setup();
        dispatch_piano_roll(&PianoRollAction::AdjustSwing(2.0), &mut state, &mut audio);
        assert!((state.session.piano_roll.swing_amount - 1.0).abs() < f32::EPSILON);

        dispatch_piano_roll(&PianoRollAction::AdjustSwing(-5.0), &mut state, &mut audio);
        assert!((state.session.piano_roll.swing_amount - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn delete_notes_in_region() {
        let (mut state, mut audio) = setup();
        let _id = state.add_instrument(crate::state::SourceType::Saw);
        // Add notes
        state.session.piano_roll.toggle_note(0, 60, 0, 480, 100);
        state.session.piano_roll.toggle_note(0, 64, 480, 480, 100);
        state.session.piano_roll.toggle_note(0, 72, 960, 480, 100);

        let action = PianoRollAction::DeleteNotesInRegion {
            track: 0,
            start_tick: 0,
            end_tick: 960,
            start_pitch: 60,
            end_pitch: 64,
        };
        dispatch_piano_roll(&action, &mut state, &mut audio);
        let notes = &state.session.piano_roll.track_at(0).unwrap().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 72);
    }

    #[test]
    fn paste_notes_skips_duplicates_and_clamps_pitch() {
        let (mut state, mut audio) = setup();
        let _id = state.add_instrument(crate::state::SourceType::Saw);
        // Pre-existing note at (60, 0)
        state.session.piano_roll.toggle_note(0, 60, 0, 480, 100);

        let clipboard_notes = vec![
            ClipboardNote { tick_offset: 0, pitch_offset: 0, duration: 480, velocity: 100, probability: 1.0 },
            ClipboardNote { tick_offset: 480, pitch_offset: -200, duration: 480, velocity: 100, probability: 1.0 }, // out of range
            ClipboardNote { tick_offset: 480, pitch_offset: 2, duration: 480, velocity: 100, probability: 1.0 },
        ];
        let action = PianoRollAction::PasteNotes {
            track: 0,
            anchor_tick: 0,
            anchor_pitch: 60,
            notes: clipboard_notes,
        };
        dispatch_piano_roll(&action, &mut state, &mut audio);
        let notes = &state.session.piano_roll.track_at(0).unwrap().notes;
        // Original + one valid paste (duplicate and out-of-range skipped)
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn copy_notes_populates_clipboard() {
        let (mut state, mut audio) = setup();
        let _id = state.add_instrument(crate::state::SourceType::Saw);
        state.session.piano_roll.toggle_note(0, 60, 0, 480, 100);
        state.session.piano_roll.toggle_note(0, 64, 240, 480, 100);

        let action = PianoRollAction::CopyNotes {
            track: 0,
            start_tick: 0,
            end_tick: 480,
            start_pitch: 60,
            end_pitch: 64,
        };
        dispatch_piano_roll(&action, &mut state, &mut audio);
        match &state.clipboard.contents {
            Some(ClipboardContents::PianoRollNotes(notes)) => {
                assert_eq!(notes.len(), 2);
                assert_eq!(notes[0].tick_offset, 0);
                assert_eq!(notes[1].tick_offset, 240);
            }
            _ => panic!("Expected PianoRollNotes in clipboard"),
        }
    }
}
