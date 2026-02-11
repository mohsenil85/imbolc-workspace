use super::{ServerPane, ServerPaneFocus, BufferSize};
use crate::state::AppState;
use crate::ui::action_id::{ActionId, ServerActionId};
use crate::ui::{Action, InputEvent, KeyCode, ServerAction};

impl ServerPane {
    pub(super) fn handle_action_impl(&mut self, action: ActionId, _event: &InputEvent, _state: &AppState) -> Action {
        let ActionId::Server(action) = action else {
            return Action::None;
        };

        match action {
            ServerActionId::Start => Action::Server(ServerAction::Start {
                input_device: self.selected_input_device(),
                output_device: self.selected_output_device(),
                buffer_size: self.selected_buffer_size().as_samples(),
                sample_rate: self.sample_rate(),
            }),
            ServerActionId::Stop => Action::Server(ServerAction::Stop),
            ServerActionId::Connect => Action::Server(ServerAction::Connect),
            ServerActionId::Disconnect => Action::Server(ServerAction::Disconnect),
            ServerActionId::Compile => Action::Server(ServerAction::CompileSynthDefs),
            ServerActionId::CompileVst => Action::Server(ServerAction::CompileVstSynthDefs),
            ServerActionId::LoadSynthDefs => Action::Server(ServerAction::LoadSynthDefs),
            ServerActionId::RecordMaster => Action::Server(ServerAction::RecordMaster),
            ServerActionId::RefreshDevices => {
                self.refresh_devices();
                self.refresh_diagnostics();
                self.refresh_log();
                if self.server_running {
                    Action::Server(ServerAction::Restart {
                        input_device: self.selected_input_device(),
                        output_device: self.selected_output_device(),
                        buffer_size: self.selected_buffer_size().as_samples(),
                        sample_rate: self.sample_rate(),
                    })
                } else {
                    Action::None
                }
            }
            ServerActionId::NextSection => {
                self.cycle_focus();
                Action::None
            }
        }
    }

    pub(super) fn handle_raw_input_impl(&mut self, event: &InputEvent, _state: &AppState) -> Action {
        match self.focus {
            ServerPaneFocus::OutputDevice => {
                let count = self.output_devices().len() + 1;
                match event.key {
                    KeyCode::Up => {
                        self.selected_output = if self.selected_output == 0 {
                            count - 1
                        } else {
                            self.selected_output - 1
                        };
                        return Action::None;
                    }
                    KeyCode::Down => {
                        self.selected_output = (self.selected_output + 1) % count;
                        return Action::None;
                    }
                    KeyCode::Enter => {
                        self.save_config();
                        if self.server_running {
                            self.device_config_dirty = false;
                            return Action::Server(ServerAction::Restart {
                                input_device: self.selected_input_device(),
                                output_device: self.selected_output_device(),
                                buffer_size: self.selected_buffer_size().as_samples(),
                                sample_rate: self.sample_rate(),
                            });
                        } else {
                            self.device_config_dirty = true;
                            return Action::None;
                        }
                    }
                    _ => {}
                }
            }
            ServerPaneFocus::InputDevice => {
                let count = self.input_devices().len() + 1;
                match event.key {
                    KeyCode::Up => {
                        self.selected_input = if self.selected_input == 0 {
                            count - 1
                        } else {
                            self.selected_input - 1
                        };
                        return Action::None;
                    }
                    KeyCode::Down => {
                        self.selected_input = (self.selected_input + 1) % count;
                        return Action::None;
                    }
                    KeyCode::Enter => {
                        self.save_config();
                        if self.server_running {
                            self.device_config_dirty = false;
                            return Action::Server(ServerAction::Restart {
                                input_device: self.selected_input_device(),
                                output_device: self.selected_output_device(),
                                buffer_size: self.selected_buffer_size().as_samples(),
                                sample_rate: self.sample_rate(),
                            });
                        } else {
                            self.device_config_dirty = true;
                            return Action::None;
                        }
                    }
                    _ => {}
                }
            }
            ServerPaneFocus::BufferSize => {
                let count = BufferSize::ALL.len();
                match event.key {
                    KeyCode::Up => {
                        self.selected_buffer_size = if self.selected_buffer_size == 0 {
                            count - 1
                        } else {
                            self.selected_buffer_size - 1
                        };
                        return Action::None;
                    }
                    KeyCode::Down => {
                        self.selected_buffer_size = (self.selected_buffer_size + 1) % count;
                        return Action::None;
                    }
                    KeyCode::Enter => {
                        self.save_config();
                        if self.server_running {
                            self.device_config_dirty = false;
                            return Action::Server(ServerAction::Restart {
                                input_device: self.selected_input_device(),
                                output_device: self.selected_output_device(),
                                buffer_size: self.selected_buffer_size().as_samples(),
                                sample_rate: self.sample_rate(),
                            });
                        } else {
                            self.device_config_dirty = true;
                            return Action::None;
                        }
                    }
                    _ => {}
                }
            }
            ServerPaneFocus::Controls => {}
        }

        Action::None
    }
}
