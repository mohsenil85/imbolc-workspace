use std::any::Any;
use std::path::PathBuf;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, ConfirmActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, Keymap, NavAction, Pane, Rect, RenderBuf, SessionAction, Style,
};

/// What to do when the user confirms the dialog
#[derive(Debug, Clone)]
#[allow(dead_code)] // NewProject and LoadFrom are wired in confirm_action() but not yet constructed externally
pub enum PendingAction {
    Quit,
    NewProject,
    LoadDefault,
    LoadFrom(PathBuf),
}

pub struct ConfirmPane {
    keymap: Keymap,
    message: String,
    pending: Option<PendingAction>,
    selected: bool, // false = No (cancel), true = Yes (confirm)
}

impl ConfirmPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            message: String::new(),
            pending: None,
            selected: false,
        }
    }

    /// Configure the dialog before showing it
    pub fn set_confirm(&mut self, message: &str, pending: PendingAction) {
        self.message = message.to_string();
        self.pending = Some(pending);
        self.selected = false;
    }

    fn confirm_action(&self) -> Action {
        match &self.pending {
            Some(PendingAction::Quit) => Action::Quit,
            Some(PendingAction::NewProject) => Action::Session(SessionAction::NewProject),
            Some(PendingAction::LoadDefault) => Action::Session(SessionAction::Load),
            Some(PendingAction::LoadFrom(path)) => {
                Action::Session(SessionAction::LoadFrom(path.clone()))
            }
            None => Action::Nav(NavAction::PopPane),
        }
    }
}

impl Pane for ConfirmPane {
    fn id(&self) -> &'static str {
        "confirm"
    }

    fn handle_action(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        _state: &AppState,
    ) -> Action {
        match action {
            ActionId::Confirm(ConfirmActionId::Cancel) => Action::Nav(NavAction::PopPane),
            ActionId::Confirm(ConfirmActionId::Confirm) => {
                if self.selected {
                    self.confirm_action()
                } else {
                    Action::Nav(NavAction::PopPane)
                }
            }
            ActionId::Confirm(ConfirmActionId::Left)
            | ActionId::Confirm(ConfirmActionId::Right)
            | ActionId::Confirm(ConfirmActionId::Toggle) => {
                self.selected = !self.selected;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match event.key {
            crate::ui::KeyCode::Char('y') | crate::ui::KeyCode::Char('Y') => self.confirm_action(),
            crate::ui::KeyCode::Char('n') | crate::ui::KeyCode::Char('N') => {
                Action::Nav(NavAction::PopPane)
            }
            crate::ui::KeyCode::Tab => {
                self.selected = !self.selected;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let width = (self.message.len() as u16 + 6)
            .max(30)
            .min(area.width.saturating_sub(4));
        let rect = center_rect(area, width, 7);

        let border_style = Style::new().fg(Color::YELLOW);
        let inner = buf.draw_block(rect, " Confirm ", border_style, border_style);

        // Message
        let msg_area = Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1);
        buf.draw_line(msg_area, &[(&self.message, Style::new().fg(Color::WHITE))]);

        // Buttons: [No]  [Yes]
        let no_style = if !self.selected {
            Style::new().fg(Color::BLACK).bg(Color::WHITE).bold()
        } else {
            Style::new().fg(Color::DARK_GRAY)
        };
        let yes_style = if self.selected {
            Style::new().fg(Color::BLACK).bg(Color::YELLOW).bold()
        } else {
            Style::new().fg(Color::DARK_GRAY)
        };

        let btn_y = inner.y + 3;
        if btn_y < inner.y + inner.height {
            let btn_area = Rect::new(inner.x + 1, btn_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(
                btn_area,
                &[
                    ("  [N]o  ", no_style),
                    ("    ", Style::new()),
                    ("  [Y]es  ", yes_style),
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
