use std::any::Any;

use crate::state::drum_sequencer::NUM_PADS;
use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, MouseEvent, MouseEventKind, MouseButton, NavAction, Pane, SequencerAction, Style};
use crate::ui::action_id::{ActionId, SequencerActionId};

pub struct SequencerPane {
    keymap: Keymap,
    pub(crate) cursor_pad: usize,
    pub(crate) cursor_step: usize,
    view_start_step: usize,
    /// Selection anchor (pad, step). None = no selection.
    pub(crate) selection_anchor: Option<(usize, usize)>,
}

impl SequencerPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            cursor_pad: 0,
            cursor_step: 0,
            view_start_step: 0,
            selection_anchor: None,
        }
    }

    /// Returns the selection region as (start_pad, end_pad, start_step, end_step),
    /// or a single cell at the cursor if no selection is active.
    pub(crate) fn selection_region(&self) -> (usize, usize, usize, usize) {
        if let Some((anchor_pad, anchor_step)) = self.selection_anchor {
            let (p0, p1) = if anchor_pad <= self.cursor_pad {
                (anchor_pad, self.cursor_pad)
            } else {
                (self.cursor_pad, anchor_pad)
            };
            let (s0, s1) = if anchor_step <= self.cursor_step {
                (anchor_step, self.cursor_step)
            } else {
                (self.cursor_step, anchor_step)
            };
            (p0, p1, s0, s1)
        } else {
            (self.cursor_pad, self.cursor_pad, self.cursor_step, self.cursor_step)
        }
    }

    fn visible_steps(&self, box_width: u16) -> usize {
        // Pad label column: 11 chars, box borders: 4 chars, step columns: 3 chars each
        let available = (box_width as usize).saturating_sub(15);
        available / 3
    }

}

impl Default for SequencerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for SequencerPane {
    fn id(&self) -> &'static str {
        "sequencer"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let seq = match state.instruments.selected_drum_sequencer() {
            Some(s) => s,
            None => return Action::None,
        };
        let pattern_length = seq.pattern().length;

        match action {
            ActionId::Sequencer(SequencerActionId::VelUp) => {
                return Action::Sequencer(SequencerAction::AdjustVelocity(
                    self.cursor_pad,
                    self.cursor_step,
                    10,
                ));
            }
            ActionId::Sequencer(SequencerActionId::VelDown) => {
                return Action::Sequencer(SequencerAction::AdjustVelocity(
                    self.cursor_pad,
                    self.cursor_step,
                    -10,
                ));
            }
            ActionId::Sequencer(SequencerActionId::PadLevelDown) => {
                return Action::Sequencer(SequencerAction::AdjustPadLevel(
                    self.cursor_pad,
                    -0.05,
                ));
            }
            ActionId::Sequencer(SequencerActionId::PadLevelUp) => {
                return Action::Sequencer(SequencerAction::AdjustPadLevel(
                    self.cursor_pad,
                    0.05,
                ));
            }
            ActionId::Sequencer(SequencerActionId::Up) => {
                self.selection_anchor = None;
                self.cursor_pad = self.cursor_pad.saturating_sub(1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::Down) => {
                self.selection_anchor = None;
                self.cursor_pad = (self.cursor_pad + 1).min(NUM_PADS - 1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::Left) => {
                self.selection_anchor = None;
                self.cursor_step = self.cursor_step.saturating_sub(1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::Right) => {
                self.selection_anchor = None;
                self.cursor_step = (self.cursor_step + 1).min(pattern_length - 1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::SelectUp) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_pad, self.cursor_step));
                }
                self.cursor_pad = self.cursor_pad.saturating_sub(1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::SelectDown) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_pad, self.cursor_step));
                }
                self.cursor_pad = (self.cursor_pad + 1).min(NUM_PADS - 1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::SelectLeft) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_pad, self.cursor_step));
                }
                self.cursor_step = self.cursor_step.saturating_sub(1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::SelectRight) => {
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some((self.cursor_pad, self.cursor_step));
                }
                self.cursor_step = (self.cursor_step + 1).min(pattern_length - 1);
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::Toggle) => Action::Sequencer(SequencerAction::ToggleStep(
                self.cursor_pad,
                self.cursor_step,
            )),
            ActionId::Sequencer(SequencerActionId::PlayStop) => Action::Sequencer(SequencerAction::PlayStop),
            ActionId::Sequencer(SequencerActionId::LoadSample) => {
                Action::Sequencer(SequencerAction::LoadSample(self.cursor_pad))
            }
            ActionId::Sequencer(SequencerActionId::Chopper) => Action::Nav(NavAction::PushPane("sample_chopper")),
            ActionId::Sequencer(SequencerActionId::ClearPad) => Action::Sequencer(SequencerAction::ClearPad(self.cursor_pad)),
            ActionId::Sequencer(SequencerActionId::ClearPattern) => Action::Sequencer(SequencerAction::ClearPattern),
            ActionId::Sequencer(SequencerActionId::PrevPattern) => Action::Sequencer(SequencerAction::PrevPattern),
            ActionId::Sequencer(SequencerActionId::NextPattern) => Action::Sequencer(SequencerAction::NextPattern),
            ActionId::Sequencer(SequencerActionId::CycleLength) => Action::Sequencer(SequencerAction::CyclePatternLength),
            ActionId::Sequencer(SequencerActionId::ToggleReverse) => Action::Sequencer(SequencerAction::ToggleReverse(self.cursor_pad)),
            ActionId::Sequencer(SequencerActionId::PitchUp) => Action::Sequencer(SequencerAction::AdjustPadPitch(self.cursor_pad, 1)),
            ActionId::Sequencer(SequencerActionId::PitchDown) => Action::Sequencer(SequencerAction::AdjustPadPitch(self.cursor_pad, -1)),
            ActionId::Sequencer(SequencerActionId::PitchUpOctave) => Action::Sequencer(SequencerAction::AdjustPadPitch(self.cursor_pad, 12)),
            ActionId::Sequencer(SequencerActionId::PitchDownOctave) => Action::Sequencer(SequencerAction::AdjustPadPitch(self.cursor_pad, -12)),
            ActionId::Sequencer(SequencerActionId::StepPitchUp) => Action::Sequencer(SequencerAction::AdjustStepPitch(self.cursor_pad, self.cursor_step, 1)),
            ActionId::Sequencer(SequencerActionId::StepPitchDown) => Action::Sequencer(SequencerAction::AdjustStepPitch(self.cursor_pad, self.cursor_step, -1)),
            ActionId::Sequencer(SequencerActionId::AssignInstrument) => {
                Action::Sequencer(SequencerAction::OpenInstrumentPicker(self.cursor_pad))
            }
            ActionId::Sequencer(SequencerActionId::ClearInstrument) => {
                Action::Sequencer(SequencerAction::ClearPadInstrument(self.cursor_pad))
            }
            ActionId::Sequencer(SequencerActionId::FreqUp) => {
                // Increase trigger freq by a semitone
                if let Some(seq) = state.instruments.selected_drum_sequencer() {
                    if let Some(pad) = seq.pads.get(self.cursor_pad) {
                        let new_freq = pad.trigger_freq * 2.0_f32.powf(1.0 / 12.0);
                        return Action::Sequencer(SequencerAction::SetPadTriggerFreq(self.cursor_pad, new_freq));
                    }
                }
                Action::None
            }
            ActionId::Sequencer(SequencerActionId::FreqDown) => {
                // Decrease trigger freq by a semitone
                if let Some(seq) = state.instruments.selected_drum_sequencer() {
                    if let Some(pad) = seq.pads.get(self.cursor_pad) {
                        let new_freq = pad.trigger_freq * 2.0_f32.powf(-1.0 / 12.0);
                        return Action::Sequencer(SequencerAction::SetPadTriggerFreq(self.cursor_pad, new_freq));
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let box_width: u16 = 97;
        let rect = center_rect(area, box_width, 29);

        let border_style = Style::new().fg(Color::ORANGE);

        let seq = match state.instruments.selected_drum_sequencer() {
            Some(s) => s,
            None => {
                let inner = buf.draw_block(rect, " Drum Sequencer ", border_style, border_style);
                let cy = rect.y + rect.height / 2;
                buf.draw_line(
                    Rect::new(inner.x + 11, cy, inner.width.saturating_sub(12), 1),
                    &[("No drum machine instrument selected. Press 1 to add one.", Style::new().fg(Color::DARK_GRAY))],
                );
                return;
            }
        };
        let pattern = seq.pattern();
        let visible = self.visible_steps(box_width);

        // Calculate effective scroll
        let mut view_start = self.view_start_step;
        if self.cursor_step < view_start {
            view_start = self.cursor_step;
        } else if self.cursor_step >= view_start + visible {
            view_start = self.cursor_step - visible + 1;
        }
        if view_start + visible > pattern.length {
            view_start = pattern.length.saturating_sub(visible);
        }

        let steps_shown = visible.min(pattern.length - view_start);

        // Draw box
        let _inner = buf.draw_block(rect, " Drum Sequencer ", border_style, border_style);

        let cx = rect.x + 2;
        let cy = rect.y + 1;

        // Header line
        let pattern_label = match seq.current_pattern {
            0 => "A", 1 => "B", 2 => "C", 3 => "D", _ => "?",
        };
        let play_label = if seq.playing { "PLAY" } else { "STOP" };
        let play_color = if seq.playing { Color::GREEN } else { Color::GRAY };

        let pat_str = format!("Pattern {}", pattern_label);
        let len_str = format!("  Length: {}", pattern.length);
        let bpm_str = format!("  BPM: {:.0}", state.audio.bpm);
        let play_str = format!("  {}", play_label);
        buf.draw_line(Rect::new(cx, cy, rect.width.saturating_sub(4), 1), &[
            (&pat_str, Style::new().fg(Color::WHITE).bold()),
            (&len_str, Style::new().fg(Color::DARK_GRAY)),
            (&bpm_str, Style::new().fg(Color::DARK_GRAY)),
            (&play_str, Style::new().fg(play_color).bold()),
        ]);

        // Step number header
        let header_y = cy + 2;
        let label_width: u16 = 11;
        let step_col_start = cx + label_width;

        let dark_gray = Style::new().fg(Color::DARK_GRAY);
        for i in 0..steps_shown {
            let step_num = view_start + i + 1;
            let x = step_col_start + (i as u16) * 3;
            let num_str = if step_num < 10 {
                format!(" {}", step_num)
            } else {
                format!("{:2}", step_num)
            };
            for (j, ch) in num_str.chars().enumerate() {
                buf.set_cell(x + j as u16, header_y, ch, dark_gray);
            }
        }

        // Grid rows
        let grid_y = header_y + 1;

        for pad_idx in 0..NUM_PADS {
            let y = grid_y + pad_idx as u16;
            let is_cursor_row = pad_idx == self.cursor_pad;

            // Pad label
            let pad = &seq.pads[pad_idx];
            let label = if pad.name.is_empty() {
                format!("{:>2} ----   ", pad_idx + 1)
            } else {
                let name = if pad.name.len() > 6 { &pad.name[..6] } else { &pad.name };
                format!("{:>2} {:<6} ", pad_idx + 1, name)
            };

            let label_style = if is_cursor_row {
                Style::new().fg(Color::WHITE).bold()
            } else {
                Style::new().fg(Color::GRAY)
            };
            for (j, ch) in label.chars().enumerate() {
                buf.set_cell(cx + j as u16, y, ch, label_style);
            }

            // Steps
            for i in 0..steps_shown {
                let step_idx = view_start + i;
                let x = step_col_start + (i as u16) * 3;
                let is_cursor = is_cursor_row && step_idx == self.cursor_step;
                let is_playhead = seq.playing && step_idx == seq.current_step;

                let step = &pattern.steps[pad_idx][step_idx];
                let is_beat = step_idx % 4 == 0;

                let in_selection = self.selection_anchor.map_or(false, |(anchor_pad, anchor_step)| {
                    let (p0, p1) = if anchor_pad <= self.cursor_pad {
                        (anchor_pad, self.cursor_pad)
                    } else {
                        (self.cursor_pad, anchor_pad)
                    };
                    let (s0, s1) = if anchor_step <= self.cursor_step {
                        (anchor_step, self.cursor_step)
                    } else {
                        (self.cursor_step, anchor_step)
                    };
                    pad_idx >= p0 && pad_idx <= p1 && step_idx >= s0 && step_idx <= s1
                });

                let (fg, bg) = if is_cursor {
                    if step.active { (Color::BLACK, Color::WHITE) } else { (Color::WHITE, Color::SELECTION_BG) }
                } else if in_selection {
                    if step.active { (Color::BLACK, Color::new(60, 30, 80)) } else { (Color::WHITE, Color::new(60, 30, 80)) }
                } else if is_playhead {
                    if step.active { (Color::BLACK, Color::GREEN) } else { (Color::GREEN, Color::new(20, 50, 20)) }
                } else if step.active {
                    let intensity = (step.velocity as f32 / 127.0 * 200.0) as u8 + 55;
                    (Color::new(intensity, intensity / 3, 0), Color::BLACK)
                } else if is_beat {
                    (Color::new(60, 60, 60), Color::BLACK)
                } else {
                    (Color::new(40, 40, 40), Color::BLACK)
                };

                let style = Style::new().fg(fg).bg(bg);
                let chars: Vec<char> = if step.active { " █ " } else { " · " }.chars().collect();
                for (j, ch) in chars.iter().enumerate() {
                    buf.set_cell(x + j as u16, y, *ch, style);
                }
            }
        }

        // Pad detail line
        let detail_y = grid_y + NUM_PADS as u16 + 1;
        let pad = &seq.pads[self.cursor_pad];

        if let Some((anchor_pad, anchor_step)) = self.selection_anchor {
            let pads = (self.cursor_pad as i32 - anchor_pad as i32).abs() + 1;
            let steps = (self.cursor_step as i32 - anchor_step as i32).abs() + 1;
            let sel_str = format!("Sel: {} pads x {} steps", pads, steps);
            buf.draw_line(Rect::new(cx, detail_y, 30, 1), &[(&sel_str, Style::new().fg(Color::ORANGE).bold())]);
        } else {
            let pad_label = format!("Pad {:>2}", self.cursor_pad + 1);
            buf.draw_line(Rect::new(cx, detail_y, 8, 1), &[(&pad_label, Style::new().fg(Color::ORANGE).bold())]);

            // Display either instrument trigger info or sample name
            let (name_display, name_color) = if pad.is_instrument_trigger() {
                // Show frequency for instrument triggers
                let freq_str = format!("{:.0}Hz", pad.trigger_freq);
                let name = if pad.name.is_empty() {
                    format!("[Inst] {}", freq_str)
                } else if pad.name.len() > 14 {
                    format!("{} {}", &pad.name[..14], freq_str)
                } else {
                    format!("{} {}", pad.name, freq_str)
                };
                (name, Color::CYAN)
            } else if pad.name.is_empty() {
                ("(empty)".to_string(), Color::DARK_GRAY)
            } else if pad.name.len() > 20 {
                (pad.name[..20].to_string(), Color::WHITE)
            } else {
                (pad.name.clone(), Color::WHITE)
            };
            buf.draw_line(Rect::new(cx + 8, detail_y, 22, 1), &[(&name_display, Style::new().fg(name_color))]);
        }

        // Level bar
        let level_x = cx + 32;
        for (j, ch) in "Level:".chars().enumerate() {
            buf.set_cell(level_x + j as u16, detail_y, ch, dark_gray);
        }

        let bar_x = level_x + 7;
        let bar_width: usize = 10;
        let filled = (pad.level * bar_width as f32) as usize;
        for i in 0..bar_width {
            let (ch, style) = if i < filled {
                ('\u{2588}', Style::new().fg(Color::ORANGE))
            } else {
                ('\u{2591}', Style::new().fg(Color::new(40, 40, 40)))
            };
            buf.set_cell(bar_x + i as u16, detail_y, ch, style);
        }

        // Reverse + Pitch indicators
        let info_x = bar_x + bar_width as u16 + 2;
        let mut info_parts: Vec<String> = Vec::new();
        if pad.reverse { info_parts.push("REV".to_string()); }
        if pad.pitch != 0 { info_parts.push(format!("{:+}st", pad.pitch)); }
        let info_str = info_parts.join(" ");
        for (j, ch) in info_str.chars().enumerate() {
            buf.set_cell(info_x + j as u16, detail_y, ch, Style::new().fg(Color::CYAN));
        }
        let info_offset = if info_str.is_empty() { 0 } else { info_str.len() as u16 + 1 };

        // Velocity
        let step = &pattern.steps[self.cursor_pad][self.cursor_step];
        let vel_str = if step.pitch_offset != 0 {
            format!("Vel: {}  P:{:+}", step.velocity, step.pitch_offset)
        } else {
            format!("Vel: {}", step.velocity)
        };
        for (j, ch) in vel_str.chars().enumerate() {
            buf.set_cell(info_x + info_offset + j as u16, detail_y, ch, dark_gray);
        }

        // Scroll indicator
        if pattern.length > visible {
            let scroll_str = format!("{}-{}/{}", view_start + 1, view_start + steps_shown, pattern.length);
            let scroll_x = rect.x + rect.width - 2 - scroll_str.len() as u16;
            for (j, ch) in scroll_str.chars().enumerate() {
                buf.set_cell(scroll_x + j as u16, detail_y, ch, dark_gray);
            }
        }

        // Help line
        let help_y = rect.y + rect.height - 2;
        buf.draw_line(
            Rect::new(cx, help_y, rect.width.saturating_sub(4), 1),
            &[("Enter:toggle  Space:play  s:sample  i:inst  I:clear  c:chop  r:rev  -/=:pitch", Style::new().fg(Color::DARK_GRAY))],
        );
    }

    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action {
        let box_width: u16 = 97;
        let rect = center_rect(area, box_width, 29);
        let cx = rect.x + 2;
        let header_y = rect.y + 3;
        let label_width: u16 = 11;
        let step_col_start = cx + label_width;
        let grid_y = header_y + 1;
        let visible = self.visible_steps(box_width);

        let seq = match state.instruments.selected_drum_sequencer() {
            Some(s) => s,
            None => return Action::None,
        };
        let pattern = seq.pattern();

        // Calculate effective scroll (same as render)
        let mut view_start = self.view_start_step;
        if self.cursor_step < view_start {
            view_start = self.cursor_step;
        } else if self.cursor_step >= view_start + visible {
            view_start = self.cursor_step - visible + 1;
        }
        if view_start + visible > pattern.length {
            view_start = pattern.length.saturating_sub(visible);
        }

        let col = event.column;
        let row = event.row;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Click on step grid
                if col >= step_col_start && row >= grid_y && row < grid_y + NUM_PADS as u16 {
                    let pad_idx = (row - grid_y) as usize;
                    let step_offset = (col - step_col_start) / 3;
                    let step_idx = view_start + step_offset as usize;
                    if pad_idx < NUM_PADS && step_idx < pattern.length {
                        self.cursor_pad = pad_idx;
                        self.cursor_step = step_idx;
                        return Action::Sequencer(SequencerAction::ToggleStep(pad_idx, step_idx));
                    }
                }
                // Click on pad label to select pad
                if col >= cx && col < step_col_start && row >= grid_y && row < grid_y + NUM_PADS as u16 {
                    let pad_idx = (row - grid_y) as usize;
                    if pad_idx < NUM_PADS {
                        self.cursor_pad = pad_idx;
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                self.cursor_pad = self.cursor_pad.saturating_sub(1);
                Action::None
            }
            MouseEventKind::ScrollDown => {
                self.cursor_pad = (self.cursor_pad + 1).min(NUM_PADS - 1);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, SourceType};
    use crate::ui::{InputEvent, KeyCode, Modifiers};

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn no_drum_sequencer_returns_none() {
        let state = AppState::new();
        let mut pane = SequencerPane::new(Keymap::new());
        let action = pane.handle_action(ActionId::Sequencer(SequencerActionId::Toggle), &dummy_event(), &state);
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn cursor_moves_with_actions_and_toggle_uses_cursor() {
        let mut state = AppState::new();
        state.add_instrument(SourceType::Kit);
        let mut pane = SequencerPane::new(Keymap::new());

        pane.cursor_pad = 0;
        pane.cursor_step = 0;

        pane.handle_action(ActionId::Sequencer(SequencerActionId::Down), &dummy_event(), &state);
        assert_eq!(pane.cursor_pad, 1);

        pane.handle_action(ActionId::Sequencer(SequencerActionId::Right), &dummy_event(), &state);
        assert_eq!(pane.cursor_step, 1);

        let action = pane.handle_action(ActionId::Sequencer(SequencerActionId::Toggle), &dummy_event(), &state);
        match action {
            Action::Sequencer(SequencerAction::ToggleStep(pad, step)) => {
                assert_eq!(pad, pane.cursor_pad);
                assert_eq!(step, pane.cursor_step);
            }
            _ => panic!("Expected ToggleStep"),
        }
    }

    #[test]
    fn chopper_pushes_sample_chopper() {
        let mut state = AppState::new();
        state.add_instrument(SourceType::Kit);
        let mut pane = SequencerPane::new(Keymap::new());

        let action = pane.handle_action(ActionId::Sequencer(SequencerActionId::Chopper), &dummy_event(), &state);
        match action {
            Action::Nav(NavAction::PushPane(id)) => assert_eq!(id, "sample_chopper"),
            _ => panic!("Expected PushPane(sample_chopper)"),
        }
    }
}
