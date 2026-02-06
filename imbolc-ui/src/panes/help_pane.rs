use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, HelpActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, MouseEvent, MouseEventKind, MouseButton, NavAction, Pane, Style};

pub struct HelpPane {
    keymap: Keymap,
    /// The keymap to display (from another pane)
    display_keymap: Vec<(String, String)>, // (key, description)
    /// Pane to return to when closing help
    return_to: &'static str,
    /// Title showing which pane's help this is
    title: String,
    /// Scroll offset for long keymaps
    scroll: usize,
}

impl HelpPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            display_keymap: Vec::new(),
            return_to: "instrument",
            title: String::new(),
            scroll: 0,
        }
    }

    /// Set the keymap to display and the pane to return to
    pub fn set_context(&mut self, pane_id: &'static str, pane_title: &str, keymap: &Keymap) {
        self.return_to = pane_id;
        self.title = pane_title.to_string();
        self.scroll = 0;

        // Convert keymap bindings to display format
        self.display_keymap = keymap
            .bindings()
            .iter()
            .map(|b| (b.pattern.display(), b.description.to_string()))
            .collect();
    }
}

impl Default for HelpPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for HelpPane {
    fn id(&self) -> &'static str {
        "help"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::Help(HelpActionId::Close) => Action::Nav(NavAction::PopPane),
            ActionId::Help(HelpActionId::Up) => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
                Action::None
            }
            ActionId::Help(HelpActionId::Down) => {
                self.scroll += 1;
                Action::None
            }
            ActionId::Help(HelpActionId::Top) => {
                self.scroll = 0;
                Action::None
            }
            ActionId::Help(HelpActionId::Bottom) => {
                self.scroll = self.display_keymap.len().saturating_sub(1);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 60, 20);
        let title = format!(" Help: {} ", self.title);

        let border_style = Style::new().fg(Color::SKY_BLUE);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let visible_lines = inner.height.saturating_sub(4) as usize;
        let max_scroll = self.display_keymap.len().saturating_sub(visible_lines);
        let scroll = self.scroll.min(max_scroll);

        let key_style = Style::new().fg(Color::CYAN).bold();
        let desc_style = Style::new().fg(Color::WHITE);

        for (i, (key, desc)) in self.display_keymap.iter().skip(scroll).take(visible_lines).enumerate() {
            let y = inner.y + 1 + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let max_desc_len = inner.width.saturating_sub(14) as usize;
            let desc_truncated: String = desc.chars().take(max_desc_len).collect();
            let key_formatted = format!("{:<12}", key);

            let line_area = Rect::new(inner.x + 1, y, inner.width.saturating_sub(1), 1);
            buf.draw_line(line_area, &[
                (&key_formatted, key_style),
                (&desc_truncated, desc_style),
            ]);
        }

        // Scroll indicator
        if self.display_keymap.len() > visible_lines {
            let indicator_y = rect.y + rect.height - 3;
            if indicator_y < area.y + area.height {
                let indicator = format!(
                    "{}-{}/{}",
                    scroll + 1,
                    (scroll + visible_lines).min(self.display_keymap.len()),
                    self.display_keymap.len()
                );
                let ind_area = Rect::new(inner.x + 1, indicator_y, inner.width.saturating_sub(1), 1);
                buf.draw_line(ind_area, &[(&indicator, Style::new().fg(Color::DARK_GRAY))]);
            }
        }

        // Help text at bottom
        let help_y = rect.y + rect.height - 2;
        if help_y < area.y + area.height {
            let help_area = Rect::new(inner.x + 1, help_y, inner.width.saturating_sub(1), 1);
            buf.draw_line(help_area, &[
                ("[ESC/F1] Close  [Up/Down] Scroll", Style::new().fg(Color::DARK_GRAY)),
            ]);
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, _area: Rect, _state: &AppState) -> Action {
        match event.kind {
            MouseEventKind::ScrollUp => {
                if self.scroll > 0 { self.scroll -= 1; }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                self.scroll += 1;
                Action::None
            }
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Down(MouseButton::Right) => {
                // Click anywhere to close
                Action::Nav(NavAction::PopPane)
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
