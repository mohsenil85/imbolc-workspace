//! Adapter layer between our custom InputEvent/MouseEvent and crossterm events.
//!
//! rat-widget's HandleEvent trait expects raw crossterm::event::Event values.
//! This module converts back from our abstracted types at the point of consumption.

use crossterm::event::{
    Event as CtEvent, KeyCode as CtKeyCode, KeyEvent as CtKeyEvent,
    KeyEventKind, KeyEventState, KeyModifiers,
    MouseButton as CtMouseButton, MouseEvent as CtMouseEvent,
    MouseEventKind as CtMouseEventKind,
};
use rat_event::Outcome;

use super::input::{InputEvent, KeyCode, MouseButton, MouseEvent, MouseEventKind};

/// Convert our InputEvent back to a crossterm Event for rat-widget consumption.
pub fn to_crossterm_key_event(event: &InputEvent) -> CtEvent {
    let code = match event.key {
        KeyCode::Char(c) => CtKeyCode::Char(c),
        KeyCode::Enter => CtKeyCode::Enter,
        KeyCode::Escape => CtKeyCode::Esc,
        KeyCode::Backspace => CtKeyCode::Backspace,
        KeyCode::Tab => CtKeyCode::Tab,
        KeyCode::Up => CtKeyCode::Up,
        KeyCode::Down => CtKeyCode::Down,
        KeyCode::Left => CtKeyCode::Left,
        KeyCode::Right => CtKeyCode::Right,
        KeyCode::Home => CtKeyCode::Home,
        KeyCode::End => CtKeyCode::End,
        KeyCode::PageUp => CtKeyCode::PageUp,
        KeyCode::PageDown => CtKeyCode::PageDown,
        KeyCode::Insert => CtKeyCode::Insert,
        KeyCode::Delete => CtKeyCode::Delete,
        KeyCode::F(n) => CtKeyCode::F(n),
    };

    let mut modifiers = KeyModifiers::empty();
    if event.modifiers.ctrl {
        modifiers |= KeyModifiers::CONTROL;
    }
    if event.modifiers.alt {
        modifiers |= KeyModifiers::ALT;
    }
    if event.modifiers.shift {
        modifiers |= KeyModifiers::SHIFT;
    }

    CtEvent::Key(CtKeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// Convert our MouseEvent to a crossterm Event for rat-widget consumption.
#[allow(dead_code)]
pub fn to_crossterm_mouse_event(event: &MouseEvent) -> CtEvent {
    let convert_button = |btn: MouseButton| match btn {
        MouseButton::Left => CtMouseButton::Left,
        MouseButton::Right => CtMouseButton::Right,
        MouseButton::Middle => CtMouseButton::Middle,
    };

    let mut modifiers = KeyModifiers::empty();
    if event.modifiers.ctrl {
        modifiers |= KeyModifiers::CONTROL;
    }
    if event.modifiers.alt {
        modifiers |= KeyModifiers::ALT;
    }
    if event.modifiers.shift {
        modifiers |= KeyModifiers::SHIFT;
    }

    let kind = match event.kind {
        MouseEventKind::Down(btn) => CtMouseEventKind::Down(convert_button(btn)),
        MouseEventKind::Up(btn) => CtMouseEventKind::Up(convert_button(btn)),
        MouseEventKind::Drag(btn) => CtMouseEventKind::Drag(convert_button(btn)),
        MouseEventKind::ScrollUp => CtMouseEventKind::ScrollUp,
        MouseEventKind::ScrollDown => CtMouseEventKind::ScrollDown,
    };

    CtEvent::Mouse(CtMouseEvent {
        kind,
        column: event.column,
        row: event.row,
        modifiers,
    })
}

/// Convert a rat-event Outcome to a bool indicating whether the event was consumed.
pub fn outcome_consumed(outcome: Outcome) -> bool {
    matches!(outcome, Outcome::Changed | Outcome::Unchanged)
}
