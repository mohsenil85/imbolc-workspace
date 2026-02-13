use std::any::Any;

use crate::state::{AppState, InstrumentId};
use crate::ui::action_id::{ActionId, ArpeggiatorActionId};
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Action, Color, InputEvent, InstrumentAction, Keymap, Pane, Rect, RenderBuf, Style};

/// Parameter indices for the arpeggiator pane
const PARAM_ENABLED: usize = 0;
const PARAM_DIRECTION: usize = 1;
const PARAM_RATE: usize = 2;
const PARAM_OCTAVES: usize = 3;
const PARAM_GATE: usize = 4;
const PARAM_CHORD: usize = 5;
const PARAM_COUNT: usize = 6;

pub struct ArpeggiatorPane {
    keymap: Keymap,
    selected_param: usize,
}

impl ArpeggiatorPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            selected_param: 0,
        }
    }
}

impl Default for ArpeggiatorPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for ArpeggiatorPane {
    fn id(&self) -> &'static str {
        "arpeggiator"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        let instrument = match state.instruments.selected_instrument() {
            Some(i) => i,
            None => return Action::None,
        };
        let id = instrument.id;

        match action {
            ActionId::Arpeggiator(ArpeggiatorActionId::PrevParam) => {
                self.selected_param = self.selected_param.saturating_sub(1);
                Action::None
            }
            ActionId::Arpeggiator(ArpeggiatorActionId::NextParam) => {
                self.selected_param = (self.selected_param + 1).min(PARAM_COUNT - 1);
                Action::None
            }
            ActionId::Arpeggiator(ArpeggiatorActionId::Toggle) => {
                Action::Instrument(InstrumentAction::ToggleArp(id))
            }
            ActionId::Arpeggiator(ArpeggiatorActionId::Increase) => {
                adjust_param(id, self.selected_param, true)
            }
            ActionId::Arpeggiator(ArpeggiatorActionId::Decrease) => {
                adjust_param(id, self.selected_param, false)
            }
            ActionId::Arpeggiator(ArpeggiatorActionId::ClearChord) => {
                Action::Instrument(InstrumentAction::ClearChordShape(id))
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 44, 12);

        let instrument = state.instruments.selected_instrument();
        let title = match instrument {
            Some(i) => format!(" Arpeggiator: {} ", i.name),
            None => " Arpeggiator: (none) ".to_string(),
        };

        let border_color = Color::new(120, 160, 220);
        let border_style = Style::new().fg(border_color);
        let inner = buf.draw_block(rect, &title, border_style, border_style);

        let instrument = match instrument {
            Some(i) => i,
            None => {
                render_centered_text(inner, buf, "(no instrument selected)", Color::DARK_GRAY);
                return;
            }
        };

        let arp = &instrument.note_input.arpeggiator;
        let chord = &instrument.note_input.chord_shape;

        let y = inner.y + 1;
        let label_x = inner.x + 2;
        let value_x = inner.x + 18;

        let normal_style = Style::new().fg(Color::WHITE);
        let selected_style = Style::new().fg(Color::new(100, 180, 255));
        let on_style = Style::new().fg(Color::new(100, 220, 100));
        let off_style = Style::new().fg(Color::DARK_GRAY);

        // Enabled
        let enabled_str = if arp.enabled { "ON" } else { "OFF" };
        let enabled_val_style = if self.selected_param == PARAM_ENABLED {
            selected_style
        } else if arp.enabled {
            on_style
        } else {
            off_style
        };
        render_param_row(buf, label_x, value_x, y, "Enabled:", enabled_str,
            self.selected_param == PARAM_ENABLED, normal_style, selected_style, enabled_val_style);

        // Direction
        render_param_row(buf, label_x, value_x, y + 1, "Direction:", arp.direction.name(),
            self.selected_param == PARAM_DIRECTION, normal_style, selected_style, normal_style);

        // Rate
        render_param_row(buf, label_x, value_x, y + 2, "Rate:", arp.rate.name(),
            self.selected_param == PARAM_RATE, normal_style, selected_style, normal_style);

        // Octaves
        let octaves_str = format!("{}", arp.octaves);
        render_param_row(buf, label_x, value_x, y + 3, "Octaves:", &octaves_str,
            self.selected_param == PARAM_OCTAVES, normal_style, selected_style, normal_style);

        // Gate
        let gate_str = format!("{:.0}%", arp.gate * 100.0);
        render_param_row(buf, label_x, value_x, y + 4, "Gate:", &gate_str,
            self.selected_param == PARAM_GATE, normal_style, selected_style, normal_style);

        // Chord shape
        let chord_str = match chord {
            Some(shape) => shape.name(),
            None => "None",
        };
        render_param_row(buf, label_x, value_x, y + 5, "Chord:", chord_str,
            self.selected_param == PARAM_CHORD, normal_style, selected_style, normal_style);
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

#[allow(clippy::too_many_arguments)]
fn render_param_row(
    buf: &mut RenderBuf,
    label_x: u16,
    value_x: u16,
    y: u16,
    label: &str,
    value: &str,
    is_selected: bool,
    normal_style: Style,
    selected_style: Style,
    value_style_override: Style,
) {
    let label_style = if is_selected { selected_style } else { normal_style };
    let value_style = if is_selected { selected_style } else { value_style_override };

    for (i, ch) in label.chars().enumerate() {
        buf.set_cell(label_x + i as u16, y, ch, label_style);
    }
    for (i, ch) in value.chars().enumerate() {
        buf.set_cell(value_x + i as u16, y, ch, value_style);
    }
}

fn adjust_param(id: InstrumentId, param_idx: usize, increase: bool) -> Action {
    match param_idx {
        PARAM_ENABLED => Action::Instrument(InstrumentAction::ToggleArp(id)),
        PARAM_DIRECTION => {
            if increase {
                Action::Instrument(InstrumentAction::CycleArpDirection(id))
            } else {
                Action::Instrument(InstrumentAction::CycleArpDirectionReverse(id))
            }
        }
        PARAM_RATE => {
            if increase {
                Action::Instrument(InstrumentAction::CycleArpRate(id))
            } else {
                Action::Instrument(InstrumentAction::CycleArpRateReverse(id))
            }
        }
        PARAM_OCTAVES => {
            let delta = if increase { 1 } else { -1 };
            Action::Instrument(InstrumentAction::AdjustArpOctaves(id, delta))
        }
        PARAM_GATE => {
            let delta = if increase { 0.1 } else { -0.1 };
            Action::Instrument(InstrumentAction::AdjustArpGate(id, delta))
        }
        PARAM_CHORD => {
            if increase {
                Action::Instrument(InstrumentAction::CycleChordShape(id))
            } else {
                Action::Instrument(InstrumentAction::CycleChordShapeReverse(id))
            }
        }
        _ => Action::None,
    }
}
