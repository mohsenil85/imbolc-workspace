use super::action_id::ActionId;
use super::{InputEvent, KeyCode};

/// Pattern for matching key inputs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPattern {
    /// Plain character key (no modifiers)
    Char(char),
    /// Special key (arrows, function keys, etc.)
    Key(KeyCode),
    /// Ctrl + character
    #[allow(dead_code)]
    Ctrl(char),
    /// Alt + character
    #[allow(dead_code)]
    Alt(char),
    /// Ctrl + special key
    #[allow(dead_code)]
    CtrlKey(KeyCode),
    /// Shift + special key (arrows, Tab, etc.)
    ShiftKey(KeyCode),
}

impl KeyPattern {
    /// Check if this pattern matches an input event
    pub fn matches(&self, event: &InputEvent) -> bool {
        match self {
            KeyPattern::Char(ch) => {
                matches!(event.key, KeyCode::Char(c) if c == *ch)
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
            }
            KeyPattern::Key(code) => {
                event.key == *code
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
                    && !event.modifiers.shift
            }
            KeyPattern::Ctrl(ch) => {
                matches!(event.key, KeyCode::Char(c) if c == *ch) && event.modifiers.ctrl
            }
            KeyPattern::Alt(ch) => {
                matches!(event.key, KeyCode::Char(c) if c == *ch) && event.modifiers.alt
            }
            KeyPattern::CtrlKey(code) => event.key == *code && event.modifiers.ctrl,
            KeyPattern::ShiftKey(code) => event.key == *code && event.modifiers.shift,
        }
    }

    /// Get a display string for this key pattern (for help screens)
    pub fn display(&self) -> String {
        match self {
            KeyPattern::Char(ch) => ch.to_string(),
            KeyPattern::Key(code) => format!("{:?}", code),
            KeyPattern::Ctrl(ch) => format!("Ctrl+{}", ch),
            KeyPattern::Alt(ch) => format!("Alt+{}", ch),
            KeyPattern::CtrlKey(code) => format!("Ctrl+{:?}", code),
            KeyPattern::ShiftKey(code) => format!("Shift+{:?}", code),
        }
    }
}

/// A single key binding
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub pattern: KeyPattern,
    pub action: ActionId,
    pub description: &'static str,
}

/// A collection of key bindings for a pane.
///
/// Available bind methods (builder pattern):
/// - `bind(char, action, desc)` — character key (no modifiers)
/// - `bind_key(KeyCode, action, desc)` — special key (arrows, F-keys, etc.)
/// - `bind_ctrl(char, action, desc)` — Ctrl + character
/// - `bind_alt(char, action, desc)` — Alt + character
/// - `bind_ctrl_key(KeyCode, action, desc)` — Ctrl + special key
///
/// Other methods:
/// - `lookup(&InputEvent) -> Option<&str>` — match an event to its action
/// - `bindings() -> &[KeyBinding]` — list all bindings (for help screens)
///
/// Use `bind_shift_key(KeyCode, action, desc)` for Shift + special key combos.
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    /// Create a new empty keymap
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Add a character key binding
    #[allow(dead_code)]
    pub fn bind(mut self, ch: char, action: ActionId, description: &'static str) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::Char(ch),
            action,
            description,
        });
        self
    }

    /// Add a special key binding
    #[allow(dead_code)]
    pub fn bind_key(mut self, key: KeyCode, action: ActionId, description: &'static str) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::Key(key),
            action,
            description,
        });
        self
    }

    /// Add a Ctrl+char binding
    #[allow(dead_code)]
    pub fn bind_ctrl(mut self, ch: char, action: ActionId, description: &'static str) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::Ctrl(ch),
            action,
            description,
        });
        self
    }

    /// Add an Alt+char binding
    #[allow(dead_code)]
    pub fn bind_alt(mut self, ch: char, action: ActionId, description: &'static str) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::Alt(ch),
            action,
            description,
        });
        self
    }

    /// Add a Ctrl+key binding
    #[allow(dead_code)]
    pub fn bind_ctrl_key(
        mut self,
        key: KeyCode,
        action: ActionId,
        description: &'static str,
    ) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::CtrlKey(key),
            action,
            description,
        });
        self
    }

    /// Add a Shift+key binding
    #[allow(dead_code)]
    pub fn bind_shift_key(
        mut self,
        key: KeyCode,
        action: ActionId,
        description: &'static str,
    ) -> Self {
        self.bindings.push(KeyBinding {
            pattern: KeyPattern::ShiftKey(key),
            action,
            description,
        });
        self
    }

    /// Create a keymap from a pre-built list of bindings
    pub fn from_bindings(bindings: Vec<KeyBinding>) -> Self {
        Self { bindings }
    }

    /// Look up the action for an input event
    pub fn lookup(&self, event: &InputEvent) -> Option<ActionId> {
        self.bindings
            .iter()
            .find(|b| b.pattern.matches(event))
            .map(|b| b.action)
    }

    /// Get all bindings (for help screens)
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }
}

#[cfg(test)]
mod tests {
    use super::super::action_id::{ActionId, GlobalActionId};
    use super::*;
    use crate::ui::Modifiers;

    #[test]
    fn test_char_pattern_matches() {
        let pattern = KeyPattern::Char('q');
        let event = InputEvent::new(KeyCode::Char('q'), Modifiers::none());
        assert!(pattern.matches(&event));

        let event_with_ctrl = InputEvent::new(KeyCode::Char('q'), Modifiers::ctrl());
        assert!(!pattern.matches(&event_with_ctrl));
    }

    #[test]
    fn test_ctrl_pattern_matches() {
        let pattern = KeyPattern::Ctrl('s');
        let event = InputEvent::new(KeyCode::Char('s'), Modifiers::ctrl());
        assert!(pattern.matches(&event));

        let event_no_ctrl = InputEvent::new(KeyCode::Char('s'), Modifiers::none());
        assert!(!pattern.matches(&event_no_ctrl));
    }

    #[test]
    fn test_keymap_lookup() {
        let keymap = Keymap::new()
            .bind('q', ActionId::Global(GlobalActionId::Quit), "Quit")
            .bind_ctrl('s', ActionId::Global(GlobalActionId::Save), "Save");

        let q_event = InputEvent::new(KeyCode::Char('q'), Modifiers::none());
        assert_eq!(
            keymap.lookup(&q_event),
            Some(ActionId::Global(GlobalActionId::Quit))
        );

        let ctrl_s_event = InputEvent::new(KeyCode::Char('s'), Modifiers::ctrl());
        assert_eq!(
            keymap.lookup(&ctrl_s_event),
            Some(ActionId::Global(GlobalActionId::Save))
        );

        let unknown_event = InputEvent::new(KeyCode::Char('x'), Modifiers::none());
        assert_eq!(keymap.lookup(&unknown_event), None);
    }
}
