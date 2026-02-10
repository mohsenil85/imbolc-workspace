use std::any::Any;

use crate::state::{AppState, CustomSynthDefRegistry, SourceType, SourceTypeExt, VstPluginRegistry};
use crate::ui::action_id::{ActionId, AddActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, FileSelectAction, InputEvent, InstrumentAction, Keymap, ListSelector,
    MouseButton, MouseEvent, MouseEventKind, NavAction, Pane, Rect, RenderBuf, SessionAction,
    Style,
};

/// Options available in the Add Instrument menu
#[derive(Debug, Clone)]
pub enum AddOption {
    Source(SourceType),
    Separator(&'static str),
    ImportCustom,
    ImportVst,
}

pub struct AddPane {
    keymap: Keymap,
    selector: ListSelector,
    /// Cached options list - rebuilt on each render_with_registry call
    cached_options: Vec<AddOption>,
}

impl AddPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selector: ListSelector::new(1), // Start on first selectable item (skip separator)
            cached_options: Self::build_options_static(),
        }
    }


    /// Build options without registries (used for initial state)
    fn build_options_static() -> Vec<AddOption> {
        vec![
            // Basic Oscillators
            AddOption::Separator("── Oscillators ──"),
            AddOption::Source(SourceType::Saw),
            AddOption::Source(SourceType::Sin),
            AddOption::Source(SourceType::Sqr),
            AddOption::Source(SourceType::Tri),
            AddOption::Source(SourceType::Noise),
            AddOption::Source(SourceType::Pulse),
            AddOption::Source(SourceType::SuperSaw),
            AddOption::Source(SourceType::Sync),
            // Modulation / FM
            AddOption::Separator("── Modulation ──"),
            AddOption::Source(SourceType::Ring),
            AddOption::Source(SourceType::FBSin),
            AddOption::Source(SourceType::FM),
            AddOption::Source(SourceType::PhaseMod),
            AddOption::Source(SourceType::FMBell),
            AddOption::Source(SourceType::FMBrass),
            // Classic Synths
            AddOption::Separator("── Classic ──"),
            AddOption::Source(SourceType::Choir),
            AddOption::Source(SourceType::EPiano),
            AddOption::Source(SourceType::Organ),
            AddOption::Source(SourceType::BrassStab),
            AddOption::Source(SourceType::Strings),
            AddOption::Source(SourceType::Acid),
            // Physical Modeling
            AddOption::Separator("── Physical ──"),
            AddOption::Source(SourceType::Pluck),
            AddOption::Source(SourceType::Formant),
            AddOption::Source(SourceType::Bowed),
            AddOption::Source(SourceType::Blown),
            AddOption::Source(SourceType::Membrane),
            // Mallet Percussion
            AddOption::Separator("── Mallet ──"),
            AddOption::Source(SourceType::Marimba),
            AddOption::Source(SourceType::Vibes),
            AddOption::Source(SourceType::Kalimba),
            AddOption::Source(SourceType::SteelDrum),
            AddOption::Source(SourceType::TubularBell),
            AddOption::Source(SourceType::Glockenspiel),
            // Plucked Strings
            AddOption::Separator("── Plucked ──"),
            AddOption::Source(SourceType::Guitar),
            AddOption::Source(SourceType::BassGuitar),
            AddOption::Source(SourceType::Harp),
            AddOption::Source(SourceType::Koto),
            // Drums
            AddOption::Separator("── Drums ──"),
            AddOption::Source(SourceType::Kick),
            AddOption::Source(SourceType::Snare),
            AddOption::Source(SourceType::HihatClosed),
            AddOption::Source(SourceType::HihatOpen),
            AddOption::Source(SourceType::Clap),
            AddOption::Source(SourceType::Cowbell),
            AddOption::Source(SourceType::Rim),
            AddOption::Source(SourceType::Tom),
            AddOption::Source(SourceType::Clave),
            AddOption::Source(SourceType::Conga),
            // Experimental
            AddOption::Separator("── Experimental ──"),
            AddOption::Source(SourceType::Gendy),
            AddOption::Source(SourceType::Chaos),
            // Synthesis
            AddOption::Separator("── Synthesis ──"),
            AddOption::Source(SourceType::Additive),
            AddOption::Source(SourceType::Wavetable),
            AddOption::Source(SourceType::Granular),
            // Audio / Routing
            AddOption::Separator("── Routing ──"),
            AddOption::Source(SourceType::AudioIn),
            AddOption::Source(SourceType::BusIn),
            // Samplers
            AddOption::Separator("── Samplers ──"),
            AddOption::Source(SourceType::PitchedSampler),
            AddOption::Source(SourceType::TimeStretch),
            AddOption::Source(SourceType::Kit),
            // Custom section
            AddOption::Separator("── Custom ──"),
            AddOption::ImportCustom,
            // VST section
            AddOption::Separator("── VST ──"),
            AddOption::ImportVst,
        ]
    }

    /// Build options with custom synthdefs and VST plugins from registries
    fn build_options(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> Vec<AddOption> {
        let mut options = vec![
            // Basic Oscillators
            AddOption::Separator("── Oscillators ──"),
            AddOption::Source(SourceType::Saw),
            AddOption::Source(SourceType::Sin),
            AddOption::Source(SourceType::Sqr),
            AddOption::Source(SourceType::Tri),
            AddOption::Source(SourceType::Noise),
            AddOption::Source(SourceType::Pulse),
            AddOption::Source(SourceType::SuperSaw),
            AddOption::Source(SourceType::Sync),
            // Modulation / FM
            AddOption::Separator("── Modulation ──"),
            AddOption::Source(SourceType::Ring),
            AddOption::Source(SourceType::FBSin),
            AddOption::Source(SourceType::FM),
            AddOption::Source(SourceType::PhaseMod),
            AddOption::Source(SourceType::FMBell),
            AddOption::Source(SourceType::FMBrass),
            // Classic Synths
            AddOption::Separator("── Classic ──"),
            AddOption::Source(SourceType::Choir),
            AddOption::Source(SourceType::EPiano),
            AddOption::Source(SourceType::Organ),
            AddOption::Source(SourceType::BrassStab),
            AddOption::Source(SourceType::Strings),
            AddOption::Source(SourceType::Acid),
            // Physical Modeling
            AddOption::Separator("── Physical ──"),
            AddOption::Source(SourceType::Pluck),
            AddOption::Source(SourceType::Formant),
            AddOption::Source(SourceType::Bowed),
            AddOption::Source(SourceType::Blown),
            AddOption::Source(SourceType::Membrane),
            // Mallet Percussion
            AddOption::Separator("── Mallet ──"),
            AddOption::Source(SourceType::Marimba),
            AddOption::Source(SourceType::Vibes),
            AddOption::Source(SourceType::Kalimba),
            AddOption::Source(SourceType::SteelDrum),
            AddOption::Source(SourceType::TubularBell),
            AddOption::Source(SourceType::Glockenspiel),
            // Plucked Strings
            AddOption::Separator("── Plucked ──"),
            AddOption::Source(SourceType::Guitar),
            AddOption::Source(SourceType::BassGuitar),
            AddOption::Source(SourceType::Harp),
            AddOption::Source(SourceType::Koto),
            // Drums
            AddOption::Separator("── Drums ──"),
            AddOption::Source(SourceType::Kick),
            AddOption::Source(SourceType::Snare),
            AddOption::Source(SourceType::HihatClosed),
            AddOption::Source(SourceType::HihatOpen),
            AddOption::Source(SourceType::Clap),
            AddOption::Source(SourceType::Cowbell),
            AddOption::Source(SourceType::Rim),
            AddOption::Source(SourceType::Tom),
            AddOption::Source(SourceType::Clave),
            AddOption::Source(SourceType::Conga),
            // Experimental
            AddOption::Separator("── Experimental ──"),
            AddOption::Source(SourceType::Gendy),
            AddOption::Source(SourceType::Chaos),
            // Synthesis
            AddOption::Separator("── Synthesis ──"),
            AddOption::Source(SourceType::Additive),
            AddOption::Source(SourceType::Wavetable),
            AddOption::Source(SourceType::Granular),
            // Audio / Routing
            AddOption::Separator("── Routing ──"),
            AddOption::Source(SourceType::AudioIn),
            AddOption::Source(SourceType::BusIn),
            // Samplers
            AddOption::Separator("── Samplers ──"),
            AddOption::Source(SourceType::PitchedSampler),
            AddOption::Source(SourceType::TimeStretch),
            AddOption::Source(SourceType::Kit),
            // Custom section
            AddOption::Separator("── Custom ──"),
        ];

        // Custom synthdefs
        for synthdef in &custom_registry.synthdefs {
            options.push(AddOption::Source(SourceType::Custom(synthdef.id)));
        }

        // Import custom option
        options.push(AddOption::ImportCustom);

        // VST section
        options.push(AddOption::Separator("── VST ──"));

        // Registered VST instruments
        for plugin in vst_registry.instruments() {
            options.push(AddOption::Source(SourceType::Vst(plugin.id)));
        }

        // Import VST option
        options.push(AddOption::ImportVst);

        options
    }

    /// Update cached options from registries
    pub fn update_options(&mut self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) {
        self.cached_options = self.build_options(custom_registry, vst_registry);
        self.selector.scroll_offset = 0;
        self.selector.clamp(self.cached_options.len());
        // Skip separator if we landed on one
        if matches!(self.cached_options.get(self.selector.selected), Some(AddOption::Separator(_))) {
            self.select_next();
        }
    }

    /// Move to next selectable item
    fn select_next(&mut self) {
        let len = self.cached_options.len();
        let opts = &self.cached_options;
        self.selector.select_next(len, |i| matches!(opts.get(i), Some(AddOption::Separator(_))));
        self.selector.adjust_scroll(22); // Conservative visible rows estimate
    }

    /// Move to previous selectable item
    fn select_prev(&mut self) {
        let len = self.cached_options.len();
        let opts = &self.cached_options;
        self.selector.select_prev(len, |i| matches!(opts.get(i), Some(AddOption::Separator(_))));
        self.selector.adjust_scroll(22);
    }

    /// Render with registries for custom synthdef and VST plugin names
    fn render_buf_with_registries(&self, area: Rect, buf: &mut RenderBuf, registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) {
        let rect = center_rect(area, 97, 29);

        let border_style = Style::new().fg(Color::LIME);
        let inner = buf.draw_block(rect, " Add Instrument ", border_style, border_style);

        let content_x = inner.x + 1;
        let content_y = inner.y + 1;

        // Title
        buf.draw_line(
            Rect::new(content_x, content_y, inner.width.saturating_sub(2), 1),
            &[("Select source type:", Style::new().fg(Color::LIME).bold())],
        );

        let list_y = content_y + 2;
        let sel_bg = Style::new().bg(Color::SELECTION_BG);

        // Scroll offset: keep selected item visible
        let visible_rows = (inner.y + inner.height).saturating_sub(list_y) as usize;
        let mut eff_scroll = self.selector.scroll_offset;
        if self.selector.selected < eff_scroll {
            eff_scroll = self.selector.selected;
        } else if visible_rows > 0 && self.selector.selected >= eff_scroll + visible_rows {
            eff_scroll = self.selector.selected - visible_rows + 1;
        }

        // Scroll indicator: items hidden above
        if eff_scroll > 0 {
            let arrow_y = list_y.saturating_sub(1);
            if arrow_y >= inner.y {
                let arrow_x = inner.x + inner.width.saturating_sub(2);
                buf.set_cell(arrow_x, arrow_y, '▲', Style::new().fg(Color::DARK_GRAY));
            }
        }

        for (i, option) in self.cached_options.iter().skip(eff_scroll).take(visible_rows).enumerate() {
            let y = list_y + i as u16;
            let is_selected = eff_scroll + i == self.selector.selected;

            match option {
                AddOption::Separator(label) => {
                    buf.draw_line(
                        Rect::new(content_x, y, inner.width.saturating_sub(2), 1),
                        &[(*label, Style::new().fg(Color::DARK_GRAY))],
                    );
                }
                AddOption::Source(source) => {
                    // Indicator
                    if is_selected {
                        buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    }

                    let color = match source {
                        SourceType::AudioIn => Color::AUDIO_IN_COLOR,
                        SourceType::BusIn => Color::BUS_IN_COLOR,
                        SourceType::PitchedSampler => Color::SAMPLE_COLOR,
                        SourceType::Custom(_) => Color::CUSTOM_COLOR,
                        SourceType::Vst(_) => Color::VST_COLOR,
                        _ => Color::OSC_COLOR,
                    };

                    let short = format!("{:12}", source.short_name_vst(registry, vst_registry));
                    let name = source.display_name_vst(registry, vst_registry);

                    let short_style = if is_selected {
                        Style::new().fg(color).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(color)
                    };
                    let name_display = format!("  {}", name);
                    let name_style = if is_selected {
                        Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(Color::DARK_GRAY)
                    };

                    buf.draw_line(
                        Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                        &[(&short, short_style), (&name_display, name_style)],
                    );

                    // Fill rest of line with selection bg
                    if is_selected {
                        let fill_start = content_x + 2 + 14 + name.len() as u16;
                        let fill_end = inner.x + inner.width;
                        for x in fill_start..fill_end {
                            buf.set_cell(x, y, ' ', sel_bg);
                        }
                    }
                }
                AddOption::ImportCustom => {
                    if is_selected {
                        buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    }

                    let text_style = if is_selected {
                        Style::new().fg(Color::PURPLE).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(Color::PURPLE)
                    };
                    buf.draw_line(
                        Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                        &[("+ Import Custom SynthDef...", text_style)],
                    );

                    if is_selected {
                        let fill_start = content_x + 2 + 27;
                        let fill_end = inner.x + inner.width;
                        for x in fill_start..fill_end {
                            buf.set_cell(x, y, ' ', sel_bg);
                        }
                    }
                }
                AddOption::ImportVst => {
                    if is_selected {
                        buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    }

                    let text_style = if is_selected {
                        Style::new().fg(Color::VST_COLOR).bg(Color::SELECTION_BG)
                    } else {
                        Style::new().fg(Color::VST_COLOR)
                    };
                    buf.draw_line(
                        Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                        &[("+ Import VST Instrument...", text_style)],
                    );

                    if is_selected {
                        let fill_start = content_x + 2 + 26;
                        let fill_end = inner.x + inner.width;
                        for x in fill_start..fill_end {
                            buf.set_cell(x, y, ' ', sel_bg);
                        }
                    }
                }
            }
        }

        // Scroll indicator: items hidden below
        if eff_scroll + visible_rows < self.cached_options.len() {
            let arrow_y = list_y + visible_rows as u16;
            if arrow_y < inner.y + inner.height {
                let arrow_x = inner.x + inner.width.saturating_sub(2);
                buf.set_cell(arrow_x, arrow_y, '▼', Style::new().fg(Color::DARK_GRAY));
            }
        }

    }

}

impl Default for AddPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for AddPane {
    fn id(&self) -> &'static str {
        "add"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::Add(AddActionId::Confirm) => {
                if let Some(option) = self.cached_options.get(self.selector.selected) {
                    match option {
                        AddOption::Source(source) => Action::Instrument(InstrumentAction::Add(*source)),
                        AddOption::ImportCustom => {
                            Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::ImportCustomSynthDef))
                        }
                        AddOption::ImportVst => {
                            Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::ImportVstInstrument))
                        }
                        AddOption::Separator(_) => Action::None,
                    }
                } else {
                    Action::None
                }
            }
            ActionId::Add(AddActionId::Cancel) => {
                if state.instruments.instruments.is_empty() {
                    Action::Nav(NavAction::SwitchPane("server"))
                } else {
                    Action::Nav(NavAction::SwitchPane("instrument"))
                }
            }
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

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, _state: &AppState) -> Action {
        let rect = center_rect(area, 97, 29);
        let inner_y = rect.y + 2;
        let content_y = inner_y + 1;
        let list_y = content_y + 2;
        let visible_rows = (rect.y + rect.height).saturating_sub(1).saturating_sub(list_y) as usize;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = event.row;
                if row >= list_y && (row - list_y) < visible_rows as u16 {
                    let idx = self.selector.scroll_offset + (row - list_y) as usize;
                    if idx < self.cached_options.len() {
                        // Skip separators
                        if matches!(self.cached_options.get(idx), Some(AddOption::Separator(_))) {
                            return Action::None;
                        }
                        self.selector.selected = idx;
                        // Confirm selection
                        match &self.cached_options[idx] {
                            AddOption::Source(source) => return Action::Instrument(InstrumentAction::Add(*source)),
                            AddOption::ImportCustom => {
                                return Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::ImportCustomSynthDef));
                            }
                            AddOption::ImportVst => {
                                return Action::Session(SessionAction::OpenFileBrowser(FileSelectAction::ImportVstInstrument));
                            }
                            AddOption::Separator(_) => {}
                        }
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
        self.render_buf_with_registries(area, buf, &state.session.custom_synthdefs, &state.session.vst_plugins);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn on_enter(&mut self, state: &AppState) {
        self.update_options(&state.session.custom_synthdefs, &state.session.vst_plugins);
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
