use std::any::Any;

use crate::state::{AppState, InstrumentId};
use crate::ui::action_id::{ActionId, AddActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, Keymap, MouseButton, MouseEvent, MouseEventKind, NavAction, Pane,
    Rect, RenderBuf, SequencerAction, Style,
};

const LIST_HEIGHT: usize = 12;

/// Instrument picker for assigning instruments to drum pads.
/// Reads the target pad from DrumSequencerState.editing_pad.
pub struct InstrumentPickerPane {
    keymap: Keymap,
    selected: usize,
    scroll_offset: usize,
    /// Cached list of (instrument_id, name) pairs
    cached_instruments: Vec<(InstrumentId, String)>,
}

impl InstrumentPickerPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected: 0,
            scroll_offset: 0,
            cached_instruments: Vec::new(),
        }
    }

    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + LIST_HEIGHT {
            self.scroll_offset = self.selected.saturating_sub(LIST_HEIGHT - 1);
        }
    }

    fn update_instruments(&mut self, state: &AppState) {
        // Get the current instrument (the Kit) to exclude from the list
        let current_kit_id = state.instruments.selected_instrument().map(|i| i.id);

        // Build list of instruments that can be triggered (exclude the current Kit)
        self.cached_instruments = state
            .instruments
            .instruments
            .iter()
            .filter(|i| {
                // Exclude the current Kit instrument
                if Some(i.id) == current_kit_id {
                    return false;
                }
                // Exclude Kit instruments (can't trigger a Kit from another Kit)
                if i.drum_sequencer().is_some() {
                    return false;
                }
                // Exclude audio inputs and bus inputs
                if i.source.is_audio_input() || i.source.is_bus_in() {
                    return false;
                }
                true
            })
            .map(|i| (i.id, i.name.clone()))
            .collect();

        // Clamp selection
        if self.selected >= self.cached_instruments.len() {
            self.selected = self.cached_instruments.len().saturating_sub(1);
        }
        self.adjust_scroll();
    }

    fn select_next(&mut self) {
        if self.cached_instruments.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.cached_instruments.len();
        self.adjust_scroll();
    }

    fn select_prev(&mut self) {
        if self.cached_instruments.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.cached_instruments.len() - 1
        } else {
            self.selected - 1
        };
        self.adjust_scroll();
    }

    fn confirm_selection(&self, state: &AppState) -> Action {
        // Get the target pad from state
        let target_pad = state
            .instruments
            .selected_drum_sequencer()
            .and_then(|seq| seq.editing_pad);

        if let (Some(pad_idx), Some((instrument_id, _))) =
            (target_pad, self.cached_instruments.get(self.selected))
        {
            // Default trigger frequency of 440 Hz (A4)
            Action::Sequencer(SequencerAction::SetPadInstrument(
                pad_idx,
                *instrument_id,
                440.0,
            ))
        } else {
            Action::None
        }
    }
}

impl Default for InstrumentPickerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for InstrumentPickerPane {
    fn id(&self) -> &'static str {
        "instrument_picker"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::Add(AddActionId::Confirm) => self.confirm_selection(state),
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
        let rect = center_rect(area, 40, 18);
        let list_y = rect.y + 4;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = event.row;
                if row >= list_y && row < list_y + LIST_HEIGHT as u16 {
                    let visual_idx = (row - list_y) as usize;
                    let idx = visual_idx + self.scroll_offset;
                    if idx < self.cached_instruments.len() {
                        self.selected = idx;
                        self.adjust_scroll();
                        return self.confirm_selection(state);
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
        let rect = center_rect(area, 40, 18);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Assign Instrument ", border_style, border_style);

        let content_x = inner.x + 1;
        let content_y = inner.y + 1;

        // Get the pad number from state
        let pad_num = state
            .instruments
            .selected_drum_sequencer()
            .and_then(|seq| seq.editing_pad)
            .map(|p| p + 1)
            .unwrap_or(0);

        // Title
        let title = format!("Select instrument for Pad {}:", pad_num);
        buf.draw_line(
            Rect::new(content_x, content_y, inner.width.saturating_sub(2), 1),
            &[(&title, Style::new().fg(Color::CYAN).bold())],
        );

        let list_y = content_y + 2;
        let sel_bg = Style::new().bg(Color::SELECTION_BG);

        if self.cached_instruments.is_empty() {
            buf.draw_line(
                Rect::new(content_x, list_y, inner.width.saturating_sub(2), 1),
                &[(
                    "No instruments available.",
                    Style::new().fg(Color::DARK_GRAY),
                )],
            );
            buf.draw_line(
                Rect::new(content_x, list_y + 1, inner.width.saturating_sub(2), 1),
                &[(
                    "Add a synth instrument first.",
                    Style::new().fg(Color::DARK_GRAY),
                )],
            );
        } else {
            for (visual_i, i) in (self.scroll_offset..self.cached_instruments.len()).enumerate() {
                if visual_i >= LIST_HEIGHT {
                    break;
                }

                let (id, name) = &self.cached_instruments[i];
                let y = list_y + visual_i as u16;
                let is_selected = i == self.selected;

                // Find instrument to get source type
                let source_label = state
                    .instruments
                    .instrument(*id)
                    .map(|inst| format!("{:?}", inst.source))
                    .unwrap_or_default();

                if is_selected {
                    buf.set_cell(
                        content_x,
                        y,
                        '>',
                        Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold(),
                    );
                }

                let name_style = if is_selected {
                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::WHITE)
                };

                let source_style = if is_selected {
                    Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(Color::DARK_GRAY)
                };

                // Display name
                let display_name = if name.len() > 20 {
                    &name[..20]
                } else {
                    name.as_str()
                };
                buf.draw_line(
                    Rect::new(content_x + 2, y, 22, 1),
                    &[(display_name, name_style)],
                );

                // Display source type
                let source_display = format!(" ({})", source_label);
                let source_x = content_x + 2 + display_name.len() as u16;
                for (j, ch) in source_display.chars().enumerate() {
                    if source_x + (j as u16) < inner.x + inner.width - 1 {
                        buf.set_cell(source_x + (j as u16), y, ch, source_style);
                    }
                }

                if is_selected {
                    let fill_start = source_x + source_display.len() as u16;
                    let fill_end = inner.x + inner.width;
                    for x in fill_start..fill_end {
                        buf.set_cell(x, y, ' ', sel_bg);
                    }
                }
            }
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn on_enter(&mut self, state: &AppState) {
        self.update_instruments(state);
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
