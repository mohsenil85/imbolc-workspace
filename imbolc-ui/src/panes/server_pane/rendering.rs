use super::{BufferSize, ServerPane, ServerPaneFocus};
use crate::audio::devices::AudioDevice;
use crate::audio::ServerStatus;
use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Color, Rect, RenderBuf, Style};

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
        let args_text = if self.editing_scsynth_args {
            format!(
                "{}_",
                if self.scsynth_args_edit.is_empty() {
                    "<empty>".to_string()
                } else {
                    self.scsynth_args_edit.clone()
                }
            )
        } else if self.scsynth_args.is_empty() {
            "(none)".to_string()
        } else {
            self.scsynth_args.clone()
        };
        let args_style = if self.editing_scsynth_args {
            Style::new().fg(Color::SKY_BLUE).bold()
        } else if self.scsynth_args.is_empty() {
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
            "[Enter] Apply+Restart  [Esc] Cancel"
        } else {
            "[Enter] Edit args"
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
