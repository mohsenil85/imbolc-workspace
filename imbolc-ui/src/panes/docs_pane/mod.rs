//! DocsPane - Educational documentation pane for Imbolc
//!
//! Two modes:
//! - Contextual: Shows docs for currently selected instrument's source type
//! - Browser: Full topic navigation

mod content;
mod rendering;

use std::any::Any;
use std::collections::HashMap;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, DocsActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Action, Color, InputEvent, Keymap, MouseButton, MouseEvent, MouseEventKind, NavAction, Pane,
    Rect, RenderBuf, Style,
};

use content::{load_doc, load_sources_map, load_topic_index, TopicEntry};
use rendering::{parse_markdown, ParsedDoc, RenderLine};

/// Documentation pane mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsMode {
    /// Shows docs for the selected instrument's source type
    Contextual,
    /// Full topic browser
    Browser,
}

pub struct DocsPane {
    keymap: Keymap,
    mode: DocsMode,
    current_doc: Option<ParsedDoc>,
    current_path: Option<String>,
    scroll_offset: usize,
    history: Vec<String>,
    topic_list: Vec<TopicEntry>,
    selected_topic: usize,
    sources_map: HashMap<String, String>,
    /// Height of the content area (set during render)
    content_height: usize,
}

impl DocsPane {
    pub fn new(keymap: Keymap) -> Self {
        let sources_map = load_sources_map();
        let topic_list = load_topic_index();

        Self {
            keymap,
            mode: DocsMode::Contextual,
            current_doc: None,
            current_path: None,
            scroll_offset: 0,
            history: Vec::new(),
            topic_list,
            selected_topic: 0,
            sources_map,
            content_height: 20,
        }
    }

    /// Open docs for a specific source type (contextual mode)
    pub fn open_for_source(&mut self, source_short_name: &str) {
        self.mode = DocsMode::Contextual;
        self.scroll_offset = 0;

        let doc_path = self
            .sources_map
            .get(source_short_name)
            .cloned()
            .unwrap_or_else(|| "sources/oscillators.md".to_string());
        self.load_doc_path(&doc_path);
    }

    /// Open the topic browser (Learn mode)
    pub fn open_browser(&mut self) {
        self.mode = DocsMode::Browser;
        self.scroll_offset = 0;
        self.selected_topic = 0;
        // Load first topic by default
        let first_path = self.topic_list.first().map(|t| t.path.clone());
        if let Some(path) = first_path {
            self.load_doc_path(&path);
        }
    }

    fn load_doc_path(&mut self, path: &str) {
        // Handle anchor in path
        let (file_path, anchor) = if let Some(idx) = path.find('#') {
            (&path[..idx], Some(&path[idx + 1..]))
        } else {
            (path, None)
        };

        if let Some(content) = load_doc(file_path) {
            let doc = parse_markdown(&content);

            // If there's an anchor, try to find it and adjust scroll
            if let Some(anchor_id) = anchor {
                if let Some(line_idx) = doc.find_anchor(anchor_id) {
                    self.scroll_offset = line_idx;
                } else {
                    self.scroll_offset = 0;
                }
            } else {
                self.scroll_offset = 0;
            }

            self.current_doc = Some(doc);
            self.current_path = Some(path.to_string());
        }
    }

    fn follow_link(&mut self) {
        if self.mode == DocsMode::Browser {
            // In browser mode, Enter loads the selected topic
            if let Some(topic) = self.topic_list.get(self.selected_topic) {
                let path = topic.path.clone();
                // Push current to history
                if let Some(ref current) = self.current_path {
                    self.history.push(current.clone());
                }
                self.load_doc_path(&path);
            }
        } else if let Some(ref doc) = self.current_doc {
            // In contextual mode, check if cursor is on a link
            let visible_line = self.scroll_offset;
            if let Some(line) = doc.lines.get(visible_line) {
                if let Some(ref link_target) = line.link_target {
                    // Push current to history
                    if let Some(ref current) = self.current_path {
                        self.history.push(current.clone());
                    }
                    let target = link_target.clone();
                    self.load_doc_path(&target);
                }
            }
        }
    }

    fn go_back(&mut self) {
        if let Some(prev_path) = self.history.pop() {
            self.load_doc_path(&prev_path);
        }
    }

    fn max_scroll(&self) -> usize {
        if let Some(ref doc) = self.current_doc {
            doc.lines.len().saturating_sub(self.content_height)
        } else {
            0
        }
    }
}

impl Pane for DocsPane {
    fn id(&self) -> &'static str {
        "docs"
    }

    fn handle_action(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        _state: &AppState,
    ) -> Action {
        match action {
            ActionId::Docs(DocsActionId::Close) => Action::Nav(NavAction::PopPane),
            ActionId::Docs(DocsActionId::ScrollUp) => {
                if self.mode == DocsMode::Browser {
                    if self.selected_topic > 0 {
                        self.selected_topic -= 1;
                        if let Some(topic) = self.topic_list.get(self.selected_topic) {
                            let path = topic.path.clone();
                            self.load_doc_path(&path);
                        }
                    }
                } else if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                Action::None
            }
            ActionId::Docs(DocsActionId::ScrollDown) => {
                if self.mode == DocsMode::Browser {
                    if self.selected_topic < self.topic_list.len().saturating_sub(1) {
                        self.selected_topic += 1;
                        if let Some(topic) = self.topic_list.get(self.selected_topic) {
                            let path = topic.path.clone();
                            self.load_doc_path(&path);
                        }
                    }
                } else {
                    let max = self.max_scroll();
                    if self.scroll_offset < max {
                        self.scroll_offset += 1;
                    }
                }
                Action::None
            }
            ActionId::Docs(DocsActionId::PageUp) => {
                let page = self.content_height.saturating_sub(2);
                self.scroll_offset = self.scroll_offset.saturating_sub(page);
                Action::None
            }
            ActionId::Docs(DocsActionId::PageDown) => {
                let page = self.content_height.saturating_sub(2);
                let max = self.max_scroll();
                self.scroll_offset = (self.scroll_offset + page).min(max);
                Action::None
            }
            ActionId::Docs(DocsActionId::Top) => {
                self.scroll_offset = 0;
                if self.mode == DocsMode::Browser {
                    self.selected_topic = 0;
                    if let Some(topic) = self.topic_list.first() {
                        let path = topic.path.clone();
                        self.load_doc_path(&path);
                    }
                }
                Action::None
            }
            ActionId::Docs(DocsActionId::Bottom) => {
                if self.mode == DocsMode::Browser {
                    self.selected_topic = self.topic_list.len().saturating_sub(1);
                    if let Some(topic) = self.topic_list.get(self.selected_topic) {
                        let path = topic.path.clone();
                        self.load_doc_path(&path);
                    }
                } else {
                    self.scroll_offset = self.max_scroll();
                }
                Action::None
            }
            ActionId::Docs(DocsActionId::FollowLink) => {
                self.follow_link();
                Action::None
            }
            ActionId::Docs(DocsActionId::Back) => {
                self.go_back();
                Action::None
            }
            ActionId::Docs(DocsActionId::ToggleMode) => {
                self.mode = match self.mode {
                    DocsMode::Contextual => DocsMode::Browser,
                    DocsMode::Browser => DocsMode::Contextual,
                };
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, _area: Rect, _state: &AppState) -> Action {
        match event.kind {
            MouseEventKind::ScrollUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                let max = self.max_scroll();
                if self.scroll_offset < max {
                    self.scroll_offset += 1;
                }
                Action::None
            }
            MouseEventKind::Down(MouseButton::Right) => Action::Nav(NavAction::PopPane),
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        // Calculate dimensions
        let width = area.width.clamp(40, 80);
        let height = area.height.saturating_sub(4).max(10);
        let rect = center_rect(area, width, height);

        // Update content height for scroll calculations
        self.content_height = (rect.height as usize).saturating_sub(4);

        // Draw background
        let bg_style = Style::new().bg(Color::new(15, 15, 25));
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                buf.set_cell(x, y, ' ', bg_style);
            }
        }

        // Draw border
        let title = match self.mode {
            DocsMode::Contextual => " Docs ",
            DocsMode::Browser => " Learn ",
        };
        let border_style = Style::new().fg(Color::SKY_BLUE);
        let inner = buf.draw_block(rect, title, border_style, border_style);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        // In browser mode, show topic list on the left
        if self.mode == DocsMode::Browser {
            self.render_browser_mode(inner, buf);
        } else {
            self.render_contextual_mode(inner, buf);
        }

        // Footer with keybindings
        let footer_y = rect.y + rect.height - 2;
        if footer_y < area.y + area.height {
            let footer = match self.mode {
                DocsMode::Contextual => "[Tab] Topics  [Backspace] Back  [Esc] Close",
                DocsMode::Browser => "[Tab] Contextual  [Enter] Select  [Esc] Close",
            };
            let footer_area = Rect::new(inner.x + 1, footer_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(footer_area, &[(footer, Style::new().fg(Color::DARK_GRAY))]);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DocsPane {
    fn render_contextual_mode(&self, inner: Rect, buf: &mut RenderBuf) {
        if let Some(ref doc) = self.current_doc {
            let visible_lines = (inner.height as usize).saturating_sub(3);

            for (i, line) in doc
                .lines
                .iter()
                .skip(self.scroll_offset)
                .take(visible_lines)
                .enumerate()
            {
                let y = inner.y + 1 + i as u16;
                if y >= inner.y + inner.height - 2 {
                    break;
                }

                let line_area = Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1);
                self.render_line(line, line_area, buf);
            }

            // Scroll indicator
            if doc.lines.len() > visible_lines {
                let indicator = format!(
                    "{}-{}/{}",
                    self.scroll_offset + 1,
                    (self.scroll_offset + visible_lines).min(doc.lines.len()),
                    doc.lines.len()
                );
                let ind_y = inner.y + inner.height - 3;
                if ind_y >= inner.y {
                    let ind_area = Rect::new(
                        inner.x + inner.width - indicator.len() as u16 - 2,
                        ind_y,
                        indicator.len() as u16 + 1,
                        1,
                    );
                    buf.draw_line(ind_area, &[(&indicator, Style::new().fg(Color::DARK_GRAY))]);
                }
            }
        } else {
            // No doc loaded
            let msg = "No documentation available";
            let msg_area = Rect::new(inner.x + 2, inner.y + 2, msg.len() as u16, 1);
            buf.draw_line(msg_area, &[(msg, Style::new().fg(Color::GRAY))]);
        }
    }

    fn render_browser_mode(&self, inner: Rect, buf: &mut RenderBuf) {
        // Split area: left for topics (30%), right for content (70%)
        let topic_width = (inner.width as f32 * 0.30).max(15.0) as u16;
        let content_width = inner.width.saturating_sub(topic_width + 1);

        // Draw topic list
        let topic_area = Rect::new(
            inner.x,
            inner.y + 1,
            topic_width,
            inner.height.saturating_sub(3),
        );
        let visible_topics = topic_area.height as usize;

        // Calculate scroll for topic list
        let topic_scroll = if self.selected_topic >= visible_topics {
            self.selected_topic - visible_topics + 1
        } else {
            0
        };

        for (i, topic) in self
            .topic_list
            .iter()
            .skip(topic_scroll)
            .take(visible_topics)
            .enumerate()
        {
            let y = topic_area.y + i as u16;
            if y >= topic_area.y + topic_area.height {
                break;
            }

            let is_selected = topic_scroll + i == self.selected_topic;
            let style = if is_selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::GRAY)
            };

            // Truncate title to fit
            let max_len = topic_width.saturating_sub(2) as usize;
            let display: String = topic.title.chars().take(max_len).collect();
            let padded = format!(" {:<width$}", display, width = max_len);

            let row_area = Rect::new(topic_area.x, y, topic_width, 1);

            // Clear row with selection bg if selected
            if is_selected {
                for x in row_area.x..row_area.x + row_area.width {
                    buf.set_cell(x, y, ' ', Style::new().bg(Color::SELECTION_BG));
                }
            }

            buf.draw_line(row_area, &[(&padded, style)]);
        }

        // Draw separator
        let sep_x = inner.x + topic_width;
        for y in inner.y + 1..inner.y + inner.height - 2 {
            buf.set_cell(sep_x, y, '\u{2502}', Style::new().fg(Color::DARK_GRAY));
        }

        // Draw content on the right
        let content_area = Rect::new(
            sep_x + 1,
            inner.y + 1,
            content_width,
            inner.height.saturating_sub(3),
        );

        if let Some(ref doc) = self.current_doc {
            let visible_lines = content_area.height as usize;

            for (i, line) in doc
                .lines
                .iter()
                .skip(self.scroll_offset)
                .take(visible_lines)
                .enumerate()
            {
                let y = content_area.y + i as u16;
                if y >= content_area.y + content_area.height {
                    break;
                }

                let line_area = Rect::new(
                    content_area.x + 1,
                    y,
                    content_area.width.saturating_sub(2),
                    1,
                );
                self.render_line(line, line_area, buf);
            }
        }
    }

    fn render_line(&self, line: &RenderLine, area: Rect, buf: &mut RenderBuf) {
        let style = match line.kind {
            rendering::LineKind::Heading1 => Style::new().fg(Color::CYAN).bold(),
            rendering::LineKind::Heading2 => Style::new().fg(Color::SKY_BLUE).bold(),
            rendering::LineKind::Heading3 => Style::new().fg(Color::WHITE).bold(),
            rendering::LineKind::Code => Style::new().fg(Color::YELLOW),
            rendering::LineKind::Link => Style::new().fg(Color::MAGENTA).underline(),
            rendering::LineKind::ListItem => Style::new().fg(Color::WHITE),
            rendering::LineKind::Normal => Style::new().fg(Color::GRAY),
        };

        // Truncate to fit
        let max_len = area.width as usize;
        let display: String = line.text.chars().take(max_len).collect();
        buf.draw_line(area, &[(&display, style)]);
    }
}
