use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

pub use ratatui::layout::Rect;

use super::style::Style;

/// Rendering abstraction layer.
///
/// Wraps a ratatui `Buffer` and accepts our `Style`/`Color` types natively.
/// Provides convenience methods for common rendering patterns. Use `raw_buf()`
/// as an escape hatch for code that still talks to ratatui directly.
pub struct RenderBuf<'a> {
    buf: &'a mut Buffer,
}

impl<'a> RenderBuf<'a> {
    pub fn new(buf: &'a mut Buffer) -> Self {
        Self { buf }
    }

    /// Set a single character at (x, y) with the given style.
    pub fn set_cell(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if let Some(cell) = self.buf.cell_mut((x, y)) {
            cell.set_char(ch)
                .set_style(ratatui::style::Style::from(style));
        }
    }

    /// Draw a string at (x, y) without wrapping. Characters beyond the buffer
    /// boundary are silently clipped.
    pub fn draw_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        let rat_style = ratatui::style::Style::from(style);
        for (i, ch) in text.chars().enumerate() {
            if let Some(cell) = self.buf.cell_mut((x + i as u16, y)) {
                cell.set_char(ch).set_style(rat_style);
            }
        }
    }

    /// Draw a bordered block with a title. Returns the inner `Rect`.
    pub fn draw_block(
        &mut self,
        area: Rect,
        title: &str,
        border_style: Style,
        title_style: Style,
    ) -> Rect {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(ratatui::style::Style::from(border_style))
            .title_style(ratatui::style::Style::from(title_style));
        let inner = block.inner(area);
        block.render(area, self.buf);
        inner
    }

    /// Draw styled spans on a single line within the given area.
    /// Replaces the `Paragraph::new(Line::from(Span::styled(...)))` pattern.
    pub fn draw_line(&mut self, area: Rect, spans: &[(&str, Style)]) {
        let rat_spans: Vec<Span> = spans
            .iter()
            .map(|(text, style)| Span::styled(*text, ratatui::style::Style::from(*style)))
            .collect();
        let line = Line::from(rat_spans);
        ratatui::widgets::Paragraph::new(line).render(area, self.buf);
    }

    /// Escape hatch: direct access to the underlying ratatui `Buffer`.
    /// Use this for code that hasn't been migrated yet, or for ratatui widgets
    /// that need a raw buffer reference.
    pub fn raw_buf(&mut self) -> &mut Buffer {
        self.buf
    }
}
