use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use rat_event::{HandleEvent, Regular};
use rat_widget::focus::HasFocus;
use rat_widget::text_input::{TextInput as RatTextInput, TextInputState};

use crate::ui::input::InputEvent;
use crate::ui::rat_compat::{outcome_consumed, to_crossterm_key_event};
use crate::ui::style::Color;
use crate::ui::theme::DawTheme;

/// A single-line text input widget backed by rat-widget.
///
/// Preserves the same API as the previous hand-rolled implementation but
/// delegates text editing and rendering to `rat_widget::text_input`.
pub struct TextInput {
    /// Label shown before the input
    label: String,
    /// rat-widget state for editing + rendering
    state: TextInputState,
}

impl TextInput {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            state: TextInputState::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_placeholder(self, _placeholder: &str) -> Self {
        // rat-widget TextInput doesn't have a placeholder concept in 2.x;
        // accept the builder call but ignore it for compatibility
        self
    }

    #[allow(dead_code)]
    pub fn with_value(mut self, value: &str) -> Self {
        self.state.set_value(value);
        self
    }

    pub fn value(&self) -> &str {
        self.state.text()
    }

    pub fn set_value(&mut self, value: &str) {
        self.state.set_value(value);
    }

    /// Select all text — next typed character replaces everything
    pub fn select_all(&mut self) {
        self.state.select_all();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.state.focus.set(focused);
    }

    #[allow(dead_code)]
    pub fn is_focused(&self) -> bool {
        self.state.is_focused()
    }

    /// Handle input, returns true if the event was consumed
    pub fn handle_input(&mut self, event: &InputEvent) -> bool {
        if !self.state.is_focused() {
            return false;
        }
        let ct_event = to_crossterm_key_event(event);
        let outcome: rat_event::Outcome = self.state.handle(&ct_event, Regular).into();
        outcome_consumed(outcome)
    }

    /// Render the text input into a ratatui buffer at the given position.
    ///
    /// Renders the label prefix manually, then delegates to the rat-widget
    /// TextInput StatefulWidget for the actual input field.
    pub fn render_buf(&mut self, buf: &mut Buffer, x: u16, y: u16, width: u16) -> u16 {
        // Draw label manually
        let label_style = ratatui::style::Style::default()
            .fg(ratatui::style::Color::from(Color::WHITE));
        for (j, ch) in self.label.chars().enumerate() {
            if let Some(cell) = buf.cell_mut((x + j as u16, y)) {
                cell.set_char(ch).set_style(label_style);
            }
        }

        let label_offset = if self.label.is_empty() {
            0
        } else {
            self.label.len() as u16 + 1
        };
        let input_x = x + label_offset;
        let input_width = width.saturating_sub(label_offset);

        if input_width == 0 {
            return 1;
        }

        // Build the rat-widget TextInput with theme styles
        let widget = RatTextInput::new()
            .style(DawTheme::text_input_style())
            .focus_style(DawTheme::text_input_focus_style())
            .select_style(DawTheme::text_input_select_style())
            .cursor_style(DawTheme::text_input_cursor_style());

        let area = Rect::new(input_x, y, input_width, 1);
        widget.render(area, buf, &mut self.state);

        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::{KeyCode, Modifiers};

    fn char_event(ch: char) -> InputEvent {
        InputEvent::new(KeyCode::Char(ch), Modifiers::none())
    }

    fn key_event(key: KeyCode) -> InputEvent {
        InputEvent::new(key, Modifiers::none())
    }

    #[test]
    fn set_and_get_value() {
        let mut input = TextInput::new("Name:");
        input.set_value("hello");
        assert_eq!(input.value(), "hello");
    }

    #[test]
    fn with_value_builder() {
        let input = TextInput::new("").with_value("initial");
        assert_eq!(input.value(), "initial");
    }

    #[test]
    fn insert_char() {
        let mut input = TextInput::new("");
        input.set_focused(true);
        input.handle_input(&char_event('a'));
        input.handle_input(&char_event('b'));
        input.handle_input(&char_event('c'));
        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn backspace_deletes_char() {
        let mut input = TextInput::new("");
        input.set_focused(true);
        input.set_value("abc");
        // Move cursor to end
        input.handle_input(&key_event(KeyCode::End));
        input.handle_input(&key_event(KeyCode::Backspace));
        assert_eq!(input.value(), "ab");
    }

    #[test]
    fn delete_key() {
        let mut input = TextInput::new("");
        input.set_focused(true);
        input.set_value("abc");
        // Move cursor to start
        input.handle_input(&key_event(KeyCode::Home));
        input.handle_input(&key_event(KeyCode::Delete));
        assert_eq!(input.value(), "bc");
    }

    #[test]
    fn unfocused_input_does_not_consume() {
        let mut input = TextInput::new("");
        input.set_focused(false);
        let consumed = input.handle_input(&char_event('a'));
        assert!(!consumed);
        assert_eq!(input.value(), "");
    }

    #[test]
    fn select_all_and_type_replaces() {
        let mut input = TextInput::new("");
        input.set_focused(true);
        input.set_value("old text");
        input.select_all();
        input.handle_input(&char_event('n'));
        assert_eq!(input.value(), "n");
    }

    #[test]
    fn clear_value() {
        let mut input = TextInput::new("");
        input.set_value("something");
        input.set_value("");
        assert_eq!(input.value(), "");
    }
}
