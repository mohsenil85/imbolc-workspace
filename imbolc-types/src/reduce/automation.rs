use crate::{AutomationAction, SessionState};

pub(super) fn reduce(action: &AutomationAction, session: &mut SessionState) -> bool {
    match action {
        AutomationAction::AddLane(target) => {
            session.automation.add_lane(target.clone());
            true
        }
        AutomationAction::RemoveLane(id) => {
            session.automation.remove_lane(*id);
            true
        }
        AutomationAction::ToggleLaneEnabled(id) => {
            if let Some(lane) = session.automation.lane_mut(*id) {
                lane.enabled = !lane.enabled;
            }
            true
        }
        AutomationAction::AddPoint(lane_id, tick, value) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                lane.add_point(*tick, *value);
            }
            true
        }
        AutomationAction::RemovePoint(lane_id, tick) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                lane.remove_point(*tick);
            }
            true
        }
        AutomationAction::MovePoint(lane_id, old_tick, new_tick, new_value) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                lane.remove_point(*old_tick);
                lane.add_point(*new_tick, *new_value);
            }
            true
        }
        AutomationAction::SetCurveType(lane_id, tick, curve) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                if let Some(point) = lane.point_at_mut(*tick) {
                    point.curve = *curve;
                }
            }
            true
        }
        AutomationAction::SelectLane(delta) => {
            if *delta > 0 {
                session.automation.select_next();
            } else {
                session.automation.select_prev();
            }
            true
        }
        AutomationAction::ClearLane(id) => {
            if let Some(lane) = session.automation.lane_mut(*id) {
                lane.points.clear();
            }
            true
        }
        AutomationAction::ToggleLaneArm(id) => {
            if let Some(lane) = session.automation.lane_mut(*id) {
                lane.record_armed = !lane.record_armed;
            }
            true
        }
        AutomationAction::ArmAllLanes => {
            for lane in &mut session.automation.lanes {
                lane.record_armed = true;
            }
            true
        }
        AutomationAction::DisarmAllLanes => {
            for lane in &mut session.automation.lanes {
                lane.record_armed = false;
            }
            true
        }
        AutomationAction::DeletePointsInRange(lane_id, start_tick, end_tick) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                lane.points
                    .retain(|p| p.tick < *start_tick || p.tick >= *end_tick);
            }
            true
        }
        AutomationAction::PastePoints(lane_id, anchor_tick, points) => {
            if let Some(lane) = session.automation.lane_mut(*lane_id) {
                for (tick_offset, value) in points {
                    let tick = *anchor_tick + tick_offset;
                    lane.add_point(tick, *value);
                }
            }
            true
        }
        // ToggleRecording: touches state.recording + state.undo_history (not available)
        AutomationAction::ToggleRecording => false,
        // RecordValue: recording depends on state.recording + state.audio.playhead
        AutomationAction::RecordValue(_, _) => true,
        // CopyPoints: clipboard only
        AutomationAction::CopyPoints(_, _, _) => true,
    }
}
