use std::any::Any;

use crate::state::AppState;
use crate::ui::action_id::ActionId;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, Style};

/// Which button is selected in the quit prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuitSelection {
    Save,
    DontSave,
    Cancel,
}

pub struct QuitPromptPane {
    keymap: Keymap,
    selected: QuitSelection,
}

impl QuitPromptPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected: QuitSelection::Save,
        }
    }
}

impl Pane for QuitPromptPane {
    fn id(&self) -> &'static str {
        "quit_prompt"
    }

    fn handle_action(&mut self, _action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        Action::None
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match event.key {
            KeyCode::Char('s') | KeyCode::Char('S') => Action::SaveAndQuit,
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Quit,
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Escape => {
                Action::Nav(NavAction::PopPane)
            }
            KeyCode::Enter => match self.selected {
                QuitSelection::Save => Action::SaveAndQuit,
                QuitSelection::DontSave => Action::Quit,
                QuitSelection::Cancel => Action::Nav(NavAction::PopPane),
            },
            KeyCode::Left => {
                self.selected = match self.selected {
                    QuitSelection::Save => QuitSelection::Save,
                    QuitSelection::DontSave => QuitSelection::Save,
                    QuitSelection::Cancel => QuitSelection::DontSave,
                };
                Action::None
            }
            KeyCode::Right => {
                self.selected = match self.selected {
                    QuitSelection::Save => QuitSelection::DontSave,
                    QuitSelection::DontSave => QuitSelection::Cancel,
                    QuitSelection::Cancel => QuitSelection::Cancel,
                };
                Action::None
            }
            KeyCode::Tab => {
                self.selected = match self.selected {
                    QuitSelection::Save => QuitSelection::DontSave,
                    QuitSelection::DontSave => QuitSelection::Cancel,
                    QuitSelection::Cancel => QuitSelection::Save,
                };
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let width = 44_u16.min(area.width.saturating_sub(4));
        let rect = center_rect(area, width, 7);

        let border_style = Style::new().fg(Color::YELLOW);
        let inner = buf.draw_block(rect, " Quit ", border_style, border_style);

        // Message
        let msg_area = Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1);
        buf.draw_line(msg_area, &[("Save changes before quitting?", Style::new().fg(Color::WHITE))]);

        // Buttons: [S]ave  [D]on't Save  [C]ancel
        let active_style = |sel: QuitSelection| {
            if self.selected == sel {
                Style::new().fg(Color::BLACK).bg(Color::YELLOW).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            }
        };

        let btn_y = inner.y + 3;
        if btn_y < inner.y + inner.height {
            let btn_area = Rect::new(inner.x + 1, btn_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(btn_area, &[
                (" [S]ave ", active_style(QuitSelection::Save)),
                ("  ", Style::new()),
                (" [D]on't Save ", active_style(QuitSelection::DontSave)),
                ("  ", Style::new()),
                (" [C]ancel ", active_style(QuitSelection::Cancel)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Modifiers;

    fn make_pane() -> QuitPromptPane {
        QuitPromptPane::new(Keymap::new())
    }

    fn key(code: KeyCode) -> InputEvent {
        InputEvent::new(code, Modifiers::none())
    }

    fn state() -> AppState {
        AppState::new()
    }

    #[test]
    fn d_key_returns_quit() {
        let mut pane = make_pane();
        let action = pane.handle_raw_input(&key(KeyCode::Char('D')), &state());
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn s_key_returns_save_and_quit() {
        let mut pane = make_pane();
        let action = pane.handle_raw_input(&key(KeyCode::Char('S')), &state());
        assert!(matches!(action, Action::SaveAndQuit));
    }

    #[test]
    fn escape_returns_pop_pane() {
        let mut pane = make_pane();
        let action = pane.handle_raw_input(&key(KeyCode::Escape), &state());
        assert!(matches!(action, Action::Nav(NavAction::PopPane)));
    }

    #[test]
    fn c_key_returns_pop_pane() {
        let mut pane = make_pane();
        let action = pane.handle_raw_input(&key(KeyCode::Char('c')), &state());
        assert!(matches!(action, Action::Nav(NavAction::PopPane)));
    }

    #[test]
    fn enter_on_dont_save_returns_quit() {
        let mut pane = make_pane();
        // Navigate to DontSave
        pane.handle_raw_input(&key(KeyCode::Right), &state());
        let action = pane.handle_raw_input(&key(KeyCode::Enter), &state());
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn enter_on_save_returns_save_and_quit() {
        let mut pane = make_pane();
        // Default selection is Save
        let action = pane.handle_raw_input(&key(KeyCode::Enter), &state());
        assert!(matches!(action, Action::SaveAndQuit));
    }

    #[test]
    fn enter_on_cancel_returns_pop_pane() {
        let mut pane = make_pane();
        // Navigate to Cancel
        pane.handle_raw_input(&key(KeyCode::Right), &state());
        pane.handle_raw_input(&key(KeyCode::Right), &state());
        let action = pane.handle_raw_input(&key(KeyCode::Enter), &state());
        assert!(matches!(action, Action::Nav(NavAction::PopPane)));
    }
}
