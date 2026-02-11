use super::widgets::TextInput;

/// Trait for items that can be filtered and tab-completed in a FilterableList.
pub trait FilterableItem {
    /// Primary text used for filtering and tab-completion.
    fn primary_text(&self) -> &str;
    /// Secondary text also checked during filtering.
    fn secondary_text(&self) -> &str {
        ""
    }
    /// Text inserted into the input on tab-complete/arrow selection.
    /// Defaults to `primary_text()`.
    fn completion_text(&self) -> String {
        self.primary_text().to_string()
    }
}

/// Shared filtering, tab-completion, scrolling, and selection logic
/// used by CommandPalette and PaneSwitcher.
pub struct FilterableList<T> {
    items: Vec<T>,
    pub text_input: TextInput,
    filtered: Vec<usize>,
    selected: usize,
    scroll: usize,
    filter_base: String,
    max_visible: usize,
}

impl<T: FilterableItem> FilterableList<T> {
    pub fn new(max_visible: usize) -> Self {
        let mut text_input = TextInput::new("");
        text_input.set_focused(true);
        Self {
            items: Vec::new(),
            text_input,
            filtered: Vec::new(),
            selected: 0,
            scroll: 0,
            filter_base: String::new(),
            max_visible,
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.text_input.set_value("");
        self.text_input.set_focused(true);
        self.filter_base.clear();
        self.selected = 0;
        self.scroll = 0;
        self.update_filter();
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn filtered(&self) -> &[usize] {
        &self.filtered
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn max_visible(&self) -> usize {
        self.max_visible
    }

    /// Returns the selected item, if any.
    pub fn selected_item(&self) -> Option<&T> {
        self.filtered
            .get(self.selected)
            .map(|&idx| &self.items[idx])
    }

    pub fn update_filter(&mut self) {
        let query = self.filter_base.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if query.is_empty() {
                    return true;
                }
                item.primary_text().to_lowercase().contains(&query)
                    || item.secondary_text().to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn tab_complete(&mut self) {
        if self.filtered.is_empty() {
            return;
        }

        let input = self.text_input.value().to_string();

        // Find longest common prefix of all filtered items' completion text
        let first = self.items[self.filtered[0]].completion_text();
        let mut lcp = first;
        for &idx in &self.filtered[1..] {
            let text = self.items[idx].completion_text();
            lcp = longest_common_prefix(&lcp, &text);
            if lcp.is_empty() {
                break;
            }
        }

        if lcp.len() > input.len() && lcp.starts_with(&input) {
            // LCP extends beyond current input — fill in LCP
            self.text_input.set_value(&lcp);
            self.filter_base = lcp;
            self.update_filter();
        } else if self.filtered.len() == 1 {
            // Single match — fill in completely
            let text = self.items[self.filtered[0]].completion_text();
            self.text_input.set_value(&text);
            self.filter_base = text;
            self.update_filter();
        } else if self.filtered.len() > 1 {
            // Already at LCP and multiple matches — cycle selected
            self.selected = (self.selected + 1) % self.filtered.len();
            self.ensure_visible();
            let text = self.items[self.filtered[self.selected]].completion_text();
            self.text_input.set_value(&text);
        }
    }

    pub fn move_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.filtered.len() - 1;
        }
        self.ensure_visible();
        let text = self.items[self.filtered[self.selected]].completion_text();
        self.text_input.set_value(&text);
    }

    pub fn move_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered.len();
        self.ensure_visible();
        let text = self.items[self.filtered[self.selected]].completion_text();
        self.text_input.set_value(&text);
    }

    /// Handle a text-editing key event: delegate to TextInput and update filter.
    pub fn handle_text_input(&mut self, event: &super::InputEvent) {
        self.text_input.handle_input(event);
        self.filter_base = self.text_input.value().to_string();
        self.update_filter();
    }

    fn ensure_visible(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + self.max_visible {
            self.scroll = self.selected.saturating_sub(self.max_visible - 1);
        }
    }
}

pub fn longest_common_prefix(a: &str, b: &str) -> String {
    a.chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestItem {
        primary: String,
        secondary: String,
    }

    impl TestItem {
        fn new(primary: &str, secondary: &str) -> Self {
            Self {
                primary: primary.to_string(),
                secondary: secondary.to_string(),
            }
        }
    }

    impl FilterableItem for TestItem {
        fn primary_text(&self) -> &str {
            &self.primary
        }
        fn secondary_text(&self) -> &str {
            &self.secondary
        }
    }

    #[test]
    fn filter_by_primary_text() {
        let mut list = FilterableList::new(10);
        list.set_items(vec![
            TestItem::new("alpha", "first"),
            TestItem::new("beta", "second"),
            TestItem::new("gamma", "third"),
        ]);
        assert_eq!(list.filtered().len(), 3);

        list.filter_base = "al".to_string();
        list.update_filter();
        assert_eq!(list.filtered().len(), 1);
        assert_eq!(list.filtered()[0], 0);
    }

    #[test]
    fn filter_by_secondary_text() {
        let mut list = FilterableList::new(10);
        list.set_items(vec![
            TestItem::new("alpha", "first"),
            TestItem::new("beta", "second"),
        ]);

        list.filter_base = "sec".to_string();
        list.update_filter();
        assert_eq!(list.filtered().len(), 1);
        assert_eq!(list.filtered()[0], 1);
    }

    #[test]
    fn move_up_down_wraps() {
        let mut list = FilterableList::new(10);
        list.set_items(vec![
            TestItem::new("a", ""),
            TestItem::new("b", ""),
            TestItem::new("c", ""),
        ]);
        assert_eq!(list.selected(), 0);

        list.move_down();
        assert_eq!(list.selected(), 1);
        list.move_down();
        assert_eq!(list.selected(), 2);
        list.move_down();
        assert_eq!(list.selected(), 0); // wrap

        list.move_up();
        assert_eq!(list.selected(), 2); // wrap back
    }

    #[test]
    fn tab_complete_single_match() {
        let mut list = FilterableList::new(10);
        list.set_items(vec![TestItem::new("alpha", ""), TestItem::new("beta", "")]);

        list.filter_base = "alp".to_string();
        list.update_filter();
        assert_eq!(list.filtered().len(), 1);

        list.text_input.set_value("alp");
        list.tab_complete();
        assert_eq!(list.text_input.value(), "alpha");
    }

    #[test]
    fn longest_common_prefix_works() {
        assert_eq!(longest_common_prefix("abc", "abd"), "ab");
        assert_eq!(longest_common_prefix("abc", "abc"), "abc");
        assert_eq!(longest_common_prefix("abc", "xyz"), "");
        assert_eq!(longest_common_prefix("", "abc"), "");
    }

    #[test]
    fn selected_item_returns_correct() {
        let mut list = FilterableList::new(10);
        list.set_items(vec![TestItem::new("alpha", ""), TestItem::new("beta", "")]);
        assert_eq!(list.selected_item().unwrap().primary_text(), "alpha");

        list.move_down();
        assert_eq!(list.selected_item().unwrap().primary_text(), "beta");
    }

    #[test]
    fn ensure_visible_scrolls() {
        let mut list = FilterableList::new(2);
        list.set_items(vec![
            TestItem::new("a", ""),
            TestItem::new("b", ""),
            TestItem::new("c", ""),
            TestItem::new("d", ""),
        ]);
        assert_eq!(list.scroll(), 0);

        list.move_down(); // selected=1, scroll=0
        list.move_down(); // selected=2, scroll should advance
        assert!(list.scroll() > 0);
    }
}
