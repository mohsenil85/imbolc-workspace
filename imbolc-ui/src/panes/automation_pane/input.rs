use crate::state::automation::{AutomationTarget, AutomationTargetExt, CurveType};
use crate::state::AppState;
use crate::ui::action_id::{ActionId, AutomationActionId};
use crate::ui::{Action, AutomationAction, InputEvent};

use super::{AutomationFocus, AutomationPane, TargetPickerState};

impl AutomationPane {
    pub(super) fn handle_action_impl(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        // If target picker is active, delegate to it
        if matches!(self.target_picker, TargetPickerState::Active { .. }) {
            return self.handle_target_picker_action(action, state);
        }

        match action {
            // Focus switching
            ActionId::Automation(AutomationActionId::SwitchFocus) => {
                self.focus = match self.focus {
                    AutomationFocus::LaneList => AutomationFocus::Timeline,
                    AutomationFocus::Timeline => AutomationFocus::LaneList,
                };
                Action::None
            }

            // Lane list actions
            ActionId::Automation(AutomationActionId::Up) | ActionId::Automation(AutomationActionId::Prev) => {
                if self.focus == AutomationFocus::LaneList {
                    Action::Automation(AutomationAction::SelectLane(-1))
                } else {
                    // Timeline: move value up
                    self.cursor_value = (self.cursor_value + 0.05).min(1.0);
                    Action::None
                }
            }
            ActionId::Automation(AutomationActionId::Down) | ActionId::Automation(AutomationActionId::Next) => {
                if self.focus == AutomationFocus::LaneList {
                    Action::Automation(AutomationAction::SelectLane(1))
                } else {
                    // Timeline: move value down
                    self.cursor_value = (self.cursor_value - 0.05).max(0.0);
                    Action::None
                }
            }
            ActionId::Automation(AutomationActionId::Left) => {
                if self.focus == AutomationFocus::Timeline {
                    let tpc = self.ticks_per_cell();
                    self.cursor_tick = self.cursor_tick.saturating_sub(tpc);
                    // Scroll view if needed
                    if self.cursor_tick < self.view_start_tick {
                        self.view_start_tick = self.cursor_tick;
                    }
                }
                Action::None
            }
            ActionId::Automation(AutomationActionId::Right) => {
                if self.focus == AutomationFocus::Timeline {
                    let tpc = self.ticks_per_cell();
                    self.cursor_tick += tpc;
                }
                Action::None
            }

            // Add lane
            ActionId::Automation(AutomationActionId::AddLane) => {
                let editing_clip = state.session.arrangement.editing_clip.is_some();
                let mut options: Vec<AutomationTarget> = Vec::new();
                if let Some(inst) = state.instruments.selected_instrument() {
                    options = AutomationTarget::targets_for_instrument_context(inst, &state.session.vst_plugins);
                    // Add send targets
                    let id = inst.id;
                    for (idx, _send) in inst.sends.iter().enumerate() {
                        options.push(AutomationTarget::SendLevel(id, idx));
                    }
                }
                // Add global targets (skip when editing a clip — only instrument targets apply)
                if !editing_clip {
                    for bus_id in 1..=8u8 {
                        options.push(AutomationTarget::BusLevel(bus_id));
                    }
                    options.push(AutomationTarget::Bpm);
                }

                self.target_picker = TargetPickerState::Active { options, cursor: 0 };
                Action::None
            }

            // Remove lane
            ActionId::Automation(AutomationActionId::RemoveLane) => {
                if let Some(id) = self.selected_lane_id(state) {
                    Action::Automation(AutomationAction::RemoveLane(id))
                } else {
                    Action::None
                }
            }

            // Toggle lane enabled
            ActionId::Automation(AutomationActionId::ToggleEnabled) => {
                if let Some(id) = self.selected_lane_id(state) {
                    Action::Automation(AutomationAction::ToggleLaneEnabled(id))
                } else {
                    Action::None
                }
            }

            // Place/remove point (timeline)
            ActionId::Automation(AutomationActionId::PlacePoint) => {
                if self.focus == AutomationFocus::Timeline {
                    if let Some(id) = self.selected_lane_id(state) {
                        let tick = self.snap_tick(self.cursor_tick);
                        let lane = state.session.automation.lane(id);
                        if let Some(lane) = lane {
                            if lane.point_at(tick).is_some() {
                                // Remove existing point
                                Action::Automation(AutomationAction::RemovePoint(id, tick))
                            } else {
                                // Add new point
                                Action::Automation(AutomationAction::AddPoint(id, tick, self.cursor_value))
                            }
                        } else {
                            Action::None
                        }
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }

            // Delete point at cursor
            ActionId::Automation(AutomationActionId::DeletePoint) => {
                if self.focus == AutomationFocus::Timeline {
                    if let Some(id) = self.selected_lane_id(state) {
                        let tick = self.snap_tick(self.cursor_tick);
                        Action::Automation(AutomationAction::RemovePoint(id, tick))
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }

            // Cycle curve type at cursor
            ActionId::Automation(AutomationActionId::CycleCurve) => {
                if self.focus == AutomationFocus::Timeline {
                    if let Some(id) = self.selected_lane_id(state) {
                        let tick = self.snap_tick(self.cursor_tick);
                        if let Some(lane) = state.session.automation.lane(id) {
                            if let Some(point) = lane.point_at(tick) {
                                let new_curve = match point.curve {
                                    CurveType::Linear => CurveType::Exponential,
                                    CurveType::Exponential => CurveType::Step,
                                    CurveType::Step => CurveType::SCurve,
                                    CurveType::SCurve => CurveType::Linear,
                                };
                                return Action::Automation(AutomationAction::SetCurveType(id, tick, new_curve));
                            }
                        }
                    }
                }
                Action::None
            }

            // Clear lane
            ActionId::Automation(AutomationActionId::ClearLane) => {
                if let Some(id) = self.selected_lane_id(state) {
                    Action::Automation(AutomationAction::ClearLane(id))
                } else {
                    Action::None
                }
            }

            // Toggle recording
            ActionId::Automation(AutomationActionId::ToggleRecording) => {
                Action::Automation(AutomationAction::ToggleRecording)
            }

            // Lane arm/disarm
            ActionId::Automation(AutomationActionId::ToggleArm) => {
                if let Some(id) = self.selected_lane_id(state) {
                    Action::Automation(AutomationAction::ToggleLaneArm(id))
                } else {
                    Action::None
                }
            }
            ActionId::Automation(AutomationActionId::ArmAll) => {
                Action::Automation(AutomationAction::ArmAllLanes)
            }
            ActionId::Automation(AutomationActionId::DisarmAll) => {
                Action::Automation(AutomationAction::DisarmAllLanes)
            }

            // Zoom
            ActionId::Automation(AutomationActionId::ZoomIn) => {
                self.zoom_level = self.zoom_level.saturating_sub(1).max(1);
                Action::None
            }
            ActionId::Automation(AutomationActionId::ZoomOut) => {
                self.zoom_level = (self.zoom_level + 1).min(5);
                Action::None
            }

            // Home / End
            ActionId::Automation(AutomationActionId::Home) => {
                self.cursor_tick = 0;
                self.view_start_tick = 0;
                Action::None
            }
            ActionId::Automation(AutomationActionId::End) => {
                // Jump to the last point in the selected lane
                if let Some(lane) = state.session.automation.selected() {
                    if let Some(last) = lane.points.last() {
                        self.cursor_tick = last.tick;
                        let tpc = self.ticks_per_cell();
                        self.view_start_tick = self.cursor_tick.saturating_sub(tpc * 10);
                    }
                }
                Action::None
            }

            // Play/stop (pass through to piano roll)
            ActionId::Automation(AutomationActionId::PlayStop) => {
                Action::PianoRoll(crate::ui::PianoRollAction::PlayStop)
            }

            _ => Action::None,
        }
    }

    /// Handle actions while the target picker is active
    pub(super) fn handle_target_picker_action(&mut self, action: ActionId, _state: &AppState) -> Action {
        if let TargetPickerState::Active { ref options, ref mut cursor } = self.target_picker {
            match action {
                ActionId::Automation(AutomationActionId::Up) | ActionId::Automation(AutomationActionId::Prev) => {
                    if *cursor > 0 { *cursor -= 1; }
                    Action::None
                }
                ActionId::Automation(AutomationActionId::Down) | ActionId::Automation(AutomationActionId::Next) => {
                    if *cursor + 1 < options.len() { *cursor += 1; }
                    Action::None
                }
                ActionId::Automation(AutomationActionId::Confirm) | ActionId::Automation(AutomationActionId::AddLane) => {
                    if let Some(target) = options.get(*cursor).cloned() {
                        self.target_picker = TargetPickerState::Inactive;
                        Action::Automation(AutomationAction::AddLane(target))
                    } else {
                        Action::None
                    }
                }
                ActionId::Automation(AutomationActionId::Cancel) | ActionId::Automation(AutomationActionId::Escape) => {
                    self.target_picker = TargetPickerState::Inactive;
                    Action::None
                }
                _ => Action::None,
            }
        } else {
            Action::None
        }
    }
}
