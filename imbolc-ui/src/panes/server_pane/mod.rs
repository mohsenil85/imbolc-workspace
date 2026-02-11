mod input;
mod rendering;

use std::any::Any;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::audio::devices::{self, AudioDevice, BufferSize};
use crate::audio::ServerStatus;
use crate::state::AppState;
use crate::ui::action_id::ActionId;
use crate::ui::{Action, InputEvent, Keymap, Pane, Rect, RenderBuf};

pub(super) struct DiagnosticCheck {
    pub label: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ServerPaneFocus {
    Controls,
    OutputDevice,
    InputDevice,
    BufferSize,
    ScsynthArgs,
}

pub struct ServerPane {
    keymap: Keymap,
    status: ServerStatus,
    message: String,
    server_running: bool,
    devices: Vec<AudioDevice>,
    selected_output: usize,
    selected_input: usize,
    selected_buffer_size: usize, // index into BufferSize::ALL
    sample_rate: u32,
    focus: ServerPaneFocus,
    scsynth_args: String,
    editing_scsynth_args: bool,
    scsynth_args_edit: String,
    device_config_dirty: bool,
    log_lines: Vec<String>,
    log_path: PathBuf,
    pub(super) diagnostics: Vec<DiagnosticCheck>,
}

impl ServerPane {
    pub fn new(keymap: Keymap) -> Self {
        let devices = devices::enumerate_devices();
        let config = devices::load_device_config();

        let selected_output = match &config.output_device {
            Some(name) => {
                let outputs: Vec<_> = devices
                    .iter()
                    .filter(|d| d.output_channels.is_some_and(|c| c > 0))
                    .collect();
                outputs
                    .iter()
                    .position(|d| d.name == *name)
                    .map(|i| i + 1)
                    .unwrap_or(0)
            }
            None => 0,
        };
        let selected_input = match &config.input_device {
            Some(name) => {
                let inputs: Vec<_> = devices
                    .iter()
                    .filter(|d| d.input_channels.is_some_and(|c| c > 0))
                    .collect();
                inputs
                    .iter()
                    .position(|d| d.name == *name)
                    .map(|i| i + 1)
                    .unwrap_or(0)
            }
            None => 0,
        };

        // Find index of buffer size in ALL array
        let selected_buffer_size = BufferSize::ALL
            .iter()
            .position(|&bs| bs == config.buffer_size)
            .unwrap_or(3); // default to B512 (index 3)

        let log_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("imbolc")
            .join("scsynth.log");

        let mut pane = Self {
            keymap,
            status: ServerStatus::Stopped,
            message: String::new(),
            server_running: false,
            devices,
            selected_output,
            selected_input,
            selected_buffer_size,
            sample_rate: config.sample_rate,
            focus: ServerPaneFocus::Controls,
            scsynth_args: config.scsynth_args,
            editing_scsynth_args: false,
            scsynth_args_edit: String::new(),
            device_config_dirty: false,
            log_lines: Vec::new(),
            log_path,
            diagnostics: Vec::new(),
        };
        pane.refresh_diagnostics();
        pane
    }

    pub fn set_status(&mut self, status: ServerStatus, message: &str) {
        self.status = status;
        self.message = message.to_string();
        self.refresh_log();
    }

    pub fn set_server_running(&mut self, running: bool) {
        self.server_running = running;
        self.refresh_log();
    }

    pub fn refresh_log(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.log_path) {
            self.log_lines = content
                .lines()
                .rev()
                .take(50)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(String::from)
                .collect();
        }
    }

    #[allow(dead_code)]
    pub fn clear_device_config_dirty(&mut self) {
        self.device_config_dirty = false;
    }

    pub fn selected_output_device(&self) -> Option<String> {
        if self.selected_output == 0 {
            return None;
        }
        self.output_devices()
            .get(self.selected_output - 1)
            .map(|d| d.name.clone())
    }

    pub fn selected_input_device(&self) -> Option<String> {
        if self.selected_input == 0 {
            return None;
        }
        self.input_devices()
            .get(self.selected_input - 1)
            .map(|d| d.name.clone())
    }

    pub fn selected_buffer_size(&self) -> BufferSize {
        BufferSize::ALL
            .get(self.selected_buffer_size)
            .copied()
            .unwrap_or_default()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn scsynth_args(&self) -> String {
        self.scsynth_args.clone()
    }

    pub fn is_editing_scsynth_args(&self) -> bool {
        self.editing_scsynth_args
    }

    fn output_devices(&self) -> Vec<&AudioDevice> {
        self.devices
            .iter()
            .filter(|d| d.output_channels.is_some_and(|c| c > 0))
            .collect()
    }

    fn input_devices(&self) -> Vec<&AudioDevice> {
        self.devices
            .iter()
            .filter(|d| d.input_channels.is_some_and(|c| c > 0))
            .collect()
    }

    fn refresh_devices(&mut self) {
        let old_output = self.selected_output_device();
        let old_input = self.selected_input_device();

        self.devices = devices::enumerate_devices();

        self.selected_output = match &old_output {
            Some(name) => self
                .output_devices()
                .iter()
                .position(|d| d.name == *name)
                .map(|i| i + 1)
                .unwrap_or(0),
            None => 0,
        };
        self.selected_input = match &old_input {
            Some(name) => self
                .input_devices()
                .iter()
                .position(|d| d.name == *name)
                .map(|i| i + 1)
                .unwrap_or(0),
            None => 0,
        };
    }

    pub(super) fn refresh_diagnostics(&mut self) {
        self.diagnostics.clear();

        // 1. scsynth
        let scsynth_paths = &[
            "scsynth",
            "/Applications/SuperCollider.app/Contents/Resources/scsynth",
            "/usr/local/bin/scsynth",
            "/usr/bin/scsynth",
        ];
        match Self::find_executable(scsynth_paths) {
            Some(path) => self.diagnostics.push(DiagnosticCheck {
                label: format!("scsynth ({})", path),
                passed: true,
            }),
            None => self.diagnostics.push(DiagnosticCheck {
                label: "scsynth (not found)".to_string(),
                passed: false,
            }),
        }

        // 2. sclang
        let sclang_paths = &[
            "sclang",
            "/Applications/SuperCollider.app/Contents/MacOS/sclang",
            "/usr/local/bin/sclang",
            "/usr/bin/sclang",
        ];
        match Self::find_executable(sclang_paths) {
            Some(path) => self.diagnostics.push(DiagnosticCheck {
                label: format!("sclang ({})", path),
                passed: true,
            }),
            None => self.diagnostics.push(DiagnosticCheck {
                label: "sclang (not found)".to_string(),
                passed: false,
            }),
        }

        // 3. Synthdefs
        let builtin_dir = Path::new("synthdefs");
        let builtin_count = Self::count_scsyndef_files(builtin_dir);
        let custom_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("imbolc")
            .join("synthdefs");
        let custom_count = Self::count_scsyndef_files(&custom_dir);
        let total = builtin_count + custom_count;
        self.diagnostics.push(DiagnosticCheck {
            label: format!(
                "Synthdefs ({} built-in, {} custom)",
                builtin_count, custom_count
            ),
            passed: total > 0,
        });

        // 4. Audio devices
        let device_count = self.devices.len();
        self.diagnostics.push(DiagnosticCheck {
            label: format!("Audio devices ({} found)", device_count),
            passed: device_count > 0,
        });

        // 5. Config directory
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("imbolc");
        let config_exists = config_dir.is_dir();
        self.diagnostics.push(DiagnosticCheck {
            label: "Config directory".to_string(),
            passed: config_exists,
        });
    }

    fn find_executable(paths: &[&str]) -> Option<String> {
        for path in paths {
            if path.starts_with('/') {
                if Path::new(path).exists() {
                    return Some(path.to_string());
                }
            } else {
                // Use `which` for bare names
                if let Ok(output) = Command::new("which").arg(path).output() {
                    if output.status.success() {
                        let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !resolved.is_empty() {
                            return Some(resolved);
                        }
                    }
                }
            }
        }
        None
    }

    fn count_scsyndef_files(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "scsyndef"))
                    .count()
            })
            .unwrap_or(0)
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            ServerPaneFocus::Controls => ServerPaneFocus::OutputDevice,
            ServerPaneFocus::OutputDevice => ServerPaneFocus::InputDevice,
            ServerPaneFocus::InputDevice => ServerPaneFocus::BufferSize,
            ServerPaneFocus::BufferSize => ServerPaneFocus::ScsynthArgs,
            ServerPaneFocus::ScsynthArgs => ServerPaneFocus::Controls,
        };
    }

    fn save_config(&self) {
        let config = devices::AudioDeviceConfig {
            input_device: self.selected_input_device(),
            output_device: self.selected_output_device(),
            buffer_size: self.selected_buffer_size(),
            sample_rate: self.sample_rate,
            scsynth_args: self.scsynth_args.clone(),
        };
        devices::save_device_config(&config);
    }
}

impl Default for ServerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for ServerPane {
    fn id(&self) -> &'static str {
        "server"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.handle_action_impl(action, event, state)
    }

    fn handle_raw_input(&mut self, event: &InputEvent, state: &AppState) -> Action {
        self.handle_raw_input_impl(event, state)
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        self.render_impl(area, buf, state);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
