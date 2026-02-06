use std::any::Any;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;
use crate::state::recent_projects::RecentProjects;
use crate::ui::action_id::{ActionId, ProjectBrowserActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, NavAction, Pane, SessionAction, Style};

pub struct ProjectBrowserPane {
    keymap: Keymap,
    entries: Vec<ProjectEntry>,
    selected: usize,
}

struct ProjectEntry {
    name: String,
    path: PathBuf,
    last_opened: SystemTime,
}

impl ProjectBrowserPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            entries: Vec::new(),
            selected: 0,
        }
    }

    /// Refresh the project list from disk
    pub fn refresh(&mut self) {
        let recent = RecentProjects::load();
        self.entries = recent.entries.into_iter().map(|e| ProjectEntry {
            name: e.name,
            path: e.path,
            last_opened: e.last_opened,
        }).collect();
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    fn format_time_ago(time: SystemTime) -> String {
        let now = SystemTime::now();
        let elapsed = now.duration_since(time).unwrap_or_default();
        let secs = elapsed.as_secs();
        if secs < 60 { return "just now".to_string(); }
        if secs < 3600 { return format!("{} min ago", secs / 60); }
        if secs < 86400 { return format!("{} hours ago", secs / 3600); }
        if secs < 604800 { return format!("{} days ago", secs / 86400); }
        // Fallback to date
        let since_epoch = time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let days = since_epoch / 86400;
        let years = 1970 + days / 365;
        format!("{}", years)
    }
}

impl Pane for ProjectBrowserPane {
    fn id(&self) -> &'static str {
        "project_browser"
    }

    fn on_enter(&mut self, _state: &AppState) {
        self.refresh();
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::ProjectBrowser(ProjectBrowserActionId::Close) => Action::Nav(NavAction::PopPane),
            ActionId::ProjectBrowser(ProjectBrowserActionId::Up) => {
                if self.selected > 0 { self.selected -= 1; }
                Action::None
            }
            ActionId::ProjectBrowser(ProjectBrowserActionId::Down) => {
                if self.selected + 1 < self.entries.len() { self.selected += 1; }
                Action::None
            }
            ActionId::ProjectBrowser(ProjectBrowserActionId::Select) => {
                if let Some(entry) = self.entries.get(self.selected) {
                    let path = entry.path.clone();
                    if state.project.dirty {
                        // Dirty check handled by caller — for now just load directly
                        // The confirm pane intercept happens in global_actions
                        return Action::Session(SessionAction::LoadFrom(path));
                    }
                    return Action::Session(SessionAction::LoadFrom(path));
                }
                Action::None
            }
            ActionId::ProjectBrowser(ProjectBrowserActionId::NewProject) => {
                Action::Session(SessionAction::NewProject)
            }
            ActionId::ProjectBrowser(ProjectBrowserActionId::DeleteEntry) => {
                if let Some(entry) = self.entries.get(self.selected) {
                    let path = entry.path.clone();
                    let mut recent = RecentProjects::load();
                    recent.remove(&path);
                    recent.save();
                    self.refresh();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_raw_input(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match event.key {
            crate::ui::KeyCode::Char('n') | crate::ui::KeyCode::Char('N') => {
                Action::Session(SessionAction::NewProject)
            }
            crate::ui::KeyCode::Char('i') | crate::ui::KeyCode::Char('I') => {
                Action::Session(SessionAction::OpenFileBrowser(
                    crate::ui::FileSelectAction::ImportProject,
                ))
            }
            crate::ui::KeyCode::Char('d') | crate::ui::KeyCode::Char('D') => {
                if let Some(entry) = self.entries.get(self.selected) {
                    let path = entry.path.clone();
                    let mut recent = RecentProjects::load();
                    recent.remove(&path);
                    recent.save();
                    self.refresh();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let width = 56_u16.min(area.width.saturating_sub(4));
        let height = (self.entries.len() as u16 + 8).min(area.height.saturating_sub(4)).max(10);
        let rect = center_rect(area, width, height);

        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(rect, " Projects ", border_style, border_style);

        // Section header
        let header_area = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1);
        buf.draw_line(header_area, &[("Recent Projects", Style::new().fg(Color::DARK_GRAY))]);

        if self.entries.is_empty() {
            let empty_y = inner.y + 2;
            if empty_y < inner.y + inner.height {
                let empty_area = Rect::new(inner.x + 1, empty_y, inner.width.saturating_sub(2), 1);
                buf.draw_line(empty_area, &[("No recent projects", Style::new().fg(Color::DARK_GRAY))]);
            }
        }

        // Project list
        let max_visible = (inner.height.saturating_sub(4)) as usize;
        let scroll = if self.selected >= max_visible {
            self.selected - max_visible + 1
        } else {
            0
        };

        for (i, entry) in self.entries.iter().skip(scroll).take(max_visible).enumerate() {
            let y = inner.y + 2 + i as u16;
            if y >= inner.y + inner.height.saturating_sub(2) {
                break;
            }

            let is_selected = scroll + i == self.selected;
            let time_str = Self::format_time_ago(entry.last_opened);

            let name_max = inner.width.saturating_sub(time_str.len() as u16 + 6) as usize;
            let display_name: String = entry.name.chars().take(name_max).collect();

            let (name_style, time_style) = if is_selected {
                (
                    Style::new().fg(Color::BLACK).bg(Color::CYAN).bold(),
                    Style::new().fg(Color::BLACK).bg(Color::CYAN),
                )
            } else {
                (
                    Style::new().fg(Color::WHITE),
                    Style::new().fg(Color::DARK_GRAY),
                )
            };

            // Clear the line for selected item
            if is_selected {
                for x in (inner.x + 1)..(inner.x + 1 + inner.width.saturating_sub(2)) {
                    buf.set_cell(x, y, ' ', Style::new().fg(Color::BLACK).bg(Color::CYAN).bold());
                }
            }

            let prefix = if is_selected { " > " } else { "   " };
            let padding_len = name_max.saturating_sub(display_name.len());
            let padding: String = " ".repeat(padding_len);
            let time_col = format!("  {}", time_str);

            let line_area = Rect::new(inner.x, y, inner.width, 1);
            buf.draw_line(line_area, &[
                (prefix, name_style),
                (&display_name, name_style),
                (&padding, name_style),
                (&time_col, time_style),
            ]);
        }

        // Footer
        let footer_y = rect.y + rect.height.saturating_sub(2);
        if footer_y < area.y + area.height {
            let hi = Style::new().fg(Color::CYAN).bold();
            let lo = Style::new().fg(Color::DARK_GRAY);
            let footer_area = Rect::new(inner.x + 1, footer_y, inner.width.saturating_sub(2), 1);
            buf.draw_line(footer_area, &[
                ("[N]", hi), ("ew  ", lo),
                ("[I]", hi), ("mport  ", lo),
                ("[Enter]", hi), (" Open  ", lo),
                ("[D]", hi), ("elete  ", lo),
                ("[Esc]", hi), (" Close", lo),
            ]);
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
