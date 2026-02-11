use std::any::Any;

use imbolc_core::state::persistence::checkpoint;
use imbolc_core::state::persistence::CheckpointInfo;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, CheckpointListActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, Keymap, NavAction, Pane, Rect, RenderBuf, SessionAction, Style,
};

pub struct CheckpointListPane {
    keymap: Keymap,
    checkpoints: Vec<CheckpointInfo>,
    selected: usize,
}

impl CheckpointListPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            checkpoints: Vec::new(),
            selected: 0,
        }
    }

    fn refresh(&mut self, state: &AppState) {
        let default_path = imbolc_core::dispatch::default_rack_path();
        let path = state.project.path.as_deref().unwrap_or(&default_path);
        self.checkpoints = checkpoint::list_checkpoints(path).unwrap_or_default();
        if self.selected >= self.checkpoints.len() {
            self.selected = self.checkpoints.len().saturating_sub(1);
        }
    }

    fn format_time_ago(created_at: &str) -> String {
        // created_at is SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
        // Just display the raw timestamp for simplicity
        created_at.to_string()
    }
}

impl Pane for CheckpointListPane {
    fn id(&self) -> &'static str {
        "checkpoint_list"
    }

    fn on_enter(&mut self, state: &AppState) {
        self.refresh(state);
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::CheckpointList(CheckpointListActionId::Close) => {
                Action::Nav(NavAction::PopPane)
            }
            ActionId::CheckpointList(CheckpointListActionId::Up) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            ActionId::CheckpointList(CheckpointListActionId::Down) => {
                if self.selected + 1 < self.checkpoints.len() {
                    self.selected += 1;
                }
                Action::None
            }
            ActionId::CheckpointList(CheckpointListActionId::Select) => {
                if let Some(cp) = self.checkpoints.get(self.selected) {
                    return Action::Session(SessionAction::RestoreCheckpoint(cp.id));
                }
                Action::None
            }
            ActionId::CheckpointList(CheckpointListActionId::Delete) => {
                if let Some(cp) = self.checkpoints.get(self.selected) {
                    let default_path = imbolc_core::dispatch::default_rack_path();
                    let path = state.project.path.as_deref().unwrap_or(&default_path);
                    let _ = checkpoint::delete_checkpoint(path, cp.id);
                    self.refresh(state);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let width = 52_u16.min(area.width.saturating_sub(4));
        let height = (self.checkpoints.len() as u16 + 8)
            .min(area.height.saturating_sub(4))
            .max(10);
        let rect = center_rect(area, width, height);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Checkpoints ", border_style, border_style);

        // Section header
        let header_area = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1);
        buf.draw_line(
            header_area,
            &[("Saved Checkpoints", Style::new().fg(Color::DARK_GRAY))],
        );

        if self.checkpoints.is_empty() {
            let empty_y = inner.y + 2;
            if empty_y < inner.y + inner.height {
                let empty_area = Rect::new(inner.x + 1, empty_y, inner.width.saturating_sub(2), 1);
                buf.draw_line(
                    empty_area,
                    &[("No checkpoints", Style::new().fg(Color::DARK_GRAY))],
                );
            }
        }

        // Checkpoint list
        let max_visible = (inner.height.saturating_sub(4)) as usize;
        let scroll = if self.selected >= max_visible {
            self.selected - max_visible + 1
        } else {
            0
        };

        for (i, cp) in self
            .checkpoints
            .iter()
            .skip(scroll)
            .take(max_visible)
            .enumerate()
        {
            let y = inner.y + 2 + i as u16;
            if y >= inner.y + inner.height.saturating_sub(2) {
                break;
            }

            let is_selected = scroll + i == self.selected;
            let time_str = Self::format_time_ago(&cp.created_at);

            let name_max = inner.width.saturating_sub(time_str.len() as u16 + 6) as usize;
            let display_name: String = cp.label.chars().take(name_max).collect();

            let (name_style, time_style) = if is_selected {
                (
                    Style::new().fg(Color::BLACK).bg(Color::CYAN).bold(),
                    Style::new().fg(Color::BLACK).bg(Color::CYAN),
                )
            } else {
                (
                    Style::new().fg(Color::WHITE),
                    Style::new().fg(Color::DARK_GRAY),
                )
            };

            // Clear the line for selected item
            if is_selected {
                for x in (inner.x + 1)..(inner.x + 1 + inner.width.saturating_sub(2)) {
                    buf.set_cell(
                        x,
                        y,
                        ' ',
                        Style::new().fg(Color::BLACK).bg(Color::CYAN).bold(),
                    );
                }
            }

            let prefix = if is_selected { " > " } else { "   " };
            let padding_len = name_max.saturating_sub(display_name.len());
            let padding: String = " ".repeat(padding_len);
            let time_col = format!("  {}", time_str);

            let line_area = Rect::new(inner.x, y, inner.width, 1);
            buf.draw_line(
                line_area,
                &[
                    (prefix, name_style),
                    (&display_name, name_style),
                    (&padding, name_style),
                    (&time_col, time_style),
                ],
            );
        }

        // Footer
        let footer_y = rect.y + rect.height.saturating_sub(2);
        if footer_y < area.y + area.height {
            let hi = Style::new().fg(Color::CYAN).bold();
            let lo = Style::new().fg(Color::DARK_GRAY);
            let footer_area = Rect::new(inner.x + 1, footer_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(
                footer_area,
                &[
                    ("[Enter]", hi),
                    (" Restore  ", lo),
                    ("[D]", hi),
                    ("elete  ", lo),
                    ("[Esc]", hi),
                    (" Close", lo),
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
