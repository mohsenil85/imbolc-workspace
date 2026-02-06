mod input;
mod rendering;

use std::any::Any;


use crate::action::VstTarget;
use crate::state::{AppState, InstrumentId};
use crate::ui::action_id::ActionId;
use crate::ui::{Rect, RenderBuf, Action, InputEvent, Keymap, Pane};

pub struct VstParamPane {
    keymap: Keymap,
    instrument_id: Option<InstrumentId>,
    target: VstTarget,
    selected_param: usize,
    scroll_offset: usize,
    search_text: String,
    search_active: bool,
    filtered_indices: Vec<usize>,
}

impl VstParamPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            instrument_id: None,
            target: VstTarget::Source,
            selected_param: 0,
            scroll_offset: 0,
            search_text: String::new(),
            search_active: false,
            filtered_indices: Vec::new(),
        }
    }

    /// Set the target instrument and VST target (source or effect), resetting selection state
    pub fn set_target(&mut self, instrument_id: InstrumentId, target: VstTarget) {
        self.instrument_id = Some(instrument_id);
        self.target = target;
        self.selected_param = 0;
        self.scroll_offset = 0;
        self.search_text.clear();
        self.search_active = false;
    }

    /// Get the VstPluginId for the current target
    fn get_plugin_id(&self, state: &AppState) -> Option<crate::state::vst_plugin::VstPluginId> {
        let inst = self.instrument_id.and_then(|id| state.instruments.instrument(id))?;
        match self.target {
            VstTarget::Source => {
                if let crate::state::SourceType::Vst(id) = inst.source {
                    Some(id)
                } else {
                    None
                }
            }
            VstTarget::Effect(effect_id) => {
                inst.effect_by_id(effect_id).and_then(|e| {
                    if let crate::state::EffectType::Vst(id) = e.effect_type {
                        Some(id)
                    } else {
                        None
                    }
                })
            }
        }
    }

    /// Rebuild filtered indices based on search text
    fn rebuild_filter(&mut self, state: &AppState) {
        let plugin_id = match self.get_plugin_id(state) {
            Some(id) => id,
            None => {
                self.filtered_indices.clear();
                return;
            }
        };

        let Some(plugin) = state.session.vst_plugins.get(plugin_id) else {
            self.filtered_indices.clear();
            return;
        };

        if self.search_text.is_empty() {
            self.filtered_indices = (0..plugin.params.len()).collect();
        } else {
            let query = self.search_text.to_lowercase();
            self.filtered_indices = plugin.params.iter()
                .enumerate()
                .filter(|(_, p)| p.name.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Sync state from current selection (only when navigated to via instrument selection, not set_target)
    fn sync_from_state(&mut self, state: &AppState) {
        let new_id = state.instruments.selected_instrument().map(|i| i.id);
        if new_id != self.instrument_id {
            self.instrument_id = new_id;
            self.target = VstTarget::Source;
            self.selected_param = 0;
            self.scroll_offset = 0;
            self.search_text.clear();
            self.search_active = false;
        }
        self.rebuild_filter(state);
    }
}

impl Pane for VstParamPane {
    fn id(&self) -> &'static str {
        "vst_params"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.sync_from_state(state);
        self.handle_action_impl(action, event, state)
    }

    fn handle_raw_input(&mut self, event: &InputEvent, state: &AppState) -> Action {
        self.sync_from_state(state);
        self.handle_raw_input_impl(event, state)
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        self.render_impl(area, buf, state);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Keymap;

    #[test]
    fn vst_param_pane_id() {
        let pane = VstParamPane::new(Keymap::new());
        assert_eq!(pane.id(), "vst_params");
    }
}
