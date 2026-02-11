use super::{BufferSize, ScsynthArgsDialogButton, ServerPane, ServerPaneFocus};
use crate::state::AppState;
use crate::ui::action_id::{ActionId, ModeActionId, ServerActionId};
use crate::ui::{Action, InputEvent, KeyCode, ServerAction};

impl ServerPane {
    fn begin_scsynth_args_edit(&mut self) -> Action {
        self.editing_scsynth_args = true;
        self.scsynth_args_edit = self.scsynth_args.clone();
        self.scsynth_args_dialog_button = ScsynthArgsDialogButton::ApplyRestart;
        Action::PushLayer("text_edit")
    }

    fn finish_scsynth_args_edit(&mut self, apply: bool) -> Action {
        self.editing_scsynth_args = false;
        if apply {
            self.scsynth_args = self.scsynth_args_edit.trim().to_string();
            self.save_config();
            if self.server_running {
                return Action::Server(ServerAction::Restart {
                    input_device: self.selected_input_device(),
                    output_device: self.selected_output_device(),
                    buffer_size: self.selected_buffer_size().as_samples(),
                    sample_rate: self.sample_rate(),
                    scsynth_args: self.scsynth_args(),
                });
            }
        }
        Action::None
    }

    fn handle_scsynth_args_edit_key(&mut self, event: &InputEvent) {
        match event.key {
            KeyCode::Char(c) if !event.modifiers.ctrl && !event.modifiers.alt => {
                self.scsynth_args_edit.push(c);
            }
            KeyCode::Backspace => {
                self.scsynth_args_edit.pop();
            }
            KeyCode::Delete => {
                self.scsynth_args_edit.clear();
            }
            KeyCode::Tab => {
                self.scsynth_args_dialog_button = match self.scsynth_args_dialog_button {
                    ScsynthArgsDialogButton::Cancel => ScsynthArgsDialogButton::ApplyRestart,
                    ScsynthArgsDialogButton::ApplyRestart => ScsynthArgsDialogButton::Cancel,
                };
            }
            KeyCode::Left => {
                self.scsynth_args_dialog_button = ScsynthArgsDialogButton::Cancel;
            }
            KeyCode::Right => {
                self.scsynth_args_dialog_button = ScsynthArgsDialogButton::ApplyRestart;
            }
            _ => {}
        }
    }

    pub(super) fn handle_action_impl(
        &mut self,
        action: ActionId,
        _event: &InputEvent,
        _state: &AppState,
    ) -> Action {
        if self.editing_scsynth_args {
            return match action {
                ActionId::Mode(ModeActionId::TextConfirm) => {
                    let apply = matches!(
                        self.scsynth_args_dialog_button,
                        ScsynthArgsDialogButton::ApplyRestart
                    );
                    self.finish_scsynth_args_edit(apply)
                }
                ActionId::Mode(ModeActionId::TextCancel) => self.finish_scsynth_args_edit(false),
                _ => Action::None,
            };
        }

        let ActionId::Server(action) = action else {
            return Action::None;
        };

        match action {
            ServerActionId::Start => Action::Server(ServerAction::Start {
                input_device: self.selected_input_device(),
                output_device: self.selected_output_device(),
                buffer_size: self.selected_buffer_size().as_samples(),
                sample_rate: self.sample_rate(),
                scsynth_args: self.scsynth_args(),
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
                        scsynth_args: self.scsynth_args(),
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

    pub(super) fn handle_raw_input_impl(
        &mut self,
        event: &InputEvent,
        _state: &AppState,
    ) -> Action {
        if self.editing_scsynth_args {
            self.handle_scsynth_args_edit_key(event);
            return Action::None;
        }

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
                                scsynth_args: self.scsynth_args(),
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
                                scsynth_args: self.scsynth_args(),
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
                                scsynth_args: self.scsynth_args(),
                            });
                        } else {
                            self.device_config_dirty = true;
                            return Action::None;
                        }
                    }
                    _ => {}
                }
            }
            ServerPaneFocus::ScsynthArgs => {
                if let KeyCode::Enter = event.key {
                    return self.begin_scsynth_args_edit();
                }
            }
            ServerPaneFocus::Controls => {}
        }

        Action::None
    }
}
