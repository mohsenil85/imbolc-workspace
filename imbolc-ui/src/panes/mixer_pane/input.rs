use super::{BusDetailSection, DetailTarget, GroupDetailSection, MixerPane, MixerSection};
use super::{CHANNEL_WIDTH, METER_HEIGHT, NUM_VISIBLE_BUSES, NUM_VISIBLE_CHANNELS, NUM_VISIBLE_GROUPS};
use crate::state::{AppState, InstrumentId, MixerSelection};
use crate::ui::action_id::{ActionId, MixerActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, BusAction, InputEvent, InstrumentAction, LayerGroupAction, MixerAction, MouseButton,
    MouseEvent, MouseEventKind, NavAction, Rect,
};

impl MixerPane {
    pub(super) fn handle_action_impl(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        state: &AppState,
    ) -> Action {
        // Detail mode handling
        if let Some(DetailTarget::Bus(_)) = self.detail_mode {
            return self.handle_bus_detail_action(action, state);
        }
        if let Some(DetailTarget::LayerGroup(_)) = self.detail_mode {
            return self.handle_group_detail_action(action, state);
        }
        if self.detail_mode.is_some() {
            return self.handle_detail_action(action, state);
        }

        // Overview mode handling
        match action {
            ActionId::Mixer(MixerActionId::Prev) => {
                self.send_target = None;
                Action::Mixer(MixerAction::Move(-1))
            }
            ActionId::Mixer(MixerActionId::Next) => {
                self.send_target = None;
                Action::Mixer(MixerAction::Move(1))
            }
            ActionId::Mixer(MixerActionId::First) => Action::Mixer(MixerAction::Jump(1)),
            ActionId::Mixer(MixerActionId::Last) => Action::Mixer(MixerAction::Jump(-1)),
            ActionId::Mixer(MixerActionId::LevelUp) => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, 0.05))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(0.05))
                }
            }
            ActionId::Mixer(MixerActionId::LevelDown) => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, -0.05))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(-0.05))
                }
            }
            ActionId::Mixer(MixerActionId::LevelUpBig) => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, 0.10))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(0.10))
                }
            }
            ActionId::Mixer(MixerActionId::LevelDownBig) => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, -0.10))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(-0.10))
                }
            }
            ActionId::Mixer(MixerActionId::Mute) => Action::Mixer(MixerAction::ToggleMute),
            ActionId::Mixer(MixerActionId::Solo) => Action::Mixer(MixerAction::ToggleSolo),
            ActionId::Mixer(MixerActionId::Output) => Action::Mixer(MixerAction::CycleOutput),
            ActionId::Mixer(MixerActionId::OutputRev) => {
                Action::Mixer(MixerAction::CycleOutputReverse)
            }
            ActionId::Mixer(MixerActionId::Section) => {
                self.send_target = None;
                Action::Mixer(MixerAction::CycleSection)
            }
            ActionId::Mixer(MixerActionId::SendNext) => {
                self.send_target = match self.send_target {
                    None => Some(1),
                    Some(8) => None,
                    Some(n) => Some(n + 1),
                };
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendPrev) => {
                self.send_target = match self.send_target {
                    None => Some(8),
                    Some(1) => None,
                    Some(n) => Some(n - 1),
                };
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendToggle) => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::ToggleSend(bus_id))
                } else {
                    Action::None
                }
            }
            ActionId::Mixer(MixerActionId::ClearSend)
            | ActionId::Mixer(MixerActionId::Escape) => {
                self.send_target = None;
                Action::None
            }
            ActionId::Mixer(MixerActionId::EnterDetail) => {
                match state.session.mixer.selection {
                    MixerSelection::Instrument(idx) => {
                        if idx < state.instruments.instruments.len() {
                            self.detail_mode = Some(DetailTarget::Instrument(idx));
                            self.detail_section = MixerSection::Effects;
                            self.detail_cursor = 0;
                            self.effect_scroll = 0;
                        }
                    }
                    MixerSelection::LayerGroup(gid) => {
                        self.detail_mode = Some(DetailTarget::LayerGroup(gid));
                        self.group_detail_section = GroupDetailSection::Effects;
                        self.detail_cursor = 0;
                        self.effect_scroll = 0;
                    }
                    MixerSelection::Bus(id) => {
                        self.detail_mode = Some(DetailTarget::Bus(id));
                        self.bus_detail_section = BusDetailSection::Effects;
                        self.detail_cursor = 0;
                        self.effect_scroll = 0;
                    }
                    _ => {}
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub(super) fn handle_mouse_impl(
        &mut self,
        event: &MouseEvent,
        area: Rect,
        state: &AppState,
    ) -> Action {
        let active_groups = state.instruments.active_layer_groups();
        let num_group_slots = active_groups.len().min(NUM_VISIBLE_GROUPS);
        let group_section_width = if num_group_slots > 0 {
            num_group_slots as u16 * CHANNEL_WIDTH + 2
        } else {
            0
        };
        let box_width = (NUM_VISIBLE_CHANNELS as u16 * CHANNEL_WIDTH)
            + 2
            + group_section_width
            + (NUM_VISIBLE_BUSES as u16 * CHANNEL_WIDTH)
            + 2
            + CHANNEL_WIDTH
            + 4;
        let box_height = METER_HEIGHT + 8;
        let rect = center_rect(area, box_width, box_height);
        let base_x = rect.x + 2;

        let col = event.column;
        let row = event.row;

        // Check if click is within the mixer box
        if col < rect.x || col >= rect.x + rect.width || row < rect.y || row >= rect.y + rect.height
        {
            return Action::None;
        }

        // Calculate scroll offsets (same as render)
        let instrument_scroll = match state.session.mixer.selection {
            MixerSelection::Instrument(idx) => {
                Self::calc_scroll_offset(idx, state.instruments.instruments.len(), NUM_VISIBLE_CHANNELS)
            }
            _ => 0,
        };
        let bus_scroll = match state.session.mixer.selection {
            MixerSelection::Bus(id) => Self::calc_scroll_offset(
                (id - 1) as usize,
                state.session.mixer.buses.len(),
                NUM_VISIBLE_BUSES,
            ),
            _ => 0,
        };

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Instrument channels region
                let inst_end_x = base_x + (NUM_VISIBLE_CHANNELS as u16 * CHANNEL_WIDTH);
                if col >= base_x && col < inst_end_x {
                    let channel = ((col - base_x) / CHANNEL_WIDTH) as usize;
                    let idx = instrument_scroll + channel;
                    if idx < state.instruments.instruments.len() {
                        self.send_target = None;
                        return Action::Mixer(MixerAction::SelectAt(MixerSelection::Instrument(
                            idx,
                        )));
                    }
                }

                // Group channels region (after first separator)
                let mut next_x = inst_end_x + 2;
                if !active_groups.is_empty() {
                    let group_end_x = next_x + (num_group_slots as u16 * CHANNEL_WIDTH);
                    if col >= next_x && col < group_end_x {
                        let channel = ((col - next_x) / CHANNEL_WIDTH) as usize;
                        if channel < active_groups.len() {
                            let gid = active_groups[channel];
                            self.send_target = None;
                            return Action::Mixer(MixerAction::SelectAt(
                                MixerSelection::LayerGroup(gid),
                            ));
                        }
                    }
                    next_x = group_end_x + 2; // +2 for separator after groups
                }

                // Bus channels region
                let bus_end_x = next_x + (NUM_VISIBLE_BUSES as u16 * CHANNEL_WIDTH);
                if col >= next_x && col < bus_end_x {
                    let channel = ((col - next_x) / CHANNEL_WIDTH) as usize;
                    let bus_idx = bus_scroll + channel;
                    if bus_idx < state.session.mixer.buses.len() {
                        let bus_id = state.session.mixer.buses[bus_idx].id;
                        self.send_target = None;
                        return Action::Mixer(MixerAction::SelectAt(MixerSelection::Bus(bus_id)));
                    }
                }

                // Master region (after bus separator)
                let master_start_x = bus_end_x + 2;
                if col >= master_start_x {
                    self.send_target = None;
                    return Action::Mixer(MixerAction::SelectAt(MixerSelection::Master));
                }

                Action::None
            }
            MouseEventKind::ScrollUp => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, 0.05))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(0.05))
                }
            }
            MouseEventKind::ScrollDown => {
                if let Some(bus_id) = self.send_target {
                    Action::Mixer(MixerAction::AdjustSend(bus_id, -0.05))
                } else {
                    Action::Mixer(MixerAction::AdjustLevel(-0.05))
                }
            }
            _ => Action::None,
        }
    }

    fn handle_detail_action(&mut self, action: ActionId, state: &AppState) -> Action {
        let Some(inst_id) = self.detail_instrument_id(state) else {
            self.detail_mode = None;
            return Action::None;
        };

        match action {
            ActionId::Mixer(MixerActionId::Escape)
            | ActionId::Mixer(MixerActionId::ClearSend) => {
                self.detail_mode = None;
                self.send_target = None;
                Action::None
            }
            ActionId::Mixer(MixerActionId::Section) => {
                self.detail_section = self.detail_section.next();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::SectionPrev) => {
                self.detail_section = self.detail_section.prev();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelUp) | ActionId::Mixer(MixerActionId::Prev) => {
                if self.detail_cursor > 0 {
                    self.detail_cursor -= 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelDown) | ActionId::Mixer(MixerActionId::Next) => {
                let max = self.max_cursor(state);
                if self.detail_cursor < max {
                    self.detail_cursor += 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelUpBig)
            | ActionId::Mixer(MixerActionId::First) => {
                self.adjust_detail_param(state, inst_id, 5.0)
            }
            ActionId::Mixer(MixerActionId::LevelDownBig)
            | ActionId::Mixer(MixerActionId::Last) => {
                self.adjust_detail_param(state, inst_id, -5.0)
            }
            ActionId::Mixer(MixerActionId::Increase)
            | ActionId::Mixer(MixerActionId::FineRight) => {
                self.adjust_detail_param(state, inst_id, 1.0)
            }
            ActionId::Mixer(MixerActionId::Decrease)
            | ActionId::Mixer(MixerActionId::FineLeft) => {
                self.adjust_detail_param(state, inst_id, -1.0)
            }
            ActionId::Mixer(MixerActionId::Mute) => Action::Mixer(MixerAction::ToggleMute),
            ActionId::Mixer(MixerActionId::Solo) => Action::Mixer(MixerAction::ToggleSolo),
            ActionId::Mixer(MixerActionId::Output) => Action::Mixer(MixerAction::CycleOutput),
            ActionId::Mixer(MixerActionId::OutputRev) => {
                Action::Mixer(MixerAction::CycleOutputReverse)
            }
            ActionId::Mixer(MixerActionId::AddEffect) => {
                Action::Nav(NavAction::PushPane("add_effect"))
            }
            ActionId::Mixer(MixerActionId::RemoveEffect) => {
                if self.detail_section == MixerSection::Effects {
                    if let Some((ei, _)) = self.decode_effect_cursor(state) {
                        let max_after = self.max_cursor(state).saturating_sub(1);
                        if self.detail_cursor > max_after {
                            self.detail_cursor = max_after;
                        }
                        return Action::Instrument(InstrumentAction::RemoveEffect(inst_id, ei));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::ToggleEffect) => {
                if self.detail_section == MixerSection::Effects {
                    if let Some((ei, _)) = self.decode_effect_cursor(state) {
                        return Action::Instrument(InstrumentAction::ToggleEffectBypass(
                            inst_id, ei,
                        ));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::ToggleFilter) => {
                Action::Instrument(InstrumentAction::ToggleFilter(inst_id))
            }
            ActionId::Mixer(MixerActionId::CycleFilterType) => {
                Action::Instrument(InstrumentAction::CycleFilterType(inst_id))
            }
            ActionId::Mixer(MixerActionId::MoveUp) => {
                if self.detail_section == MixerSection::Effects {
                    if let Some((ei, _)) = self.decode_effect_cursor(state) {
                        if ei > 0 {
                            return Action::Instrument(InstrumentAction::MoveEffect(
                                inst_id, ei, -1,
                            ));
                        }
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::MoveDown) => {
                if self.detail_section == MixerSection::Effects {
                    if let Some((ei, _)) = self.decode_effect_cursor(state) {
                        return Action::Instrument(InstrumentAction::MoveEffect(inst_id, ei, 1));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::PanLeft) => {
                Action::Mixer(MixerAction::AdjustPan(-0.05))
            }
            ActionId::Mixer(MixerActionId::PanRight) => {
                Action::Mixer(MixerAction::AdjustPan(0.05))
            }
            ActionId::Mixer(MixerActionId::EnterDetail) => {
                match self.detail_section {
                    MixerSection::Effects => Action::None,
                    _ => Action::None,
                }
            }
            ActionId::Mixer(MixerActionId::SendNext) => {
                if self.detail_section == MixerSection::Sends {
                    let max = self.max_cursor(state);
                    if self.detail_cursor < max {
                        self.detail_cursor += 1;
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendPrev) => {
                if self.detail_section == MixerSection::Sends {
                    if self.detail_cursor > 0 {
                        self.detail_cursor -= 1;
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendToggle) => {
                if self.detail_section == MixerSection::Sends {
                    if let Some((_, inst)) = self.detail_instrument(state) {
                        if let Some(send) = inst.sends.get(self.detail_cursor) {
                            return Action::Mixer(MixerAction::ToggleSend(send.bus_id));
                        }
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_bus_detail_action(&mut self, action: ActionId, state: &AppState) -> Action {
        let Some(bus_id) = self.detail_bus_id() else {
            self.detail_mode = None;
            return Action::None;
        };

        match action {
            ActionId::Mixer(MixerActionId::Escape)
            | ActionId::Mixer(MixerActionId::ClearSend) => {
                self.detail_mode = None;
                self.send_target = None;
                Action::None
            }
            ActionId::Mixer(MixerActionId::Section) => {
                self.bus_detail_section = self.bus_detail_section.next();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::SectionPrev) => {
                self.bus_detail_section = self.bus_detail_section.prev();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelUp) | ActionId::Mixer(MixerActionId::Prev) => {
                if self.detail_cursor > 0 {
                    self.detail_cursor -= 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelDown) | ActionId::Mixer(MixerActionId::Next) => {
                let max = self.bus_max_cursor(state);
                if self.detail_cursor < max {
                    self.detail_cursor += 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::Increase)
            | ActionId::Mixer(MixerActionId::FineRight)
            | ActionId::Mixer(MixerActionId::LevelUpBig)
            | ActionId::Mixer(MixerActionId::First) => {
                let delta = if matches!(
                    action,
                    ActionId::Mixer(MixerActionId::LevelUpBig)
                        | ActionId::Mixer(MixerActionId::First)
                ) {
                    5.0
                } else {
                    1.0
                };
                self.adjust_bus_detail_param(state, bus_id, delta)
            }
            ActionId::Mixer(MixerActionId::Decrease)
            | ActionId::Mixer(MixerActionId::FineLeft)
            | ActionId::Mixer(MixerActionId::LevelDownBig)
            | ActionId::Mixer(MixerActionId::Last) => {
                let delta = if matches!(
                    action,
                    ActionId::Mixer(MixerActionId::LevelDownBig)
                        | ActionId::Mixer(MixerActionId::Last)
                ) {
                    -5.0
                } else {
                    -1.0
                };
                self.adjust_bus_detail_param(state, bus_id, delta)
            }
            ActionId::Mixer(MixerActionId::AddEffect) => {
                Action::Nav(NavAction::PushPane("add_effect"))
            }
            ActionId::Mixer(MixerActionId::RemoveEffect) => {
                if self.bus_detail_section == BusDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_bus_effect_cursor(state) {
                        let max_after = self.bus_max_cursor(state).saturating_sub(1);
                        if self.detail_cursor > max_after {
                            self.detail_cursor = max_after;
                        }
                        return Action::Bus(BusAction::RemoveEffect(bus_id, ei));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::ToggleEffect) => {
                if self.bus_detail_section == BusDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_bus_effect_cursor(state) {
                        return Action::Bus(BusAction::ToggleEffectBypass(bus_id, ei));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::MoveUp) => {
                if self.bus_detail_section == BusDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_bus_effect_cursor(state) {
                        if ei > 0 {
                            return Action::Bus(BusAction::MoveEffect(bus_id, ei, -1));
                        }
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::MoveDown) => {
                if self.bus_detail_section == BusDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_bus_effect_cursor(state) {
                        return Action::Bus(BusAction::MoveEffect(bus_id, ei, 1));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::Mute) => Action::Mixer(MixerAction::ToggleMute),
            ActionId::Mixer(MixerActionId::Solo) => Action::Mixer(MixerAction::ToggleSolo),
            ActionId::Mixer(MixerActionId::PanLeft) => {
                Action::Mixer(MixerAction::AdjustPan(-0.05))
            }
            ActionId::Mixer(MixerActionId::PanRight) => {
                Action::Mixer(MixerAction::AdjustPan(0.05))
            }
            _ => Action::None,
        }
    }

    fn handle_group_detail_action(&mut self, action: ActionId, state: &AppState) -> Action {
        let Some(gid) = self.detail_group_id() else {
            self.detail_mode = None;
            return Action::None;
        };

        match action {
            ActionId::Mixer(MixerActionId::Escape)
            | ActionId::Mixer(MixerActionId::ClearSend) => {
                self.detail_mode = None;
                self.send_target = None;
                Action::None
            }
            ActionId::Mixer(MixerActionId::Section) => {
                self.group_detail_section = self.group_detail_section.next();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::SectionPrev) => {
                self.group_detail_section = self.group_detail_section.prev();
                self.detail_cursor = 0;
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelUp) | ActionId::Mixer(MixerActionId::Prev) => {
                if self.detail_cursor > 0 {
                    self.detail_cursor -= 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::LevelDown) | ActionId::Mixer(MixerActionId::Next) => {
                let max = self.group_max_cursor(state);
                if self.detail_cursor < max {
                    self.detail_cursor += 1;
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::Increase)
            | ActionId::Mixer(MixerActionId::FineRight)
            | ActionId::Mixer(MixerActionId::LevelUpBig)
            | ActionId::Mixer(MixerActionId::First) => {
                let delta = if matches!(
                    action,
                    ActionId::Mixer(MixerActionId::LevelUpBig)
                        | ActionId::Mixer(MixerActionId::First)
                ) {
                    5.0
                } else {
                    1.0
                };
                self.adjust_group_detail_param(state, gid, delta)
            }
            ActionId::Mixer(MixerActionId::Decrease)
            | ActionId::Mixer(MixerActionId::FineLeft)
            | ActionId::Mixer(MixerActionId::LevelDownBig)
            | ActionId::Mixer(MixerActionId::Last) => {
                let delta = if matches!(
                    action,
                    ActionId::Mixer(MixerActionId::LevelDownBig)
                        | ActionId::Mixer(MixerActionId::Last)
                ) {
                    -5.0
                } else {
                    -1.0
                };
                self.adjust_group_detail_param(state, gid, delta)
            }
            ActionId::Mixer(MixerActionId::AddEffect) => {
                Action::Nav(NavAction::PushPane("add_effect"))
            }
            ActionId::Mixer(MixerActionId::RemoveEffect) => {
                if self.group_detail_section == GroupDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_group_effect_cursor(state) {
                        let max_after = self.group_max_cursor(state).saturating_sub(1);
                        if self.detail_cursor > max_after {
                            self.detail_cursor = max_after;
                        }
                        return Action::LayerGroup(LayerGroupAction::RemoveEffect(gid, ei));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::ToggleEffect) => {
                if self.group_detail_section == GroupDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_group_effect_cursor(state) {
                        return Action::LayerGroup(LayerGroupAction::ToggleEffectBypass(gid, ei));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::MoveUp) => {
                if self.group_detail_section == GroupDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_group_effect_cursor(state) {
                        if ei > 0 {
                            return Action::LayerGroup(LayerGroupAction::MoveEffect(gid, ei, -1));
                        }
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::MoveDown) => {
                if self.group_detail_section == GroupDetailSection::Effects {
                    if let Some((ei, _)) = self.decode_group_effect_cursor(state) {
                        return Action::LayerGroup(LayerGroupAction::MoveEffect(gid, ei, 1));
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::Mute) => Action::Mixer(MixerAction::ToggleMute),
            ActionId::Mixer(MixerActionId::Solo) => Action::Mixer(MixerAction::ToggleSolo),
            ActionId::Mixer(MixerActionId::PanLeft) => {
                Action::Mixer(MixerAction::AdjustPan(-0.05))
            }
            ActionId::Mixer(MixerActionId::PanRight) => {
                Action::Mixer(MixerAction::AdjustPan(0.05))
            }
            ActionId::Mixer(MixerActionId::Output) => Action::Mixer(MixerAction::CycleOutput),
            ActionId::Mixer(MixerActionId::OutputRev) => {
                Action::Mixer(MixerAction::CycleOutputReverse)
            }
            ActionId::Mixer(MixerActionId::SendNext) => {
                if self.group_detail_section == GroupDetailSection::Sends {
                    let max = self.group_max_cursor(state);
                    if self.detail_cursor < max {
                        self.detail_cursor += 1;
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendPrev) => {
                if self.group_detail_section == GroupDetailSection::Sends {
                    if self.detail_cursor > 0 {
                        self.detail_cursor -= 1;
                    }
                }
                Action::None
            }
            ActionId::Mixer(MixerActionId::SendToggle) => {
                if self.group_detail_section == GroupDetailSection::Sends {
                    if let Some(gm) = state.session.mixer.layer_group_mixer(gid) {
                        if let Some(send) = gm.sends.get(self.detail_cursor) {
                            return Action::Mixer(MixerAction::ToggleSend(send.bus_id));
                        }
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn adjust_detail_param(
        &self,
        state: &AppState,
        inst_id: InstrumentId,
        delta: f32,
    ) -> Action {
        match self.detail_section {
            MixerSection::Effects => {
                if let Some((ei, Some(pi))) = self.decode_effect_cursor(state) {
                    return Action::Instrument(InstrumentAction::AdjustEffectParam(
                        inst_id, ei, pi, delta,
                    ));
                }
                Action::None
            }
            MixerSection::Sends => {
                if let Some((_, inst)) = self.detail_instrument(state) {
                    if let Some(send) = inst.sends.get(self.detail_cursor) {
                        return Action::Mixer(MixerAction::AdjustSend(send.bus_id, delta * 0.01));
                    }
                }
                Action::None
            }
            MixerSection::Filter => match self.detail_cursor {
                0 => Action::Instrument(InstrumentAction::CycleFilterType(inst_id)),
                1 => Action::Instrument(InstrumentAction::AdjustFilterCutoff(inst_id, delta)),
                2 => Action::Instrument(InstrumentAction::AdjustFilterResonance(inst_id, delta)),
                _ => Action::None,
            },
            MixerSection::Lfo => Action::None,
            MixerSection::Output => match self.detail_cursor {
                0 => Action::Mixer(MixerAction::AdjustPan(delta * 0.01)),
                1 => Action::Mixer(MixerAction::AdjustLevel(delta * 0.01)),
                2 => {
                    if delta > 0.0 {
                        Action::Mixer(MixerAction::CycleOutput)
                    } else {
                        Action::Mixer(MixerAction::CycleOutputReverse)
                    }
                }
                _ => Action::None,
            },
        }
    }

    fn adjust_bus_detail_param(&self, state: &AppState, bus_id: u8, delta: f32) -> Action {
        match self.bus_detail_section {
            BusDetailSection::Effects => {
                if let Some((ei, Some(pi))) = self.decode_bus_effect_cursor(state) {
                    return Action::Bus(BusAction::AdjustEffectParam(bus_id, ei, pi, delta));
                }
                Action::None
            }
            BusDetailSection::Output => match self.detail_cursor {
                0 => Action::Mixer(MixerAction::AdjustPan(delta * 0.01)),
                1 => Action::Mixer(MixerAction::AdjustLevel(delta * 0.01)),
                _ => Action::None,
            },
        }
    }

    fn adjust_group_detail_param(&self, state: &AppState, gid: u32, delta: f32) -> Action {
        match self.group_detail_section {
            GroupDetailSection::Effects => {
                if let Some((ei, Some(pi))) = self.decode_group_effect_cursor(state) {
                    return Action::LayerGroup(LayerGroupAction::AdjustEffectParam(
                        gid, ei, pi, delta,
                    ));
                }
                Action::None
            }
            GroupDetailSection::Sends => {
                if let Some(gm) = state.session.mixer.layer_group_mixer(gid) {
                    if let Some(send) = gm.sends.get(self.detail_cursor) {
                        return Action::Mixer(MixerAction::AdjustSend(send.bus_id, delta * 0.01));
                    }
                }
                Action::None
            }
            GroupDetailSection::Output => match self.detail_cursor {
                0 => Action::Mixer(MixerAction::AdjustPan(delta * 0.01)),
                1 => Action::Mixer(MixerAction::AdjustLevel(delta * 0.01)),
                _ => Action::None,
            },
        }
    }
}
