use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::widgets::TextInput;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, Style};

pub struct CommandPalettePane {
    keymap: Keymap,
    /// (action, description, keybinding display)
    commands: Vec<(ActionId, String, String)>,
    text_input: TextInput,
    /// Indices into `commands` matching current filter
    filtered: Vec<usize>,
    /// Index within `filtered`
    selected: usize,
    scroll: usize,
    /// The manually-typed prefix (separate from input which changes during tab cycling)
    filter_base: String,
    pending_command: Option<ActionId>,
}

impl CommandPalettePane {
    pub fn new(keymap: Keymap) -> Self {
        let mut text_input = TextInput::new("");
        text_input.set_focused(true);
        Self {
            keymap,
            commands: Vec::new(),
            text_input,
            filtered: Vec::new(),
            selected: 0,
            scroll: 0,
            filter_base: String::new(),
            pending_command: None,
        }
    }

    /// Called before push to populate the palette with available commands.
    pub fn open(&mut self, commands: Vec<(ActionId, &'static str, String)>) {
        self.commands = commands
            .into_iter()
            .map(|(a, d, k)| (a, d.to_string(), k))
            .collect();
        self.text_input.set_value("");
        self.text_input.set_focused(true);
        self.filter_base.clear();
        self.pending_command = None;
        self.selected = 0;
        self.scroll = 0;
        self.update_filter();
    }

    /// Called by main.rs after pop to get the confirmed command.
    pub fn take_command(&mut self) -> Option<ActionId> {
        self.pending_command.take()
    }

    fn update_filter(&mut self) {
        let query = self.filter_base.to_lowercase();
        self.filtered = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, (action, desc, _))| {
                if query.is_empty() {
                    return true;
                }
                action.as_str().to_lowercase().contains(&query)
                    || desc.to_lowercase().contains(&query)
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

        // Find longest common prefix of all filtered action names
        let first_action_str = self.commands[self.filtered[0]].0.as_str();
        let mut lcp = first_action_str.to_string();
        for &idx in &self.filtered[1..] {
            let action_str = self.commands[idx].0.as_str();
            lcp = longest_common_prefix(&lcp, action_str);
            if lcp.is_empty() {
                break;
            }
        }

        if lcp.len() > input.len() && lcp.starts_with(&input) {
            // LCP extends beyond current input — fill in LCP
            self.text_input.set_value(&lcp);
            self.filter_base = lcp;
            self.update_filter();
        } else if self.filtered.len() == 1 {
            // Single match — fill in completely
            let action_str = self.commands[self.filtered[0]].0.as_str();
            self.text_input.set_value(action_str);
            self.filter_base = action_str.to_string();
            self.update_filter();
        } else if self.filtered.len() > 1 {
            // Already at LCP and multiple matches — cycle selected
            self.selected = (self.selected + 1) % self.filtered.len();
            self.ensure_visible();
            let idx = self.filtered[self.selected];
            let action_str = self.commands[idx].0.as_str();
            self.text_input.set_value(action_str);
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

impl Pane for CommandPalettePane {
    fn id(&self) -> &'static str {
        "command_palette"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::Mode(ModeActionId::PaletteConfirm) => {
                if !self.filtered.is_empty() {
                    let idx = self.filtered[self.selected];
                    self.pending_command = Some(self.commands[idx].0);
                }
                Action::Nav(NavAction::PopPane)
            }
            ActionId::Mode(ModeActionId::PaletteCancel) => {
                self.pending_command = None;
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
                    let action_str = self.commands[idx].0.as_str();
                    self.text_input.set_value(action_str);
                }
            }
            KeyCode::Down => {
                if !self.filtered.is_empty() {
                    self.selected = (self.selected + 1) % self.filtered.len();
                    self.ensure_visible();
                    let idx = self.filtered[self.selected];
                    let action_str = self.commands[idx].0.as_str();
                    self.text_input.set_value(action_str);
                }
            }
            _ => {
                // Delegate text editing to rat-widget TextInput
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
        let width = 60u16.min(area.width.saturating_sub(4));
        let rect = center_rect(area, width, total_height);

        // Clear background
        let bg_style = Style::new().bg(Color::new(20, 20, 30));
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                buf.set_cell(x, y, ' ', bg_style);
            }
        }

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Command Palette ", border_style, border_style);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Prompt line: render ": " prefix then TextInput
        let prompt_y = inner.y;
        buf.draw_line(Rect::new(inner.x, prompt_y, 2, 1), &[(": ", Style::new().fg(Color::CYAN).bold())]);

        // TextInput renders after the ": " prefix
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
            let cmd_idx = self.filtered[filter_idx];
            let (ref action, ref desc, ref key) = self.commands[cmd_idx];
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

            let action_style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::WHITE)
            };
            let desc_style = if is_selected {
                Style::new().fg(Color::GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::GRAY)
            };
            let key_style = if is_selected {
                Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };

            // Layout: " action_name  description     keybinding "
            let w = inner.width as usize;
            let key_display = format!(" {} ", key);
            let key_len = key_display.len();
            let action_display = format!(" {}", action.as_str());
            let desc_display = format!("  {}", desc);

            // Truncate if needed
            let remaining = w.saturating_sub(key_len);
            let action_len = action_display.len().min(remaining);
            let desc_remaining = remaining.saturating_sub(action_len);
            let desc_len = desc_display.len().min(desc_remaining);
            let pad_len = w.saturating_sub(action_len + desc_len + key_len);

            let padding = " ".repeat(pad_len);
            buf.draw_line(row_area, &[
                (&action_display[..action_len], action_style),
                (&desc_display[..desc_len], desc_style),
                (&padding, desc_style),
                (&key_display, key_style),
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
