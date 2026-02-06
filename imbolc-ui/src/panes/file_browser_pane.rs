use std::any::Any;
use std::fs;
use std::path::PathBuf;

use crate::state::AppState;
use crate::ui::action_id::{ActionId, FileBrowserActionId};
use crate::ui::layout_helpers::center_rect;
use crate::state::VstPluginKind;
use crate::ui::{
    Rect, RenderBuf, Action, ChopperAction, Color, FileSelectAction, InputEvent, InstrumentAction, Keymap, MouseEvent,
    MouseEventKind, MouseButton, NavAction, Pane, SequencerAction, SessionAction, Style,
};

struct DirEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

pub struct FileBrowserPane {
    keymap: Keymap,
    current_dir: PathBuf,
    entries: Vec<DirEntry>,
    selected: usize,
    filter_extensions: Option<Vec<String>>,
    /// Extensions that are directory bundles but should be treated as selectable files (e.g. vst3, vst)
    bundle_extensions: Option<Vec<String>>,
    on_select_action: FileSelectAction,
    scroll_offset: usize,
    show_hidden: bool,
}

impl FileBrowserPane {
    pub fn new(keymap: Keymap) -> Self {
        let start_dir = std::env::current_dir().unwrap_or_else(|_| {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        });
        let mut pane = Self {
            keymap,
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            filter_extensions: Some(vec!["scd".to_string()]),
            bundle_extensions: None,
            on_select_action: FileSelectAction::ImportCustomSynthDef,
            scroll_offset: 0,
            show_hidden: false,
        };
        pane.refresh_entries();
        pane
    }

    /// Open for a specific action with optional start directory
    pub fn open_for(&mut self, action: FileSelectAction, start_dir: Option<PathBuf>) {
        self.on_select_action = action.clone();
        self.bundle_extensions = None;
        self.filter_extensions = match action {
            FileSelectAction::ImportCustomSynthDef => Some(vec!["scd".to_string()]),
            FileSelectAction::ImportVstInstrument | FileSelectAction::ImportVstEffect => {
                self.bundle_extensions = Some(vec!["vst3".to_string(), "vst".to_string()]);
                Some(vec!["vst3".to_string(), "vst".to_string()])
            }
            FileSelectAction::LoadDrumSample(_) | FileSelectAction::LoadChopperSample | FileSelectAction::LoadPitchedSample(_) | FileSelectAction::LoadImpulseResponse(_, _) => {
                Some(vec!["wav".to_string(), "aiff".to_string(), "aif".to_string()])
            }
            FileSelectAction::ImportProject => Some(vec!["sqlite".to_string()]),
        };
        let default_dir = match &self.on_select_action {
            FileSelectAction::ImportVstInstrument | FileSelectAction::ImportVstEffect => {
                let vst3_dir = PathBuf::from("/Library/Audio/Plug-Ins/VST3");
                if vst3_dir.exists() { Some(vst3_dir) } else { None }
            }
            FileSelectAction::ImportProject => dirs::home_dir(),
            _ => None,
        };
        self.current_dir = start_dir.or(default_dir).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| {
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            })
        });
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh_entries();
    }

    fn refresh_entries(&mut self) {
        self.entries.clear();

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<DirEntry> = Vec::new();
            let mut files: Vec<DirEntry> = Vec::new();

            for entry in read_dir.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                let mut is_dir = path.is_dir();

                // Treat bundle directories (e.g. .vst3, .vst) as selectable files
                if is_dir {
                    if let Some(ref bundles) = self.bundle_extensions {
                        if path.extension()
                            .map_or(false, |e| bundles.iter().any(|ext| e == ext.as_str()))
                        {
                            is_dir = false; // Treat as file
                        }
                    }
                }

                // Filter files by extension if set
                if !is_dir {
                    if let Some(ref exts) = self.filter_extensions {
                        if path
                            .extension()
                            .map_or(true, |e| !exts.iter().any(|ext| e == ext.as_str()))
                        {
                            continue;
                        }
                    }
                }

                let entry = DirEntry { name, path, is_dir };
                if is_dir {
                    dirs.push(entry);
                } else {
                    files.push(entry);
                }
            }

            // Sort alphabetically
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            self.entries.extend(dirs);
            self.entries.extend(files);
        }

        // Clamp selection
        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

}

impl Default for FileBrowserPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for FileBrowserPane {
    fn id(&self) -> &'static str {
        "file_browser"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        match action {
            ActionId::FileBrowser(FileBrowserActionId::Select) => {
                if let Some(entry) = self.entries.get(self.selected) {
                    if entry.is_dir {
                        self.current_dir = entry.path.clone();
                        self.selected = 0;
                        self.scroll_offset = 0;
                        self.refresh_entries();
                        Action::None
                    } else {
                        // File selected
                        match self.on_select_action {
                            FileSelectAction::ImportCustomSynthDef => {
                                Action::Session(SessionAction::ImportCustomSynthDef(entry.path.clone()))
                            }
                            FileSelectAction::ImportVstInstrument => {
                                Action::Session(SessionAction::ImportVstPlugin(entry.path.clone(), VstPluginKind::Instrument))
                            }
                            FileSelectAction::ImportVstEffect => {
                                Action::Session(SessionAction::ImportVstPlugin(entry.path.clone(), VstPluginKind::Effect))
                            }
                            FileSelectAction::LoadDrumSample(pad_idx) => {
                                Action::Sequencer(SequencerAction::LoadSampleResult(pad_idx, entry.path.clone()))
                            }
                            FileSelectAction::LoadChopperSample => {
                                Action::Chopper(ChopperAction::LoadSampleResult(entry.path.clone()))
                            }
                            FileSelectAction::LoadPitchedSample(id) => {
                                Action::Instrument(InstrumentAction::LoadSampleResult(id, entry.path.clone()))
                            }
                            FileSelectAction::LoadImpulseResponse(id, fx_idx) => {
                                Action::Instrument(InstrumentAction::LoadIRResult(id, fx_idx, entry.path.clone()))
                            }
                            FileSelectAction::ImportProject => {
                                Action::Session(SessionAction::LoadFrom(entry.path.clone()))
                            }
                        }
                    }
                } else {
                    Action::None
                }
            }
            ActionId::FileBrowser(FileBrowserActionId::Cancel) => Action::Nav(NavAction::PopPane),
            ActionId::FileBrowser(FileBrowserActionId::Parent) => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.selected = 0;
                    self.scroll_offset = 0;
                    self.refresh_entries();
                }
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::Home) => {
                if let Some(home) = dirs::home_dir() {
                    self.current_dir = home;
                    self.selected = 0;
                    self.scroll_offset = 0;
                    self.refresh_entries();
                }
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::Next) => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + 1) % self.entries.len();
                }
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::Prev) => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + self.entries.len() - 1) % self.entries.len();
                }
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::GotoTop) => {
                self.selected = 0;
                self.scroll_offset = 0;
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::GotoBottom) => {
                if !self.entries.is_empty() {
                    self.selected = self.entries.len() - 1;
                }
                Action::None
            }
            ActionId::FileBrowser(FileBrowserActionId::ToggleHidden) => {
                self.show_hidden = !self.show_hidden;
                self.refresh_entries();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, _state: &AppState) {
        let rect = center_rect(area, 97, 29);

        let title = match self.on_select_action {
            FileSelectAction::ImportCustomSynthDef => " Import Custom SynthDef ",
            FileSelectAction::ImportVstInstrument => " Import VST Instrument ",
            FileSelectAction::ImportVstEffect => " Import VST Effect ",
            FileSelectAction::LoadDrumSample(_) | FileSelectAction::LoadChopperSample => " Load Sample ",
            FileSelectAction::LoadPitchedSample(_) => " Load Sample ",
            FileSelectAction::LoadImpulseResponse(_, _) => " Load Impulse Response ",
            FileSelectAction::ImportProject => " Import Project ",
        };
        let border_style = Style::new().fg(Color::PURPLE);
        let inner = buf.draw_block(rect, title, border_style, border_style);

        let content_x = inner.x + 1;
        let content_y = inner.y + 1;

        // Current path
        let path_str = self.current_dir.to_string_lossy();
        let max_path_width = inner.width.saturating_sub(2) as usize;
        let display_path = if path_str.len() > max_path_width {
            format!("...{}", &path_str[path_str.len() - max_path_width + 3..])
        } else {
            path_str.to_string()
        };
        buf.draw_line(
            Rect::new(content_x, content_y, inner.width.saturating_sub(2), 1),
            &[(&display_path, Style::new().fg(Color::CYAN).bold())],
        );

        // File list
        let list_y = content_y + 2;
        let visible_height = inner.height.saturating_sub(6) as usize;

        let entries = &self.entries;
        let selected = self.selected;
        let scroll_offset = self.scroll_offset;

        let mut eff_scroll = scroll_offset;
        if selected < eff_scroll {
            eff_scroll = selected;
        } else if selected >= eff_scroll + visible_height {
            eff_scroll = selected - visible_height + 1;
        }

        let sel_bg = Style::new().bg(Color::SELECTION_BG);

        if entries.is_empty() {
            let ext_label = self
                .filter_extensions
                .as_ref()
                .map(|exts| exts.join("/"))
                .unwrap_or_default();
            let empty_msg = format!("(no .{} files found)", ext_label);
            buf.draw_line(
                Rect::new(content_x, list_y, inner.width.saturating_sub(2), 1),
                &[(&empty_msg, Style::new().fg(Color::DARK_GRAY))],
            );
        } else {
            for (i, entry) in entries.iter().skip(eff_scroll).take(visible_height).enumerate() {
                let y = list_y + i as u16;
                if y >= inner.y + inner.height {
                    break;
                }
                let is_selected = eff_scroll + i == selected;

                // Fill selection background
                if is_selected {
                    for x in content_x..(inner.x + inner.width) {
                        buf.set_cell(x, y, ' ', sel_bg);
                    }
                    buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                }

                let (icon, icon_color) = if entry.is_dir {
                    ("/", Color::CYAN)
                } else {
                    (" ", Color::CUSTOM_COLOR)
                };

                let icon_style = if is_selected {
                    Style::new().fg(icon_color).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(icon_color)
                };

                let max_name_width = inner.width.saturating_sub(6) as usize;
                let display_name = if entry.name.len() > max_name_width {
                    format!("{}...", &entry.name[..max_name_width - 3])
                } else {
                    entry.name.clone()
                };

                let name_color = if entry.is_dir { Color::CYAN } else { Color::WHITE };
                let name_style = if is_selected {
                    Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
                } else {
                    Style::new().fg(name_color)
                };

                let name_display = format!(" {}", display_name);
                buf.draw_line(
                    Rect::new(content_x + 2, y, inner.width.saturating_sub(4), 1),
                    &[(icon, icon_style), (&name_display, name_style)],
                );
            }

            // Scroll indicators
            let scroll_style = Style::new().fg(Color::DARK_GRAY);
            if eff_scroll > 0 {
                buf.draw_line(
                    Rect::new(rect.x + rect.width - 5, list_y, 3, 1),
                    &[("...", scroll_style)],
                );
            }
            if eff_scroll + visible_height < entries.len() {
                buf.draw_line(
                    Rect::new(rect.x + rect.width - 5, list_y + visible_height as u16 - 1, 3, 1),
                    &[("...", scroll_style)],
                );
            }
        }

        // Help text
        let help_y = rect.y + rect.height - 2;
        if help_y < area.y + area.height {
            buf.draw_line(
                Rect::new(content_x, help_y, inner.width.saturating_sub(2), 1),
                &[("Enter: select | Backspace: parent | ~: home | &: hidden | Esc: cancel", Style::new().fg(Color::DARK_GRAY))],
            );
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, _state: &AppState) -> Action {
        let rect = center_rect(area, 97, 29);
        let inner_y = rect.y + 2;
        let content_y = inner_y + 1;
        let list_y = content_y + 2;
        let inner_height = rect.height.saturating_sub(4);
        let visible_height = inner_height.saturating_sub(6) as usize;

        // Calculate effective scroll offset (same as render)
        let mut eff_scroll = self.scroll_offset;
        if self.selected < eff_scroll {
            eff_scroll = self.selected;
        } else if self.selected >= eff_scroll + visible_height {
            eff_scroll = self.selected - visible_height + 1;
        }

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = event.row;
                if row >= list_y && row < list_y + visible_height as u16 {
                    let clicked_idx = eff_scroll + (row - list_y) as usize;
                    if clicked_idx < self.entries.len() {
                        if self.selected == clicked_idx {
                            // Click on already-selected item: open it
                            if self.entries[clicked_idx].is_dir {
                                self.current_dir = self.entries[clicked_idx].path.clone();
                                self.selected = 0;
                                self.scroll_offset = 0;
                                self.refresh_entries();
                            } else {
                                match self.on_select_action {
                                    FileSelectAction::ImportCustomSynthDef => {
                                        return Action::Session(SessionAction::ImportCustomSynthDef(
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                    FileSelectAction::LoadDrumSample(pad_idx) => {
                                        return Action::Sequencer(SequencerAction::LoadSampleResult(
                                            pad_idx,
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                    FileSelectAction::LoadChopperSample => {
                                        return Action::Chopper(ChopperAction::LoadSampleResult(
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                    FileSelectAction::LoadPitchedSample(id) => {
                                        return Action::Instrument(InstrumentAction::LoadSampleResult(
                                            id,
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                    FileSelectAction::ImportVstInstrument => {
                                        return Action::Session(SessionAction::ImportVstPlugin(
                                            self.entries[clicked_idx].path.clone(),
                                            VstPluginKind::Instrument,
                                        ));
                                    }
                                    FileSelectAction::ImportVstEffect => {
                                        return Action::Session(SessionAction::ImportVstPlugin(
                                            self.entries[clicked_idx].path.clone(),
                                            VstPluginKind::Effect,
                                        ));
                                    }
                                    FileSelectAction::LoadImpulseResponse(id, fx_idx) => {
                                        return Action::Instrument(InstrumentAction::LoadIRResult(
                                            id,
                                            fx_idx,
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                    FileSelectAction::ImportProject => {
                                        return Action::Session(SessionAction::LoadFrom(
                                            self.entries[clicked_idx].path.clone(),
                                        ));
                                    }
                                }
                            }
                        } else {
                            self.selected = clicked_idx;
                        }
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + self.entries.len() - 1) % self.entries.len();
                }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + 1) % self.entries.len();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_enter(&mut self, _state: &AppState) {
        self.refresh_entries();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{InputEvent, KeyCode, Modifiers};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    fn make_temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("imbolc_file_browser_test_{}", nanos));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn selection_wraps_with_next_and_prev() {
        let dir = make_temp_dir();
        std::fs::write(dir.join("a.scd"), "a").unwrap();
        std::fs::write(dir.join("b.scd"), "b").unwrap();

        let mut pane = FileBrowserPane::new(Keymap::new());
        pane.open_for(FileSelectAction::ImportCustomSynthDef, Some(dir.clone()));
        let state = AppState::new();

        assert!(pane.entries.len() >= 2);

        use crate::ui::action_id::{ActionId, FileBrowserActionId};
        pane.selected = pane.entries.len() - 1;
        pane.handle_action(ActionId::FileBrowser(FileBrowserActionId::Next), &dummy_event(), &state);
        assert_eq!(pane.selected, 0);

        pane.selected = 0;
        pane.handle_action(ActionId::FileBrowser(FileBrowserActionId::Prev), &dummy_event(), &state);
        assert_eq!(pane.selected, pane.entries.len() - 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
