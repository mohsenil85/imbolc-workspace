use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use crate::ui::action_id::{ActionId, VstParamsActionId};
use crate::ui::{Action, AutomationAction, InputEvent, KeyCode, VstParamAction};

use super::VstParamPane;

impl VstParamPane {
    pub(super) fn handle_action_impl(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        state: &AppState,
    ) -> Action {
        let ActionId::VstParams(action) = action else {
            return Action::None;
        };

        if self.search_active {
            return match action {
                VstParamsActionId::Escape | VstParamsActionId::Cancel => {
                    self.search_active = false;
                    self.search_text.clear();
                    self.rebuild_filter(state);
                    Action::None
                }
                VstParamsActionId::Up
                | VstParamsActionId::Down
                | VstParamsActionId::Prev
                | VstParamsActionId::Next
                | VstParamsActionId::Left
                | VstParamsActionId::Right
                | VstParamsActionId::AdjustDown
                | VstParamsActionId::AdjustUp
                | VstParamsActionId::CoarseLeft
                | VstParamsActionId::CoarseRight
                | VstParamsActionId::Search
                | VstParamsActionId::Reset
                | VstParamsActionId::Automate
                | VstParamsActionId::Discover
                | VstParamsActionId::GotoTop
                | VstParamsActionId::GotoBottom => Action::None,
            };
        }

        let Some(instrument_id) = self.instrument_id else {
            return Action::None;
        };
        let target = self.target;

        match action {
            VstParamsActionId::Up | VstParamsActionId::Prev => {
                if self.selected_param > 0 {
                    self.selected_param -= 1;
                    // Adjust scroll
                    if self.selected_param < self.scroll_offset {
                        self.scroll_offset = self.selected_param;
                    }
                }
                Action::None
            }
            VstParamsActionId::Down | VstParamsActionId::Next => {
                if !self.filtered_indices.is_empty()
                    && self.selected_param + 1 < self.filtered_indices.len()
                {
                    self.selected_param += 1;
                }
                Action::None
            }
            VstParamsActionId::Left | VstParamsActionId::AdjustDown => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(
                            instrument_id,
                            target,
                            idx,
                            -0.01,
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::Right | VstParamsActionId::AdjustUp => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(
                            instrument_id,
                            target,
                            idx,
                            0.01,
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::CoarseLeft => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(
                            instrument_id,
                            target,
                            idx,
                            -0.1,
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::CoarseRight => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(
                            instrument_id,
                            target,
                            idx,
                            0.1,
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::Reset => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::ResetParam(
                            instrument_id,
                            target,
                            idx,
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::Automate => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::Automation(AutomationAction::AddLane(
                            AutomationTarget::vst_param(instrument_id, idx),
                        ));
                    }
                }
                Action::None
            }
            VstParamsActionId::Discover => {
                Action::VstParam(VstParamAction::DiscoverParams(instrument_id, target))
            }
            VstParamsActionId::Search => {
                self.search_active = true;
                self.search_text.clear();
                Action::None
            }
            VstParamsActionId::GotoTop => {
                self.selected_param = 0;
                self.scroll_offset = 0;
                Action::None
            }
            VstParamsActionId::GotoBottom => {
                if !self.filtered_indices.is_empty() {
                    self.selected_param = self.filtered_indices.len() - 1;
                }
                Action::None
            }
            VstParamsActionId::Escape | VstParamsActionId::Cancel => Action::None,
        }
    }

    pub(super) fn handle_raw_input_impl(&mut self, event: &InputEvent, state: &AppState) -> Action {
        if self.search_active {
            match event.key {
                KeyCode::Char(c) => {
                    self.search_text.push(c);
                    self.rebuild_filter(state);
                    self.selected_param = 0;
                    self.scroll_offset = 0;
                    return Action::None;
                }
                KeyCode::Backspace => {
                    self.search_text.pop();
                    self.rebuild_filter(state);
                    self.selected_param = 0;
                    self.scroll_offset = 0;
                    return Action::None;
                }
                KeyCode::Escape | KeyCode::Enter => {
                    self.search_active = false;
                    return Action::None;
                }
                _ => {}
            }
        }
        Action::None
    }

    /// Get the VST parameter index for a filtered param index
    fn get_param_index(&self, filtered_idx: usize, state: &AppState) -> Option<u32> {
        let plugin_id = self.get_plugin_id(state)?;
        let plugin = state.session.vst_plugins.get(plugin_id)?;
        plugin.params.get(filtered_idx).map(|p| p.index)
    }
}
