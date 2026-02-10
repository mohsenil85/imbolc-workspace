use super::{BusDetailSection, GroupDetailSection, MixerPane, MixerSection};
use super::{CHANNEL_WIDTH, METER_HEIGHT, NUM_VISIBLE_CHANNELS, NUM_VISIBLE_GROUPS, NUM_VISIBLE_BUSES, BLOCK_CHARS};
use crate::state::{AppState, MixerSelection, OutputTarget, ParamValue};
use crate::ui::{Rect, RenderBuf, Color, Style};
use crate::ui::layout_helpers::center_rect;
use imbolc_types::BusId;

impl MixerPane {
    fn level_to_db(level: f32) -> String {
        if level <= 0.0 {
            "-\u{221e}".to_string()
        } else {
            let db = 20.0 * level.log10();
            format!("{:+.0}", db.max(-99.0))
        }
    }

    fn meter_color(row: u16, height: u16) -> Color {
        let frac = row as f32 / height as f32;
        if frac > 0.85 {
            Color::METER_HIGH
        } else if frac > 0.6 {
            Color::METER_MID
        } else {
            Color::METER_LOW
        }
    }

    fn format_output(target: OutputTarget) -> &'static str {
        match target {
            OutputTarget::Master => ">MST",
            OutputTarget::Bus(id) => match id.get() {
                1 => ">B1",
                2 => ">B2",
                3 => ">B3",
                4 => ">B4",
                5 => ">B5",
                6 => ">B6",
                7 => ">B7",
                8 => ">B8",
                _ => ">??",
            },
        }
    }

    fn write_str(buf: &mut RenderBuf, x: u16, y: u16, text: &str, style: Style) {
        for (i, ch) in text.chars().enumerate() {
            buf.set_cell(x + i as u16, y, ch, style);
        }
    }

    pub(super) fn render_mixer_buf(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        let active_groups = state.instruments.active_layer_groups();
        let num_group_slots = active_groups.len().min(NUM_VISIBLE_GROUPS);
        let group_section_width = if num_group_slots > 0 {
            num_group_slots as u16 * CHANNEL_WIDTH + 2  // +2 for separator
        } else {
            0
        };
        let box_width = (NUM_VISIBLE_CHANNELS as u16 * CHANNEL_WIDTH) + 2 +
                        group_section_width +
                        (NUM_VISIBLE_BUSES as u16 * CHANNEL_WIDTH) + 2 +
                        CHANNEL_WIDTH + 4;
        let box_height = METER_HEIGHT + 8;
        let rect = center_rect(area, box_width, box_height);

        buf.draw_block(rect, " MIXER ", Style::new().fg(Color::CYAN), Style::new().fg(Color::CYAN));

        let base_x = rect.x + 2;
        let base_y = rect.y + 1;

        let label_y = base_y;
        let name_y = base_y + 1;
        let meter_top_y = base_y + 2;
        let db_y = meter_top_y + METER_HEIGHT;
        let indicator_y = db_y + 1;
        let output_y = indicator_y + 1;

        // Calculate scroll offsets
        let instrument_scroll = match state.session.mixer.selection {
            MixerSelection::Instrument(idx) => {
                Self::calc_scroll_offset(idx, state.instruments.instruments.len(), NUM_VISIBLE_CHANNELS)
            }
            _ => 0,
        };

        let group_scroll = match state.session.mixer.selection {
            MixerSelection::LayerGroup(gid) => {
                let group_idx = active_groups.iter().position(|&g| g == gid).unwrap_or(0);
                Self::calc_scroll_offset(group_idx, active_groups.len(), NUM_VISIBLE_GROUPS)
            }
            _ => 0,
        };

        let bus_scroll = match state.session.mixer.selection {
            MixerSelection::Bus(id) => {
                Self::calc_scroll_offset((id.get() - 1) as usize, state.session.mixer.buses.len(), NUM_VISIBLE_BUSES)
            }
            _ => 0,
        };

        let mut x = base_x;

        // Render instrument channels
        for i in 0..NUM_VISIBLE_CHANNELS {
            let idx = instrument_scroll + i;
            if idx < state.instruments.instruments.len() {
                let instrument = &state.instruments.instruments[idx];
                let is_selected = matches!(state.session.mixer.selection, MixerSelection::Instrument(s) if s == idx);

                let label = if instrument.layer.group.is_some() {
                    format!("I{}L", instrument.id)
                } else {
                    format!("I{}", instrument.id)
                };
                Self::render_channel_buf(
                    buf, x, &label, &instrument.name,
                    instrument.level, instrument.mute, instrument.solo, Some(instrument.output_target), is_selected,
                    label_y, name_y, meter_top_y, db_y, indicator_y, output_y,
                );
            } else {
                Self::render_empty_channel_buf(
                    buf, x, &format!("I{}", idx + 1),
                    label_y, name_y, meter_top_y, db_y, indicator_y,
                );
            }

            x += CHANNEL_WIDTH;
        }

        // Separator before groups (if any) or buses
        let teal_style = Style::new().fg(Color::TEAL);
        if !active_groups.is_empty() {
            for y in label_y..=output_y {
                buf.set_cell(x, y, '│', teal_style);
            }
            x += 2;

            // Render layer group channels
            for i in 0..NUM_VISIBLE_GROUPS {
                let gidx = group_scroll + i;
                if gidx >= active_groups.len() {
                    break;
                }
                let group_id = active_groups[gidx];
                let is_selected = matches!(state.session.mixer.selection, MixerSelection::LayerGroup(g) if g == group_id);

                if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                    let label = format!("G{}", group_id);
                    Self::render_channel_buf(
                        buf, x, &label, &gm.name,
                        gm.level, gm.mute, gm.solo, Some(gm.output_target), is_selected,
                        label_y, name_y, meter_top_y, db_y, indicator_y, output_y,
                    );
                }

                x += CHANNEL_WIDTH;
            }
        }

        // Separator before buses
        let purple_style = Style::new().fg(Color::PURPLE);
        for y in label_y..=output_y {
            buf.set_cell(x, y, '│', purple_style);
        }
        x += 2;

        // Render buses
        for i in 0..NUM_VISIBLE_BUSES {
            let bus_idx = bus_scroll + i;
            if bus_idx >= state.session.mixer.buses.len() {
                break;
            }
            let bus = &state.session.mixer.buses[bus_idx];
            let is_selected = matches!(state.session.mixer.selection, MixerSelection::Bus(id) if id == bus.id);

            Self::render_channel_buf(
                buf, x, &format!("BUS{}", bus.id), &bus.name,
                bus.level, bus.mute, bus.solo, None, is_selected,
                label_y, name_y, meter_top_y, db_y, indicator_y, output_y,
            );

            x += CHANNEL_WIDTH;
        }

        // Separator before master
        let gold_style = Style::new().fg(Color::GOLD);
        for y in label_y..=output_y {
            buf.set_cell(x, y, '│', gold_style);
        }
        x += 2;

        // Master
        let is_master_selected = matches!(state.session.mixer.selection, MixerSelection::Master);
        Self::render_channel_buf(
            buf, x, "MASTER", "",
            state.session.mixer.master_level, state.session.mixer.master_mute, false, None, is_master_selected,
            label_y, name_y, meter_top_y, db_y, indicator_y, output_y,
        );

        // Send info line
        let send_y = output_y + 1;
        if let Some(bus_id) = self.send_target {
            let send_info = match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    state.instruments.instruments.get(idx).and_then(|instrument| {
                        instrument.sends.get(&bus_id).map(|send| {
                            let status = if send.enabled { "ON" } else { "OFF" };
                            format!("Send→B{}: {:.0}% [{}]", bus_id, send.level * 100.0, status)
                        })
                    })
                }
                MixerSelection::LayerGroup(gid) => {
                    state.session.mixer.layer_group_mixer(gid).and_then(|gm| {
                        gm.sends.get(&bus_id).map(|send| {
                            let status = if send.enabled { "ON" } else { "OFF" };
                            format!("G{} Send→B{}: {:.0}% [{}]", gid, bus_id, send.level * 100.0, status)
                        })
                    })
                }
                _ => None,
            };
            if let Some(info) = send_info {
                buf.draw_line(
                    Rect::new(base_x, send_y, rect.width.saturating_sub(4), 1),
                    &[(&info, Style::new().fg(Color::TEAL).bold())],
                );
            }
        }

    }

    pub(super) fn render_detail_buf(&self, buf: &mut RenderBuf, area: Rect, state: &AppState) {
        let Some((_, inst)) = self.detail_instrument(state) else {
            return;
        };

        let source_label = format!("{:?}", inst.source).chars().take(12).collect::<String>();
        let title = format!(" MIXER --- I{}: {} [{}] ", inst.id, inst.name, source_label);

        let box_width = area.width.min(90);
        let box_height = area.height.min(28);
        let rect = center_rect(area, box_width, box_height);

        buf.draw_block(rect, &title, Style::new().fg(Color::CYAN), Style::new().fg(Color::CYAN));

        let inner_x = rect.x + 2;
        let inner_y = rect.y + 1;
        let inner_w = rect.width.saturating_sub(4);
        let inner_h = rect.height.saturating_sub(3);

        // 3-column layout
        let col1_w = inner_w * 40 / 100;
        let col2_w = inner_w * 28 / 100;
        let _col3_w = inner_w.saturating_sub(col1_w + col2_w + 2);

        let col1_x = inner_x;
        let col2_x = col1_x + col1_w + 1;
        let col3_x = col2_x + col2_w + 1;

        let dim = Style::new().fg(Color::DARK_GRAY);
        let normal = Style::new().fg(Color::WHITE);
        let header_style = Style::new().fg(Color::CYAN).bold();
        let active_section = Style::new().fg(Color::WHITE).bold();
        let selected_style = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG);

        // Column separators
        for y in inner_y..(inner_y + inner_h) {
            buf.set_cell(col2_x - 1, y, '│', dim);
            buf.set_cell(col3_x - 1, y, '│', dim);
        }

        // ── Column 1: Effects Chain ──
        let effects_header = if self.detail_section == MixerSection::Effects {
            active_section
        } else {
            header_style
        };
        Self::write_str(buf, col1_x, inner_y, "EFFECTS CHAIN", effects_header);

        let mut ey = inner_y + 1;
        let mut cursor_pos = 0;
        let effects: Vec<_> = inst.effects().cloned().collect();
        for (ei, effect) in effects.iter().enumerate() {
            if ey >= inner_y + inner_h { break; }

            let bypass_char = if effect.enabled { '\u{25CF}' } else { '\u{25CB}' };
            let effect_label = format!("{} [{}] {:?}", ei + 1, bypass_char, effect.effect_type);
            let style = if self.detail_section == MixerSection::Effects && self.detail_cursor == cursor_pos {
                selected_style
            } else {
                normal
            };
            Self::write_str(buf, col1_x, ey, &effect_label, style);
            ey += 1;
            cursor_pos += 1;

            for (pi, param) in effect.params.iter().take(4).enumerate() {
                if ey >= inner_y + inner_h { break; }
                let val_str = match &param.value {
                    crate::state::ParamValue::Float(v) => format!("{:.2}", v),
                    crate::state::ParamValue::Int(v) => format!("{}", v),
                    crate::state::ParamValue::Bool(b) => if *b { "ON".to_string() } else { "OFF".to_string() },
                };
                let param_text = format!("  {} {}", param.name, val_str);
                let pstyle = if self.detail_section == MixerSection::Effects && self.detail_cursor == cursor_pos {
                    selected_style
                } else {
                    dim
                };
                Self::write_str(buf, col1_x + 1, ey, &param_text, pstyle);
                ey += 1;
                cursor_pos += 1;
                let _ = pi;
            }
        }
        if effects.is_empty() {
            Self::write_str(buf, col1_x, ey, "(no effects)", dim);
        }

        // ── Column 2 top: Sends ──
        let sends_header = if self.detail_section == MixerSection::Sends {
            active_section
        } else {
            header_style
        };
        Self::write_str(buf, col2_x, inner_y, "SENDS", sends_header);

        let mut sy = inner_y + 1;
        for (si, send) in inst.sends.values().enumerate() {
            if sy >= inner_y + inner_h / 2 { break; }
            let bar_len = (send.level * 5.0) as usize;
            let bar: String = "\u{2588}".repeat(bar_len) + &"\u{2591}".repeat(5 - bar_len);
            let status = if send.enabled {
                format!("{:.0}%", send.level * 100.0)
            } else {
                "OFF".to_string()
            };
            let send_text = format!("\u{2192}B{} {} {}", send.bus_id, bar, status);
            let sstyle = if self.detail_section == MixerSection::Sends && self.detail_cursor == si {
                selected_style
            } else if send.enabled {
                normal
            } else {
                dim
            };
            Self::write_str(buf, col2_x, sy, &send_text, sstyle);
            sy += 1;
        }

        // ── Column 2 bottom: Filter ──
        let filter_y = inner_y + inner_h / 2;
        let filter_header = if self.detail_section == MixerSection::Filter {
            active_section
        } else {
            header_style
        };
        Self::write_str(buf, col2_x, filter_y, "FILTER", filter_header);

        let mut fy = filter_y + 1;
        if let Some(filter) = inst.filter() {
            let type_text = format!("{:?}", filter.filter_type);
            let type_style = if self.detail_section == MixerSection::Filter && self.detail_cursor == 0 {
                selected_style
            } else {
                normal
            };
            Self::write_str(buf, col2_x, fy, &type_text, type_style);
            fy += 1;

            let cut_text = format!("Cut: {:.0} Hz", filter.cutoff.value);
            let cut_style = if self.detail_section == MixerSection::Filter && self.detail_cursor == 1 {
                selected_style
            } else {
                dim
            };
            Self::write_str(buf, col2_x, fy, &cut_text, cut_style);
            fy += 1;

            let res_text = format!("Res: {:.2}", filter.resonance.value);
            let res_style = if self.detail_section == MixerSection::Filter && self.detail_cursor == 2 {
                selected_style
            } else {
                dim
            };
            Self::write_str(buf, col2_x, fy, &res_text, res_style);
        } else {
            Self::write_str(buf, col2_x, fy, "(off)", dim);
        }

        // ── Column 3 top: Output ──
        let output_header = if self.detail_section == MixerSection::Output {
            active_section
        } else {
            header_style
        };
        Self::write_str(buf, col3_x, inner_y, "OUTPUT", output_header);

        let mut oy = inner_y + 1;

        let pan_text = format!("Pan: {:+.2}", inst.pan);
        let pan_style = if self.detail_section == MixerSection::Output && self.detail_cursor == 0 {
            selected_style
        } else {
            normal
        };
        Self::write_str(buf, col3_x, oy, &pan_text, pan_style);
        oy += 1;

        let db_str = Self::level_to_db(inst.level);
        let meter_len = (inst.level * 10.0) as usize;
        let meter_bar: String = "\u{258E}".repeat(meter_len) + &"\u{2591}".repeat(10usize.saturating_sub(meter_len));
        let level_text = format!("{} {}", meter_bar, db_str);
        let level_style = if self.detail_section == MixerSection::Output && self.detail_cursor == 1 {
            selected_style
        } else {
            normal
        };
        Self::write_str(buf, col3_x, oy, &level_text, level_style);
        oy += 1;

        let out_text = format!("\u{25B8} {}", match inst.output_target {
            OutputTarget::Master => "Master".to_string(),
            OutputTarget::Bus(id) => format!("Bus {}", id),
        });
        let out_style = if self.detail_section == MixerSection::Output && self.detail_cursor == 2 {
            selected_style
        } else {
            dim
        };
        Self::write_str(buf, col3_x, oy, &out_text, out_style);
        oy += 1;

        let mute_str = if inst.mute { "[M]" } else { " M " };
        let solo_str = if inst.solo { "[S]" } else { " S " };
        let mute_style = if inst.mute {
            Style::new().fg(Color::MUTE_COLOR).bold()
        } else {
            dim
        };
        let solo_style = if inst.solo {
            Style::new().fg(Color::SOLO_COLOR).bold()
        } else {
            dim
        };
        Self::write_str(buf, col3_x, oy, mute_str, mute_style);
        Self::write_str(buf, col3_x + 4, oy, solo_str, solo_style);

        // ── Column 3 bottom: LFO ──
        let lfo_y = inner_y + inner_h / 2;
        let lfo_header = if self.detail_section == MixerSection::Lfo {
            active_section
        } else {
            header_style
        };
        Self::write_str(buf, col3_x, lfo_y, "LFO", lfo_header);

        let mut ly = lfo_y + 1;
        let lfo = &inst.lfo;
        if lfo.enabled {
            let shape_text = format!("{:?} {:.1}Hz", lfo.shape, lfo.rate);
            let shape_style = if self.detail_section == MixerSection::Lfo && self.detail_cursor == 0 {
                selected_style
            } else {
                normal
            };
            Self::write_str(buf, col3_x, ly, &shape_text, shape_style);
            ly += 1;

            let depth_text = format!("Depth: {:.2}", lfo.depth);
            let depth_style = if self.detail_section == MixerSection::Lfo && self.detail_cursor == 1 {
                selected_style
            } else {
                dim
            };
            Self::write_str(buf, col3_x, ly, &depth_text, depth_style);
            ly += 1;

            let target_text = format!("Tgt: {:?}", lfo.target);
            let target_style = if self.detail_section == MixerSection::Lfo && self.detail_cursor == 2 {
                selected_style
            } else {
                dim
            };
            Self::write_str(buf, col3_x, ly, &target_text, target_style);
        } else {
            Self::write_str(buf, col3_x, ly, "(off)", dim);
        }


        // Section indicator bar (just below title)
        let section_bar_y = rect.y;
        let sections = [MixerSection::Effects, MixerSection::Sends, MixerSection::Filter, MixerSection::Lfo, MixerSection::Output];
        let mut sx = rect.x + (title.len() as u16) + 1;
        for &section in &sections {
            if sx + section.label().len() as u16 + 2 >= rect.x + rect.width { break; }
            let sstyle = if section == self.detail_section {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let label = format!(" {} ", section.label());
            Self::write_str(buf, sx, section_bar_y, &label, sstyle);
            sx += label.len() as u16 + 1;
        }
    }

    pub(super) fn render_group_detail_buf(&self, buf: &mut RenderBuf, area: Rect, state: &AppState, group_id: u32) {
        let gm = match state.session.mixer.layer_group_mixer(group_id) {
            Some(gm) => gm,
            None => return,
        };

        let title = format!(" MIXER --- Group {} [{}] ", group_id, gm.name);

        let box_width = area.width.min(60);
        let box_height = area.height.min(24);
        let rect = center_rect(area, box_width, box_height);

        buf.draw_block(rect, &title, Style::new().fg(Color::TEAL), Style::new().fg(Color::TEAL));

        let inner_x = rect.x + 2;
        let inner_y = rect.y + 1;
        let inner_h = rect.height.saturating_sub(3);

        let dim = Style::new().fg(Color::DARK_GRAY);
        let normal = Style::new().fg(Color::WHITE);
        let active_section = Style::new().fg(Color::WHITE).bold();
        let selected_style = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG);

        // Section indicator bar
        let section_bar_y = rect.y;
        let sections = [GroupDetailSection::Effects, GroupDetailSection::Sends, GroupDetailSection::Output];
        let mut sx = rect.x + (title.len() as u16) + 1;
        for &section in &sections {
            if sx + section.label().len() as u16 + 2 >= rect.x + rect.width { break; }
            let sstyle = if section == self.group_detail_section {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let label = format!(" {} ", section.label());
            Self::write_str(buf, sx, section_bar_y, &label, sstyle);
            sx += label.len() as u16 + 1;
        }

        match self.group_detail_section {
            GroupDetailSection::Effects => {
                Self::write_str(buf, inner_x, inner_y, "EFFECTS CHAIN", active_section);

                let mut ey = inner_y + 1;
                let mut cursor_pos = 0;
                for (ei, effect) in gm.effect_chain.effects.iter().enumerate() {
                    if ey >= inner_y + inner_h { break; }

                    let bypass_char = if effect.enabled { '\u{25CF}' } else { '\u{25CB}' };
                    let effect_label = format!("{} [{}] {:?}", ei + 1, bypass_char, effect.effect_type);
                    let style = if self.detail_cursor == cursor_pos {
                        selected_style
                    } else {
                        normal
                    };
                    Self::write_str(buf, inner_x, ey, &effect_label, style);
                    ey += 1;
                    cursor_pos += 1;

                    for param in effect.params.iter().take(4) {
                        if ey >= inner_y + inner_h { break; }
                        let val_str = match &param.value {
                            ParamValue::Float(v) => format!("{:.2}", v),
                            ParamValue::Int(v) => format!("{}", v),
                            ParamValue::Bool(b) => if *b { "ON".to_string() } else { "OFF".to_string() },
                        };
                        let param_text = format!("  {} {}", param.name, val_str);
                        let pstyle = if self.detail_cursor == cursor_pos {
                            selected_style
                        } else {
                            dim
                        };
                        Self::write_str(buf, inner_x + 1, ey, &param_text, pstyle);
                        ey += 1;
                        cursor_pos += 1;
                    }
                }
                if gm.effect_chain.effects.is_empty() {
                    Self::write_str(buf, inner_x, inner_y + 1, "(no effects)", dim);
                }
            }
            GroupDetailSection::Sends => {
                Self::write_str(buf, inner_x, inner_y, "SENDS", active_section);

                let mut sy = inner_y + 1;
                for (si, send) in gm.sends.values().enumerate() {
                    if sy >= inner_y + inner_h { break; }
                    let bar_len = (send.level * 5.0) as usize;
                    let bar: String = "\u{2588}".repeat(bar_len) + &"\u{2591}".repeat(5 - bar_len);
                    let status = if send.enabled {
                        format!("{:.0}%", send.level * 100.0)
                    } else {
                        "OFF".to_string()
                    };
                    let send_text = format!("\u{2192}B{} {} {}", send.bus_id, bar, status);
                    let sstyle = if self.detail_cursor == si {
                        selected_style
                    } else if send.enabled {
                        normal
                    } else {
                        dim
                    };
                    Self::write_str(buf, inner_x, sy, &send_text, sstyle);
                    sy += 1;
                }
            }
            GroupDetailSection::Output => {
                Self::write_str(buf, inner_x, inner_y, "OUTPUT", active_section);

                let mut oy = inner_y + 1;

                let pan_text = format!("Pan: {:+.2}", gm.pan);
                let pan_style = if self.detail_cursor == 0 {
                    selected_style
                } else {
                    normal
                };
                Self::write_str(buf, inner_x, oy, &pan_text, pan_style);
                oy += 1;

                let db_str = Self::level_to_db(gm.level);
                let meter_len = (gm.level * 10.0) as usize;
                let meter_bar: String = "\u{258E}".repeat(meter_len) + &"\u{2591}".repeat(10usize.saturating_sub(meter_len));
                let level_text = format!("{} {}", meter_bar, db_str);
                let level_style = if self.detail_cursor == 1 {
                    selected_style
                } else {
                    normal
                };
                Self::write_str(buf, inner_x, oy, &level_text, level_style);
                oy += 1;

                let out_text = format!("\u{25B8} {}", match gm.output_target {
                    OutputTarget::Master => "Master".to_string(),
                    OutputTarget::Bus(id) => format!("Bus {}", id),
                });
                Self::write_str(buf, inner_x, oy, &out_text, dim);
                oy += 1;

                let mute_str = if gm.mute { "[M]" } else { " M " };
                let solo_str = if gm.solo { "[S]" } else { " S " };
                let mute_style = if gm.mute {
                    Style::new().fg(Color::MUTE_COLOR).bold()
                } else {
                    dim
                };
                let solo_style = if gm.solo {
                    Style::new().fg(Color::SOLO_COLOR).bold()
                } else {
                    dim
                };
                Self::write_str(buf, inner_x, oy, mute_str, mute_style);
                Self::write_str(buf, inner_x + 4, oy, solo_str, solo_style);
            }
        }

        // Hint line at bottom
        let hint_y = rect.y + rect.height - 1;
        let hint = "Tab:section  a:add  d:del  e:bypass  Esc:back";
        let hint_x = rect.x + (rect.width.saturating_sub(hint.len() as u16)) / 2;
        Self::write_str(buf, hint_x, hint_y, hint, dim);
    }

    pub(super) fn render_bus_detail_buf(&self, buf: &mut RenderBuf, area: Rect, state: &AppState, bus_id: BusId) {
        let Some(bus) = state.session.bus(bus_id) else { return };

        let title = format!(" MIXER --- BUS {} [{}] ", bus_id, bus.name);

        let box_width = area.width.min(60);
        let box_height = area.height.min(24);
        let rect = center_rect(area, box_width, box_height);

        buf.draw_block(rect, &title, Style::new().fg(Color::PURPLE), Style::new().fg(Color::PURPLE));

        let inner_x = rect.x + 2;
        let inner_y = rect.y + 1;
        let inner_h = rect.height.saturating_sub(3);

        let dim = Style::new().fg(Color::DARK_GRAY);
        let normal = Style::new().fg(Color::WHITE);
        let active_section = Style::new().fg(Color::WHITE).bold();
        let selected_style = Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG);

        // Section indicator bar
        let section_bar_y = rect.y;
        let sections = [BusDetailSection::Effects, BusDetailSection::Output];
        let mut sx = rect.x + (title.len() as u16) + 1;
        for &section in &sections {
            if sx + section.label().len() as u16 + 2 >= rect.x + rect.width { break; }
            let sstyle = if section == self.bus_detail_section {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
            } else {
                Style::new().fg(Color::DARK_GRAY)
            };
            let label = format!(" {} ", section.label());
            Self::write_str(buf, sx, section_bar_y, &label, sstyle);
            sx += label.len() as u16 + 1;
        }

        match self.bus_detail_section {
            BusDetailSection::Effects => {
                let effects_header = active_section;
                Self::write_str(buf, inner_x, inner_y, "EFFECTS CHAIN", effects_header);

                let mut ey = inner_y + 1;
                let mut cursor_pos = 0;
                for (ei, effect) in bus.effect_chain.effects.iter().enumerate() {
                    if ey >= inner_y + inner_h { break; }

                    let bypass_char = if effect.enabled { '\u{25CF}' } else { '\u{25CB}' };
                    let effect_label = format!("{} [{}] {:?}", ei + 1, bypass_char, effect.effect_type);
                    let style = if self.detail_cursor == cursor_pos {
                        selected_style
                    } else {
                        normal
                    };
                    Self::write_str(buf, inner_x, ey, &effect_label, style);
                    ey += 1;
                    cursor_pos += 1;

                    for param in effect.params.iter().take(4) {
                        if ey >= inner_y + inner_h { break; }
                        let val_str = match &param.value {
                            ParamValue::Float(v) => format!("{:.2}", v),
                            ParamValue::Int(v) => format!("{}", v),
                            ParamValue::Bool(b) => if *b { "ON".to_string() } else { "OFF".to_string() },
                        };
                        let param_text = format!("  {} {}", param.name, val_str);
                        let pstyle = if self.detail_cursor == cursor_pos {
                            selected_style
                        } else {
                            dim
                        };
                        Self::write_str(buf, inner_x + 1, ey, &param_text, pstyle);
                        ey += 1;
                        cursor_pos += 1;
                    }
                }
                if bus.effect_chain.effects.is_empty() {
                    Self::write_str(buf, inner_x, inner_y + 1, "(no effects)", dim);
                }
            }
            BusDetailSection::Output => {
                let output_header = active_section;
                Self::write_str(buf, inner_x, inner_y, "OUTPUT", output_header);

                let mut oy = inner_y + 1;

                let pan_text = format!("Pan: {:+.2}", bus.pan);
                let pan_style = if self.detail_cursor == 0 {
                    selected_style
                } else {
                    normal
                };
                Self::write_str(buf, inner_x, oy, &pan_text, pan_style);
                oy += 1;

                let db_str = Self::level_to_db(bus.level);
                let meter_len = (bus.level * 10.0) as usize;
                let meter_bar: String = "\u{258E}".repeat(meter_len) + &"\u{2591}".repeat(10usize.saturating_sub(meter_len));
                let level_text = format!("{} {}", meter_bar, db_str);
                let level_style = if self.detail_cursor == 1 {
                    selected_style
                } else {
                    normal
                };
                Self::write_str(buf, inner_x, oy, &level_text, level_style);
                oy += 1;

                let mute_str = if bus.mute { "[M]" } else { " M " };
                let solo_str = if bus.solo { "[S]" } else { " S " };
                let mute_style = if bus.mute {
                    Style::new().fg(Color::MUTE_COLOR).bold()
                } else {
                    dim
                };
                let solo_style = if bus.solo {
                    Style::new().fg(Color::SOLO_COLOR).bold()
                } else {
                    dim
                };
                Self::write_str(buf, inner_x, oy, mute_str, mute_style);
                Self::write_str(buf, inner_x + 4, oy, solo_str, solo_style);
            }
        }

        // Hint line at bottom
        let hint_y = rect.y + rect.height - 1;
        let hint = "Tab:section  a:add  d:del  e:bypass  Esc:back";
        let hint_x = rect.x + (rect.width.saturating_sub(hint.len() as u16)) / 2;
        Self::write_str(buf, hint_x, hint_y, hint, dim);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_channel_buf(
        buf: &mut RenderBuf,
        x: u16,
        label: &str,
        name: &str,
        level: f32,
        mute: bool,
        solo: bool,
        output: Option<OutputTarget>,
        selected: bool,
        label_y: u16,
        name_y: u16,
        meter_top_y: u16,
        db_y: u16,
        indicator_y: u16,
        output_y: u16,
    ) {
        let channel_w = (CHANNEL_WIDTH - 1) as usize;

        let label_style = if selected {
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold()
        } else if label.starts_with("G") && label.len() <= 3 && label[1..].chars().all(|c| c.is_ascii_digit()) {
            Style::new().fg(Color::TEAL).bold()
        } else if label.starts_with("BUS") {
            Style::new().fg(Color::PURPLE).bold()
        } else if label == "MASTER" {
            Style::new().fg(Color::GOLD).bold()
        } else {
            Style::new().fg(Color::CYAN)
        };
        for (j, ch) in label.chars().take(channel_w).enumerate() {
            buf.set_cell(x + j as u16, label_y, ch, label_style);
        }

        let text_style = if selected {
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
        } else {
            Style::new().fg(Color::DARK_GRAY)
        };
        let name_display = if name.is_empty() && label.starts_with('I') { "---" } else { name };
        for (j, ch) in name_display.chars().take(channel_w).enumerate() {
            buf.set_cell(x + j as u16, name_y, ch, text_style);
        }

        // Vertical meter
        let meter_x = x + (CHANNEL_WIDTH / 2).saturating_sub(1);
        Self::render_meter_buf(buf, meter_x, meter_top_y, METER_HEIGHT, level);

        // Selection indicator
        if selected {
            let sel_x = meter_x + 1;
            buf.set_cell(sel_x, meter_top_y, '▼', Style::new().fg(Color::WHITE).bold());
        }

        // dB display
        let db_style = if selected {
            Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
        } else {
            Style::new().fg(Color::SKY_BLUE)
        };
        let db_str = Self::level_to_db(level);
        for (j, ch) in db_str.chars().enumerate() {
            buf.set_cell(x + j as u16, db_y, ch, db_style);
        }

        // Mute/Solo indicator
        let (indicator, indicator_style) = if mute {
            ("M", Style::new().fg(Color::MUTE_COLOR).bold())
        } else if solo {
            ("S", Style::new().fg(Color::SOLO_COLOR).bold())
        } else {
            ("●", Style::new().fg(Color::DARK_GRAY))
        };
        for (j, ch) in indicator.chars().enumerate() {
            buf.set_cell(x + j as u16, indicator_y, ch, indicator_style);
        }

        // Output routing
        if let Some(target) = output {
            let routing_style = if selected {
                Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG)
            } else {
                Style::new().fg(Color::TEAL)
            };
            for (j, ch) in Self::format_output(target).chars().enumerate() {
                buf.set_cell(x + j as u16, output_y, ch, routing_style);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_empty_channel_buf(
        buf: &mut RenderBuf,
        x: u16,
        label: &str,
        label_y: u16,
        name_y: u16,
        meter_top_y: u16,
        db_y: u16,
        indicator_y: u16,
    ) {
        let channel_w = (CHANNEL_WIDTH - 1) as usize;
        let dark_gray = Style::new().fg(Color::DARK_GRAY);

        for (j, ch) in label.chars().take(channel_w).enumerate() {
            buf.set_cell(x + j as u16, label_y, ch, dark_gray);
        }
        for (j, ch) in "---".chars().enumerate() {
            buf.set_cell(x + j as u16, name_y, ch, dark_gray);
        }

        let meter_x = x + (CHANNEL_WIDTH / 2).saturating_sub(1);
        for row in 0..METER_HEIGHT {
            buf.set_cell(meter_x, meter_top_y + row, '·', dark_gray);
        }

        for (j, ch) in "--".chars().enumerate() {
            buf.set_cell(x + j as u16, db_y, ch, dark_gray);
        }
        for (j, ch) in "●".chars().enumerate() {
            buf.set_cell(x + j as u16, indicator_y, ch, dark_gray);
        }
    }

    fn render_meter_buf(buf: &mut RenderBuf, x: u16, top_y: u16, height: u16, level: f32) {
        let total_sub = height as f32 * 8.0;
        let filled_sub = (level * total_sub) as u16;

        for row in 0..height {
            let inverted_row = height - 1 - row;
            let y = top_y + row;
            let row_start = inverted_row * 8;
            let row_end = row_start + 8;
            let color = Self::meter_color(inverted_row, height);

            let (ch, c) = if filled_sub >= row_end {
                ('\u{2588}', color)
            } else if filled_sub > row_start {
                let sub_level = (filled_sub - row_start) as usize;
                (BLOCK_CHARS[sub_level.saturating_sub(1).min(7)], color)
            } else {
                ('·', Color::DARK_GRAY)
            };

            buf.set_cell(x, y, ch, Style::new().fg(c));
        }
    }
}
