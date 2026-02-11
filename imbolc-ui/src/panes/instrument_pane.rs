use std::any::Any;

use crate::state::{AppState, OwnershipDisplayStatus, SourceType};
use crate::ui::action_id::{ActionId, InstrumentListActionId, ModeActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::performance::PerformanceController;
use crate::ui::{
    translate_key, Action, Color, InputEvent, InstrumentAction, KeyCode, Keymap, MouseButton,
    MouseEvent, MouseEventKind, NavAction, Pane, PaneId, Rect, RenderBuf, SessionAction, Style,
    ToggleResult,
};
use imbolc_types::InstrumentId;

fn source_color(source: SourceType) -> Color {
    match source {
        // Oscillators and synths
        SourceType::Saw | SourceType::Sin | SourceType::Sqr | SourceType::Tri
        | SourceType::Noise | SourceType::Pulse | SourceType::SuperSaw | SourceType::Sync
        | SourceType::Ring | SourceType::FBSin | SourceType::FM | SourceType::PhaseMod
        | SourceType::FMBell | SourceType::FMBrass
        | SourceType::Pluck | SourceType::Formant | SourceType::Gendy | SourceType::Chaos
        | SourceType::Additive | SourceType::Wavetable | SourceType::Granular
        | SourceType::Bowed | SourceType::Blown | SourceType::Membrane
        // Mallet percussion
        | SourceType::Marimba | SourceType::Vibes | SourceType::Kalimba | SourceType::SteelDrum
        | SourceType::TubularBell | SourceType::Glockenspiel
        // Plucked strings
        | SourceType::Guitar | SourceType::BassGuitar | SourceType::Harp | SourceType::Koto
        // Drums
        | SourceType::Kick | SourceType::Snare | SourceType::HihatClosed | SourceType::HihatOpen
        | SourceType::Clap | SourceType::Cowbell | SourceType::Rim | SourceType::Tom
        | SourceType::Clave | SourceType::Conga
        // Classic synths
        | SourceType::Choir | SourceType::EPiano | SourceType::Organ | SourceType::BrassStab
        | SourceType::Strings | SourceType::Acid => Color::OSC_COLOR,
        SourceType::AudioIn => Color::AUDIO_IN_COLOR,
        SourceType::PitchedSampler | SourceType::TimeStretch => Color::SAMPLE_COLOR,
        SourceType::Kit => Color::KIT_COLOR,
        SourceType::BusIn => Color::BUS_IN_COLOR,
        SourceType::Custom(_) => Color::CUSTOM_COLOR,
        SourceType::Vst(_) => Color::VST_COLOR,
    }
}

pub struct InstrumentPane {
    keymap: Keymap,
    perf: PerformanceController,
    /// When Some, we're waiting for the user to select a target instrument to link with
    linking_from: Option<crate::state::InstrumentId>,
}

impl InstrumentPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            perf: PerformanceController::new(),
            linking_from: None,
        }
    }

    pub fn set_enhanced_keyboard(&mut self, enabled: bool) {
        self.perf.set_enhanced_keyboard(enabled);
    }

    fn format_filter(instrument: &crate::state::instrument::Instrument) -> String {
        match instrument.filter() {
            Some(f) => format!("[{}]", f.filter_type.name()),
            None => "---".to_string(),
        }
    }

    fn format_eq(instrument: &crate::state::instrument::Instrument) -> &'static str {
        if instrument.eq().is_some() {
            "[EQ]"
        } else {
            ""
        }
    }

    fn format_effects(instrument: &crate::state::instrument::Instrument) -> String {
        let effects: Vec<_> = instrument.effects().collect();
        if effects.is_empty() {
            return "---".to_string();
        }
        effects
            .iter()
            .map(|e| e.effect_type.name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_level(level: f32) -> String {
        let filled = (level * 5.0) as usize;
        let bar: String = (0..5).map(|i| if i < filled { '▊' } else { '░' }).collect();
        format!("{} {:.0}%", bar, level * 100.0)
    }
}

impl Default for InstrumentPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for InstrumentPane {
    fn id(&self) -> &'static str {
        "instrument"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        // If we're in linking mode, handle confirm/cancel/navigate
        if let Some(from_id) = self.linking_from {
            match action {
                // Press 'l' again to confirm the link
                ActionId::InstrumentList(InstrumentListActionId::LinkLayer) => {
                    self.linking_from = None;
                    if let Some(target) = state.instruments.selected_instrument() {
                        let target_id = target.id;
                        if target_id != from_id {
                            return Action::Instrument(InstrumentAction::LinkLayer(
                                from_id, target_id,
                            ));
                        }
                    }
                    return Action::None;
                }
                // Navigation passes through to normal handling below
                ActionId::InstrumentList(InstrumentListActionId::Next)
                | ActionId::InstrumentList(InstrumentListActionId::Prev)
                | ActionId::InstrumentList(InstrumentListActionId::GotoTop)
                | ActionId::InstrumentList(InstrumentListActionId::GotoBottom) => {
                    // Let navigation proceed normally
                }
                // Any other action cancels linking mode
                _ => {
                    self.linking_from = None;
                }
            }
        }

        match action {
            ActionId::InstrumentList(InstrumentListActionId::Quit) => Action::QuitIntent,
            ActionId::InstrumentList(InstrumentListActionId::Next) => {
                Action::Instrument(InstrumentAction::SelectNext)
            }
            ActionId::InstrumentList(InstrumentListActionId::Prev) => {
                Action::Instrument(InstrumentAction::SelectPrev)
            }
            ActionId::InstrumentList(InstrumentListActionId::GotoTop) => {
                Action::Instrument(InstrumentAction::SelectFirst)
            }
            ActionId::InstrumentList(InstrumentListActionId::GotoBottom) => {
                Action::Instrument(InstrumentAction::SelectLast)
            }
            ActionId::InstrumentList(InstrumentListActionId::Add) => {
                Action::Nav(NavAction::SwitchPane(PaneId::Add))
            }
            ActionId::InstrumentList(InstrumentListActionId::Delete) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    Action::Instrument(InstrumentAction::Delete(instrument.id))
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentList(InstrumentListActionId::Edit) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    Action::Instrument(InstrumentAction::Edit(instrument.id))
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentList(InstrumentListActionId::Save) => {
                Action::Session(SessionAction::Save)
            }
            ActionId::InstrumentList(InstrumentListActionId::Load) => {
                Action::Session(SessionAction::Load)
            }
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    self.linking_from = Some(instrument.id);
                }
                Action::None
            }
            ActionId::InstrumentList(InstrumentListActionId::UnlinkLayer) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    Action::Instrument(InstrumentAction::UnlinkLayer(instrument.id))
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentList(InstrumentListActionId::LayerOctaveUp) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    Action::Instrument(InstrumentAction::AdjustLayerOctaveOffset(instrument.id, 1))
                } else {
                    Action::None
                }
            }
            ActionId::InstrumentList(InstrumentListActionId::LayerOctaveDown) => {
                if let Some(instrument) = state.instruments.selected_instrument() {
                    Action::Instrument(InstrumentAction::AdjustLayerOctaveOffset(instrument.id, -1))
                } else {
                    Action::None
                }
            }

            // Piano layer actions
            ActionId::Mode(ModeActionId::PianoEscape) => {
                let was_active = self.perf.piano.is_active();
                self.perf.piano.handle_escape();
                if was_active && !self.perf.piano.is_active() {
                    Action::ExitPerformanceMode
                } else {
                    Action::None
                }
            }
            ActionId::Mode(ModeActionId::PianoOctaveDown) => {
                self.perf.piano.octave_down();
                Action::None
            }
            ActionId::Mode(ModeActionId::PianoOctaveUp) => {
                self.perf.piano.octave_up();
                Action::None
            }
            ActionId::Mode(ModeActionId::PianoKey) | ActionId::Mode(ModeActionId::PianoSpace) => {
                if let KeyCode::Char(c) = event.key {
                    let c = translate_key(c, state.keyboard_layout);
                    if let Some(pitches) = self.perf.piano.key_to_pitches(c) {
                        // Check if this is a new press or key repeat (sustain)
                        if let Some(new_pitches) = self.perf.piano.key_pressed(
                            c,
                            pitches.clone(),
                            event.timestamp,
                            event.is_repeat,
                        ) {
                            // NEW press - spawn voice(s)
                            if new_pitches.len() == 1 {
                                return Action::Instrument(InstrumentAction::PlayNote(
                                    new_pitches[0],
                                    100,
                                ));
                            } else {
                                return Action::Instrument(InstrumentAction::PlayNotes(
                                    new_pitches,
                                    100,
                                ));
                            }
                        }
                        // Key repeat - sustain, no action needed
                    }
                }
                Action::None
            }

            // Pad layer actions
            ActionId::Mode(ModeActionId::PadEscape) => {
                self.perf.pad.deactivate();
                Action::ExitPerformanceMode
            }
            ActionId::Mode(ModeActionId::PadKey) => {
                if let KeyCode::Char(c) = event.key {
                    let c = translate_key(c, state.keyboard_layout);
                    if let Some(pad_idx) = self.perf.pad.key_to_pad(c) {
                        return Action::Instrument(InstrumentAction::PlayDrumPad(pad_idx));
                    }
                }
                Action::None
            }

            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 97, 29);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Instruments ", border_style, border_style);

        let content_x = inner.x + 1;
        let content_y = inner.y + 1;

        buf.draw_line(
            Rect::new(content_x, content_y, inner.width.saturating_sub(2), 1),
            &[("Instruments:", Style::new().fg(Color::CYAN).bold())],
        );

        let list_y = content_y + 2;
        let max_visible = ((inner.height.saturating_sub(7)) as usize).max(3);

        if state.instruments.instruments.is_empty() {
            buf.draw_line(
                Rect::new(content_x + 2, list_y, inner.width.saturating_sub(4), 1),
                &[(
                    "(no instruments — press 'a' to add)",
                    Style::new().fg(Color::DARK_GRAY),
                )],
            );
        }

        let scroll_offset = state
            .instruments
            .selected
            .map(|s| {
                if s >= max_visible {
                    s - max_visible + 1
                } else {
                    0
                }
            })
            .unwrap_or(0);
        let sel_bg = Style::new().bg(Color::SELECTION_BG);

        for (i, instrument) in state
            .instruments
            .instruments
            .iter()
            .enumerate()
            .skip(scroll_offset)
        {
            let row = i - scroll_offset;
            if row >= max_visible {
                break;
            }
            let y = list_y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let is_selected = state.instruments.selected == Some(i);

            // Selection indicator
            if is_selected {
                buf.set_cell(
                    content_x,
                    y,
                    '>',
                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
                );
            }

            let mk_style = |fg: Color| -> Style {
                if is_selected {
                    Style::new().fg(fg).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(fg)
                }
            };

            // Build row as a Line with multiple spans
            let name_str = format!("{:14}", &instrument.name[..instrument.name.len().min(14)]);
            let source_str = format!(" {:10}", instrument.source.name());
            let filter_str = format!(" {:12}", Self::format_filter(instrument));
            let eq_str = format!(" {:4}", Self::format_eq(instrument));
            let fx_raw = Self::format_effects(instrument);
            let fx_str = format!(" {:18}", &fx_raw[..fx_raw.len().min(18)]);
            let level_str = format!(" {}", Self::format_level(instrument.mixer.level));

            let source_c = source_color(instrument.source);

            let layer_str = match instrument.layer.group {
                Some(g) if instrument.layer.octave_offset != 0 => {
                    format!(" [L{}:{:+}]", g, instrument.layer.octave_offset)
                }
                Some(g) => format!(" [L{}]", g),
                None => String::new(),
            };

            // Ownership indicator for network mode
            let ownership_str = match state.ownership_status(instrument.id) {
                OwnershipDisplayStatus::OwnedByMe => " [ME]".to_string(),
                OwnershipDisplayStatus::OwnedByOther(ref name) => {
                    let short = if name.len() > 6 { &name[..6] } else { name };
                    format!(" [{}]", short)
                }
                OwnershipDisplayStatus::Unowned => String::new(),
                OwnershipDisplayStatus::Local => String::new(),
            };
            let ownership_color = match state.ownership_status(instrument.id) {
                OwnershipDisplayStatus::OwnedByMe => Color::LIME,
                OwnershipDisplayStatus::OwnedByOther(_) => Color::ORANGE,
                _ => Color::DARK_GRAY,
            };

            let mut spans: Vec<(&str, Style)> = vec![
                (&name_str, mk_style(Color::WHITE)),
                (&source_str, mk_style(source_c)),
                (&filter_str, mk_style(Color::FILTER_COLOR)),
                (&eq_str, mk_style(Color::EQ_COLOR)),
                (&fx_str, mk_style(Color::FX_COLOR)),
                (&level_str, mk_style(Color::LIME)),
            ];
            if !layer_str.is_empty() {
                spans.push((&layer_str, mk_style(Color::ORANGE)));
            }
            if !ownership_str.is_empty() {
                spans.push((&ownership_str, mk_style(ownership_color)));
            }
            let line_width = inner.width.saturating_sub(3);
            buf.draw_line(Rect::new(content_x + 2, y, line_width, 1), &spans);

            // Fill rest of line with selection bg
            if is_selected {
                let fill_start = content_x + 2 + line_width;
                let fill_end = inner.x + inner.width;
                for x in fill_start..fill_end {
                    buf.set_cell(x, y, ' ', sel_bg);
                }
            }
        }

        // Scroll indicators
        let scroll_style = Style::new().fg(Color::ORANGE);
        if scroll_offset > 0 {
            buf.draw_line(
                Rect::new(rect.x + rect.width - 5, list_y, 3, 1),
                &[("...", scroll_style)],
            );
        }
        if scroll_offset + max_visible < state.instruments.instruments.len() {
            buf.draw_line(
                Rect::new(
                    rect.x + rect.width - 5,
                    list_y + max_visible as u16 - 1,
                    3,
                    1,
                ),
                &[("...", scroll_style)],
            );
        }

        // Piano/Pad mode indicator
        if self.perf.pad.is_active() {
            let pad_str = self.perf.pad.status_label();
            let pad_x = rect.x + rect.width - pad_str.len() as u16 - 1;
            buf.draw_line(
                Rect::new(pad_x, rect.y, pad_str.len() as u16, 1),
                &[(&pad_str, Style::new().fg(Color::BLACK).bg(Color::KIT_COLOR))],
            );
        } else if self.perf.piano.is_active() {
            let piano_str = self.perf.piano.status_label();
            let piano_x = rect.x + rect.width - piano_str.len() as u16 - 1;
            buf.draw_line(
                Rect::new(piano_x, rect.y, piano_str.len() as u16, 1),
                &[(&piano_str, Style::new().fg(Color::BLACK).bg(Color::PINK))],
            );
        }

        // Link mode indicator
        if self.linking_from.is_some() {
            let link_str = " LINK: \u{2191}/\u{2193} navigate, l confirm, Esc cancel ";
            let link_x = rect.x + rect.width - link_str.len() as u16 - 1;
            buf.draw_line(
                Rect::new(link_x, rect.y, link_str.len() as u16, 1),
                &[(link_str, Style::new().fg(Color::BLACK).bg(Color::ORANGE))],
            );
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action {
        let rect = center_rect(area, 97, 29);
        let inner_x = rect.x + 2;
        let inner_y = rect.y + 2;
        let content_y = inner_y + 1;
        let list_y = content_y + 2;
        let inner_height = rect.height.saturating_sub(4);
        let max_visible = ((inner_height.saturating_sub(7)) as usize).max(3);

        let scroll_offset = state
            .instruments
            .selected
            .map(|s| {
                if s >= max_visible {
                    s - max_visible + 1
                } else {
                    0
                }
            })
            .unwrap_or(0);

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let col = event.column;
                let row = event.row;
                // Click on instrument list
                if col >= inner_x && row >= list_y && row < list_y + max_visible as u16 {
                    let clicked_idx = scroll_offset + (row - list_y) as usize;
                    if clicked_idx < state.instruments.instruments.len() {
                        return Action::Instrument(InstrumentAction::Select(clicked_idx));
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => Action::Instrument(InstrumentAction::SelectPrev),
            MouseEventKind::ScrollDown => Action::Instrument(InstrumentAction::SelectNext),
            _ => Action::None,
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn tick(&mut self, state: &AppState) -> Vec<Action> {
        let instrument_id = state
            .instruments
            .selected_instrument()
            .map(|inst| inst.id)
            .unwrap_or(InstrumentId::new(0));
        self.perf.tick_releases(instrument_id)
    }

    fn toggle_performance_mode(&mut self, state: &AppState) -> ToggleResult {
        let is_kit = state
            .instruments
            .selected_instrument()
            .is_some_and(|s| s.source.is_kit());
        self.perf.toggle(is_kit)
    }

    fn activate_piano(&mut self) {
        self.perf.activate_piano();
    }

    fn activate_pad(&mut self) {
        self.perf.activate_pad();
    }

    fn deactivate_performance(&mut self) {
        self.perf.deactivate();
    }

    fn supports_performance_mode(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, SourceType};
    use crate::ui::{InputEvent, KeyCode, Modifiers};

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn delete_returns_selected_instrument_id() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Delete),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Instrument(InstrumentAction::Delete(got)) => assert_eq!(got, id),
            _ => panic!("Expected InstrumentAction::Delete"),
        }
    }

    #[test]
    fn edit_returns_selected_instrument_id() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Sin);
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Edit),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Instrument(InstrumentAction::Edit(got)) => assert_eq!(got, id),
            _ => panic!("Expected InstrumentAction::Edit"),
        }
    }

    #[test]
    fn add_navigates_to_add_pane() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let state = AppState::new();
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Add),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Nav(NavAction::SwitchPane(id)) => assert_eq!(id, crate::ui::PaneId::Add),
            _ => panic!("Expected SwitchPane(add)"),
        }
    }

    #[test]
    fn next_prev_return_select_actions() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let state = AppState::new();
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Next),
            &dummy_event(),
            &state,
        );
        assert!(matches!(
            action,
            Action::Instrument(InstrumentAction::SelectNext)
        ));

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Prev),
            &dummy_event(),
            &state,
        );
        assert!(matches!(
            action,
            Action::Instrument(InstrumentAction::SelectPrev)
        ));
    }

    #[test]
    fn link_mode_navigation_passes_through() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id0 = state.add_instrument(SourceType::Saw);
        let _id1 = state.add_instrument(SourceType::Sin);

        // Select first instrument before entering link mode
        state.instruments.selected = Some(0);

        let mut pane = InstrumentPane::new(Keymap::new());
        // Enter linking mode
        pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );
        assert_eq!(pane.linking_from, Some(id0));

        // Navigation should pass through (return SelectNext), not complete the link
        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Next),
            &dummy_event(),
            &state,
        );
        assert!(matches!(
            action,
            Action::Instrument(InstrumentAction::SelectNext)
        ));
        // Should still be in linking mode
        assert_eq!(pane.linking_from, Some(id0));
    }

    #[test]
    fn link_mode_confirm_with_different_target() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id0 = state.add_instrument(SourceType::Saw);
        let id1 = state.add_instrument(SourceType::Sin);

        // Select first instrument, enter linking mode
        state.instruments.selected = Some(0);
        let mut pane = InstrumentPane::new(Keymap::new());
        pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );
        assert_eq!(pane.linking_from, Some(id0));

        // Move selection to second instrument
        state.instruments.selected = Some(1);

        // Press 'l' again to confirm
        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Instrument(InstrumentAction::LinkLayer(from, to)) => {
                assert_eq!(from, id0);
                assert_eq!(to, id1);
            }
            _ => panic!("Expected LinkLayer action, got {:?}", action),
        }
        assert!(pane.linking_from.is_none());
    }

    #[test]
    fn link_mode_confirm_same_instrument_no_action() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let _id0 = state.add_instrument(SourceType::Saw);

        let mut pane = InstrumentPane::new(Keymap::new());
        // Enter linking mode
        pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );

        // Press 'l' again without moving — same instrument selected
        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );
        assert!(matches!(action, Action::None));
        assert!(pane.linking_from.is_none());
    }

    #[test]
    fn link_mode_cancelled_by_other_action() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let _id0 = state.add_instrument(SourceType::Saw);

        let mut pane = InstrumentPane::new(Keymap::new());
        // Enter linking mode
        pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LinkLayer),
            &dummy_event(),
            &state,
        );
        assert!(pane.linking_from.is_some());

        // Any non-nav, non-link action should cancel
        pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::Delete),
            &dummy_event(),
            &state,
        );
        assert!(pane.linking_from.is_none());
    }

    #[test]
    fn layer_octave_up_returns_adjust_action() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LayerOctaveUp),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Instrument(InstrumentAction::AdjustLayerOctaveOffset(got_id, delta)) => {
                assert_eq!(got_id, id);
                assert_eq!(delta, 1);
            }
            _ => panic!("Expected AdjustLayerOctaveOffset(+1), got {:?}", action),
        }
    }

    #[test]
    fn layer_octave_down_returns_adjust_action() {
        use crate::ui::action_id::{ActionId, InstrumentListActionId};
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        let mut pane = InstrumentPane::new(Keymap::new());

        let action = pane.handle_action(
            ActionId::InstrumentList(InstrumentListActionId::LayerOctaveDown),
            &dummy_event(),
            &state,
        );
        match action {
            Action::Instrument(InstrumentAction::AdjustLayerOctaveOffset(got_id, delta)) => {
                assert_eq!(got_id, id);
                assert_eq!(delta, -1);
            }
            _ => panic!("Expected AdjustLayerOctaveOffset(-1), got {:?}", action),
        }
    }
}
