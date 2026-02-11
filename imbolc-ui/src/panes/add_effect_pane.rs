use std::any::Any;

use crate::state::{AppState, EffectType, EffectTypeExt, VstPluginRegistry};
use crate::ui::action_id::{ActionId, AddActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Rect, RenderBuf, Action, Color, FileSelectAction, InputEvent, InstrumentAction, Keymap, MouseEvent,
    MouseEventKind, MouseButton, NavAction, Pane, SessionAction, Style,
};
use crate::action::{BusAction, LayerGroupAction};
use imbolc_types::{BusId, VstPluginId};

/// Target for the add-effect modal: which entity receives the new effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTarget {
    Instrument,
    Bus(BusId),
    LayerGroup(u32),
}

/// Options available in the Add Effect menu
#[derive(Debug, Clone)]
enum AddEffectOption {
    Effect(EffectType),
    Separator(&'static str),
    ImportVst,
}

const LIST_HEIGHT: usize = 14;

pub struct AddEffectPane {
    keymap: Keymap,
    selected: usize,
    scroll_offset: usize,
    cached_options: Vec<AddEffectOption>,
    effect_target: EffectTarget,
}

impl AddEffectPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected: 0,
            scroll_offset: 0,
            cached_options: Self::build_options_static(),
            effect_target: EffectTarget::Instrument,
        }
    }

    pub fn set_effect_target(&mut self, target: EffectTarget) {
        self.effect_target = target;
    }

    #[allow(dead_code)]
    pub fn effect_target(&self) -> EffectTarget {
        self.effect_target
    }

    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + LIST_HEIGHT {
            self.scroll_offset = self.selected.saturating_sub(LIST_HEIGHT - 1);
        }
    }

    fn build_options_static() -> Vec<AddEffectOption> {
        Self::build_effect_list(&[])
    }

    fn build_effect_list(vst_effects: &[(VstPluginId, EffectType)]) -> Vec<AddEffectOption> {
        let mut options = vec![
            AddEffectOption::Separator("── Dynamics ──"),
            AddEffectOption::Effect(EffectType::TapeComp),
            AddEffectOption::Effect(EffectType::SidechainComp),
            AddEffectOption::Effect(EffectType::Gate),
            AddEffectOption::Effect(EffectType::Limiter),
            AddEffectOption::Effect(EffectType::MultibandComp),
            AddEffectOption::Separator("── Modulation ──"),
            AddEffectOption::Effect(EffectType::Chorus),
            AddEffectOption::Effect(EffectType::Flanger),
            AddEffectOption::Effect(EffectType::Phaser),
            AddEffectOption::Effect(EffectType::Tremolo),
            AddEffectOption::Effect(EffectType::RingMod),
            AddEffectOption::Effect(EffectType::Resonator),
            AddEffectOption::Effect(EffectType::Leslie),
            AddEffectOption::Separator("── Distortion ──"),
            AddEffectOption::Effect(EffectType::Distortion),
            AddEffectOption::Effect(EffectType::Bitcrusher),
            AddEffectOption::Effect(EffectType::Wavefolder),
            AddEffectOption::Effect(EffectType::Saturator),
            AddEffectOption::Separator("── EQ ──"),
            AddEffectOption::Effect(EffectType::TiltEq),
            AddEffectOption::Effect(EffectType::ParaEq),
            AddEffectOption::Separator("── Stereo ──"),
            AddEffectOption::Effect(EffectType::StereoWidener),
            AddEffectOption::Effect(EffectType::FreqShifter),
            AddEffectOption::Effect(EffectType::Autopan),
            AddEffectOption::Effect(EffectType::MidSide),
            AddEffectOption::Separator("── Delay / Reverb ──"),
            AddEffectOption::Effect(EffectType::Delay),
            AddEffectOption::Effect(EffectType::Reverb),
            AddEffectOption::Effect(EffectType::ConvolutionReverb),
            AddEffectOption::Effect(EffectType::SpringReverb),
            AddEffectOption::Separator("── Utility ──"),
            AddEffectOption::Effect(EffectType::PitchShifter),
            AddEffectOption::Effect(EffectType::EnvFollower),
            AddEffectOption::Effect(EffectType::Denoise),
            AddEffectOption::Effect(EffectType::Crossfader),
            AddEffectOption::Separator("── Lo-fi ──"),
            AddEffectOption::Effect(EffectType::Vinyl),
            AddEffectOption::Effect(EffectType::Cabinet),
            AddEffectOption::Effect(EffectType::Glitch),
            AddEffectOption::Separator("── Granular ──"),
            AddEffectOption::Effect(EffectType::GranularDelay),
            AddEffectOption::Effect(EffectType::GranularFreeze),
            AddEffectOption::Effect(EffectType::SpectralFreeze),
            AddEffectOption::Separator("── Sidechain ──"),
            AddEffectOption::Effect(EffectType::Vocoder),
        ];

        options.push(AddEffectOption::Separator("── VST ──"));

        for &(_, effect_type) in vst_effects {
            options.push(AddEffectOption::Effect(effect_type));
        }

        options.push(AddEffectOption::ImportVst);

        options
    }

    fn build_options(&self, vst_registry: &VstPluginRegistry) -> Vec<AddEffectOption> {
        let vst_effects: Vec<(VstPluginId, EffectType)> = vst_registry
            .effects()
            .map(|p| (p.id, EffectType::Vst(p.id)))
            .collect();
        Self::build_effect_list(&vst_effects)
    }

    fn update_options(&mut self, vst_registry: &VstPluginRegistry) {
        self.cached_options = self.build_options(vst_registry);
        if self.selected >= self.cached_options.len() {
            self.selected = self.cached_options.len().saturating_sub(1);
        }
        // Ensure selection is not on a separator
        if matches!(self.cached_options.get(self.selected), Some(AddEffectOption::Separator(_))) {
            self.select_next();
        }
        self.adjust_scroll();
    }

    fn select_next(&mut self) {
        let len = self.cached_options.len();
        if len == 0 {
            return;
        }
        let mut next = (self.selected + 1) % len;
        while matches!(self.cached_options.get(next), Some(AddEffectOption::Separator(_))) {
            next = (next + 1) % len;
        }
        self.selected = next;
        self.adjust_scroll();
    }

    /// Convert the given option to an Action based on current effect target
    fn option_to_action(&self, option: &AddEffectOption, state: &AppState) -> Action {
        match option {
            AddEffectOption::Effect(effect_type) => match self.effect_target {
                EffectTarget::Bus(bus_id) => {
                    Action::Bus(BusAction::AddEffect(bus_id, *effect_type))
                }
                EffectTarget::LayerGroup(group_id) => {
                    Action::LayerGroup(LayerGroupAction::AddEffect(group_id, *effect_type))
                }
                EffectTarget::Instrument => {
                    if let Some(inst) = state.instruments.selected_instrument() {
                        Action::Instrument(InstrumentAction::AddEffect(inst.id, *effect_type))
                    } else {
                        Action::None
                    }
                }
            },
            AddEffectOption::ImportVst => {
                Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::ImportVstEffect))
            }
            AddEffectOption::Separator(_) => Action::None,
        }
    }

    fn select_prev(&mut self) {
        let len = self.cached_options.len();
        if len == 0 {
            return;
        }
        let mut prev = if self.selected == 0 { len - 1 } else { self.selected - 1 };
        while matches!(self.cached_options.get(prev), Some(AddEffectOption::Separator(_))) {
            prev = if prev == 0 { len - 1 } else { prev - 1 };
        }
        self.selected = prev;
        self.adjust_scroll();
    }
}

impl Default for AddEffectPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for AddEffectPane {
    fn id(&self) -> &'static str {
        "add_effect"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::Add(AddActionId::Confirm) => {
                if let Some(option) = self.cached_options.get(self.selected) {
                    self.option_to_action(option, state)
                } else {
                    Action::None
                }
            }
            ActionId::Add(AddActionId::Cancel) => Action::Nav(NavAction::PopPane),
            ActionId::Add(AddActionId::Next) => {
                self.select_next();
                Action::None
            }
            ActionId::Add(AddActionId::Prev) => {
                self.select_prev();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action {
        let rect = center_rect(area, 40, 20);
        let inner_y = rect.y + 2;
        let content_y = inner_y + 1;
        let list_y = content_y + 2;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = event.row;
                if row >= list_y && row < list_y + LIST_HEIGHT as u16 {
                    let visual_idx = (row - list_y) as usize;
                    let idx = visual_idx + self.scroll_offset;
                    if idx < self.cached_options.len() {
                        if matches!(self.cached_options.get(idx), Some(AddEffectOption::Separator(_))) {
                            return Action::None;
                        }
                        self.selected = idx;
                        self.adjust_scroll();
                        return self.option_to_action(&self.cached_options[idx].clone(), state);
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                self.select_prev();
                Action::None
            }
            MouseEventKind::ScrollDown => {
                self.select_next();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let vst_registry = &state.session.vst_plugins;
        let rect = center_rect(area, 40, 20);

        let border_style = Style::new().fg(Color::FX_COLOR);
        let inner = buf.draw_block(rect, " Add Effect ", border_style, border_style);

        let content_x = inner.x + 1;
        let content_y = inner.y + 1;

        // Title
        buf.draw_line(
            Rect::new(content_x, content_y, inner.width.saturating_sub(2), 1),
            &[("Select effect type:", Style::new().fg(Color::FX_COLOR).bold())],
        );

        let list_y = content_y + 2;
        let sel_bg = Style::new().bg(Color::SELECTION_BG);

        for (visual_i, i) in (self.scroll_offset..self.cached_options.len()).enumerate() {
            if visual_i >= LIST_HEIGHT {
                break;
            }

            let option = &self.cached_options[i];
            let y = list_y + visual_i as u16;

            let is_selected = i == self.selected;

            match option {
                AddEffectOption::Separator(label) => {
                    buf.draw_line(
                        Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
                        &[(*label, Style::new().fg(Color::DARK_GRAY))],
                    );
                }
                AddEffectOption::Effect(effect_type) => {
                    if is_selected {
                        buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    }

                    let color = if effect_type.is_vst() { Color::VST_COLOR } else { Color::FX_COLOR };
                    let name = effect_type.display_name(vst_registry);

                    let name_style = if is_selected {
                        Style::new().fg(color).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(color)
                    };

                    buf.draw_line(
                        Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                        &[(&name, name_style)],
                    );

                    if is_selected {
                        let fill_start = content_x + 2 + name.len() as u16;
                        let fill_end = inner.x + inner.width;
                        for x in fill_start..fill_end {
                            buf.set_cell(x, y, ' ', sel_bg);
                        }
                    }
                }
                AddEffectOption::ImportVst => {
                    if is_selected {
                        buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    }

                    let text_style = if is_selected {
                        Style::new().fg(Color::VST_COLOR).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(Color::VST_COLOR)
                    };
                    let label = "+ Import VST Effect...";
                    buf.draw_line(
                        Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                        &[(label, text_style)],
                    );

                    if is_selected {
                        let fill_start = content_x + 2 + label.len() as u16;
                        let fill_end = inner.x + inner.width;
                        for x in fill_start..fill_end {
                            buf.set_cell(x, y, ' ', sel_bg);
                        }
                    }
                }
            }
        }

    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn on_enter(&mut self, state: &AppState) {
        self.update_options(&state.session.vst_plugins);
    }

    fn on_exit(&mut self, _state: &AppState) {
        self.effect_target = EffectTarget::Instrument;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
