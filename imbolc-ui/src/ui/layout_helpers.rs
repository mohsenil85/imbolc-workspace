use crate::ui::Rect;

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
