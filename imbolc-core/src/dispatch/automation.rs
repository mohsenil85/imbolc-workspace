use crate::action::{AudioEffect, AutomationAction, DispatchResult};
use crate::state::automation::AutomationTarget;
use crate::state::{AppState, ClipboardContents};
use imbolc_audio::AudioHandle;
use imbolc_types::DomainAction;

/// Minimum value change threshold for recording (0.5%)
const RECORD_VALUE_THRESHOLD: f32 = 0.005;
/// Minimum tick delta between recorded points (1/10th beat)
const RECORD_MIN_TICK_DELTA: u32 = 48;

fn reduce(state: &mut AppState, action: &AutomationAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Automation(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn dispatch_automation(
    action: &AutomationAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    let mut result = DispatchResult::none();

    // Actions NOT handled by the reducer (keep inline)
    match action {
        AutomationAction::ToggleRecording => {
            if !state.recording.automation_recording {
                state
                    .undo_history
                    .push_from(state.session.clone(), state.instruments.clone());
            }
            state.recording.automation_recording = !state.recording.automation_recording;
            return result;
        }
        AutomationAction::RecordValue(target, value) => {
            // Always apply immediately for audio feedback (e.g. MIDI CC)
            if audio.is_running() {
                let (min, max) = target.default_range();
                let actual_value = min + value * (max - min);
                let _ = audio.apply_automation(target, actual_value);
            }
            // Only record to automation lane when recording + playing
            if state.recording.automation_recording && state.audio.playing {
                record_automation_point(state, target.clone(), *value);
                result.audio_effects.push(AudioEffect::UpdateAutomation);
            }
            return result;
        }
        AutomationAction::CopyPoints(lane_id, start_tick, end_tick) => {
            if *start_tick < *end_tick {
                if let Some(lane) = state.session.automation.lane(*lane_id) {
                    let mut points = Vec::new();
                    for point in &lane.points {
                        if point.tick >= *start_tick && point.tick <= *end_tick {
                            points.push((point.tick - start_tick, point.value));
                        }
                    }
                    if !points.is_empty() {
                        state.clipboard.contents =
                            Some(ClipboardContents::AutomationPoints { points });
                    }
                }
            }
            return result;
        }
        _ => {}
    }

    // Delegate pure state mutation to the shared reducer
    reduce(state, action);

    // Orchestration: AudioEffects
    match action {
        AutomationAction::AddLane(_) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::RemoveLane(_) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::ToggleLaneEnabled(_) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::AddPoint(_, _, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::RemovePoint(_, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::MovePoint(_, _, _, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::SetCurveType(_, _, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::SelectLane(_) => {}
        AutomationAction::ClearLane(_) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::ToggleLaneArm(_) => {}
        AutomationAction::ArmAllLanes => {}
        AutomationAction::DisarmAllLanes => {}
        AutomationAction::DeletePointsInRange(_, _, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        AutomationAction::PastePoints(_, _, _) => {
            result.audio_effects.push(AudioEffect::UpdateAutomation);
        }
        // Already handled above with early returns
        AutomationAction::ToggleRecording
        | AutomationAction::RecordValue(_, _)
        | AutomationAction::CopyPoints(_, _, _) => unreachable!(),
    }

    result
}

/// Record an automation point with thinning.
/// Respects per-lane arm state: auto-arms newly created lanes, skips unarmed lanes.
pub(crate) fn record_automation_point(state: &mut AppState, target: AutomationTarget, value: f32) {
    let playhead = state.audio.playhead;

    // Check if lane already exists before adding
    let is_new = state.session.automation.lane_for_target(&target).is_none();
    let lane_id = state.session.automation.add_lane(target);

    if let Some(lane) = state.session.automation.lane_mut(lane_id) {
        // Auto-arm newly created (empty) lanes during recording
        if is_new {
            lane.record_armed = true;
        }

        // Skip if lane is not armed for recording
        if !lane.record_armed {
            return;
        }

        // Point thinning: skip if value changed less than threshold and tick delta is small
        if let Some(last) = lane.points.last() {
            let value_delta = (value - last.value).abs();
            let tick_delta = playhead.abs_diff(last.tick);
            if value_delta < RECORD_VALUE_THRESHOLD && tick_delta < RECORD_MIN_TICK_DELTA {
                return;
            }
        }
        lane.add_point(playhead, value);
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use crate::state::automation::AutomationLaneId;
    use imbolc_audio::AudioHandle;
    use imbolc_types::InstrumentId;

    fn setup() -> (AppState, AudioHandle) {
        let state = AppState::new();
        let audio = AudioHandle::new();
        (state, audio)
    }

    fn add_lane(state: &mut AppState) -> AutomationLaneId {
        state
            .session
            .automation
            .add_lane(AutomationTarget::level(InstrumentId::new(0)))
    }

    #[test]
    fn add_lane_creates_and_is_idempotent() {
        let (mut state, mut audio) = setup();
        let target = AutomationTarget::level(InstrumentId::new(0));
        let result = dispatch_automation(
            &AutomationAction::AddLane(target.clone()),
            &mut state,
            &mut audio,
        );
        assert!(result
            .audio_effects
            .contains(&AudioEffect::UpdateAutomation));
        assert_eq!(state.session.automation.lanes.len(), 1);

        // Adding same target again doesn't create new lane
        dispatch_automation(&AutomationAction::AddLane(target), &mut state, &mut audio);
        assert_eq!(state.session.automation.lanes.len(), 1);
    }

    #[test]
    fn remove_lane() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        assert_eq!(state.session.automation.lanes.len(), 1);
        dispatch_automation(&AutomationAction::RemoveLane(id), &mut state, &mut audio);
        assert!(state.session.automation.lanes.is_empty());
    }

    #[test]
    fn toggle_lane_enabled() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        assert!(state.session.automation.lane(id).unwrap().enabled);
        dispatch_automation(
            &AutomationAction::ToggleLaneEnabled(id),
            &mut state,
            &mut audio,
        );
        assert!(!state.session.automation.lane(id).unwrap().enabled);
    }

    #[test]
    fn add_and_remove_point() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        dispatch_automation(
            &AutomationAction::AddPoint(id, 100, 0.5),
            &mut state,
            &mut audio,
        );
        assert_eq!(state.session.automation.lane(id).unwrap().points.len(), 1);

        dispatch_automation(
            &AutomationAction::RemovePoint(id, 100),
            &mut state,
            &mut audio,
        );
        assert!(state.session.automation.lane(id).unwrap().points.is_empty());
    }

    #[test]
    fn move_point() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        dispatch_automation(
            &AutomationAction::AddPoint(id, 100, 0.5),
            &mut state,
            &mut audio,
        );
        dispatch_automation(
            &AutomationAction::MovePoint(id, 100, 200, 0.8),
            &mut state,
            &mut audio,
        );
        let lane = state.session.automation.lane(id).unwrap();
        assert_eq!(lane.points.len(), 1);
        assert_eq!(lane.points[0].tick, 200);
        assert!((lane.points[0].value - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn toggle_recording_pushes_undo() {
        let (mut state, mut audio) = setup();
        assert!(!state.undo_history.can_undo());
        dispatch_automation(&AutomationAction::ToggleRecording, &mut state, &mut audio);
        assert!(state.recording.automation_recording);
        assert!(state.undo_history.can_undo());

        dispatch_automation(&AutomationAction::ToggleRecording, &mut state, &mut audio);
        assert!(!state.recording.automation_recording);
    }

    #[test]
    fn arm_all_and_disarm_all() {
        let (mut state, mut audio) = setup();
        add_lane(&mut state);
        state
            .session
            .automation
            .add_lane(AutomationTarget::pan(InstrumentId::new(0)));

        dispatch_automation(&AutomationAction::ArmAllLanes, &mut state, &mut audio);
        assert!(state
            .session
            .automation
            .lanes
            .iter()
            .all(|l| l.record_armed));

        dispatch_automation(&AutomationAction::DisarmAllLanes, &mut state, &mut audio);
        assert!(state
            .session
            .automation
            .lanes
            .iter()
            .all(|l| !l.record_armed));
    }

    #[test]
    fn toggle_lane_arm() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        assert!(!state.session.automation.lane(id).unwrap().record_armed);
        dispatch_automation(&AutomationAction::ToggleLaneArm(id), &mut state, &mut audio);
        assert!(state.session.automation.lane(id).unwrap().record_armed);
    }

    #[test]
    fn record_automation_point_thinning() {
        let (mut state, _audio) = setup();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        let target = AutomationTarget::level(InstrumentId::new(0));

        // First point always added
        state.audio.playhead = 0;
        record_automation_point(&mut state, target.clone(), 0.5);
        let lane_id = state
            .session
            .automation
            .lane_for_target(&target)
            .unwrap()
            .id;
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            1
        );
        // New lane should be auto-armed
        assert!(state.session.automation.lane(lane_id).unwrap().record_armed);

        // Second point too close in both value and tick — should be skipped
        state.audio.playhead = 10;
        record_automation_point(&mut state, target.clone(), 0.502);
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            1
        );

        // Third point: enough tick delta
        state.audio.playhead = 100;
        record_automation_point(&mut state, target.clone(), 0.502);
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            2
        );
    }

    #[test]
    fn record_automation_point_skips_unarmed() {
        let (mut state, _audio) = setup();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        let target = AutomationTarget::level(InstrumentId::new(0));
        let lane_id = state.session.automation.add_lane(target.clone());

        // Disarm the lane
        state
            .session
            .automation
            .lane_mut(lane_id)
            .unwrap()
            .record_armed = false;

        state.audio.playhead = 0;
        record_automation_point(&mut state, target, 0.5);
        assert!(state
            .session
            .automation
            .lane(lane_id)
            .unwrap()
            .points
            .is_empty());
    }

    #[test]
    fn record_value_no_points_when_not_recording() {
        let (mut state, mut audio) = setup();
        // Not recording — RecordValue should NOT add any points
        state.recording.automation_recording = false;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;
        let target = AutomationTarget::level(InstrumentId::new(0));
        dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.5),
            &mut state,
            &mut audio,
        );
        // No lane should be created
        assert!(state.session.automation.lane_for_target(&target).is_none());
    }

    #[test]
    fn record_value_no_points_when_not_playing() {
        let (mut state, mut audio) = setup();
        // Recording but not playing — RecordValue should NOT add points
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = false;
        let target = AutomationTarget::level(InstrumentId::new(0));
        dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.5),
            &mut state,
            &mut audio,
        );
        assert!(state.session.automation.lane_for_target(&target).is_none());
    }

    #[test]
    fn record_value_adds_points_when_recording_and_playing() {
        let (mut state, mut audio) = setup();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;
        state.audio.playhead = 100;
        let target = AutomationTarget::level(InstrumentId::new(0));
        let result = dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.5),
            &mut state,
            &mut audio,
        );
        assert!(result
            .audio_effects
            .contains(&AudioEffect::UpdateAutomation));
        let lane = state.session.automation.lane_for_target(&target).unwrap();
        assert_eq!(lane.points.len(), 1);
        assert_eq!(lane.points[0].tick, 100);
    }

    #[test]
    fn record_value_uses_thinning() {
        let (mut state, mut audio) = setup();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;
        let target = AutomationTarget::level(InstrumentId::new(0));

        // First point
        state.audio.playhead = 0;
        dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.5),
            &mut state,
            &mut audio,
        );
        let lane_id = state
            .session
            .automation
            .lane_for_target(&target)
            .unwrap()
            .id;
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            1
        );

        // Second point: too close in value and tick — should be thinned out
        state.audio.playhead = 10;
        dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.502),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            1
        );

        // Third point: enough tick delta — should be added
        state.audio.playhead = 100;
        dispatch_automation(
            &AutomationAction::RecordValue(target.clone(), 0.502),
            &mut state,
            &mut audio,
        );
        assert_eq!(
            state.session.automation.lane(lane_id).unwrap().points.len(),
            2
        );
    }

    #[test]
    fn delete_points_in_range() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        dispatch_automation(
            &AutomationAction::AddPoint(id, 100, 0.5),
            &mut state,
            &mut audio,
        );
        dispatch_automation(
            &AutomationAction::AddPoint(id, 200, 0.6),
            &mut state,
            &mut audio,
        );
        dispatch_automation(
            &AutomationAction::AddPoint(id, 300, 0.7),
            &mut state,
            &mut audio,
        );

        dispatch_automation(
            &AutomationAction::DeletePointsInRange(id, 100, 250),
            &mut state,
            &mut audio,
        );
        let lane = state.session.automation.lane(id).unwrap();
        assert_eq!(lane.points.len(), 1);
        assert_eq!(lane.points[0].tick, 300);
    }

    #[test]
    fn copy_and_paste_points() {
        let (mut state, mut audio) = setup();
        let id = add_lane(&mut state);
        dispatch_automation(
            &AutomationAction::AddPoint(id, 100, 0.5),
            &mut state,
            &mut audio,
        );
        dispatch_automation(
            &AutomationAction::AddPoint(id, 200, 0.8),
            &mut state,
            &mut audio,
        );

        dispatch_automation(
            &AutomationAction::CopyPoints(id, 50, 250),
            &mut state,
            &mut audio,
        );
        match &state.clipboard.contents {
            Some(ClipboardContents::AutomationPoints { points }) => {
                assert_eq!(points.len(), 2);
            }
            _ => panic!("Expected AutomationPoints"),
        }

        // Paste at offset
        let paste_points = vec![(0, 0.5), (100, 0.8)];
        dispatch_automation(
            &AutomationAction::PastePoints(id, 500, paste_points),
            &mut state,
            &mut audio,
        );
        let lane = state.session.automation.lane(id).unwrap();
        assert_eq!(lane.points.len(), 4);
    }
}
