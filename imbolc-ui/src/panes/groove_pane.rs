use std::any::Any;

use crate::state::{AppState, InstrumentId, SwingGrid};
use crate::ui::action_id::{ActionId, GrooveActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Action, Color, InputEvent, InstrumentAction, Keymap, Pane, Rect, RenderBuf, Style};

/// Parameter indices for the groove pane
const PARAM_SWING: usize = 0;
const PARAM_SWING_GRID: usize = 1;
const PARAM_HUMANIZE_VEL: usize = 2;
const PARAM_HUMANIZE_TIME: usize = 3;
const PARAM_TIMING_OFFSET: usize = 4;
const PARAM_COUNT: usize = 5;

pub struct GroovePane {
    keymap: Keymap,
    selected_param: usize,
}

impl GroovePane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected_param: 0,
        }
    }
}

impl Default for GroovePane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for GroovePane {
    fn id(&self) -> &'static str {
        "groove"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let instrument = match state.instruments.selected_instrument() {
            Some(i) => i,
            None => return Action::None,
        };
        let instrument_id = instrument.id;
        let groove = &instrument.groove;

        match action {
            ActionId::Groove(GrooveActionId::PrevParam) => {
                self.selected_param = self.selected_param.saturating_sub(1);
                Action::None
            }
            ActionId::Groove(GrooveActionId::NextParam) => {
                self.selected_param = (self.selected_param + 1).min(PARAM_COUNT - 1);
                Action::None
            }
            ActionId::Groove(GrooveActionId::Increase)
            | ActionId::Groove(GrooveActionId::IncreaseBig)
            | ActionId::Groove(GrooveActionId::IncreaseTiny) => {
                adjust_param(instrument_id, groove, self.selected_param, true, action)
            }
            ActionId::Groove(GrooveActionId::Decrease)
            | ActionId::Groove(GrooveActionId::DecreaseBig)
            | ActionId::Groove(GrooveActionId::DecreaseTiny) => {
                adjust_param(instrument_id, groove, self.selected_param, false, action)
            }
            ActionId::Groove(GrooveActionId::CycleSwingGrid) => {
                let current = groove.swing_grid.unwrap_or(SwingGrid::Eighths);
                let next = current.next();
                Action::Instrument(InstrumentAction::SetTrackSwingGrid(instrument_id, Some(next)))
            }
            ActionId::Groove(GrooveActionId::CycleTimeSig) => {
                Action::Instrument(InstrumentAction::CycleTrackTimeSignature(instrument_id))
            }
            ActionId::Groove(GrooveActionId::Reset) => {
                Action::Instrument(InstrumentAction::ResetTrackGroove(instrument_id))
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 40, 12);

        let instrument = state.instruments.selected_instrument();
        let title = match instrument {
            Some(i) => format!(" Groove: {} ", i.name),
            None => " Groove: (none) ".to_string(),
        };

        let border_color = Color::new(180, 140, 80);
        let border_style = Style::new().fg(border_color);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let instrument = match instrument {
            Some(i) => i,
            None => {
                render_centered_text(inner, buf, "(no instrument selected)", Color::DARK_GRAY);
                return;
            }
        };

        let groove = &instrument.groove;
        let global_swing = state.session.piano_roll.swing_amount;
        let global_grid = SwingGrid::Eighths; // Default global grid
        let global_humanize_vel = state.session.humanize.velocity;
        let global_humanize_time = state.session.humanize.timing;

        // Calculate effective values
        let swing = groove.effective_swing(global_swing);
        let swing_grid = groove.effective_swing_grid(global_grid);
        let humanize_vel = groove.effective_humanize_velocity(global_humanize_vel);
        let humanize_time = groove.effective_humanize_timing(global_humanize_time);
        let timing_offset = groove.timing_offset_ms;

        // Is using global?
        let swing_is_global = groove.swing_amount.is_none();
        let grid_is_global = groove.swing_grid.is_none();
        let hvel_is_global = groove.humanize_velocity.is_none();
        let htime_is_global = groove.humanize_timing.is_none();

        let y = inner.y + 1;
        let label_x = inner.x + 2;
        let value_x = inner.x + 18;

        let normal_style = Style::new().fg(Color::WHITE);
        let global_style = Style::new().fg(Color::DARK_GRAY);
        let selected_style = Style::new().fg(Color::new(255, 200, 50));

        // Swing amount
        render_param_row(
            buf, label_x, value_x, y,
            "Swing:",
            &format!("{:.0}%", swing * 100.0),
            swing_is_global,
            self.selected_param == PARAM_SWING,
            normal_style, global_style, selected_style,
        );

        // Swing grid
        render_param_row(
            buf, label_x, value_x, y + 1,
            "Swing Grid:",
            swing_grid.name(),
            grid_is_global,
            self.selected_param == PARAM_SWING_GRID,
            normal_style, global_style, selected_style,
        );

        // Humanize velocity
        render_param_row(
            buf, label_x, value_x, y + 2,
            "Humanize Vel:",
            &format!("{:.0}%", humanize_vel * 100.0),
            hvel_is_global,
            self.selected_param == PARAM_HUMANIZE_VEL,
            normal_style, global_style, selected_style,
        );

        // Humanize timing
        render_param_row(
            buf, label_x, value_x, y + 3,
            "Humanize Time:",
            &format!("{:.0}%", humanize_time * 100.0),
            htime_is_global,
            self.selected_param == PARAM_HUMANIZE_TIME,
            normal_style, global_style, selected_style,
        );

        // Timing offset
        let offset_str = if timing_offset >= 0.0 {
            format!("+{:.1}ms", timing_offset)
        } else {
            format!("{:.1}ms", timing_offset)
        };
        render_param_row(
            buf, label_x, value_x, y + 4,
            "Push/Pull:",
            &offset_str,
            false, // Timing offset has no global default
            self.selected_param == PARAM_TIMING_OFFSET,
            normal_style, global_style, selected_style,
        );

        // Reset hint
        let hint_y = y + 6;
        let hint = "[r] Reset to global";
        let hint_style = Style::new().fg(Color::DARK_GRAY);
        render_text_at(inner.x + 2, hint_y, hint, hint_style, inner.width, buf);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// -- Helpers --

fn render_centered_text(area: Rect, buf: &mut RenderBuf, text: &str, color: Color) {
    let x = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
    let y = area.y + area.height / 2;
    let style = Style::new().fg(color);
    buf.draw_line(Rect::new(x, y, text.len() as u16, 1), &[(text, style)]);
}

fn render_param_row(
    buf: &mut RenderBuf,
    label_x: u16,
    value_x: u16,
    y: u16,
    label: &str,
    value: &str,
    is_global: bool,
    is_selected: bool,
    normal_style: Style,
    global_style: Style,
    selected_style: Style,
) {
    let label_style = if is_selected { selected_style } else { normal_style };
    let value_style = if is_selected {
        selected_style
    } else if is_global {
        global_style
    } else {
        normal_style
    };

    // Render label
    for (i, ch) in label.chars().enumerate() {
        buf.set_cell(label_x + i as u16, y, ch, label_style);
    }

    // Render value
    for (i, ch) in value.chars().enumerate() {
        buf.set_cell(value_x + i as u16, y, ch, value_style);
    }

    // Render "(global)" suffix if using global
    if is_global && !is_selected {
        let suffix = " (global)";
        for (i, ch) in suffix.chars().enumerate() {
            buf.set_cell(value_x + value.len() as u16 + i as u16, y, ch, global_style);
        }
    }
}

fn render_text_at(x: u16, y: u16, text: &str, style: Style, max_width: u16, buf: &mut RenderBuf) {
    for (i, ch) in text.chars().enumerate() {
        let px = x + i as u16;
        if px < x + max_width {
            buf.set_cell(px, y, ch, style);
        }
    }
}

fn adjust_param(
    instrument_id: InstrumentId,
    groove: &crate::state::GrooveConfig,
    param_idx: usize,
    increase: bool,
    action: ActionId,
) -> Action {
    match param_idx {
        PARAM_SWING => {
            let delta = match action {
                ActionId::Groove(GrooveActionId::IncreaseBig)
                | ActionId::Groove(GrooveActionId::DecreaseBig) => 0.1,
                ActionId::Groove(GrooveActionId::IncreaseTiny)
                | ActionId::Groove(GrooveActionId::DecreaseTiny) => 0.01,
                _ => 0.05,
            };
            let signed_delta = if increase { delta } else { -delta };
            Action::Instrument(InstrumentAction::AdjustTrackSwing(instrument_id, signed_delta))
        }
        PARAM_SWING_GRID => {
            // Cycle swing grid
            let current = groove.swing_grid.unwrap_or(SwingGrid::Eighths);
            let next = if increase { current.next() } else { cycle_swing_grid_rev(current) };
            Action::Instrument(InstrumentAction::SetTrackSwingGrid(instrument_id, Some(next)))
        }
        PARAM_HUMANIZE_VEL => {
            let delta = match action {
                ActionId::Groove(GrooveActionId::IncreaseBig)
                | ActionId::Groove(GrooveActionId::DecreaseBig) => 0.1,
                ActionId::Groove(GrooveActionId::IncreaseTiny)
                | ActionId::Groove(GrooveActionId::DecreaseTiny) => 0.01,
                _ => 0.05,
            };
            let signed_delta = if increase { delta } else { -delta };
            Action::Instrument(InstrumentAction::AdjustTrackHumanizeVelocity(instrument_id, signed_delta))
        }
        PARAM_HUMANIZE_TIME => {
            let delta = match action {
                ActionId::Groove(GrooveActionId::IncreaseBig)
                | ActionId::Groove(GrooveActionId::DecreaseBig) => 0.1,
                ActionId::Groove(GrooveActionId::IncreaseTiny)
                | ActionId::Groove(GrooveActionId::DecreaseTiny) => 0.01,
                _ => 0.05,
            };
            let signed_delta = if increase { delta } else { -delta };
            Action::Instrument(InstrumentAction::AdjustTrackHumanizeTiming(instrument_id, signed_delta))
        }
        PARAM_TIMING_OFFSET => {
            let delta = match action {
                ActionId::Groove(GrooveActionId::IncreaseBig)
                | ActionId::Groove(GrooveActionId::DecreaseBig) => 5.0,
                ActionId::Groove(GrooveActionId::IncreaseTiny)
                | ActionId::Groove(GrooveActionId::DecreaseTiny) => 0.5,
                _ => 1.0,
            };
            let signed_delta = if increase { delta } else { -delta };
            Action::Instrument(InstrumentAction::AdjustTrackTimingOffset(instrument_id, signed_delta))
        }
        _ => Action::None,
    }
}

fn cycle_swing_grid_rev(grid: SwingGrid) -> SwingGrid {
    match grid {
        SwingGrid::Eighths => SwingGrid::Both,
        SwingGrid::Sixteenths => SwingGrid::Eighths,
        SwingGrid::Both => SwingGrid::Sixteenths,
    }
}
