use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, HomeActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, MouseEvent, MouseEventKind, MouseButton, NavAction, Pane, Style};

/// Menu item for the home screen
struct MenuItem {
    label: &'static str,
    description: &'static str,
    pane_id: &'static str,
}

pub struct HomePane {
    keymap: Keymap,
    selected: usize,
    items: Vec<MenuItem>,
}

impl HomePane {
    pub fn new(keymap: Keymap) -> Self {
        let items = vec![
            MenuItem {
                label: "Instruments",
                description: "Instrument list - add and edit synths",
                pane_id: "instrument",
            },
            MenuItem {
                label: "Mixer",
                description: "Mixing console - adjust levels and routing",
                pane_id: "mixer",
            },
            MenuItem {
                label: "Server",
                description: "Audio server - start/stop and manage SuperCollider",
                pane_id: "server",
            },
        ];

        Self {
            keymap,
            selected: 0,
            items,
        }
    }
}

impl Default for HomePane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for HomePane {
    fn id(&self) -> &'static str {
        "home"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::Home(HomeActionId::Up) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            ActionId::Home(HomeActionId::Down) => {
                if self.selected < self.items.len() - 1 {
                    self.selected += 1;
                }
                Action::None
            }
            ActionId::Home(HomeActionId::Select) => Action::Nav(NavAction::SwitchPane(self.items[self.selected].pane_id)),
            ActionId::Home(HomeActionId::Quit) => Action::Quit,
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 50, 12);

        let border_style = Style::new().fg(Color::MAGENTA);
        let inner = buf.draw_block(rect, " IMBOLC ", border_style, border_style);

        let item_colors = [Color::CYAN, Color::PURPLE, Color::GOLD];

        for (i, item) in self.items.iter().enumerate() {
            let y = inner.y + 1 + (i as u16 * 2);
            let is_selected = i == self.selected;
            let item_color = item_colors.get(i).copied().unwrap_or(Color::WHITE);

            let label_text = format!(" [{}] {} ", i + 1, item.label);

            let label_style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(item_color)
            };

            let desc_style = if is_selected {
                Style::new().fg(Color::SKY_BLUE)
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let desc_text = format!("  {}", item.description);

            if y < inner.y + inner.height {
                let label_area = Rect::new(inner.x + 2, y, inner.width.saturating_sub(2), 1);
                buf.draw_line(label_area, &[(&label_text, label_style)]);
            }
            if y + 1 < inner.y + inner.height {
                let desc_area = Rect::new(inner.x + 2, y + 1, inner.width.saturating_sub(2), 1);
                buf.draw_line(desc_area, &[(&desc_text, desc_style)]);
            }
        }

        // Help text
        let help_y = rect.y + rect.height - 2;
        if help_y < area.y + area.height {
            let help_area = Rect::new(inner.x + 2, help_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(help_area, &[("[1-3] Jump  [Enter] Select  [q] Quit", Style::new().fg(Color::DARK_GRAY))]);
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, _state: &AppState) -> Action {
        let rect = center_rect(area, 50, 12);
        let inner_x = rect.x + 1;
        let inner_y = rect.y + 1;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let col = event.column;
                let row = event.row;
                // Each item occupies 2 rows, starting at inner_y + 1
                for (i, item) in self.items.iter().enumerate() {
                    let item_y = inner_y + 1 + (i as u16 * 2);
                    if col >= inner_x && row >= item_y && row <= item_y + 1 {
                        self.selected = i;
                        return Action::Nav(NavAction::SwitchPane(item.pane_id));
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                if self.selected > 0 { self.selected -= 1; }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                if self.selected < self.items.len() - 1 { self.selected += 1; }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
