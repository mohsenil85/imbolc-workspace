use std::any::Any;

use crate::action::{Action, MidiAction};
use crate::state::AppState;
use crate::ui::action_id::{ActionId, MidiSettingsActionId};
use crate::ui::{Color, InputEvent, Keymap, Pane, Rect, RenderBuf, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Ports,
    CcMappings,
    Settings,
}

pub struct MidiSettingsPane {
    keymap: Keymap,
    section: Section,
    port_cursor: usize,
    mapping_cursor: usize,
}

impl MidiSettingsPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            section: Section::Ports,
            port_cursor: 0,
            mapping_cursor: 0,
        }
    }
}

impl Pane for MidiSettingsPane {
    fn id(&self) -> &'static str {
        "midi_settings"
    }

    fn handle_action(&mut self, action: ActionId, _event: &InputEvent, state: &AppState) -> Action {
        match action {
            ActionId::MidiSettings(MidiSettingsActionId::SwitchSection) => {
                self.section = match self.section {
                    Section::Ports => Section::CcMappings,
                    Section::CcMappings => Section::Settings,
                    Section::Settings => Section::Ports,
                };
                Action::None
            }
            ActionId::MidiSettings(MidiSettingsActionId::Up) => {
                match self.section {
                    Section::Ports => {
                        self.port_cursor = self.port_cursor.saturating_sub(1);
                    }
                    Section::CcMappings => {
                        self.mapping_cursor = self.mapping_cursor.saturating_sub(1);
                    }
                    Section::Settings => {}
                }
                Action::None
            }
            ActionId::MidiSettings(MidiSettingsActionId::Down) => {
                match self.section {
                    Section::Ports => {
                        let max = state.midi.port_names.len().saturating_sub(1);
                        self.port_cursor = (self.port_cursor + 1).min(max);
                    }
                    Section::CcMappings => {
                        let max = state
                            .session
                            .midi_recording
                            .cc_mappings
                            .len()
                            .saturating_sub(1);
                        self.mapping_cursor = (self.mapping_cursor + 1).min(max);
                    }
                    Section::Settings => {}
                }
                Action::None
            }
            ActionId::MidiSettings(MidiSettingsActionId::Connect) => {
                if self.section == Section::Ports && !state.midi.port_names.is_empty() {
                    Action::Midi(MidiAction::ConnectPort(self.port_cursor))
                } else {
                    Action::None
                }
            }
            ActionId::MidiSettings(MidiSettingsActionId::Disconnect) => {
                Action::Midi(MidiAction::DisconnectPort)
            }
            ActionId::MidiSettings(MidiSettingsActionId::RemoveMapping) => {
                if self.section == Section::CcMappings {
                    let mappings = &state.session.midi_recording.cc_mappings;
                    if let Some(m) = mappings.get(self.mapping_cursor) {
                        let cc = m.cc_number;
                        let ch = m.channel;
                        return Action::Midi(MidiAction::RemoveCcMapping { cc, channel: ch });
                    }
                }
                Action::None
            }
            ActionId::MidiSettings(MidiSettingsActionId::TogglePassthrough) => {
                Action::Midi(MidiAction::ToggleNotePassthrough)
            }
            ActionId::MidiSettings(MidiSettingsActionId::SetChannelAll) => {
                Action::Midi(MidiAction::SetChannelFilter(None))
            }
            ActionId::MidiSettings(MidiSettingsActionId::SetLiveInstrument) => {
                if let Some(inst) = state.instruments.selected_instrument() {
                    Action::Midi(MidiAction::SetLiveInputInstrument(Some(inst.id)))
                } else {
                    Action::None
                }
            }
            ActionId::MidiSettings(MidiSettingsActionId::ClearLiveInstrument) => {
                Action::Midi(MidiAction::SetLiveInputInstrument(None))
            }
            _ => Action::None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let border_style = Style::new().fg(Color::CYAN);
        let inner = buf.draw_block(area, " MIDI Settings ", border_style, border_style);

        if inner.height < 3 || inner.width < 20 {
            return;
        }

        let section_style = |s: Section| {
            if s == self.section {
                Style::new().fg(Color::CYAN).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            }
        };
        let normal = Style::new().fg(Color::GRAY);
        let dim = Style::new().fg(Color::DARK_GRAY);
        let highlight = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold();

        let mut y = inner.y;
        let x = inner.x + 1;
        let w = inner.width.saturating_sub(2);

        // Section: Ports
        let conn_text = if let Some(ref name) = state.midi.connected_port {
            format!("  [Connected: {}]", name)
        } else {
            "  [Not connected]".to_string()
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[
                (" Ports ", section_style(Section::Ports)),
                (&conn_text, dim),
            ],
        );
        y += 1;

        if self.section == Section::Ports {
            if state.midi.port_names.is_empty() {
                buf.draw_line(Rect::new(x, y, w, 1), &[("  (no MIDI ports found)", dim)]);
                y += 1;
            } else {
                for (i, name) in state.midi.port_names.iter().enumerate() {
                    if y >= inner.y + inner.height {
                        break;
                    }
                    let is_connected = state.midi.connected_port.as_deref() == Some(name);
                    let prefix = if is_connected { " * " } else { "   " };
                    let text = format!("{}{}", prefix, name);
                    let style = if i == self.port_cursor {
                        highlight
                    } else {
                        normal
                    };
                    buf.draw_line(Rect::new(x, y, w, 1), &[(&text, style)]);
                    y += 1;
                }
            }
        }
        y += 1;

        // Section: CC Mappings
        if y >= inner.y + inner.height {
            return;
        }
        let mapping_title = format!(
            " CC Mappings ({})",
            state.session.midi_recording.cc_mappings.len()
        );
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(&mapping_title, section_style(Section::CcMappings))],
        );
        y += 1;

        if self.section == Section::CcMappings {
            if state.session.midi_recording.cc_mappings.is_empty() {
                if y < inner.y + inner.height {
                    buf.draw_line(Rect::new(x, y, w, 1), &[("  (no CC mappings)", dim)]);
                    y += 1;
                }
            } else {
                for (i, mapping) in state.session.midi_recording.cc_mappings.iter().enumerate() {
                    if y >= inner.y + inner.height {
                        break;
                    }
                    let ch_str = match mapping.channel {
                        Some(ch) => format!("ch{}", ch + 1),
                        None => "any".to_string(),
                    };
                    let text = format!(
                        "  CC{:<3} {} -> {}",
                        mapping.cc_number,
                        ch_str,
                        mapping.target.name()
                    );
                    let style = if i == self.mapping_cursor {
                        highlight
                    } else {
                        normal
                    };
                    buf.draw_line(Rect::new(x, y, w, 1), &[(&text, style)]);
                    y += 1;
                }
            }
        }
        y += 1;

        // Section: Settings
        if y >= inner.y + inner.height {
            return;
        }
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(" Settings", section_style(Section::Settings))],
        );
        y += 1;

        if self.section == Section::Settings {
            let settings = [
                format!(
                    "  Note passthrough: {}",
                    if state.session.midi_recording.note_passthrough {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                format!(
                    "  Channel filter: {}",
                    match state.session.midi_recording.channel_filter {
                        Some(ch) => format!("Ch {}", ch + 1),
                        None => "All".to_string(),
                    }
                ),
                format!(
                    "  Live input instrument: {}",
                    match state.session.midi_recording.live_input_instrument {
                        Some(id) => {
                            state
                                .instruments
                                .instruments
                                .iter()
                                .find(|i| i.id == id)
                                .map(|i| i.name.clone())
                                .unwrap_or_else(|| format!("#{}", id))
                        }
                        None => "(selected)".to_string(),
                    }
                ),
            ];

            for line in &settings {
                if y >= inner.y + inner.height {
                    break;
                }
                buf.draw_line(Rect::new(x, y, w, 1), &[(line.as_str(), normal)]);
                y += 1;
            }
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

    #[test]
    fn midi_settings_pane_id() {
        let pane = MidiSettingsPane::new(Keymap::new());
        assert_eq!(pane.id(), "midi_settings");
    }
}
