use std::any::Any;

use crate::state::{AppState, CustomSynthDefRegistry, SourceType, SourceTypeExt, VstPluginRegistry};
use crate::ui::action_id::{ActionId, AddActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, FileSelectAction, InputEvent, InstrumentAction, Keymap, MouseEvent, MouseEventKind, MouseButton, NavAction, Pane, SessionAction, Style};

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
    selected: usize,
    scroll_offset: usize,
    /// Cached options list - rebuilt on each render_with_registry call
    cached_options: Vec<AddOption>,
}

impl AddPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected: 1, // Start on first selectable item (skip separator)
            scroll_offset: 0,
            cached_options: Self::build_options_static(),
        }
    }

    /// Build options without registries (used for initial state)
    fn build_options_static() -> Vec<AddOption> {
        let mut options = Vec::new();

        // Basic Oscillators
        options.push(AddOption::Separator("── Oscillators ──"));
        options.push(AddOption::Source(SourceType::Saw));
        options.push(AddOption::Source(SourceType::Sin));
        options.push(AddOption::Source(SourceType::Sqr));
        options.push(AddOption::Source(SourceType::Tri));
        options.push(AddOption::Source(SourceType::Noise));
        options.push(AddOption::Source(SourceType::Pulse));
        options.push(AddOption::Source(SourceType::SuperSaw));
        options.push(AddOption::Source(SourceType::Sync));

        // Modulation / FM
        options.push(AddOption::Separator("── Modulation ──"));
        options.push(AddOption::Source(SourceType::Ring));
        options.push(AddOption::Source(SourceType::FBSin));
        options.push(AddOption::Source(SourceType::FM));
        options.push(AddOption::Source(SourceType::PhaseMod));
        options.push(AddOption::Source(SourceType::FMBell));
        options.push(AddOption::Source(SourceType::FMBrass));

        // Classic Synths
        options.push(AddOption::Separator("── Classic ──"));
        options.push(AddOption::Source(SourceType::Choir));
        options.push(AddOption::Source(SourceType::EPiano));
        options.push(AddOption::Source(SourceType::Organ));
        options.push(AddOption::Source(SourceType::BrassStab));
        options.push(AddOption::Source(SourceType::Strings));
        options.push(AddOption::Source(SourceType::Acid));

        // Physical Modeling
        options.push(AddOption::Separator("── Physical ──"));
        options.push(AddOption::Source(SourceType::Pluck));
        options.push(AddOption::Source(SourceType::Formant));
        options.push(AddOption::Source(SourceType::Bowed));
        options.push(AddOption::Source(SourceType::Blown));
        options.push(AddOption::Source(SourceType::Membrane));

        // Mallet Percussion
        options.push(AddOption::Separator("── Mallet ──"));
        options.push(AddOption::Source(SourceType::Marimba));
        options.push(AddOption::Source(SourceType::Vibes));
        options.push(AddOption::Source(SourceType::Kalimba));
        options.push(AddOption::Source(SourceType::SteelDrum));
        options.push(AddOption::Source(SourceType::TubularBell));
        options.push(AddOption::Source(SourceType::Glockenspiel));

        // Plucked Strings
        options.push(AddOption::Separator("── Plucked ──"));
        options.push(AddOption::Source(SourceType::Guitar));
        options.push(AddOption::Source(SourceType::BassGuitar));
        options.push(AddOption::Source(SourceType::Harp));
        options.push(AddOption::Source(SourceType::Koto));

        // Drums
        options.push(AddOption::Separator("── Drums ──"));
        options.push(AddOption::Source(SourceType::Kick));
        options.push(AddOption::Source(SourceType::Snare));
        options.push(AddOption::Source(SourceType::HihatClosed));
        options.push(AddOption::Source(SourceType::HihatOpen));
        options.push(AddOption::Source(SourceType::Clap));
        options.push(AddOption::Source(SourceType::Cowbell));
        options.push(AddOption::Source(SourceType::Rim));
        options.push(AddOption::Source(SourceType::Tom));
        options.push(AddOption::Source(SourceType::Clave));
        options.push(AddOption::Source(SourceType::Conga));

        // Experimental
        options.push(AddOption::Separator("── Experimental ──"));
        options.push(AddOption::Source(SourceType::Gendy));
        options.push(AddOption::Source(SourceType::Chaos));

        // Synthesis
        options.push(AddOption::Separator("── Synthesis ──"));
        options.push(AddOption::Source(SourceType::Additive));
        options.push(AddOption::Source(SourceType::Wavetable));
        options.push(AddOption::Source(SourceType::Granular));

        // Audio / Routing
        options.push(AddOption::Separator("── Routing ──"));
        options.push(AddOption::Source(SourceType::AudioIn));
        options.push(AddOption::Source(SourceType::BusIn));

        // Samplers
        options.push(AddOption::Separator("── Samplers ──"));
        options.push(AddOption::Source(SourceType::PitchedSampler));
        options.push(AddOption::Source(SourceType::TimeStretch));
        options.push(AddOption::Source(SourceType::Kit));

        // Custom section
        options.push(AddOption::Separator("── Custom ──"));
        options.push(AddOption::ImportCustom);

        // VST section
        options.push(AddOption::Separator("── VST ──"));
        options.push(AddOption::ImportVst);

        options
    }

    /// Build options with custom synthdefs and VST plugins from registries
    fn build_options(&self, custom_registry: &CustomSynthDefRegistry, vst_registry: &VstPluginRegistry) -> Vec<AddOption> {
        let mut options = Vec::new();

        // Basic Oscillators
        options.push(AddOption::Separator("── Oscillators ──"));
        options.push(AddOption::Source(SourceType::Saw));
        options.push(AddOption::Source(SourceType::Sin));
        options.push(AddOption::Source(SourceType::Sqr));
        options.push(AddOption::Source(SourceType::Tri));
        options.push(AddOption::Source(SourceType::Noise));
        options.push(AddOption::Source(SourceType::Pulse));
        options.push(AddOption::Source(SourceType::SuperSaw));
        options.push(AddOption::Source(SourceType::Sync));

        // Modulation / FM
        options.push(AddOption::Separator("── Modulation ──"));
        options.push(AddOption::Source(SourceType::Ring));
        options.push(AddOption::Source(SourceType::FBSin));
        options.push(AddOption::Source(SourceType::FM));
        options.push(AddOption::Source(SourceType::PhaseMod));
        options.push(AddOption::Source(SourceType::FMBell));
        options.push(AddOption::Source(SourceType::FMBrass));

        // Classic Synths
        options.push(AddOption::Separator("── Classic ──"));
        options.push(AddOption::Source(SourceType::Choir));
        options.push(AddOption::Source(SourceType::EPiano));
        options.push(AddOption::Source(SourceType::Organ));
        options.push(AddOption::Source(SourceType::BrassStab));
        options.push(AddOption::Source(SourceType::Strings));
        options.push(AddOption::Source(SourceType::Acid));

        // Physical Modeling
        options.push(AddOption::Separator("── Physical ──"));
        options.push(AddOption::Source(SourceType::Pluck));
        options.push(AddOption::Source(SourceType::Formant));
        options.push(AddOption::Source(SourceType::Bowed));
        options.push(AddOption::Source(SourceType::Blown));
        options.push(AddOption::Source(SourceType::Membrane));

        // Mallet Percussion
        options.push(AddOption::Separator("── Mallet ──"));
        options.push(AddOption::Source(SourceType::Marimba));
        options.push(AddOption::Source(SourceType::Vibes));
        options.push(AddOption::Source(SourceType::Kalimba));
        options.push(AddOption::Source(SourceType::SteelDrum));
        options.push(AddOption::Source(SourceType::TubularBell));
        options.push(AddOption::Source(SourceType::Glockenspiel));

        // Plucked Strings
        options.push(AddOption::Separator("── Plucked ──"));
        options.push(AddOption::Source(SourceType::Guitar));
        options.push(AddOption::Source(SourceType::BassGuitar));
        options.push(AddOption::Source(SourceType::Harp));
        options.push(AddOption::Source(SourceType::Koto));

        // Drums
        options.push(AddOption::Separator("── Drums ──"));
        options.push(AddOption::Source(SourceType::Kick));
        options.push(AddOption::Source(SourceType::Snare));
        options.push(AddOption::Source(SourceType::HihatClosed));
        options.push(AddOption::Source(SourceType::HihatOpen));
        options.push(AddOption::Source(SourceType::Clap));
        options.push(AddOption::Source(SourceType::Cowbell));
        options.push(AddOption::Source(SourceType::Rim));
        options.push(AddOption::Source(SourceType::Tom));
        options.push(AddOption::Source(SourceType::Clave));
        options.push(AddOption::Source(SourceType::Conga));

        // Experimental
        options.push(AddOption::Separator("── Experimental ──"));
        options.push(AddOption::Source(SourceType::Gendy));
        options.push(AddOption::Source(SourceType::Chaos));

        // Synthesis
        options.push(AddOption::Separator("── Synthesis ──"));
        options.push(AddOption::Source(SourceType::Additive));
        options.push(AddOption::Source(SourceType::Wavetable));
        options.push(AddOption::Source(SourceType::Granular));

        // Audio / Routing
        options.push(AddOption::Separator("── Routing ──"));
        options.push(AddOption::Source(SourceType::AudioIn));
        options.push(AddOption::Source(SourceType::BusIn));

        // Samplers
        options.push(AddOption::Separator("── Samplers ──"));
        options.push(AddOption::Source(SourceType::PitchedSampler));
        options.push(AddOption::Source(SourceType::TimeStretch));
        options.push(AddOption::Source(SourceType::Kit));

        // Custom section
        options.push(AddOption::Separator("── Custom ──"));

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
        self.scroll_offset = 0;
        // Clamp selection and ensure it's not on a separator
        if self.selected >= self.cached_options.len() {
            self.selected = self.cached_options.len().saturating_sub(1);
        }
        // Skip separator if we landed on one
        while matches!(self.cached_options.get(self.selected), Some(AddOption::Separator(_))) {
            self.selected = (self.selected + 1) % self.cached_options.len();
        }
    }

    /// Move to next selectable item
    fn select_next(&mut self) {
        let len = self.cached_options.len();
        if len == 0 {
            return;
        }

        let mut next = (self.selected + 1) % len;
        // Skip separators
        while matches!(self.cached_options.get(next), Some(AddOption::Separator(_))) {
            next = (next + 1) % len;
        }
        self.selected = next;
        self.adjust_scroll();
    }

    /// Move to previous selectable item
    fn select_prev(&mut self) {
        let len = self.cached_options.len();
        if len == 0 {
            return;
        }

        let mut prev = if self.selected == 0 {
            len - 1
        } else {
            self.selected - 1
        };
        // Skip separators
        while matches!(self.cached_options.get(prev), Some(AddOption::Separator(_))) {
            prev = if prev == 0 { len - 1 } else { prev - 1 };
        }
        self.selected = prev;
        self.adjust_scroll();
    }

    /// Adjust scroll_offset so the selected item stays visible.
    /// Uses the dialog's fixed height (29) to estimate visible rows.
    fn adjust_scroll(&mut self) {
        // Dialog is 29 tall, border=1 each side, padding=1 top, title line, gap = list starts at +5
        // Bottom border + help line + gap = 3 rows reserved at bottom
        // visible_rows ≈ 29 - 2 (borders) - 1 (padding) - 1 (title) - 1 (gap) - 2 (help+border) = ~22
        // But the exact value is computed in render from inner rect.
        // Use a conservative estimate here; render will correct if needed.
        let visible_rows = 22usize;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected - visible_rows + 1;
        }
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
        let mut eff_scroll = self.scroll_offset;
        if self.selected < eff_scroll {
            eff_scroll = self.selected;
        } else if visible_rows > 0 && self.selected >= eff_scroll + visible_rows {
            eff_scroll = self.selected - visible_rows + 1;
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
            let is_selected = eff_scroll + i == self.selected;

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

        // Help text
        let help_y = rect.y + rect.height - 2;
        if help_y < area.y + area.height {
            buf.draw_line(
                Rect::new(content_x, help_y, inner.width.saturating_sub(2), 1),
                &[("Enter: add | Escape: cancel | Up/Down: navigate", Style::new().fg(Color::DARK_GRAY))],
            );
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
                if let Some(option) = self.cached_options.get(self.selected) {
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
                    let idx = self.scroll_offset + (row - list_y) as usize;
                    if idx < self.cached_options.len() {
                        // Skip separators
                        if matches!(self.cached_options.get(idx), Some(AddOption::Separator(_))) {
                            return Action::None;
                        }
                        self.selected = idx;
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
