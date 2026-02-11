use super::{BufferSize, ScsynthArgsDialogButton, ServerPane, ServerPaneFocus};
use crate::audio::devices::AudioDevice;
use crate::audio::ServerStatus;
use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Color, Rect, RenderBuf, Style};

const DEFAULT_SCSYNTH_HELP_LINE: &str = "-w <number-of-wire-buffers>         (default 64)";
const SCSYNTH_ARG_HELP: [(&str, &str); 15] = [
    (
        "-u",
        "-u <udp-port-number>               (default 57110 in imbolc)",
    ),
    (
        "-H",
        "-H <hardware-device-name>          (default system device)",
    ),
    ("-Z", "-Z <hardware-buffer-size>          (default 0)"),
    ("-S", "-S <hardware-sample-rate>          (default 0)"),
    ("-n", "-n <max-number-of-nodes>           (default 1024)"),
    ("-d", "-d <max-number-of-synth-defs>      (default 1024)"),
    ("-m", "-m <real-time-memory-size>         (default 8192)"),
    ("-w", "-w <number-of-wire-buffers>        (default 64)"),
    ("-a", "-a <number-of-audio-bus-channels>  (default 1024)"),
    ("-c", "-c <number-of-control-bus-channels>(default 16384)"),
    ("-b", "-b <number-of-sample-buffers>      (default 1024)"),
    ("-i", "-i <number-of-input-bus-channels>  (default 8)"),
    ("-o", "-o <number-of-output-bus-channels> (default 8)"),
    ("-l", "-l <max-logins>                    (default 64)"),
    ("-V", "-V <verbosity>                     (default 0)"),
];

impl ServerPane {
    pub(super) fn render_impl(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let output_devs = self.output_devices();
        let input_devs = self.input_devices();

        let rect = center_rect(area, 70, area.height.saturating_sub(2).max(15));

        let border_style = Style::new().fg(Color::GOLD);
        let inner = buf.draw_block(rect, " Audio Server (scsynth) ", border_style, border_style);

        let x = inner.x + 1;
        let w = inner.width.saturating_sub(2);
        let label_style = Style::new().fg(Color::CYAN);
        let mut y = inner.y + 1;

        // Server process status
        let (server_text, server_color) = if self.server_running {
            ("Running", Color::METER_LOW)
        } else {
            ("Stopped", Color::MUTE_COLOR)
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[
                ("Server:     ", label_style),
                (server_text, Style::new().fg(server_color).bold()),
            ],
        );
        y += 1;

        // Connection status
        let (status_text, status_color) = match self.status {
            ServerStatus::Stopped => ("Not connected", Color::DARK_GRAY),
            ServerStatus::Starting => ("Starting...", Color::ORANGE),
            ServerStatus::Running => ("Ready (not connected)", Color::SOLO_COLOR),
            ServerStatus::Connected => ("Connected", Color::METER_LOW),
            ServerStatus::Error => ("Error", Color::MUTE_COLOR),
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[
                ("Connection: ", label_style),
                (status_text, Style::new().fg(status_color).bold()),
            ],
        );
        y += 1;

        // Message
        if !self.message.is_empty() {
            let max_len = w as usize;
            let msg: String = self.message.chars().take(max_len).collect();
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(&msg, Style::new().fg(Color::SKY_BLUE))],
            );
        }
        y += 1;

        // Recording status
        if state.recording.recording {
            let mins = state.recording.recording_secs / 60;
            let secs = state.recording.recording_secs % 60;
            let rec_text = format!("REC {:02}:{:02}", mins, secs);
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    ("Recording:  ", label_style),
                    (&rec_text, Style::new().fg(Color::MUTE_COLOR).bold()),
                ],
            );
        }
        y += 1;

        // Imbolc audio-thread telemetry
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── Imbolc Telemetry ──", Style::new().fg(Color::DARK_GRAY))],
        );
        y += 1;
        let telemetry_line = format!(
            "avg={}us  max={}us  p95={}us  overruns={}  lookahead={:.1}ms  osc_q={}",
            state.audio.telemetry_avg_tick_us,
            state.audio.telemetry_max_tick_us,
            state.audio.telemetry_p95_tick_us,
            state.audio.telemetry_overruns,
            state.audio.telemetry_lookahead_ms,
            state.audio.telemetry_osc_queue_depth,
        );
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(&telemetry_line, Style::new().fg(Color::WHITE))],
        );
        y += 1;

        // Output Device section
        let output_focused = self.focus == ServerPaneFocus::OutputDevice;
        let section_color = if output_focused {
            Color::GOLD
        } else {
            Color::DARK_GRAY
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── Output Device ──", Style::new().fg(section_color))],
        );
        y += 1;

        y = self.render_device_list(
            buf,
            x,
            y,
            w,
            &output_devs,
            self.selected_output,
            output_focused,
        );
        y += 1;

        // Input Device section
        let input_focused = self.focus == ServerPaneFocus::InputDevice;
        let section_color = if input_focused {
            Color::GOLD
        } else {
            Color::DARK_GRAY
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── Input Device ──", Style::new().fg(section_color))],
        );
        y += 1;

        y = self.render_device_list(
            buf,
            x,
            y,
            w,
            &input_devs,
            self.selected_input,
            input_focused,
        );
        y += 1;

        // Buffer Size section
        let buffer_focused = self.focus == ServerPaneFocus::BufferSize;
        let section_color = if buffer_focused {
            Color::GOLD
        } else {
            Color::DARK_GRAY
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── Buffer Size ──", Style::new().fg(section_color))],
        );
        y += 1;

        y = self.render_buffer_size_list(buf, x, y, w, buffer_focused);
        y += 1;

        // scsynth extra args section
        let args_focused = self.focus == ServerPaneFocus::ScsynthArgs;
        let section_color = if args_focused {
            Color::GOLD
        } else {
            Color::DARK_GRAY
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── scsynth Args ──", Style::new().fg(section_color))],
        );
        y += 1;

        let marker_style = if args_focused {
            Style::new().fg(Color::GOLD)
        } else {
            Style::new().fg(Color::WHITE)
        };
        let marker = if args_focused { "> " } else { "  " };
        let args_text = if self.scsynth_args.is_empty() {
            "(none)".to_string()
        } else {
            self.scsynth_args.clone()
        };
        let args_style = if self.scsynth_args.is_empty() {
            Style::new().fg(Color::DARK_GRAY)
        } else {
            Style::new().fg(Color::WHITE)
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(marker, marker_style), (&args_text, args_style)],
        );
        y += 1;

        let args_hint = if self.editing_scsynth_args {
            "scsynth args editor open..."
        } else {
            "[Enter] Open args editor"
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(args_hint, Style::new().fg(Color::DARK_GRAY))],
        );
        y += 1;

        // Restart hint if config is dirty and server is running
        if self.device_config_dirty && self.server_running && y < rect.y + rect.height - 3 {
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(
                    "(restart server to apply device changes)",
                    Style::new().fg(Color::ORANGE),
                )],
            );
            y += 1;
        }

        // Network section (only in network mode)
        if let Some(ref net) = state.network {
            use crate::state::NetworkConnectionStatus;
            let section_style = Style::new().fg(Color::DARK_GRAY);
            buf.draw_line(Rect::new(x, y, w, 1), &[("── Network ──", section_style)]);
            y += 1;

            let (status_text, status_color) = match net.connection_status {
                NetworkConnectionStatus::Connected => ("Connected", Color::METER_LOW),
                NetworkConnectionStatus::Reconnecting => ("Reconnecting...", Color::SOLO_COLOR),
                NetworkConnectionStatus::Disconnected => ("Disconnected", Color::MUTE_COLOR),
            };
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    ("Status:     ", label_style),
                    (status_text, Style::new().fg(status_color).bold()),
                ],
            );
            y += 1;

            if let Some(ref priv_name) = net.privileged_client_name {
                buf.draw_line(
                    Rect::new(x, y, w, 1),
                    &[
                        ("Privilege:  ", label_style),
                        (priv_name, Style::new().fg(Color::METER_LOW)),
                    ],
                );
            } else {
                buf.draw_line(
                    Rect::new(x, y, w, 1),
                    &[
                        ("Privilege:  ", label_style),
                        ("(none)", Style::new().fg(Color::DARK_GRAY)),
                    ],
                );
            }
            y += 1;

            if !net.connected_clients.is_empty() {
                buf.draw_line(Rect::new(x, y, w, 1), &[("Clients:", label_style)]);
                y += 1;
                for client in &net.connected_clients {
                    if y >= rect.y + rect.height - 2 {
                        break;
                    }
                    let priv_marker = if client.is_privileged { " [P]" } else { "" };
                    let info = format!(
                        "  {} ({} instr){}",
                        client.name, client.owned_instrument_count, priv_marker
                    );
                    buf.draw_line(
                        Rect::new(x, y, w, 1),
                        &[(&info, Style::new().fg(Color::WHITE))],
                    );
                    y += 1;
                }
            }
            y += 1;
        }

        // Diagnostics section
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[("── Diagnostics ──", Style::new().fg(Color::DARK_GRAY))],
        );
        y += 1;

        for check in &self.diagnostics {
            if y >= rect.y + rect.height - 2 {
                break;
            }
            let (marker, marker_color, label_color) = if check.passed {
                ("[ok] ", Color::METER_LOW, Color::WHITE)
            } else {
                ("[--] ", Color::MUTE_COLOR, Color::DARK_GRAY)
            };
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    (marker, Style::new().fg(marker_color)),
                    (check.label.as_str(), Style::new().fg(label_color)),
                ],
            );
            y += 1;
        }
        y += 1;

        // Server log section
        let log_bottom = rect.y + rect.height - 2;
        if y < log_bottom {
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[("── Server Log ──", Style::new().fg(Color::DARK_GRAY))],
            );
            y += 1;

            let log_style = Style::new().fg(Color::DARK_GRAY);
            let available = (log_bottom.saturating_sub(y)) as usize;
            let skip = self.log_lines.len().saturating_sub(available);
            for line_text in self.log_lines.iter().skip(skip) {
                if y >= log_bottom {
                    break;
                }
                let truncated: String = line_text.chars().take(w as usize).collect();
                buf.draw_line(Rect::new(x, y, w, 1), &[(&truncated, log_style)]);
                y += 1;
            }
        }

        if self.editing_scsynth_args {
            self.render_scsynth_args_popup(area, buf);
        }
    }

    fn current_scsynth_flag(&self) -> Option<String> {
        self.scsynth_args_edit
            .split_whitespace()
            .rev()
            .find(|token| token.starts_with('-'))
            .map(|token| token.split('=').next().unwrap_or(token).to_string())
    }

    fn scsynth_help_line_for(flag: &str) -> Option<&'static str> {
        SCSYNTH_ARG_HELP
            .iter()
            .find(|(known_flag, _)| *known_flag == flag)
            .map(|(_, help_line)| *help_line)
    }

    fn current_scsynth_help_line(&self) -> String {
        if let Some(flag) = self.current_scsynth_flag() {
            if let Some(help_line) = Self::scsynth_help_line_for(&flag) {
                return help_line.to_string();
            }
            return format!("{flag} (unknown flag; run `scsynth -h` for full help)");
        }
        DEFAULT_SCSYNTH_HELP_LINE.to_string()
    }

    fn render_scsynth_args_popup(&self, area: Rect, buf: &mut RenderBuf) {
        let popup_width = area.width.saturating_sub(4).min(100);
        let popup_rect = center_rect(area, popup_width, 12);

        for row in popup_rect.y..popup_rect.y.saturating_add(popup_rect.height) {
            buf.fill_line_bg(
                popup_rect.x,
                row,
                popup_rect.width,
                Style::new().bg(Color::BLACK),
            );
        }

        let border_style = Style::new().fg(Color::GOLD);
        let inner = buf.draw_block(
            popup_rect,
            " scsynth Args Editor ",
            border_style,
            border_style,
        );

        let x = inner.x + 1;
        let w = inner.width.saturating_sub(2);
        let bottom = inner.y.saturating_add(inner.height);
        let mut y = inner.y + 1;

        if y < bottom {
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(
                    "Help for current flag (from `scsynth -h`):",
                    Style::new().fg(Color::DARK_GRAY),
                )],
            );
            y += 1;
        }

        if y < bottom {
            let help_line = self.current_scsynth_help_line();
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(&help_line, Style::new().fg(Color::CYAN).bold())],
            );
            y += 2;
        }

        if y < bottom {
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[("Launch args:", Style::new().fg(Color::DARK_GRAY))],
            );
            y += 1;
        }

        if y < bottom {
            let field_text = format!(
                "{}_",
                if self.scsynth_args_edit.is_empty() {
                    "<empty>".to_string()
                } else {
                    self.scsynth_args_edit.clone()
                }
            );
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(&field_text, Style::new().fg(Color::SKY_BLUE).bold())],
            );
            y += 2;
        }

        if y < bottom {
            let cancel_selected =
                self.scsynth_args_dialog_button == ScsynthArgsDialogButton::Cancel;
            let apply_selected =
                self.scsynth_args_dialog_button == ScsynthArgsDialogButton::ApplyRestart;
            let cancel_style = if cancel_selected {
                Style::new().fg(Color::BLACK).bg(Color::WHITE).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let apply_style = if apply_selected {
                Style::new().fg(Color::BLACK).bg(Color::GOLD).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    ("[ Cancel ]", cancel_style),
                    ("   ", Style::new()),
                    ("[ Apply and Restart Server ]", apply_style),
                ],
            );
            y += 1;
        }

        if y < bottom {
            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[(
                    "[Type] edit args  [Tab/←/→] select button  [Enter] activate  [Esc] cancel",
                    Style::new().fg(Color::DARK_GRAY),
                )],
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_device_list(
        &self,
        buf: &mut RenderBuf,
        x: u16,
        mut y: u16,
        w: u16,
        devices: &[&AudioDevice],
        selected: usize,
        focused: bool,
    ) -> u16 {
        let normal_style = Style::new().fg(Color::WHITE);
        let selected_style = if focused {
            Style::new().fg(Color::GOLD).bold()
        } else {
            Style::new().fg(Color::WHITE).bold()
        };
        let marker_style = if focused {
            Style::new().fg(Color::GOLD)
        } else {
            Style::new().fg(Color::WHITE)
        };

        // "System Default" entry (index 0)
        let is_selected = selected == 0;
        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            selected_style
        } else {
            normal_style
        };
        buf.draw_line(
            Rect::new(x, y, w, 1),
            &[(marker, marker_style), ("System Default", style)],
        );
        y += 1;

        // Device entries
        for (i, device) in devices.iter().enumerate() {
            let is_selected = selected == i + 1;
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                selected_style
            } else {
                normal_style
            };

            let mut info_parts = Vec::new();
            if let Some(sr) = device.sample_rate {
                info_parts.push(format!("{}Hz", sr));
            }
            if let Some(ch) = device.output_channels {
                if ch > 0 {
                    info_parts.push(format!("{}out", ch));
                }
            }
            if let Some(ch) = device.input_channels {
                if ch > 0 {
                    info_parts.push(format!("{}in", ch));
                }
            }

            let suffix = if info_parts.is_empty() {
                String::new()
            } else {
                format!("  ({})", info_parts.join(", "))
            };

            let info_style = Style::new().fg(Color::DARK_GRAY);

            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    (marker, marker_style),
                    (&device.name, style),
                    (&suffix, info_style),
                ],
            );
            y += 1;
        }

        y
    }

    fn render_buffer_size_list(
        &self,
        buf: &mut RenderBuf,
        x: u16,
        mut y: u16,
        w: u16,
        focused: bool,
    ) -> u16 {
        let normal_style = Style::new().fg(Color::WHITE);
        let selected_style = if focused {
            Style::new().fg(Color::GOLD).bold()
        } else {
            Style::new().fg(Color::WHITE).bold()
        };
        let marker_style = if focused {
            Style::new().fg(Color::GOLD)
        } else {
            Style::new().fg(Color::WHITE)
        };
        let info_style = Style::new().fg(Color::DARK_GRAY);

        for (i, &bs) in BufferSize::ALL.iter().enumerate() {
            let is_selected = self.selected_buffer_size == i;
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                selected_style
            } else {
                normal_style
            };

            let samples = bs.as_samples();
            let latency = bs.latency_ms(self.sample_rate);
            let label = format!("{} samples", samples);
            let suffix = format!("  (~{:.1}ms)", latency);

            buf.draw_line(
                Rect::new(x, y, w, 1),
                &[
                    (marker, marker_style),
                    (&label, style),
                    (&suffix, info_style),
                ],
            );
            y += 1;
        }

        y
    }
}
