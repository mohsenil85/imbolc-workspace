use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId};
use crate::ui::filterable_list::{FilterableItem, FilterableList};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, Rect, RenderBuf, Style,
};

struct CommandEntry {
    action: ActionId,
    description: String,
    keybinding: String,
}

impl FilterableItem for CommandEntry {
    fn primary_text(&self) -> &str {
        self.action.as_str()
    }
    fn secondary_text(&self) -> &str {
        &self.description
    }
}

pub struct CommandPalettePane {
    keymap: Keymap,
    list: FilterableList<CommandEntry>,
    pending_command: Option<ActionId>,
}

impl CommandPalettePane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            list: FilterableList::new(10),
            pending_command: None,
        }
    }

    /// Called before push to populate the palette with available commands.
    pub fn open(&mut self, commands: Vec<(ActionId, &'static str, String)>) {
        let entries = commands
            .into_iter()
            .map(|(action, desc, keybinding)| CommandEntry {
                action,
                description: desc.to_string(),
                keybinding,
            })
            .collect();
        self.list.set_items(entries);
        self.pending_command = None;
    }

    /// Called by main.rs after pop to get the confirmed command.
    pub fn take_command(&mut self) -> Option<ActionId> {
        self.pending_command.take()
    }
}

impl Pane for CommandPalettePane {
    fn id(&self) -> &'static str {
        "command_palette"
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
                    self.pending_command = Some(entry.action);
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
        buf.draw_line(
            Rect::new(inner.x, prompt_y, 2, 1),
            &[(": ", Style::new().fg(Color::CYAN).bold())],
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
            let cmd_idx = filtered[filter_idx];
            let entry = &items[cmd_idx];
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

            let w = inner.width as usize;
            let key_display = format!(" {} ", entry.keybinding);
            let key_len = key_display.len();
            let action_display = format!(" {}", entry.action.as_str());
            let desc_display = format!("  {}", entry.description);

            let remaining = w.saturating_sub(key_len);
            let action_len = action_display.len().min(remaining);
            let desc_remaining = remaining.saturating_sub(action_len);
            let desc_len = desc_display.len().min(desc_remaining);
            let pad_len = w.saturating_sub(action_len + desc_len + key_len);

            let padding = " ".repeat(pad_len);
            buf.draw_line(
                row_area,
                &[
                    (&action_display[..action_len], action_style),
                    (&desc_display[..desc_len], desc_style),
                    (&padding, desc_style),
                    (&key_display, key_style),
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
