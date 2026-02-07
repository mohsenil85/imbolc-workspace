use std::any::Any;

use crate::panes::FileBrowserPane;
use crate::state::AppState;
use crate::ui::action_id::{ActionId, SampleChopperActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{
    Rect, RenderBuf, Action, ChopperAction, Color, FileSelectAction, InputEvent, Keymap, NavAction, Pane, Style,
};

pub struct SampleChopperPane {
    keymap: Keymap,
    cursor_pos: f32, // 0.0-1.0
    auto_slice_n: usize,
    file_browser: FileBrowserPane,
}

impl SampleChopperPane {
    pub fn new(keymap: Keymap, file_browser_keymap: Keymap) -> Self {
        Self {
            keymap,
            cursor_pos: 0.5,
            auto_slice_n: 4,
            file_browser: FileBrowserPane::new(file_browser_keymap),
        }
    }

    fn selected_drum_sequencer<'a>(&self, state: &'a AppState) -> Option<&'a crate::state::drum_sequencer::DrumSequencerState> {
        state.instruments.selected_instrument()
            .and_then(|i| i.drum_sequencer.as_ref())
    }

    fn get_chopper_state<'a>(&self, state: &'a AppState) -> Option<&'a crate::state::drum_sequencer::ChopperState> {
        self.selected_drum_sequencer(state)
            .and_then(|d| d.chopper.as_ref())
    }

    fn should_show_file_browser(&self, state: &AppState) -> bool {
        self.selected_drum_sequencer(state)
            .map(|d| d.chopper.is_none())
            .unwrap_or(false)
    }
}

impl Default for SampleChopperPane {
    fn default() -> Self {
        Self::new(Keymap::new(), Keymap::new())
    }
}

impl Pane for SampleChopperPane {
    fn id(&self) -> &'static str {
        "sample_chopper"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        if self.should_show_file_browser(state) {
            if let Some(fb_action) = self.file_browser.keymap().lookup(event) {
                return self.file_browser.handle_action(fb_action, event, state);
            }
            return Action::None;
        }

        match action {
            ActionId::SampleChopper(SampleChopperActionId::MoveLeft) => {
                self.cursor_pos = (self.cursor_pos - 0.01).max(0.0);
                Action::Chopper(ChopperAction::MoveCursor(-1))
            }
            ActionId::SampleChopper(SampleChopperActionId::MoveRight) => {
                self.cursor_pos = (self.cursor_pos + 0.01).min(1.0);
                Action::Chopper(ChopperAction::MoveCursor(1))
            }
            ActionId::SampleChopper(SampleChopperActionId::NextSlice) => Action::Chopper(ChopperAction::SelectSlice(1)),
            ActionId::SampleChopper(SampleChopperActionId::PrevSlice) => Action::Chopper(ChopperAction::SelectSlice(-1)),
            ActionId::SampleChopper(SampleChopperActionId::NudgeStart) => Action::Chopper(ChopperAction::NudgeSliceStart(-0.005)),
            ActionId::SampleChopper(SampleChopperActionId::NudgeEnd) => Action::Chopper(ChopperAction::NudgeSliceEnd(0.005)),
            ActionId::SampleChopper(SampleChopperActionId::Chop) => {
                Action::Chopper(ChopperAction::AddSlice(self.cursor_pos))
            }
            ActionId::SampleChopper(SampleChopperActionId::Delete) => Action::Chopper(ChopperAction::RemoveSlice),
            ActionId::SampleChopper(SampleChopperActionId::AutoSlice) => {
                let n = self.auto_slice_n;
                self.auto_slice_n = match n {
                    4 => 8,
                    8 => 12,
                    12 => 16,
                    _ => 4,
                };
                Action::Chopper(ChopperAction::AutoSlice(n))
            }
            ActionId::SampleChopper(SampleChopperActionId::Commit) => Action::Chopper(ChopperAction::CommitAll),
            ActionId::SampleChopper(SampleChopperActionId::LoadSample) => Action::Chopper(ChopperAction::LoadSample),
            ActionId::SampleChopper(SampleChopperActionId::Preview) => Action::Chopper(ChopperAction::PreviewSlice),
            ActionId::SampleChopper(SampleChopperActionId::Back) => Action::Nav(NavAction::PopPane),
            ActionId::SampleChopper(SampleChopperActionId::AssignToPad(pad_num)) => {
                Action::Chopper(ChopperAction::AssignToPad(pad_num.saturating_sub(1) as usize))
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        // Delegate to file browser before unwrapping RenderBuf
        if let Some(drum_seq) = self.selected_drum_sequencer(state) {
            if drum_seq.chopper.is_none() {
                self.file_browser.render(area, buf, state);
                return;
            }
        }

        let rect = center_rect(area, 97, 29);

        if self.selected_drum_sequencer(state).is_none() {
            let border_style = Style::new().fg(Color::GRAY);
            let inner = buf.draw_block(rect, " Sample Chopper ", border_style, border_style);
            buf.draw_line(
                Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1),
                &[("No drum machine instrument selected. Press 1 to add one.", Style::new().fg(Color::DARK_GRAY))],
            );
            return;
        }

        let border_style = Style::new().fg(Color::GRAY);
        let _inner = buf.draw_block(rect, " Sample Chopper ", border_style, border_style);

        // Get chopper state
        let chopper = match self.get_chopper_state(state) {
            Some(c) => c,
            None => {
                buf.draw_line(
                    Rect::new(rect.x + 2, rect.y + 2, rect.width.saturating_sub(4), 1),
                    &[("No sample loaded.", Style::new().fg(Color::DARK_GRAY))],
                );
                return;
            }
        };

        let content_x = rect.x + 2;
        let content_y = rect.y + 2;

        // Header info
        let filename = chopper.path.as_ref()
            .map(|p| std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| "No Sample".to_string());
        buf.draw_line(
            Rect::new(content_x, content_y, rect.width.saturating_sub(4), 1),
            &[(&filename, Style::new().fg(Color::CYAN).bold())],
        );

        let info = format!("{:.1}s   {} slices", chopper.duration_secs, chopper.slices.len());
        let info_x = rect.x + rect.width - 2 - info.len() as u16;
        buf.draw_line(
            Rect::new(info_x, content_y, rect.width.saturating_sub(info_x - rect.x), 1),
            &[(&info, Style::new().fg(Color::DARK_GRAY))],
        );

        // Waveform
        let wave_y = content_y + 2;
        let wave_height: u16 = 8;
        let wave_width = (rect.width - 4) as usize;

        let green_style = Style::new().fg(Color::GREEN);
        if !chopper.waveform_peaks.is_empty() {
            let peaks = &chopper.waveform_peaks;
            for i in 0..wave_width {
                let peak_idx = (i as f32 / wave_width as f32 * peaks.len() as f32) as usize;
                if let Some(&val) = peaks.get(peak_idx) {
                    let bar_h = (val * wave_height as f32) as u16;
                    let center_y = wave_y + wave_height / 2;
                    let top = center_y.saturating_sub(bar_h / 2);
                    let bottom = center_y.saturating_add(bar_h / 2);
                    for y in top..=bottom {
                        buf.set_cell(content_x + i as u16, y, '│', green_style);
                    }
                }
            }
        } else {
            buf.draw_line(
                Rect::new(content_x, wave_y + wave_height / 2, 20, 1),
                &[("(No waveform data)", Style::new().fg(Color::DARK_GRAY))],
            );
        }

        // Draw slices
        let slice_y_start = wave_y;
        let slice_y_end = wave_y + wave_height;
        let dark_gray_style = Style::new().fg(Color::DARK_GRAY);
        let sel_bg_style = Style::new().bg(Color::SELECTION_BG);
        let sel_white_style = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG);

        for (i, slice) in chopper.slices.iter().enumerate() {
            let start_x = (slice.start * wave_width as f32) as u16;
            let end_x = (slice.end * wave_width as f32) as u16;
            let center_x = (start_x + end_x) / 2;

            // Draw slice boundaries
            if i > 0 {
                for y in slice_y_start..=slice_y_end {
                    buf.set_cell(content_x + start_x, y, '|', dark_gray_style);
                }
            }

            // Highlight selected slice
            if i == chopper.selected_slice {
                for x in start_x..end_x {
                    if x >= wave_width as u16 { break; }
                    buf.set_cell(content_x + x, slice_y_end + 1, ' ', sel_bg_style);
                }
                let label = format!("{}", i + 1);
                let lx = content_x + center_x.saturating_sub(label.len() as u16 / 2);
                for (j, ch) in label.chars().enumerate() {
                    buf.set_cell(lx + j as u16, slice_y_end + 1, ch, sel_white_style);
                }
            } else {
                let label = format!("{}", i + 1);
                if end_x - start_x > label.len() as u16 {
                    let lx = content_x + center_x.saturating_sub(label.len() as u16 / 2);
                    for (j, ch) in label.chars().enumerate() {
                        buf.set_cell(lx + j as u16, slice_y_end + 1, ch, dark_gray_style);
                    }
                }
            }
        }

        // Draw cursor
        let cursor_screen_x = (self.cursor_pos * wave_width as f32) as u16;
        let yellow_style = Style::new().fg(Color::YELLOW);
        for y in slice_y_start..=slice_y_end {
            buf.set_cell(content_x + cursor_screen_x, y, '┆', yellow_style);
        }
        buf.set_cell(content_x + cursor_screen_x, slice_y_end + 2, '▲', yellow_style);

        // List slices
        let list_y = slice_y_end + 4;
        for i in 0..8 {
            if i >= chopper.slices.len() { break; }
            let slice = &chopper.slices[i];
            let y = list_y + i as u16;

            if i == chopper.selected_slice {
                buf.set_cell(content_x, y, '>', Style::new().fg(Color::WHITE).bold());
            }

            let text = format!("{:<2} {:.3}-{:.3}", i + 1, slice.start, slice.end);
            let style = Style::new().fg(
                if i == chopper.selected_slice { Color::WHITE } else { Color::GRAY }
            );
            buf.draw_line(
                Rect::new(content_x + 2, y, text.len() as u16, 1),
                &[(&text, style)],
            );

            // Check pad assignments
            if let Some(inst) = state.instruments.selected_instrument() {
                if let Some(ds) = &inst.drum_sequencer {
                    for (pad_idx, pad) in ds.pads.iter().enumerate() {
                        if pad.buffer_id == chopper.buffer_id &&
                           (pad.slice_start - slice.start).abs() < 0.001 &&
                           (pad.slice_end - slice.end).abs() < 0.001 {
                            let pad_label = format!("→ Pad {}", pad_idx + 1);
                            buf.draw_line(
                                Rect::new(content_x + 25, y, pad_label.len() as u16, 1),
                                &[(&pad_label, style)],
                            );
                        }
                    }
                }
            }
        }

    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }


    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_enter(&mut self, state: &AppState) {
        if self.should_show_file_browser(state) {
            self.file_browser.open_for(FileSelectAction::LoadChopperSample, None);
        }
    }
}
