mod input;
mod rendering;

use std::any::Any;

use crate::state::{AppState, InstrumentId};
use crate::ui::{Rect, RenderBuf, Action, InputEvent, Keymap, MouseEvent, Pane};
use crate::ui::action_id::ActionId;

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

pub struct MixerPane {
    keymap: Keymap,
    send_target: Option<u8>,
    detail_mode: Option<DetailTarget>,
    detail_section: MixerSection,
    detail_cursor: usize,
    effect_scroll: usize,
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
        }
    }

    #[allow(dead_code)]
    pub fn send_target(&self) -> Option<u8> {
        self.send_target
    }

    /// Get the instrument index and ID for the current detail mode target (instrument only)
    fn detail_instrument<'a>(&self, state: &'a AppState) -> Option<(usize, &'a crate::state::Instrument)> {
        match self.detail_mode? {
            DetailTarget::Instrument(idx) => {
                state.instruments.instruments.get(idx).map(|inst| (idx, inst))
            }
            DetailTarget::LayerGroup(_) => None,
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

    /// Max cursor position for current section
    fn max_cursor(&self, state: &AppState) -> usize {
        let Some((_, inst)) = self.detail_instrument(state) else { return 0 };
        match self.detail_section {
            MixerSection::Effects => {
                if inst.effects.is_empty() { 0 }
                else {
                    let mut count = 0;
                    for effect in &inst.effects {
                        count += 1 + effect.params.len();
                    }
                    count.saturating_sub(1)
                }
            }
            MixerSection::Sends => inst.sends.len().saturating_sub(1),
            MixerSection::Filter => {
                if inst.filter.is_some() { 2 } else { 0 }
            }
            MixerSection::Lfo => 2,
            MixerSection::Output => 2,
        }
    }

    /// Decode effect cursor into (effect_id, param_index_within_effect) where None = header
    fn decode_effect_cursor(&self, state: &AppState) -> Option<(crate::state::EffectId, Option<usize>)> {
        let (_, inst) = self.detail_instrument(state)?;
        inst.decode_effect_cursor(self.detail_cursor)
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
            Some(DetailTarget::LayerGroup(gid)) => self.render_group_detail_buf(buf, area, state, gid),
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
    use crate::state::AppState;
    use crate::ui::{InputEvent, KeyCode, MixerAction, Modifiers};
    use crate::ui::action_id::MixerActionId;

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn send_target_cycles_and_adjusts_send() {
        let mut pane = MixerPane::new(Keymap::new());
        let state = AppState::new();

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::SendNext), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
        assert_eq!(pane.send_target, Some(1));

        let action = pane.handle_action(ActionId::Mixer(MixerActionId::LevelUp), &dummy_event(), &state);
        match action {
            Action::Mixer(MixerAction::AdjustSend(bus_id, delta)) => {
                assert_eq!(bus_id, 1);
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

        pane.send_target = Some(3);
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::Prev), &dummy_event(), &state);
        assert!(matches!(action, Action::Mixer(MixerAction::Move(-1))));
        assert_eq!(pane.send_target, None);

        pane.send_target = Some(2);
        let action = pane.handle_action(ActionId::Mixer(MixerActionId::Next), &dummy_event(), &state);
        assert!(matches!(action, Action::Mixer(MixerAction::Move(1))));
        assert_eq!(pane.send_target, None);
    }
}
