//! Keyboard shortcut handling for the GUI.

use std::collections::HashMap;

use dioxus::prelude::*;

/// Modifier keys state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn from_keyboard_data(data: &dioxus::events::KeyboardData) -> Self {
        Self {
            ctrl: data.modifiers().ctrl(),
            shift: data.modifiers().shift(),
            alt: data.modifiers().alt(),
            meta: data.modifiers().meta(),
        }
    }
}

/// A key pattern for matching keyboard shortcuts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyPattern {
    pub key: String,
    pub modifiers: Modifiers,
}

impl KeyPattern {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_lowercase(),
            modifiers: Modifiers::default(),
        }
    }

    pub fn ctrl(mut self) -> Self {
        self.modifiers.ctrl = true;
        self
    }

    pub fn shift(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }

    #[allow(dead_code)]
    pub fn alt(mut self) -> Self {
        self.modifiers.alt = true;
        self
    }

    #[allow(dead_code)]
    pub fn meta(mut self) -> Self {
        self.modifiers.meta = true;
        self
    }
}

/// GUI actions that can be triggered by keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiAction {
    // Transport
    TogglePlay,
    Stop,
    Record,

    // Navigation
    FocusTrackList,
    FocusMixer,
    FocusArrangement,
    FocusDetail,

    // File
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,

    // Edit
    Undo,
    Redo,
    Delete,

    // Mixer
    ToggleMute,
    ToggleSolo,
}

/// Keybindings map.
pub struct Keybindings {
    bindings: HashMap<KeyPattern, GuiAction>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self::new()
    }
}

impl Keybindings {
    pub fn new() -> Self {
        let mut bindings = HashMap::new();

        // Transport
        bindings.insert(KeyPattern::new(" "), GuiAction::TogglePlay);
        bindings.insert(KeyPattern::new("escape"), GuiAction::Stop);
        bindings.insert(KeyPattern::new("r").ctrl(), GuiAction::Record);

        // Navigation (number keys)
        bindings.insert(KeyPattern::new("1"), GuiAction::FocusTrackList);
        bindings.insert(KeyPattern::new("2"), GuiAction::FocusMixer);
        bindings.insert(KeyPattern::new("3"), GuiAction::FocusArrangement);
        bindings.insert(KeyPattern::new("4"), GuiAction::FocusDetail);

        // File
        bindings.insert(KeyPattern::new("n").ctrl(), GuiAction::NewProject);
        bindings.insert(KeyPattern::new("o").ctrl(), GuiAction::OpenProject);
        bindings.insert(KeyPattern::new("s").ctrl(), GuiAction::SaveProject);
        bindings.insert(KeyPattern::new("s").ctrl().shift(), GuiAction::SaveProjectAs);

        // Edit
        bindings.insert(KeyPattern::new("z").ctrl(), GuiAction::Undo);
        bindings.insert(KeyPattern::new("z").ctrl().shift(), GuiAction::Redo);
        bindings.insert(KeyPattern::new("y").ctrl(), GuiAction::Redo);
        bindings.insert(KeyPattern::new("delete"), GuiAction::Delete);
        bindings.insert(KeyPattern::new("backspace"), GuiAction::Delete);

        // Mixer
        bindings.insert(KeyPattern::new("m"), GuiAction::ToggleMute);
        bindings.insert(KeyPattern::new("s"), GuiAction::ToggleSolo);

        Self { bindings }
    }

    /// Look up an action for a key event.
    pub fn lookup(&self, data: &dioxus::events::KeyboardData) -> Option<GuiAction> {
        let pattern = KeyPattern {
            key: data.key().to_string().to_lowercase(),
            modifiers: Modifiers::from_keyboard_data(data),
        };

        self.bindings.get(&pattern).copied()
    }

    /// Get a description of a keybinding for display.
    #[allow(dead_code)]
    pub fn describe(&self, action: GuiAction) -> Option<String> {
        for (pattern, act) in &self.bindings {
            if *act == action {
                let mut parts = Vec::new();
                if pattern.modifiers.ctrl {
                    parts.push("Ctrl");
                }
                if pattern.modifiers.shift {
                    parts.push("Shift");
                }
                if pattern.modifiers.alt {
                    parts.push("Alt");
                }
                if pattern.modifiers.meta {
                    parts.push("Cmd");
                }
                parts.push(&pattern.key);
                return Some(parts.join("+"));
            }
        }
        None
    }
}
