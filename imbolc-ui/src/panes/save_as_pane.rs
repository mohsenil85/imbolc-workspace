use std::any::Any;
use std::path::PathBuf;

use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::widgets::TextInput;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, KeyCode, Keymap, NavAction, Pane, SessionAction, Style};

pub struct SaveAsPane {
    keymap: Keymap,
    text_input: TextInput,
    error: Option<String>,
}

impl SaveAsPane {
    pub fn new(keymap: Keymap) -> Self {
        let mut text_input = TextInput::new("");
        text_input.set_focused(true);
        Self {
            keymap,
            text_input,
            error: None,
        }
    }

    /// Reset state when opening
    pub fn reset(&mut self, default_name: &str) {
        self.text_input.set_value(default_name);
        self.text_input.select_all();
        self.text_input.set_focused(true);
        self.error = None;
    }

    fn projects_dir() -> PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("imbolc")
                .join("projects")
        } else {
            PathBuf::from("projects")
        }
    }
}

impl Pane for SaveAsPane {
    fn id(&self) -> &'static str {
        "save_as"
    }

    fn handle_action(&mut self, _action: crate::ui::action_id::ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        Action::None
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match event.key {
            KeyCode::Enter => {
                let name = self.text_input.value().trim().to_string();
                if name.is_empty() {
                    self.error = Some("Name cannot be empty".to_string());
                    return Action::None;
                }

                let dir = Self::projects_dir();
                let path = dir.join(format!("{}.sqlite", name));
                Action::Session(SessionAction::SaveAs(path))
            }
            KeyCode::Escape => {
                Action::Nav(NavAction::PopPane)
            }
            _ => {
                // Delegate text editing to rat-widget TextInput
                self.text_input.handle_input(event);
                self.error = None;
                Action::None
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let width = 46_u16.min(area.width.saturating_sub(4));
        let height = if self.error.is_some() { 8 } else { 7 };
        let rect = center_rect(area, width, height);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Save As ", border_style, border_style);

        // Label
        let label_area = Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1);
        buf.draw_line(label_area, &[("Project name:", Style::new().fg(Color::DARK_GRAY))]);

        // Text input field (rat-widget backed)
        let field_y = inner.y + 2;
        let field_width = inner.width.saturating_sub(2);
        self.text_input.render_buf(buf.raw_buf(), inner.x + 1, field_y, field_width);

        // Error message
        if let Some(ref error) = self.error {
            let err_y = inner.y + 3;
            if err_y < inner.y + inner.height {
                let err_area = Rect::new(inner.x + 1, err_y, inner.width.saturating_sub(2), 1);
                buf.draw_line(err_area, &[(error.as_str(), Style::new().fg(Color::MUTE_COLOR))]);
            }
        }

        // Footer
        let footer_y = rect.y + rect.height.saturating_sub(2);
        if footer_y < area.y + area.height {
            let footer_area = Rect::new(inner.x + 1, footer_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(footer_area, &[("[Enter] Save  [Esc] Cancel", Style::new().fg(Color::DARK_GRAY))]);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
