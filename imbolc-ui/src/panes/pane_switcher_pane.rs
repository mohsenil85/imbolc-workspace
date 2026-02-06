use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::widgets::TextInput;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, Style};

/// Pane switcher entry: (pane_id, display_name, shortcut_hint)
type PaneEntry = (&'static str, &'static str, &'static str);

/// Available panes for the switcher
const PANE_ENTRIES: &[PaneEntry] = &[
    ("instrument_edit", "Instrument Editor", "F1"),
    ("piano_roll", "Piano Roll", "F2"),
    ("sequencer", "Drum Sequencer", "F2"),
    ("waveform", "Waveform Display", "F2"),
    ("track", "Track View", "F3"),
    ("mixer", "Mixer", "F4"),
    ("server", "Audio Server", "F5"),
    ("automation", "Automation", "F7"),
    ("eq", "Parametric EQ", "F8"),
    ("instrument", "Instrument List", "Ctrl+g"),
    ("sample_chopper", "Sample Chopper", ""),
    ("frame_edit", "Frame Settings", "Ctrl+f"),
    ("midi_settings", "MIDI Settings", "Ctrl+m"),
    ("vst_params", "VST Parameters", ""),
    ("file_browser", "File Browser", ""),
];

pub struct PaneSwitcherPane {
    keymap: Keymap,
    text_input: TextInput,
    /// Indices into `PANE_ENTRIES` matching current filter
    filtered: Vec<usize>,
    /// Index within `filtered`
    selected: usize,
    scroll: usize,
    /// The manually-typed prefix (separate from input which changes during tab cycling)
    filter_base: String,
    pending_pane: Option<&'static str>,
}

impl PaneSwitcherPane {
    pub fn new(keymap: Keymap) -> Self {
        let mut text_input = TextInput::new("");
        text_input.set_focused(true);
        let filtered: Vec<usize> = (0..PANE_ENTRIES.len()).collect();
        Self {
            keymap,
            text_input,
            filtered,
            selected: 0,
            scroll: 0,
            filter_base: String::new(),
            pending_pane: None,
        }
    }

    /// Called before push to prepare the switcher.
    pub fn open(&mut self) {
        self.text_input.set_value("");
        self.text_input.set_focused(true);
        self.filter_base.clear();
        self.pending_pane = None;
        self.selected = 0;
        self.scroll = 0;
        self.update_filter();
    }

    /// Called by main.rs after pop to get the selected pane.
    pub fn take_pane(&mut self) -> Option<&'static str> {
        self.pending_pane.take()
    }

    fn update_filter(&mut self) {
        let query = self.filter_base.to_lowercase();
        self.filtered = PANE_ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, (id, name, _))| {
                if query.is_empty() {
                    return true;
                }
                id.to_lowercase().contains(&query)
                    || name.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll = 0;
    }

    fn tab_complete(&mut self) {
        if self.filtered.is_empty() {
            return;
        }

        let input = self.text_input.value().to_string();

        // Find longest common prefix of all filtered pane names
        let first_name = PANE_ENTRIES[self.filtered[0]].1;
        let mut lcp = first_name.to_lowercase();
        for &idx in &self.filtered[1..] {
            let name = PANE_ENTRIES[idx].1.to_lowercase();
            lcp = longest_common_prefix(&lcp, &name);
            if lcp.is_empty() {
                break;
            }
        }

        let input_lower = input.to_lowercase();
        if lcp.len() > input_lower.len() && lcp.starts_with(&input_lower) {
            // LCP extends beyond current input — fill in LCP
            self.text_input.set_value(&lcp);
            self.filter_base = lcp;
            self.update_filter();
        } else if self.filtered.len() == 1 {
            // Single match — fill in completely
            let name = PANE_ENTRIES[self.filtered[0]].1.to_lowercase();
            self.text_input.set_value(&name);
            self.filter_base = name;
            self.update_filter();
        } else if self.filtered.len() > 1 {
            // Already at LCP and multiple matches — cycle selected
            self.selected = (self.selected + 1) % self.filtered.len();
            self.ensure_visible();
            let idx = self.filtered[self.selected];
            let name = PANE_ENTRIES[idx].1.to_lowercase();
            self.text_input.set_value(&name);
            // Don't change filter_base — keep showing all matches
        }
    }

    fn ensure_visible(&mut self) {
        let max_visible = 10;
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + max_visible {
            self.scroll = self.selected.saturating_sub(max_visible - 1);
        }
    }
}

fn longest_common_prefix(a: &str, b: &str) -> String {
    a.chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c)
        .collect()
}

impl Pane for PaneSwitcherPane {
    fn id(&self) -> &'static str {
        "pane_switcher"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::Mode(ModeActionId::PaletteConfirm) => {
                if !self.filtered.is_empty() {
                    let idx = self.filtered[self.selected];
                    self.pending_pane = Some(PANE_ENTRIES[idx].0);
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
            KeyCode::Tab => {
                self.tab_complete();
            }
            KeyCode::Up => {
                if !self.filtered.is_empty() {
                    if self.selected > 0 {
                        self.selected -= 1;
                    } else {
                        self.selected = self.filtered.len() - 1;
                    }
                    self.ensure_visible();
                    let idx = self.filtered[self.selected];
                    let name = PANE_ENTRIES[idx].1.to_lowercase();
                    self.text_input.set_value(&name);
                }
            }
            KeyCode::Down => {
                if !self.filtered.is_empty() {
                    self.selected = (self.selected + 1) % self.filtered.len();
                    self.ensure_visible();
                    let idx = self.filtered[self.selected];
                    let name = PANE_ENTRIES[idx].1.to_lowercase();
                    self.text_input.set_value(&name);
                }
            }
            _ => {
                // Delegate text editing to TextInput
                self.text_input.handle_input(event);
                self.filter_base = self.text_input.value().to_string();
                self.update_filter();
            }
        }
        Action::None
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let max_visible: usize = 10;
        let list_height = self.filtered.len().min(max_visible);
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
        buf.draw_line(Rect::new(inner.x, prompt_y, 2, 1), &[("; ", Style::new().fg(Color::MAGENTA).bold())]);

        // TextInput renders after the "; " prefix
        self.text_input.render_buf(buf.raw_buf(), inner.x + 2, prompt_y, inner.width.saturating_sub(2));

        // Divider
        if inner.height > 1 {
            let div_y = inner.y + 1;
            let divider = "\u{2500}".repeat(inner.width as usize);
            buf.draw_line(Rect::new(inner.x, div_y, inner.width, 1), &[(&divider, Style::new().fg(Color::DARK_GRAY))]);
        }

        // Filtered list
        let list_start_y = inner.y + 2;
        let available_rows = (inner.height as usize).saturating_sub(2);

        if self.filtered.is_empty() {
            if available_rows > 0 {
                let no_match_area = Rect::new(inner.x + 1, list_start_y, inner.width.saturating_sub(2), 1);
                buf.draw_line(no_match_area, &[("No matches", Style::new().fg(Color::DARK_GRAY))]);
            }
            return;
        }

        let visible_count = available_rows.min(self.filtered.len().saturating_sub(self.scroll));
        for row in 0..visible_count {
            let filter_idx = self.scroll + row;
            if filter_idx >= self.filtered.len() {
                break;
            }
            let pane_idx = self.filtered[filter_idx];
            let (_, ref name, ref shortcut) = PANE_ENTRIES[pane_idx];
            let y = list_start_y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = filter_idx == self.selected;
            let row_area = Rect::new(inner.x, y, inner.width, 1);

            // Clear row with selection bg if selected
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

            // Layout: " name     shortcut "
            let w = inner.width as usize;
            let shortcut_display = if shortcut.is_empty() {
                String::new()
            } else {
                format!(" {} ", shortcut)
            };
            let shortcut_len = shortcut_display.len();
            let name_display = format!(" {}", name);

            // Truncate if needed
            let remaining = w.saturating_sub(shortcut_len);
            let name_len = name_display.len().min(remaining);
            let pad_len = w.saturating_sub(name_len + shortcut_len);

            let padding = " ".repeat(pad_len);
            buf.draw_line(row_area, &[
                (&name_display[..name_len], name_style),
                (&padding, name_style),
                (&shortcut_display, shortcut_style),
            ]);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
