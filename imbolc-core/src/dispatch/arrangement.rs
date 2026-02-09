use crate::action::{ArrangementAction, DispatchResult, NavIntent};
use crate::audio::AudioHandle;
use crate::state::arrangement::{ClipEditContext, PlayMode};
use crate::state::AppState;

use super::side_effects::AudioSideEffect;

pub(super) fn dispatch_arrangement(
    action: &ArrangementAction,
    state: &mut AppState,
    audio: &AudioHandle,
    effects: &mut Vec<AudioSideEffect>,
) -> DispatchResult {
    match action {
        ArrangementAction::TogglePlayMode => {
            let arr = &mut state.session.arrangement;
            arr.play_mode = match arr.play_mode {
                PlayMode::Pattern => PlayMode::Song,
                PlayMode::Song => PlayMode::Pattern,
            };
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result
        }
        ArrangementAction::CreateClip { instrument_id, length_ticks } => {
            let clip_id = state.session.arrangement.add_clip("Clip".to_string(), *instrument_id, *length_ticks);
            if let Some(clip) = state.session.arrangement.clip_mut(clip_id) {
                clip.name = format!("Clip {}", clip_id);
            }
            DispatchResult::none()
        }
        ArrangementAction::CaptureClipFromPianoRoll { instrument_id } => {
            let (loop_start, loop_end, track_notes) = {
                let pr = &state.session.piano_roll;
                let notes = pr
                    .tracks
                    .get(instrument_id)
                    .map(|t| t.notes.clone())
                    .unwrap_or_default();
                (pr.loop_start, pr.loop_end, notes)
            };

            let length_ticks = loop_end.saturating_sub(loop_start);
            let mut notes = Vec::new();
            if length_ticks > 0 {
                for note in track_notes {
                    if note.tick >= loop_start && note.tick < loop_end {
                        let mut new_note = note.clone();
                        new_note.tick = note.tick - loop_start;
                        if new_note.tick + new_note.duration > length_ticks {
                            new_note.duration = length_ticks.saturating_sub(new_note.tick);
                        }
                        if new_note.duration > 0 {
                            notes.push(new_note);
                        }
                    }
                }
            }

            // Capture automation lanes for this instrument within the loop region
            let mut clip_automation_lanes = Vec::new();
            if length_ticks > 0 {
                for lane in state.session.automation.lanes_for_instrument(*instrument_id) {
                    let points: Vec<_> = lane
                        .points
                        .iter()
                        .filter(|p| p.tick >= loop_start && p.tick < loop_end)
                        .map(|p| {
                            let mut new_point = p.clone();
                            new_point.tick = p.tick - loop_start;
                            new_point
                        })
                        .collect();
                    if !points.is_empty() {
                        let mut new_lane = lane.clone();
                        new_lane.points = points;
                        new_lane.id = state.session.arrangement.next_clip_lane_id();
                        clip_automation_lanes.push(new_lane);
                    }
                }
            }

            let clip_id = state.session.arrangement.add_clip("Clip".to_string(), *instrument_id, length_ticks);
            if let Some(clip) = state.session.arrangement.clip_mut(clip_id) {
                clip.name = format!("Clip {}", clip_id);
                clip.notes = notes;
                clip.automation_lanes = clip_automation_lanes;
            }
            DispatchResult::none()
        }
        ArrangementAction::DeleteClip(clip_id) => {
            state.session.arrangement.remove_clip(*clip_id);
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::RenameClip(clip_id, name) => {
            if let Some(clip) = state.session.arrangement.clip_mut(*clip_id) {
                clip.name = name.clone();
            }
            DispatchResult::none()
        }
        ArrangementAction::PlaceClip { clip_id, instrument_id, start_tick } => {
            state.session.arrangement.add_placement(*clip_id, *instrument_id, *start_tick);
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::RemovePlacement(placement_id) => {
            state.session.arrangement.remove_placement(*placement_id);
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::MovePlacement { placement_id, new_start_tick } => {
            state.session.arrangement.move_placement(*placement_id, *new_start_tick);
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::ResizePlacement { placement_id, new_length } => {
            state.session.arrangement.resize_placement(*placement_id, *new_length);
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::DuplicatePlacement(placement_id) => {
            let original = state
                .session
                .arrangement
                .placements
                .iter()
                .find(|p| p.id == *placement_id)
                .cloned();

            if let Some(original) = original {
                if let Some(clip) = state.session.arrangement.clip(original.clip_id) {
                    let new_start = original.end_tick(clip);
                    let new_id = state
                        .session
                        .arrangement
                        .add_placement(original.clip_id, original.instrument_id, new_start);
                    if let Some(new_placement) = state
                        .session
                        .arrangement
                        .placements
                        .iter_mut()
                        .find(|p| p.id == new_id)
                    {
                        new_placement.length_override = original.length_override;
                    }
                }
            }
            let mut result = DispatchResult::none();
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::SelectPlacement(selection) => {
            state.session.arrangement.selected_placement = *selection;
            DispatchResult::none()
        }
        ArrangementAction::SelectLane(lane) => {
            let max_lane = state.instruments.instruments.len().saturating_sub(1);
            state.session.arrangement.selected_lane = (*lane).min(max_lane);
            DispatchResult::none()
        }
        ArrangementAction::MoveCursor(delta_cols) => {
            let arr = &mut state.session.arrangement;
            let delta_ticks = *delta_cols as i64 * arr.ticks_per_col as i64;
            let new_tick = (arr.cursor_tick as i64 + delta_ticks).max(0) as u32;
            arr.cursor_tick = new_tick;
            DispatchResult::none()
        }
        ArrangementAction::ScrollView(delta) => {
            let arr = &mut state.session.arrangement;
            let delta_ticks = *delta as i64 * arr.ticks_per_col as i64;
            let new_start = (arr.view_start_tick as i64 + delta_ticks).max(0) as u32;
            arr.view_start_tick = new_start;
            DispatchResult::none()
        }
        ArrangementAction::ZoomIn => {
            let arr = &mut state.session.arrangement;
            arr.ticks_per_col = (arr.ticks_per_col / 2).max(30);
            DispatchResult::none()
        }
        ArrangementAction::ZoomOut => {
            let arr = &mut state.session.arrangement;
            arr.ticks_per_col = (arr.ticks_per_col * 2).min(1920);
            DispatchResult::none()
        }
        ArrangementAction::EnterClipEdit(clip_id) => {
            let clip = match state.session.arrangement.clip(*clip_id).cloned() {
                Some(clip) => clip,
                None => return DispatchResult::none(),
            };

            let (stashed_notes, stashed_loop_start, stashed_loop_end, stashed_looping) = {
                let pr = &mut state.session.piano_roll;
                let track = match pr.tracks.get_mut(&clip.instrument_id) {
                    Some(track) => track,
                    None => return DispatchResult::none(),
                };
                let stashed_notes = track.notes.clone();
                let stashed_loop_start = pr.loop_start;
                let stashed_loop_end = pr.loop_end;
                let stashed_looping = pr.looping;

                track.notes = clip.notes.clone();
                pr.loop_start = 0;
                pr.loop_end = clip.length_ticks;
                pr.looping = true;
                state.audio.playhead = 0;

                (stashed_notes, stashed_loop_start, stashed_loop_end, stashed_looping)
            };

            // Stash session automation lanes for this instrument, then load clip automation
            let stashed_automation_lanes: Vec<_> = state
                .session
                .automation
                .lanes
                .iter()
                .filter(|l| l.target.instrument_id() == Some(clip.instrument_id))
                .cloned()
                .collect();
            let stashed_selected_automation_lane = state.session.automation.selected_lane;

            state.session.automation.remove_lanes_for_instrument(clip.instrument_id);

            // Load clip automation lanes into session
            for clip_lane in &clip.automation_lanes {
                let lane_id = state.session.automation.add_lane(clip_lane.target.clone());
                if let Some(session_lane) = state.session.automation.lane_mut(lane_id) {
                    session_lane.points = clip_lane.points.clone();
                    session_lane.enabled = clip_lane.enabled;
                    session_lane.record_armed = clip_lane.record_armed;
                }
            }

            state.session.arrangement.editing_clip = Some(ClipEditContext {
                clip_id: clip.id,
                instrument_id: clip.instrument_id,
                stashed_notes,
                stashed_loop_start,
                stashed_loop_end,
                stashed_looping,
                stashed_automation_lanes,
                stashed_selected_automation_lane,
            });

            let mut result = DispatchResult::with_nav(NavIntent::PushTo("piano_roll"));
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::ExitClipEdit => {
            let ctx = match state.session.arrangement.editing_clip.take() {
                Some(ctx) => ctx,
                None => return DispatchResult::none(),
            };

            let (edited_notes, loop_end) = {
                let pr = &state.session.piano_roll;
                let notes = pr
                    .tracks
                    .get(&ctx.instrument_id)
                    .map(|t| t.notes.clone())
                    .unwrap_or_default();
                (notes, pr.loop_end)
            };

            // Save session automation lanes for this instrument back into the clip
            let edited_automation_lanes: Vec<_> = state
                .session
                .automation
                .lanes
                .iter()
                .filter(|l| l.target.instrument_id() == Some(ctx.instrument_id))
                .cloned()
                .collect();

            if let Some(clip) = state.session.arrangement.clip_mut(ctx.clip_id) {
                clip.notes = edited_notes;
                clip.length_ticks = loop_end;
                clip.automation_lanes = edited_automation_lanes;
            }

            // Remove clip automation lanes from session and restore stashed lanes
            state.session.automation.remove_lanes_for_instrument(ctx.instrument_id);

            for stashed_lane in &ctx.stashed_automation_lanes {
                let lane_id = state.session.automation.add_lane(stashed_lane.target.clone());
                if let Some(session_lane) = state.session.automation.lane_mut(lane_id) {
                    session_lane.points = stashed_lane.points.clone();
                    session_lane.enabled = stashed_lane.enabled;
                    session_lane.record_armed = stashed_lane.record_armed;
                }
            }
            state.session.automation.selected_lane = ctx.stashed_selected_automation_lane;

            {
                let pr = &mut state.session.piano_roll;
                if let Some(track) = pr.tracks.get_mut(&ctx.instrument_id) {
                    track.notes = ctx.stashed_notes;
                }
                pr.loop_start = ctx.stashed_loop_start;
                pr.loop_end = ctx.stashed_loop_end;
                pr.looping = ctx.stashed_looping;
            }

            let mut result = DispatchResult::with_nav(NavIntent::PopOrSwitchTo("track"));
            result.audio_dirty.piano_roll = true;
            result.audio_dirty.automation = true;
            result
        }
        ArrangementAction::PlayStop => {
            let pr = &mut state.session.piano_roll;
            pr.playing = !pr.playing;
            effects.push(AudioSideEffect::SetPlaying { playing: pr.playing });
            if !pr.playing {
                state.audio.playhead = 0;
                effects.push(AudioSideEffect::ResetPlayhead);
                if audio.is_running() {
                    effects.push(AudioSideEffect::ReleaseAllVoices);
                }
                effects.push(AudioSideEffect::ClearActiveNotes);
            }
            pr.recording = false;
            DispatchResult::none()
        }
    }
}
