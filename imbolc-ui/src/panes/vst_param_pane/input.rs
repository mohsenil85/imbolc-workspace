use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use crate::ui::action_id::{ActionId, VstParamsActionId};
use crate::ui::{Action, AutomationAction, InputEvent, KeyCode, VstParamAction};

use super::VstParamPane;

impl VstParamPane {
    pub(super) fn handle_action_impl(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        if self.search_active {
            return match action {
                ActionId::VstParams(VstParamsActionId::Escape) | ActionId::VstParams(VstParamsActionId::Cancel) => {
                    self.search_active = false;
                    self.search_text.clear();
                    self.rebuild_filter(state);
                    Action::None
                }
                _ => Action::None,
            };
        }

        let Some(instrument_id) = self.instrument_id else {
            return Action::None;
        };
        let target = self.target;

        match action {
            ActionId::VstParams(VstParamsActionId::Up) | ActionId::VstParams(VstParamsActionId::Prev) => {
                if self.selected_param > 0 {
                    self.selected_param -= 1;
                    // Adjust scroll
                    if self.selected_param < self.scroll_offset {
                        self.scroll_offset = self.selected_param;
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Down) | ActionId::VstParams(VstParamsActionId::Next) => {
                if !self.filtered_indices.is_empty() && self.selected_param + 1 < self.filtered_indices.len() {
                    self.selected_param += 1;
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Left) | ActionId::VstParams(VstParamsActionId::AdjustDown) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(instrument_id, target, idx, -0.01));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Right) | ActionId::VstParams(VstParamsActionId::AdjustUp) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(instrument_id, target, idx, 0.01));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::CoarseLeft) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(instrument_id, target, idx, -0.1));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::CoarseRight) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::AdjustParam(instrument_id, target, idx, 0.1));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Reset) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::VstParam(VstParamAction::ResetParam(instrument_id, target, idx));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Automate) => {
                if let Some(&param_idx) = self.filtered_indices.get(self.selected_param) {
                    let idx = self.get_param_index(param_idx, state);
                    if let Some(idx) = idx {
                        return Action::Automation(AutomationAction::AddLane(
                            AutomationTarget::VstParam(instrument_id, idx),
                        ));
                    }
                }
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::Discover) => {
                Action::VstParam(VstParamAction::DiscoverParams(instrument_id, target))
            }
            ActionId::VstParams(VstParamsActionId::Search) => {
                self.search_active = true;
                self.search_text.clear();
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::GotoTop) => {
                self.selected_param = 0;
                self.scroll_offset = 0;
                Action::None
            }
            ActionId::VstParams(VstParamsActionId::GotoBottom) => {
                if !self.filtered_indices.is_empty() {
                    self.selected_param = self.filtered_indices.len() - 1;
                }
                Action::None
            }
            _ => Action::None,
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
