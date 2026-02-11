use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as CrosstermKeyCode,
        KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        MouseButton as CrosstermMouseButton, MouseEvent as CrosstermMouseEvent,
        MouseEventKind as CrosstermMouseEventKind, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect as RatatuiRect,
    style::{Color as RatatuiColor, Style as RatatuiStyle},
    widgets::Widget,
    Terminal,
};

use super::{
    AppEvent, InputEvent, InputSource, KeyCode, Modifiers, MouseButton, MouseEvent, MouseEventKind,
};

/// Ratatui-based terminal backend
pub struct RatatuiBackend {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhancement_enabled: bool,
}

impl RatatuiBackend {
    /// Create a new ratatui backend (does not start terminal mode)
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            keyboard_enhancement_enabled: false,
        })
    }

    /// Enter raw mode and alternate screen with mouse capture
    pub fn start(&mut self) -> io::Result<()> {
        enable_raw_mode()?;

        // Check terminal support BEFORE entering alternate screen
        let supports_enhancement = matches!(supports_keyboard_enhancement(), Ok(true));

        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

        // Enable Kitty keyboard protocol if supported
        if supports_enhancement
            && execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )
            .is_ok()
        {
            self.keyboard_enhancement_enabled = true;
        }

        self.terminal.clear()?;
        Ok(())
    }

    /// Leave raw mode and alternate screen
    pub fn stop(&mut self) -> io::Result<()> {
        // Pop keyboard enhancement flags BEFORE leaving alternate screen
        if self.keyboard_enhancement_enabled {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
            self.keyboard_enhancement_enabled = false;
        }

        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Begin a new frame for drawing (black background)
    pub fn begin_frame(&self) -> io::Result<RatatuiFrame> {
        let size = self.terminal.size()?;
        let area = RatatuiRect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        // Fill entire buffer with black background
        let bg_style = RatatuiStyle::default().bg(RatatuiColor::Rgb(0, 0, 0));
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buffer.cell_mut((x, y)) {
                    cell.set_style(bg_style);
                }
            }
        }
        Ok(RatatuiFrame {
            buffer,
            size: (area.width, area.height),
        })
    }

    /// End the current frame and render to screen
    pub fn end_frame(&mut self, frame: RatatuiFrame) -> io::Result<()> {
        self.terminal.draw(|f| {
            let area = f.area();
            f.render_widget(BufferWidget(frame.buffer), area);
        })?;
        Ok(())
    }

    /// Clear the terminal screen (useful for recovering from display corruption)
    pub fn clear(&mut self) -> io::Result<()> {
        self.terminal.clear()
    }

    /// Whether the Kitty keyboard protocol was successfully enabled.
    pub fn keyboard_enhancement_enabled(&self) -> bool {
        self.keyboard_enhancement_enabled
    }
}

/// A frame for drawing operations
pub struct RatatuiFrame {
    buffer: Buffer,
    size: (u16, u16),
}

impl RatatuiFrame {
    /// Get mutable access to the underlying buffer
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Get the full terminal area as a ratatui Rect
    pub fn area(&self) -> RatatuiRect {
        RatatuiRect::new(0, 0, self.size.0, self.size.1)
    }
}

impl InputSource for RatatuiBackend {
    fn poll_event(&mut self, timeout: Duration) -> Option<AppEvent> {
        let mut t = timeout;
        loop {
            if !event::poll(t).ok()? {
                return None;
            }
            match event::read().ok()? {
                Event::Key(key_event) => {
                    // Skip Release events — we use timeout-based release detection
                    if key_event.kind == KeyEventKind::Release {
                        t = Duration::ZERO;
                        continue;
                    }
                    return Some(AppEvent::Key(convert_key_event(key_event)));
                }
                Event::Mouse(mouse_event) => {
                    if let Some(me) = convert_mouse_event(mouse_event) {
                        return Some(AppEvent::Mouse(me));
                    }
                    // Discarded mouse event (Moved, etc.) — drain with zero timeout
                    t = Duration::ZERO;
                }
                Event::Resize(w, h) => {
                    return Some(AppEvent::Resize(w, h));
                }
                _ => {
                    // Discarded event (FocusGained, etc.) — drain with zero timeout
                    t = Duration::ZERO;
                }
            }
        }
    }
}

fn convert_key_event(event: KeyEvent) -> InputEvent {
    use std::time::Instant;

    let key = match event.code {
        CrosstermKeyCode::Char(c) => KeyCode::Char(c),
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::Esc => KeyCode::Escape,
        CrosstermKeyCode::Backspace => KeyCode::Backspace,
        CrosstermKeyCode::Tab => KeyCode::Tab,
        CrosstermKeyCode::Up => KeyCode::Up,
        CrosstermKeyCode::Down => KeyCode::Down,
        CrosstermKeyCode::Left => KeyCode::Left,
        CrosstermKeyCode::Right => KeyCode::Right,
        CrosstermKeyCode::Home => KeyCode::Home,
        CrosstermKeyCode::End => KeyCode::End,
        CrosstermKeyCode::PageUp => KeyCode::PageUp,
        CrosstermKeyCode::PageDown => KeyCode::PageDown,
        CrosstermKeyCode::Insert => KeyCode::Insert,
        CrosstermKeyCode::Delete => KeyCode::Delete,
        CrosstermKeyCode::F(n) => KeyCode::F(n),
        _ => KeyCode::Char('\0'),
    };

    let modifiers = Modifiers {
        ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
        alt: event.modifiers.contains(KeyModifiers::ALT),
        shift: event.modifiers.contains(KeyModifiers::SHIFT),
    };

    let is_repeat = event.kind == KeyEventKind::Repeat;

    InputEvent {
        key,
        modifiers,
        timestamp: Instant::now(),
        is_repeat,
    }
}

fn convert_mouse_button(button: CrosstermMouseButton) -> MouseButton {
    match button {
        CrosstermMouseButton::Left => MouseButton::Left,
        CrosstermMouseButton::Right => MouseButton::Right,
        CrosstermMouseButton::Middle => MouseButton::Middle,
    }
}

fn convert_mouse_event(event: CrosstermMouseEvent) -> Option<MouseEvent> {
    let kind = match event.kind {
        CrosstermMouseEventKind::Down(btn) => MouseEventKind::Down(convert_mouse_button(btn)),
        CrosstermMouseEventKind::Up(btn) => MouseEventKind::Up(convert_mouse_button(btn)),
        CrosstermMouseEventKind::Drag(btn) => MouseEventKind::Drag(convert_mouse_button(btn)),
        CrosstermMouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        CrosstermMouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        _ => return None, // Ignore Moved and other events
    };

    let modifiers = Modifiers {
        ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
        alt: event.modifiers.contains(KeyModifiers::ALT),
        shift: event.modifiers.contains(KeyModifiers::SHIFT),
    };

    Some(MouseEvent {
        kind,
        column: event.column,
        row: event.row,
        modifiers,
    })
}

/// Widget that renders a pre-built buffer
struct BufferWidget(Buffer);

impl Widget for BufferWidget {
    fn render(self, area: RatatuiRect, buf: &mut Buffer) {
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                if x < self.0.area.width && y < self.0.area.height {
                    if let (Some(src), Some(dst)) = (self.0.cell((x, y)), buf.cell_mut((x, y))) {
                        *dst = src.clone();
                    }
                }
            }
        }
    }
}
