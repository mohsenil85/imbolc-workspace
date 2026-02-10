use crate::ui::style::{Color, Style};
use crate::ui::{Rect, RenderBuf};

/// Center a rect of `width x height` within the given `area`.
/// Clamps dimensions to available space with padding to prevent overflow.
pub fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    // Clamp to available size with padding
    let max_w = area.width.saturating_sub(2);
    let max_h = area.height.saturating_sub(2);
    let w = width.min(max_w);
    let h = height.min(max_h);

    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Render a centered dialog frame with border and title, return inner area.
/// Combines center_rect and draw_block into a single helper for modal dialogs.
#[allow(dead_code)]
pub fn render_dialog_frame(
    area: Rect,
    buf: &mut RenderBuf,
    title: &str,
    width: u16,
    height: u16,
    border_color: Color,
) -> Rect {
    let rect = center_rect(area, width, height);
    let border_style = Style::new().fg(border_color);
    buf.draw_block(rect, title, border_style, border_style)
}
