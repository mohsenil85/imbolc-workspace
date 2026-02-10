mod input;
mod rendering;

use std::any::Any;

use crate::panes::add_effect_pane::EffectTarget;
use crate::state::{AppState, InstrumentId};
use crate::ui::action_id::ActionId;
use crate::ui::{Action, InputEvent, Keymap, MouseEvent, Pane, Rect, RenderBuf};
use imbolc_types::BusId;

const CHANNEL_WIDTH: u16 = 8;
const METER_HEIGHT: u16 = 12;
const NUM_VISIBLE_CHANNELS: usize = 8;
const NUM_VISIBLE_GROUPS: usize = 2;
const NUM_VISIBLE_BUSES: usize = 2;

/// Block characters for vertical meter
const BLOCK_CHARS: [char; 8] = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailTarget {
    Instrument(usize),
    LayerGroup(u32),
    Bus(BusId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MixerSection {
    Effects,
    Sends,
    Filter,
    Lfo,
    Output,
}

impl MixerSection {
    fn next(self) -> Self {
        match self {
            Self::Effects => Self::Sends,
            Self::Sends => Self::Filter,
            Self::Filter => Self::Lfo,
            Self::Lfo => Self::Output,
            Self::Output => Self::Effects,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Effects => Self::Output,
            Self::Sends => Self::Effects,
            Self::Filter => Self::Sends,
            Self::Lfo => Self::Filter,
            Self::Output => Self::Lfo,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Effects => "EFFECTS",
            Self::Sends => "SENDS",
            Self::Filter => "FILTER",
            Self::Lfo => "LFO",
            Self::Output => "OUTPUT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusDetailSection {
    Effects,
    Output,
}

impl BusDetailSection {
    fn next(self) -> Self {
        match self {
            Self::Effects => Self::Output,
            Self::Output => Self::Effects,
        }
    }

    fn prev(self) -> Self {
        self.next()
    }

    fn label(self) -> &'static str {
        match self {
            Self::Effects => "EFFECTS",
            Self::Output => "OUTPUT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupDetailSection {
    Effects,
    Sends,
    Output,
}

impl GroupDetailSection {
    fn next(self) -> Self {
        match self {
            Self::Effects => Self::Sends,
            Self::Sends => Self::Output,
            Self::Output => Self::Effects,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Effects => Self::Output,
            Self::Sends => Self::Effects,
            Self::Output => Self::Sends,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Effects => "EFFECTS",
            Self::Sends => "SENDS",
            Self::Output => "OUTPUT",
        }
    }
}

pub struct MixerPane {
    keymap: Keymap,
    send_target: Option<BusId>,
    detail_mode: Option<DetailTarget>,
    detail_section: MixerSection,
    detail_cursor: usize,
    effect_scroll: usize,
    bus_detail_section: BusDetailSection,
    group_detail_section: GroupDetailSection,
}

impl MixerPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            send_target: None,
            detail_mode: None,
            detail_section: MixerSection::Effects,
            detail_cursor: 0,
            effect_scroll: 0,
            bus_detail_section: BusDetailSection::Effects,
            group_detail_section: GroupDetailSection::Effects,
        }
    }

    #[allow(dead_code)]
    pub fn send_target(&self) -> Option<BusId> {
        self.send_target
    }

    /// Get the instrument index and ID for the current detail mode target (instrument only)
    fn detail_instrument<'a>(&self, state: &'a AppState) -> Option<(usize, &'a crate::state::Instrument)> {
        match self.detail_mode? {
            DetailTarget::Instrument(idx) => {
                state.instruments.instruments.get(idx).map(|inst| (idx, inst))
            }
            DetailTarget::LayerGroup(_) | DetailTarget::Bus(_) => None,
        }
    }

    fn detail_instrument_id(&self, state: &AppState) -> Option<InstrumentId> {
        self.detail_instrument(state).map(|(_, inst)| inst.id)
    }

    /// Get the layer group ID if in group detail mode
    fn detail_group_id(&self) -> Option<u32> {
        match self.detail_mode? {
            DetailTarget::LayerGroup(gid) => Some(gid),
            _ => None,
        }
    }

    fn detail_bus_id(&self) -> Option<BusId> {
        match self.detail_mode? {
            DetailTarget::Bus(id) => Some(id),
            _ => None,
        }
    }

    /// Max cursor position for current section
    fn max_cursor(&self, state: &AppState) -> usize {
        let Some((_, inst)) = self.detail_instrument(state) else { return 0 };
        match self.detail_section {
            MixerSection::Effects => {
                if inst.effects().next().is_none() { 0 }
                else {
                    let mut count: usize = 0;
                    for effect in inst.effects() {
                        count += 1 + effect.params.len();
                    }
                    count.saturating_sub(1)
                }
            }
            MixerSection::Sends => inst.sends.len().saturating_sub(1),
            MixerSection::Filter => {
                if inst.filter().is_some() { 2 } else { 0 }
            }
            MixerSection::Lfo => 2,
            MixerSection::Output => 2,
        }
    }

    /// Decode effect cursor into (effect_id, param_index_within_effect) where None = header
    fn decode_effect_cursor(&self, state: &AppState) -> Option<(crate::state::EffectId, Option<imbolc_types::ParamIndex>)> {
        let (_, inst) = self.detail_instrument(state)?;
        inst.decode_effect_cursor(self.detail_cursor)
    }

    /// Max cursor for bus detail section
    fn bus_max_cursor(&self, state: &AppState) -> usize {
        let Some(bus_id) = self.detail_bus_id() else { return 0 };
        let Some(bus) = state.session.bus(bus_id) else { return 0 };
        match self.bus_detail_section {
            BusDetailSection::Effects => crate::state::effects_max_cursor(&bus.effects),
            BusDetailSection::Output => 1, // pan, level
        }
    }

    /// Max cursor for group detail section
    fn group_max_cursor(&self, state: &AppState) -> usize {
        let Some(gid) = self.detail_group_id() else { return 0 };
        let Some(gm) = state.session.mixer.layer_group_mixer(gid) else { return 0 };
        match self.group_detail_section {
            GroupDetailSection::Effects => crate::state::effects_max_cursor(&gm.effects),
            GroupDetailSection::Sends => gm.sends.len().saturating_sub(1),
            GroupDetailSection::Output => 1, // pan, level
        }
    }

    /// Decode bus effect cursor into (effect_id, param_index) where None = header
    fn decode_bus_effect_cursor(&self, state: &AppState) -> Option<(crate::state::EffectId, Option<imbolc_types::ParamIndex>)> {
        let bus_id = self.detail_bus_id()?;
        let bus = state.session.bus(bus_id)?;
        crate::state::decode_effect_cursor_from_slice(&bus.effects, self.detail_cursor)
    }

    /// Decode group effect cursor into (effect_id, param_index) where None = header
    fn decode_group_effect_cursor(&self, state: &AppState) -> Option<(crate::state::EffectId, Option<imbolc_types::ParamIndex>)> {
        let gid = self.detail_group_id()?;
        let gm = state.session.mixer.layer_group_mixer(gid)?;
        crate::state::decode_effect_cursor_from_slice(&gm.effects, self.detail_cursor)
    }

    /// Get the current effect target based on detail mode (for add_effect pane bridging)
    pub fn effect_target(&self) -> EffectTarget {
        match self.detail_mode {
            Some(DetailTarget::Bus(id)) => EffectTarget::Bus(id),
            Some(DetailTarget::LayerGroup(gid)) => EffectTarget::LayerGroup(gid),
            _ => EffectTarget::Instrument,
        }
    }

    fn calc_scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
        if selected >= visible {
            (selected - visible + 1).min(total.saturating_sub(visible))
        } else {
            0
        }
    }
}

impl Default for MixerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for MixerPane {
    fn id(&self) -> &'static str {
        "mixer"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.handle_action_impl(action, event, state)
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action {
        self.handle_mouse_impl(event, area, state)
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        match self.detail_mode {
            Some(DetailTarget::Instrument(_)) => self.render_detail_buf(buf, area, state),
            Some(DetailTarget::LayerGroup(gid)) => {
                self.render_group_detail_buf(buf, area, state, gid)
            }
            Some(DetailTarget::Bus(id)) => self.render_bus_detail_buf(buf, area, state, id),
            None => self.render_mixer_buf(buf, area, state),
        }
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
    use crate::state::{AppState, MixerSelection};
    use crate::ui::{BusAction, InputEvent, KeyCode, LayerGroupAction, MixerAction, Modifiers, NavAction};
    use crate::ui::action_id::MixerActionId;
    use imbolc_types::BusId;

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn send_target_cycles_and_adjusts_send() {
        let mut pane = MixerPane::new(Keymap::new());
        let state = AppState::new();

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::SendNext), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.send_target, Some(BusId::new(1)));

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::LevelUp), &dummy_event(), &state);
        match action {
            Action::Mixer(MixerAction::AdjustSend(bus_id, delta)) => {
                assert_eq!(bus_id, BusId::new(1));
                assert!((delta - 0.05).abs() < 0.0001);
            }
            _ => panic!("Expected AdjustSend when send_target is set"),
        }

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::ClearSend), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.send_target, None);
    }

    #[test]
    fn prev_next_clear_send_target() {
        let mut pane = MixerPane::new(Keymap::new());
        let state = AppState::new();

        pane.send_target = Some(BusId::new(3));
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::Prev), &dummy_event(), &state);
        assert!(matches!(action, Action::Mixer(MixerAction::Move(-1))));
        assert_eq!(pane.send_target, None);

        pane.send_target = Some(BusId::new(2));
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::Next), &dummy_event(), &state);
        assert!(matches!(action, Action::Mixer(MixerAction::Move(1))));
        assert_eq!(pane.send_target, None);
    }

    #[test]
    fn bus_detail_entry_and_escape() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));

        // Enter bus detail
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.detail_mode, Some(DetailTarget::Bus(BusId::new(1))));
        assert_eq!(pane.bus_detail_section, BusDetailSection::Effects);
        assert_eq!(pane.detail_cursor, 0);

        // Escape back to overview
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::Escape), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.detail_mode, None);
    }

    #[test]
    fn group_detail_entry_starts_on_effects() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state.session.mixer.selection = MixerSelection::LayerGroup(1);

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.detail_mode, Some(DetailTarget::LayerGroup(1)));
        assert_eq!(pane.group_detail_section, GroupDetailSection::Effects);
    }

    #[test]
    fn bus_detail_section_cycling() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));

        // Enter bus detail
        pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);
        assert_eq!(pane.bus_detail_section, BusDetailSection::Effects);

        // Cycle to Output
        pane.handle_action(ActionId::Mixer(MixerActionId::Section), &dummy_event(), &state);
        assert_eq!(pane.bus_detail_section, BusDetailSection::Output);
        assert_eq!(pane.detail_cursor, 0);

        // Cycle back to Effects
        pane.handle_action(ActionId::Mixer(MixerActionId::Section), &dummy_event(), &state);
        assert_eq!(pane.bus_detail_section, BusDetailSection::Effects);
    }

    #[test]
    fn bus_detail_add_effect_returns_push_pane() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));

        pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::AddEffect), &dummy_event(), &state);
        assert!(matches!(action, Action::Nav(NavAction::PushPane("add_effect"))));
    }

    #[test]
    fn bus_detail_remove_effect() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.bus_mut(BusId::new(1)).unwrap().add_effect(crate::state::EffectType::Reverb);
        let effect_id = state.session.bus(BusId::new(1)).unwrap().effects[0].id;

        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
        pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::RemoveEffect), &dummy_event(), &state);
        assert!(matches!(action, Action::Bus(BusAction::RemoveEffect(id, eid)) if id == BusId::new(1) && eid == effect_id));
    }

    #[test]
    fn group_detail_section_cycling() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state.session.mixer.selection = MixerSelection::LayerGroup(1);

        pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);
        assert_eq!(pane.group_detail_section, GroupDetailSection::Effects);

        pane.handle_action(ActionId::Mixer(MixerActionId::Section), &dummy_event(), &state);
        assert_eq!(pane.group_detail_section, GroupDetailSection::Sends);

        pane.handle_action(ActionId::Mixer(MixerActionId::Section), &dummy_event(), &state);
        assert_eq!(pane.group_detail_section, GroupDetailSection::Output);

        pane.handle_action(ActionId::Mixer(MixerActionId::Section), &dummy_event(), &state);
        assert_eq!(pane.group_detail_section, GroupDetailSection::Effects);
    }

    #[test]
    fn group_detail_remove_effect() {
        let mut pane = MixerPane::new(Keymap::new());
        let mut state = AppState::new();
        state.session.mixer.add_layer_group_mixer(1, &[]);
        state.session.mixer.layer_group_mixer_mut(1).unwrap().add_effect(crate::state::EffectType::Delay);
        let effect_id = state.session.mixer.layer_group_mixer(1).unwrap().effects[0].id;

        state.session.mixer.selection = MixerSelection::LayerGroup(1);
        pane.handle_action(ActionId::Mixer(MixerActionId::EnterDetail), &dummy_event(), &state);

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::RemoveEffect), &dummy_event(), &state);
        assert!(matches!(action, Action::LayerGroup(LayerGroupAction::RemoveEffect(1, id)) if id == effect_id));
    }

    #[test]
    fn effect_target_reflects_detail_mode() {
        let mut pane = MixerPane::new(Keymap::new());
        assert_eq!(pane.effect_target(), EffectTarget::Instrument);

        pane.detail_mode = Some(DetailTarget::Bus(BusId::new(2)));
        assert_eq!(pane.effect_target(), EffectTarget::Bus(BusId::new(2)));

        pane.detail_mode = Some(DetailTarget::LayerGroup(5));
        assert_eq!(pane.effect_target(), EffectTarget::LayerGroup(5));
    }
}
