//! Reusable list navigation state for panes with scrolling lists.

/// Reusable list navigation state.
/// Handles selection, scrolling, and navigation with optional item skipping.
#[derive(Debug, Clone, Default)]
pub struct ListSelector {
    pub selected: usize,
    pub scroll_offset: usize,
}

impl ListSelector {
    pub fn new(initial: usize) -> Self {
        Self {
            selected: initial,
            scroll_offset: 0,
        }
    }

    /// Move to next item, optionally skipping items that match the predicate.
    /// Wraps around to the beginning when reaching the end.
    pub fn select_next<F>(&mut self, len: usize, skip: F)
    where
        F: Fn(usize) -> bool,
    {
        if len == 0 {
            return;
        }
        let mut next = (self.selected + 1) % len;
        // Prevent infinite loop if all items should be skipped
        let start = next;
        while skip(next) {
            next = (next + 1) % len;
            if next == start {
                break;
            }
        }
        self.selected = next;
    }

    /// Move to previous item, optionally skipping items that match the predicate.
    /// Wraps around to the end when reaching the beginning.
    pub fn select_prev<F>(&mut self, len: usize, skip: F)
    where
        F: Fn(usize) -> bool,
    {
        if len == 0 {
            return;
        }
        let mut prev = if self.selected == 0 {
            len - 1
        } else {
            self.selected - 1
        };
        // Prevent infinite loop if all items should be skipped
        let start = prev;
        while skip(prev) {
            prev = if prev == 0 { len - 1 } else { prev - 1 };
            if prev == start {
                break;
            }
        }
        self.selected = prev;
    }

    /// Adjust scroll_offset to keep the selected item visible.
    pub fn adjust_scroll(&mut self, visible_rows: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if visible_rows > 0 && self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected - visible_rows + 1;
        }
    }

    /// Clamp selection to valid bounds. Call this after the item list changes.
    pub fn clamp(&mut self, len: usize) {
        if len > 0 && self.selected >= len {
            self.selected = len - 1;
        }
    }

    /// Reset selection and scroll to beginning.
    pub fn reset(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Combined helper: select next, then adjust scroll.
    pub fn next_and_scroll<F>(&mut self, len: usize, visible_rows: usize, skip: F)
    where
        F: Fn(usize) -> bool,
    {
        self.select_next(len, skip);
        self.adjust_scroll(visible_rows);
    }

    /// Combined helper: select prev, then adjust scroll.
    pub fn prev_and_scroll<F>(&mut self, len: usize, visible_rows: usize, skip: F)
    where
        F: Fn(usize) -> bool,
    {
        self.select_prev(len, skip);
        self.adjust_scroll(visible_rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_next_wraps() {
        let mut sel = ListSelector::new(2);
        sel.select_next(3, |_| false);
        assert_eq!(sel.selected, 0);
    }

    #[test]
    fn select_prev_wraps() {
        let mut sel = ListSelector::new(0);
        sel.select_prev(3, |_| false);
        assert_eq!(sel.selected, 2);
    }

    #[test]
    fn select_next_skips() {
        let mut sel = ListSelector::new(0);
        // Skip index 1
        sel.select_next(3, |i| i == 1);
        assert_eq!(sel.selected, 2);
    }

    #[test]
    fn select_prev_skips() {
        let mut sel = ListSelector::new(2);
        // Skip index 1
        sel.select_prev(3, |i| i == 1);
        assert_eq!(sel.selected, 0);
    }

    #[test]
    fn adjust_scroll_keeps_visible() {
        let mut sel = ListSelector::new(10);
        sel.scroll_offset = 0;
        sel.adjust_scroll(5);
        assert_eq!(sel.scroll_offset, 6);
    }

    #[test]
    fn clamp_reduces_selection() {
        let mut sel = ListSelector::new(10);
        sel.clamp(5);
        assert_eq!(sel.selected, 4);
    }

    #[test]
    fn clamp_does_nothing_if_valid() {
        let mut sel = ListSelector::new(3);
        sel.clamp(5);
        assert_eq!(sel.selected, 3);
    }
}
