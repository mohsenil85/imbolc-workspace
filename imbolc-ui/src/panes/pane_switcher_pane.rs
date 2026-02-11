use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId};
use crate::ui::filterable_list::{FilterableItem, FilterableList};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, PaneId, Rect, RenderBuf, Style,
};

struct PaneEntry {
    id: PaneId,
    name: &'static str,
    shortcut: &'static str,
}

impl FilterableItem for PaneEntry {
    fn primary_text(&self) -> &str {
        self.id.as_str()
    }
    fn secondary_text(&self) -> &str {
        self.name
    }
    fn completion_text(&self) -> String {
        self.name.to_lowercase()
    }
}

/// Available panes for the switcher
const PANE_ENTRIES: &[(PaneId, &str, &str)] = &[
    (PaneId::InstrumentEdit, "Instrument Editor", "F1"),
    (PaneId::PianoRoll, "Piano Roll", "F2"),
    (PaneId::Sequencer, "Drum Sequencer", "F2"),
    (PaneId::Waveform, "Waveform Display", "F2"),
    (PaneId::Track, "Track View", "F3"),
    (PaneId::Mixer, "Mixer", "F4"),
    (PaneId::Server, "Audio Server", "F5"),
    (PaneId::Automation, "Automation", "F7"),
    (PaneId::Eq, "Parametric EQ", "F8"),
    (PaneId::Instrument, "Instrument List", "Ctrl+g"),
    (PaneId::SampleChopper, "Sample Chopper", ""),
    (PaneId::FrameEdit, "Frame Settings", "Ctrl+f"),
    (PaneId::MidiSettings, "MIDI Settings", "Ctrl+m"),
    (PaneId::VstParams, "VST Parameters", ""),
    (PaneId::FileBrowser, "File Browser", ""),
];

pub struct PaneSwitcherPane {
    keymap: Keymap,
    list: FilterableList<PaneEntry>,
    pending_pane: Option<PaneId>,
}

impl PaneSwitcherPane {
    pub fn new(keymap: Keymap) -> Self {
        let entries: Vec<PaneEntry> = PANE_ENTRIES
            .iter()
            .map(|&(id, name, shortcut)| PaneEntry { id, name, shortcut })
            .collect();
        let mut pane = Self {
            keymap,
            list: FilterableList::new(10),
            pending_pane: None,
        };
        pane.list.set_items(entries);
        pane
    }

    /// Called before push to prepare the switcher.
    pub fn open(&mut self) {
        let entries: Vec<PaneEntry> = PANE_ENTRIES
            .iter()
            .map(|&(id, name, shortcut)| PaneEntry { id, name, shortcut })
            .collect();
        self.list.set_items(entries);
        self.pending_pane = None;
    }

    /// Called by main.rs after pop to get the selected pane.
    pub fn take_pane(&mut self) -> Option<PaneId> {
        self.pending_pane.take()
    }
}

impl Pane for PaneSwitcherPane {
    fn id(&self) -> &'static str {
        "pane_switcher"
    }

    fn handle_action(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        _state: &AppState,
    ) -> Action {
        match action {
            ActionId::Mode(ModeActionId::PaletteConfirm) => {
                if let Some(entry) = self.list.selected_item() {
                    self.pending_pane = Some(entry.id);
                }
                Action::Nav(NavAction::PopPane)
            }
            ActionId::Mode(ModeActionId::PaletteCancel) => {
                self.pending_pane = None;
                Action::Nav(NavAction::PopPane)
            }
            _ => Action::None,
        }
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match event.key {
            KeyCode::Tab => self.list.tab_complete(),
            KeyCode::Up => self.list.move_up(),
            KeyCode::Down => self.list.move_down(),
            _ => self.list.handle_text_input(event),
        }
        Action::None
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let max_visible = self.list.max_visible();
        let list_height = self.list.filtered().len().min(max_visible);
        let total_height = (3 + list_height).max(5) as u16;
        let width = 50u16.min(area.width.saturating_sub(4));
        let rect = center_rect(area, width, total_height);

        // Clear background
        let bg_style = Style::new().bg(Color::new(20, 20, 30));
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                buf.set_cell(x, y, ' ', bg_style);
            }
        }

        let border_style = Style::new().fg(Color::MAGENTA);
        let inner = buf.draw_block(rect, " Switch Pane ", border_style, border_style);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Prompt line: render "; " prefix then TextInput
        let prompt_y = inner.y;
        buf.draw_line(
            Rect::new(inner.x, prompt_y, 2, 1),
            &[("; ", Style::new().fg(Color::MAGENTA).bold())],
        );
        self.list.text_input.render_buf(
            buf.raw_buf(),
            inner.x + 2,
            prompt_y,
            inner.width.saturating_sub(2),
        );

        // Divider
        if inner.height > 1 {
            let div_y = inner.y + 1;
            let divider = "\u{2500}".repeat(inner.width as usize);
            buf.draw_line(
                Rect::new(inner.x, div_y, inner.width, 1),
                &[(&divider, Style::new().fg(Color::DARK_GRAY))],
            );
        }

        // Filtered list
        let list_start_y = inner.y + 2;
        let available_rows = (inner.height as usize).saturating_sub(2);

        if self.list.filtered().is_empty() {
            if available_rows > 0 {
                let no_match_area =
                    Rect::new(inner.x + 1, list_start_y, inner.width.saturating_sub(2), 1);
                buf.draw_line(
                    no_match_area,
                    &[("No matches", Style::new().fg(Color::DARK_GRAY))],
                );
            }
            return;
        }

        let items = self.list.items();
        let filtered = self.list.filtered();
        let scroll = self.list.scroll();
        let selected = self.list.selected();
        let visible_count = available_rows.min(filtered.len().saturating_sub(scroll));
        for row in 0..visible_count {
            let filter_idx = scroll + row;
            if filter_idx >= filtered.len() {
                break;
            }
            let pane_idx = filtered[filter_idx];
            let entry = &items[pane_idx];
            let y = list_start_y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = filter_idx == selected;
            let row_area = Rect::new(inner.x, y, inner.width, 1);

            if is_selected {
                for x in row_area.x..row_area.x + row_area.width {
                    buf.set_cell(x, y, ' ', Style::new().bg(Color::SELECTION_BG));
                }
            }

            let name_style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::WHITE)
            };
            let shortcut_style = if is_selected {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };

            let w = inner.width as usize;
            let shortcut_display = if entry.shortcut.is_empty() {
                String::new()
            } else {
                format!(" {} ", entry.shortcut)
            };
            let shortcut_len = shortcut_display.len();
            let name_display = format!(" {}", entry.name);

            let remaining = w.saturating_sub(shortcut_len);
            let name_len = name_display.len().min(remaining);
            let pad_len = w.saturating_sub(name_len + shortcut_len);

            let padding = " ".repeat(pad_len);
            buf.draw_line(
                row_area,
                &[
                    (&name_display[..name_len], name_style),
                    (&padding, name_style),
                    (&shortcut_display, shortcut_style),
                ],
            );
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
