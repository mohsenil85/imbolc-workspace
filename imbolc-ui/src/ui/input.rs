use std::time::{Duration, Instant};

/// Mouse button identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Types of mouse events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    ScrollUp,
    ScrollDown,
}

/// Mouse event with position and type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}

/// Top-level input event: keyboard, mouse, or resize
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Key(InputEvent),
    Mouse(MouseEvent),
    #[allow(dead_code)]
    Resize(u16, u16),
}

/// Key codes for keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
}

/// Modifier key state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    #[allow(dead_code)]
    pub const fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    #[allow(dead_code)]
    pub const fn ctrl() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }
}

/// Input event from the user
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub key: KeyCode,
    pub modifiers: Modifiers,
    pub timestamp: Instant,
    pub is_repeat: bool,
}

impl PartialEq for InputEvent {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.modifiers == other.modifiers
    }
}

impl Eq for InputEvent {}

impl InputEvent {
    pub fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers, timestamp: Instant::now(), is_repeat: false }
    }

    #[allow(dead_code)]
    pub fn key(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::none(),
            timestamp: Instant::now(),
            is_repeat: false,
        }
    }

    /// Check if this is a specific character without modifiers
    #[allow(dead_code)]
    pub fn is_char(&self, ch: char) -> bool {
        matches!(self.key, KeyCode::Char(c) if c == ch)
            && !self.modifiers.ctrl
            && !self.modifiers.alt
    }
}

/// Trait for reading input events
pub trait InputSource {
    /// Poll for an input event with a timeout
    /// Returns None if no event is available within the timeout
    fn poll_event(&mut self, timeout: Duration) -> Option<AppEvent>;
}
