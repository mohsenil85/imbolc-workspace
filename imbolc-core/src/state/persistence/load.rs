use std::path::PathBuf;

use rusqlite::{params, Connection, Result as SqlResult, OptionalExtension};

use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

/// Load project state from relational tables.
pub fn load_relational(conn: &Connection) -> SqlResult<(SessionState, InstrumentState)> {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();

    load_session(conn, &mut session, &mut instruments)?;
    load_theme(conn, &mut session)?;
    load_mixer(conn, &mut session)?;
    load_layer_group_mixers(conn, &mut session)?;
    load_musical_settings(conn, &mut session)?;
    load_piano_roll(conn, &mut session)?;
    load_custom_synthdefs(conn, &mut session)?;
    load_vst_plugins(conn, &mut session)?;
    load_instruments(conn, &mut instruments)?;
    load_automation(conn, &mut session)?;
    load_midi_recording(conn, &mut session)?;
    load_arrangement(conn, &mut session)?;

    // Recompute derived state
    session.recompute_next_bus_id();

    Ok((session, instruments))
}

// ============================================================
// Session
// ============================================================

fn load_session(conn: &Connection, session: &mut SessionState, instruments: &mut InstrumentState) -> SqlResult<()> {
    let row = conn.query_row(
        "SELECT bpm, time_sig_num, time_sig_denom, key, scale, tuning_a4, snap,
                next_instrument_id, next_sampler_buffer_id, selected_instrument, next_layer_group_id,
                humanize_velocity, humanize_timing,
                click_enabled, click_volume, click_muted
         FROM session WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, f32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, u32>(10)?,
                row.get::<_, f32>(11)?,
                row.get::<_, f32>(12)?,
                row.get::<_, i32>(13)?,
                row.get::<_, f32>(14)?,
                row.get::<_, i32>(15)?,
            ))
        },
    )?;

    session.bpm = row.0 as u16;
    session.time_signature = (row.1 as u8, row.2 as u8);
    session.key = decode_key(&row.3);
    session.scale = decode_scale(&row.4);
    session.tuning_a4 = row.5;
    session.snap = row.6 != 0;
    instruments.next_id = row.7;
    instruments.next_sampler_buffer_id = row.8;
    instruments.selected = row.9.map(|v| v as usize);
    instruments.next_layer_group_id = row.10;
    session.humanize.velocity = row.11;
    session.humanize.timing = row.12;
    session.click_track.enabled = row.13 != 0;
    session.click_track.volume = row.14;
    session.click_track.muted = row.15 != 0;

    Ok(())
}

fn load_theme(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::state::ThemeColor;

    let result: Option<String> = conn.query_row(
        "SELECT name FROM theme WHERE id = 1",
        [],
        |row| row.get(0),
    ).optional()?;

    if result.is_none() {
        return Ok(()); // No theme row, use default
    }

    let row = conn.query_row(
        "SELECT name,
            background_r, background_g, background_b,
            foreground_r, foreground_g, foreground_b,
            border_r, border_g, border_b,
            selection_bg_r, selection_bg_g, selection_bg_b,
            selection_fg_r, selection_fg_g, selection_fg_b,
            muted_r, muted_g, muted_b,
            error_r, error_g, error_b,
            warning_r, warning_g, warning_b,
            success_r, success_g, success_b,
            osc_color_r, osc_color_g, osc_color_b,
            filter_color_r, filter_color_g, filter_color_b,
            env_color_r, env_color_g, env_color_b,
            lfo_color_r, lfo_color_g, lfo_color_b,
            fx_color_r, fx_color_g, fx_color_b,
            sample_color_r, sample_color_g, sample_color_b,
            midi_color_r, midi_color_g, midi_color_b,
            audio_in_color_r, audio_in_color_g, audio_in_color_b,
            meter_low_r, meter_low_g, meter_low_b,
            meter_mid_r, meter_mid_g, meter_mid_b,
            meter_high_r, meter_high_g, meter_high_b,
            waveform_grad_0_r, waveform_grad_0_g, waveform_grad_0_b,
            waveform_grad_1_r, waveform_grad_1_g, waveform_grad_1_b,
            waveform_grad_2_r, waveform_grad_2_g, waveform_grad_2_b,
            waveform_grad_3_r, waveform_grad_3_g, waveform_grad_3_b,
            playing_r, playing_g, playing_b,
            recording_r, recording_g, recording_b,
            armed_r, armed_g, armed_b
         FROM theme WHERE id = 1",
        [],
        |row| {
            let mut vals: Vec<u8> = Vec::new();
            // name at 0, then 81 u8 values (27 colors * 3 components)
            for i in 1..=81 {
                vals.push(row.get::<_, u8>(i)?);
            }
            Ok((row.get::<_, String>(0)?, vals))
        },
    )?;

    let (name, v) = row;
    let c = |i: usize| ThemeColor { r: v[i], g: v[i + 1], b: v[i + 2] };

    session.theme.name = name;
    session.theme.background = c(0);
    session.theme.foreground = c(3);
    session.theme.border = c(6);
    session.theme.selection_bg = c(9);
    session.theme.selection_fg = c(12);
    session.theme.muted = c(15);
    session.theme.error = c(18);
    session.theme.warning = c(21);
    session.theme.success = c(24);
    session.theme.osc_color = c(27);
    session.theme.filter_color = c(30);
    session.theme.env_color = c(33);
    session.theme.lfo_color = c(36);
    session.theme.fx_color = c(39);
    session.theme.sample_color = c(42);
    session.theme.midi_color = c(45);
    session.theme.audio_in_color = c(48);
    session.theme.meter_low = c(51);
    session.theme.meter_mid = c(54);
    session.theme.meter_high = c(57);
    session.theme.waveform_gradient = [c(60), c(63), c(66), c(69)];
    session.theme.playing = c(72);
    session.theme.recording = c(75);
    session.theme.armed = c(78);

    Ok(())
}

// ============================================================
// Mixer
// ============================================================

fn load_mixer(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::MixerBus;

    session.mixer.buses.clear();

    let mut stmt = conn.prepare("SELECT id, name, level, pan, mute, solo FROM mixer_buses ORDER BY id")?;
    let buses = stmt.query_map([], |row| {
        Ok(MixerBus {
            id: row.get::<_, i32>(0)? as u8,
            name: row.get(1)?,
            level: row.get(2)?,
            pan: row.get(3)?,
            mute: row.get::<_, i32>(4)? != 0,
            solo: row.get::<_, i32>(5)? != 0,
        })
    })?;

    for bus in buses {
        session.mixer.buses.push(bus?);
    }

    // Master
    let result: Option<(f32, i32)> = conn.query_row(
        "SELECT level, mute FROM mixer_master WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;

    if let Some((level, mute)) = result {
        session.mixer.master_level = level;
        session.mixer.master_mute = mute != 0;
    }

    Ok(())
}

fn load_layer_group_mixers(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::LayerGroupMixer;
    use crate::state::instrument::MixerSend;

    session.mixer.layer_group_mixers.clear();

    let mut stmt = conn.prepare(
        "SELECT group_id, name, level, pan, mute, solo, output_target FROM layer_group_mixers ORDER BY group_id"
    )?;
    let rows: Vec<(u32, String, f32, f32, i32, i32, String)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)? as u32,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?.collect::<SqlResult<_>>()?;

    for (group_id, name, level, pan, mute, solo, output_target_str) in rows {
        let output_target = decode_output_target(&output_target_str);

        // Load sends
        let mut send_stmt = conn.prepare(
            "SELECT bus_id, level, enabled FROM layer_group_sends WHERE group_id = ?1 ORDER BY bus_id"
        )?;
        let sends: Vec<MixerSend> = send_stmt.query_map(params![group_id as i32], |row| {
            Ok(MixerSend {
                bus_id: row.get::<_, i32>(0)? as u8,
                level: row.get(1)?,
                enabled: row.get::<_, i32>(2)? != 0,
            })
        })?.collect::<SqlResult<_>>()?;

        session.mixer.layer_group_mixers.push(LayerGroupMixer {
            group_id,
            name,
            level,
            pan,
            mute: mute != 0,
            solo: solo != 0,
            output_target,
            sends,
        });
    }

    Ok(())
}

// ============================================================
// Musical Settings / Piano Roll
// ============================================================

fn load_musical_settings(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    let result = conn.query_row(
        "SELECT bpm, time_sig_num, time_sig_denom, ticks_per_beat, loop_start, loop_end, looping, swing_amount
         FROM musical_settings WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, f32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, u32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, f32>(7)?,
            ))
        },
    ).optional()?;

    if let Some((bpm, tsn, tsd, tpb, ls, le, looping, swing)) = result {
        session.piano_roll.bpm = bpm;
        session.piano_roll.time_signature = (tsn as u8, tsd as u8);
        session.piano_roll.ticks_per_beat = tpb;
        session.piano_roll.loop_start = ls;
        session.piano_roll.loop_end = le;
        session.piano_roll.looping = looping != 0;
        session.piano_roll.swing_amount = swing;
    }

    Ok(())
}

fn load_piano_roll(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::piano_roll::Note;

    // Clear existing tracks
    session.piano_roll.tracks.clear();
    session.piano_roll.track_order.clear();

    // Load tracks ordered by position
    let mut track_stmt = conn.prepare(
        "SELECT instrument_id, polyphonic FROM piano_roll_tracks ORDER BY position"
    )?;
    let tracks: Vec<(u32, bool)> = track_stmt.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, i32>(1)? != 0))
    })?.collect::<SqlResult<_>>()?;

    for (inst_id, polyphonic) in &tracks {
        session.piano_roll.add_track(*inst_id);
        if let Some(track) = session.piano_roll.tracks.get_mut(inst_id) {
            track.polyphonic = *polyphonic;
        }
    }

    // Load notes
    let mut note_stmt = conn.prepare(
        "SELECT track_instrument_id, tick, duration, pitch, velocity, probability
         FROM piano_roll_notes ORDER BY track_instrument_id, tick"
    )?;
    let notes: Vec<(u32, Note)> = note_stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            Note {
                tick: row.get::<_, u32>(1)?,
                duration: row.get::<_, u32>(2)?,
                pitch: row.get::<_, i32>(3)? as u8,
                velocity: row.get::<_, i32>(4)? as u8,
                probability: row.get::<_, f32>(5)?,
            },
        ))
    })?.collect::<SqlResult<_>>()?;

    for (inst_id, note) in notes {
        if let Some(track) = session.piano_roll.tracks.get_mut(&inst_id) {
            track.notes.push(note);
        }
    }

    Ok(())
}

// ============================================================
// Custom SynthDefs
// ============================================================

fn load_custom_synthdefs(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};

    let mut registry = CustomSynthDefRegistry::new();

    let mut stmt = conn.prepare("SELECT id, name, synthdef_name, source_path FROM custom_synthdefs ORDER BY id")?;
    let synthdefs: Vec<(u32, String, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<SqlResult<_>>()?;

    for (id, name, synthdef_name, source_path) in synthdefs {
        let mut param_stmt = conn.prepare(
            "SELECT name, default_val, min_val, max_val FROM custom_synthdef_params WHERE synthdef_id = ?1 ORDER BY position"
        )?;
        let params: Vec<ParamSpec> = param_stmt.query_map(params![id], |row| {
            Ok(ParamSpec {
                name: row.get(0)?,
                default: row.get(1)?,
                min: row.get(2)?,
                max: row.get(3)?,
            })
        })?.collect::<SqlResult<_>>()?;

        registry.add(CustomSynthDef {
            id,
            name,
            synthdef_name,
            source_path: PathBuf::from(source_path),
            params,
        });
    }

    session.custom_synthdefs = registry;
    Ok(())
}

// ============================================================
// VST Plugins
// ============================================================

fn load_vst_plugins(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::vst_plugin::{VstPlugin, VstParamSpec, VstPluginKind, VstPluginRegistry};

    let mut registry = VstPluginRegistry::new();

    let mut stmt = conn.prepare("SELECT id, name, plugin_path, kind FROM vst_plugins ORDER BY id")?;
    let plugins: Vec<(u32, String, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<SqlResult<_>>()?;

    for (id, name, plugin_path, kind_str) in plugins {
        let kind = match kind_str.as_str() {
            "Effect" => VstPluginKind::Effect,
            _ => VstPluginKind::Instrument,
        };

        let mut param_stmt = conn.prepare(
            "SELECT param_index, name, default_val, label FROM vst_plugin_params WHERE plugin_id = ?1 ORDER BY position"
        )?;
        let params: Vec<VstParamSpec> = param_stmt.query_map(params![id], |row| {
            Ok(VstParamSpec {
                index: row.get(0)?,
                name: row.get(1)?,
                default: row.get(2)?,
                label: row.get(3)?,
            })
        })?.collect::<SqlResult<_>>()?;

        registry.add(VstPlugin {
            id,
            name,
            plugin_path: PathBuf::from(plugin_path),
            kind,
            params,
        });
    }

    session.vst_plugins = registry;
    Ok(())
}

// ============================================================
// Instruments
// ============================================================

fn load_instruments(conn: &Connection, instruments: &mut InstrumentState) -> SqlResult<()> {
    use crate::state::instrument::*;
    use crate::state::arpeggiator::ArpeggiatorConfig;
    use imbolc_types::state::groove::GrooveConfig;

    instruments.instruments.clear();

    let mut stmt = conn.prepare(
        "SELECT id, name, source_type,
            filter_type, filter_cutoff, filter_cutoff_min, filter_cutoff_max,
            filter_resonance, filter_resonance_min, filter_resonance_max,
            filter_enabled,
            lfo_enabled, lfo_rate, lfo_depth, lfo_shape, lfo_target,
            amp_attack, amp_decay, amp_sustain, amp_release,
            polyphonic, level, pan, mute, solo, active,
            output_target, channel_config, convolution_ir_path, layer_group,
            next_effect_id, eq_enabled,
            arp_enabled, arp_direction, arp_rate, arp_octaves, arp_gate,
            chord_shape, vst_state_path,
            groove_swing_amount, groove_swing_grid,
            groove_humanize_velocity, groove_humanize_timing,
            groove_timing_offset_ms, groove_time_sig_num, groove_time_sig_denom
         FROM instruments ORDER BY position"
    )?;

    let rows: Vec<InstrumentRow> = stmt.query_map([], |row| {
        Ok(InstrumentRow {
            id: row.get(0)?,
            name: row.get(1)?,
            source_type: row.get(2)?,
            filter_type: row.get(3)?,
            filter_cutoff: row.get(4)?,
            filter_cutoff_min: row.get(5)?,
            filter_cutoff_max: row.get(6)?,
            filter_resonance: row.get(7)?,
            filter_resonance_min: row.get(8)?,
            filter_resonance_max: row.get(9)?,
            filter_enabled: row.get(10)?,
            lfo_enabled: row.get(11)?,
            lfo_rate: row.get(12)?,
            lfo_depth: row.get(13)?,
            lfo_shape: row.get(14)?,
            lfo_target: row.get(15)?,
            amp_attack: row.get(16)?,
            amp_decay: row.get(17)?,
            amp_sustain: row.get(18)?,
            amp_release: row.get(19)?,
            polyphonic: row.get(20)?,
            level: row.get(21)?,
            pan: row.get(22)?,
            mute: row.get(23)?,
            solo: row.get(24)?,
            active: row.get(25)?,
            output_target: row.get(26)?,
            channel_config: row.get(27)?,
            convolution_ir_path: row.get(28)?,
            layer_group: row.get(29)?,
            next_effect_id: row.get(30)?,
            eq_enabled: row.get(31)?,
            arp_enabled: row.get(32)?,
            arp_direction: row.get(33)?,
            arp_rate: row.get(34)?,
            arp_octaves: row.get(35)?,
            arp_gate: row.get(36)?,
            chord_shape: row.get(37)?,
            vst_state_path: row.get(38)?,
            groove_swing_amount: row.get(39)?,
            groove_swing_grid: row.get(40)?,
            groove_humanize_velocity: row.get(41)?,
            groove_humanize_timing: row.get(42)?,
            groove_timing_offset_ms: row.get(43)?,
            groove_time_sig_num: row.get(44)?,
            groove_time_sig_denom: row.get(45)?,
        })
    })?.collect::<SqlResult<_>>()?;

    for r in rows {
        let source = decode_source_type(&r.source_type);

        // Build filter
        let filter = if let Some(ref ft) = r.filter_type {
            let filter_type = decode_filter_type(ft);
            let mut fc = FilterConfig::new(filter_type);
            if let Some(v) = r.filter_cutoff { fc.cutoff.value = v; }
            if let Some(v) = r.filter_cutoff_min { fc.cutoff.min = v; }
            if let Some(v) = r.filter_cutoff_max { fc.cutoff.max = v; }
            if let Some(v) = r.filter_resonance { fc.resonance.value = v; }
            if let Some(v) = r.filter_resonance_min { fc.resonance.min = v; }
            if let Some(v) = r.filter_resonance_max { fc.resonance.max = v; }
            fc.enabled = r.filter_enabled.map_or(true, |v| v != 0);

            // Load filter modulations
            load_modulation(conn, r.id, "cutoff", &mut fc.cutoff.mod_source)?;
            load_modulation(conn, r.id, "resonance", &mut fc.resonance.mod_source)?;

            // Load filter extra params
            fc.extra_params = load_params(conn, "instrument_filter_extra_params", "instrument_id", r.id)?;

            Some(fc)
        } else {
            None
        };

        // Build EQ
        let eq = if let Some(eq_enabled) = r.eq_enabled {
            let mut eq_config = crate::state::instrument::EqConfig::default();
            eq_config.enabled = eq_enabled != 0;

            let mut eq_stmt = conn.prepare(
                "SELECT band_index, band_type, freq, gain, q, enabled FROM instrument_eq_bands WHERE instrument_id = ?1 ORDER BY band_index"
            )?;
            let bands: Vec<(usize, String, f32, f32, f32, bool)> = eq_stmt.query_map(params![r.id], |row| {
                Ok((
                    row.get::<_, i32>(0)? as usize,
                    row.get::<_, String>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, f32>(4)?,
                    row.get::<_, i32>(5)? != 0,
                ))
            })?.collect::<SqlResult<_>>()?;

            for (idx, band_type, freq, gain, q, enabled) in bands {
                if idx < eq_config.bands.len() {
                    eq_config.bands[idx].band_type = decode_eq_band_type(&band_type);
                    eq_config.bands[idx].freq = freq;
                    eq_config.bands[idx].gain = gain;
                    eq_config.bands[idx].q = q;
                    eq_config.bands[idx].enabled = enabled;
                }
            }
            Some(eq_config)
        } else {
            None
        };

        let lfo = LfoConfig {
            enabled: r.lfo_enabled.map_or(false, |v| v != 0),
            rate: r.lfo_rate.unwrap_or(2.0),
            depth: r.lfo_depth.unwrap_or(0.5),
            shape: decode_lfo_shape(&r.lfo_shape.unwrap_or_else(|| "Sine".to_string())),
            target: decode_parameter_target(&r.lfo_target.unwrap_or_else(|| "FilterCutoff".to_string())),
        };

        let amp_envelope = crate::state::instrument::EnvConfig {
            attack: r.amp_attack,
            decay: r.amp_decay,
            sustain: r.amp_sustain,
            release: r.amp_release,
        };

        let arpeggiator = ArpeggiatorConfig {
            enabled: r.arp_enabled.map_or(false, |v| v != 0),
            direction: decode_arp_direction(&r.arp_direction.unwrap_or_else(|| "Up".to_string())),
            rate: decode_arp_rate(&r.arp_rate.unwrap_or_else(|| "Eighth".to_string())),
            octaves: r.arp_octaves.unwrap_or(1) as u8,
            gate: r.arp_gate.unwrap_or(0.5),
        };

        let groove = GrooveConfig {
            swing_amount: r.groove_swing_amount,
            swing_grid: r.groove_swing_grid.as_deref().map(decode_swing_grid),
            humanize_velocity: r.groove_humanize_velocity,
            humanize_timing: r.groove_humanize_timing,
            timing_offset_ms: r.groove_timing_offset_ms.unwrap_or(0.0),
            time_signature: match (r.groove_time_sig_num, r.groove_time_sig_denom) {
                (Some(n), Some(d)) => Some((n as u8, d as u8)),
                _ => None,
            },
        };

        let chord_shape = r.chord_shape.as_deref().map(decode_chord_shape);

        // Create instrument with defaults, then override
        let mut inst = Instrument::new(r.id, source);
        inst.name = r.name;
        inst.filter = filter;
        inst.eq = eq;
        inst.lfo = lfo;
        inst.amp_envelope = amp_envelope;
        inst.polyphonic = r.polyphonic != 0;
        inst.level = r.level;
        inst.pan = r.pan;
        inst.mute = r.mute != 0;
        inst.solo = r.solo != 0;
        inst.active = r.active != 0;
        inst.output_target = decode_output_target(&r.output_target);
        inst.channel_config = decode_channel_config(&r.channel_config);
        inst.convolution_ir_path = r.convolution_ir_path;
        inst.layer_group = r.layer_group;
        inst.next_effect_id = r.next_effect_id;
        inst.arpeggiator = arpeggiator;
        inst.chord_shape = chord_shape;
        inst.vst_state_path = r.vst_state_path.map(PathBuf::from);
        inst.groove = groove;

        // Source params
        inst.source_params = load_params(conn, "instrument_source_params", "instrument_id", r.id)?;

        // Effects
        inst.effects = load_effects(conn, r.id)?;

        // Sends
        inst.sends = load_sends(conn, r.id)?;

        // VST param values
        inst.vst_param_values = load_vst_param_values(conn, r.id)?;

        // Sampler config
        inst.sampler_config = load_sampler_config(conn, r.id)?;

        // Drum sequencer
        inst.drum_sequencer = load_drum_sequencer(conn, r.id)?;

        instruments.instruments.push(inst);
    }

    Ok(())
}

#[derive(Debug)]
struct InstrumentRow {
    id: u32,
    name: String,
    source_type: String,
    filter_type: Option<String>,
    filter_cutoff: Option<f32>,
    filter_cutoff_min: Option<f32>,
    filter_cutoff_max: Option<f32>,
    filter_resonance: Option<f32>,
    filter_resonance_min: Option<f32>,
    filter_resonance_max: Option<f32>,
    filter_enabled: Option<i32>,
    lfo_enabled: Option<i32>,
    lfo_rate: Option<f32>,
    lfo_depth: Option<f32>,
    lfo_shape: Option<String>,
    lfo_target: Option<String>,
    amp_attack: f32,
    amp_decay: f32,
    amp_sustain: f32,
    amp_release: f32,
    polyphonic: i32,
    level: f32,
    pan: f32,
    mute: i32,
    solo: i32,
    active: i32,
    output_target: String,
    channel_config: String,
    convolution_ir_path: Option<String>,
    layer_group: Option<u32>,
    next_effect_id: u32,
    eq_enabled: Option<i32>,
    arp_enabled: Option<i32>,
    arp_direction: Option<String>,
    arp_rate: Option<String>,
    arp_octaves: Option<i32>,
    arp_gate: Option<f32>,
    chord_shape: Option<String>,
    vst_state_path: Option<String>,
    groove_swing_amount: Option<f32>,
    groove_swing_grid: Option<String>,
    groove_humanize_velocity: Option<f32>,
    groove_humanize_timing: Option<f32>,
    groove_timing_offset_ms: Option<f32>,
    groove_time_sig_num: Option<i32>,
    groove_time_sig_denom: Option<i32>,
}

fn load_params(
    conn: &Connection,
    table: &str,
    id_col: &str,
    id: u32,
) -> SqlResult<Vec<crate::state::param::Param>> {
    use crate::state::param::{Param, ParamValue};

    let sql = format!(
        "SELECT param_name, param_value_type, param_value_float, param_value_int, param_value_bool, param_min, param_max
         FROM {} WHERE {} = ?1 ORDER BY position",
        table, id_col
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<Param> = stmt.query_map(params![id], |row| {
        let name: String = row.get(0)?;
        let vtype: String = row.get(1)?;
        let vf: Option<f64> = row.get(2)?;
        let vi: Option<i64> = row.get(3)?;
        let vb: Option<i32> = row.get(4)?;
        let min: f32 = row.get(5)?;
        let max: f32 = row.get(6)?;

        let value = match vtype.as_str() {
            "Int" => ParamValue::Int(vi.unwrap_or(0) as i32),
            "Bool" => ParamValue::Bool(vb.unwrap_or(0) != 0),
            _ => ParamValue::Float(vf.unwrap_or(0.0) as f32),
        };

        Ok(Param { name, value, min, max })
    })?.collect::<SqlResult<_>>()?;

    Ok(params)
}

fn load_effects(conn: &Connection, instrument_id: u32) -> SqlResult<Vec<crate::state::instrument::EffectSlot>> {
    use crate::state::instrument::EffectSlot;

    let mut stmt = conn.prepare(
        "SELECT effect_id, effect_type, enabled, vst_state_path FROM instrument_effects WHERE instrument_id = ?1 ORDER BY position"
    )?;
    let effect_rows: Vec<(u32, String, i32, Option<String>)> = stmt.query_map(params![instrument_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<SqlResult<_>>()?;

    let mut effects = Vec::new();
    for (effect_id, effect_type_str, enabled, vst_state) in effect_rows {
        let effect_type = decode_effect_type(&effect_type_str);
        let mut slot = EffectSlot::new(effect_id, effect_type);
        slot.enabled = enabled != 0;
        slot.vst_state_path = vst_state.map(PathBuf::from);

        // Effect params
        slot.params = load_effect_params(conn, instrument_id, effect_id)?;

        // Effect VST param values
        let mut vst_stmt = conn.prepare(
            "SELECT param_index, value FROM effect_vst_params WHERE instrument_id = ?1 AND effect_id = ?2 ORDER BY param_index"
        )?;
        slot.vst_param_values = vst_stmt.query_map(params![instrument_id, effect_id], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, f32>(1)?))
        })?.collect::<SqlResult<_>>()?;

        effects.push(slot);
    }

    Ok(effects)
}

fn load_effect_params(conn: &Connection, instrument_id: u32, effect_id: u32) -> SqlResult<Vec<crate::state::param::Param>> {
    use crate::state::param::{Param, ParamValue};

    let mut stmt = conn.prepare(
        "SELECT param_name, param_value_type, param_value_float, param_value_int, param_value_bool, param_min, param_max
         FROM instrument_effect_params WHERE instrument_id = ?1 AND effect_id = ?2 ORDER BY position"
    )?;
    let params: Vec<Param> = stmt.query_map(params![instrument_id, effect_id], |row| {
        let name: String = row.get(0)?;
        let vtype: String = row.get(1)?;
        let vf: Option<f64> = row.get(2)?;
        let vi: Option<i64> = row.get(3)?;
        let vb: Option<i32> = row.get(4)?;
        let min: f32 = row.get(5)?;
        let max: f32 = row.get(6)?;

        let value = match vtype.as_str() {
            "Int" => ParamValue::Int(vi.unwrap_or(0) as i32),
            "Bool" => ParamValue::Bool(vb.unwrap_or(0) != 0),
            _ => ParamValue::Float(vf.unwrap_or(0.0) as f32),
        };

        Ok(Param { name, value, min, max })
    })?.collect::<SqlResult<_>>()?;

    Ok(params)
}

fn load_sends(conn: &Connection, instrument_id: u32) -> SqlResult<Vec<crate::state::instrument::MixerSend>> {
    use crate::state::instrument::MixerSend;

    let mut stmt = conn.prepare(
        "SELECT bus_id, level, enabled FROM instrument_sends WHERE instrument_id = ?1 ORDER BY bus_id"
    )?;
    let sends: Vec<MixerSend> = stmt.query_map(params![instrument_id], |row| {
        Ok(MixerSend {
            bus_id: row.get::<_, i32>(0)? as u8,
            level: row.get(1)?,
            enabled: row.get::<_, i32>(2)? != 0,
        })
    })?.collect::<SqlResult<_>>()?;

    Ok(sends)
}

fn load_vst_param_values(conn: &Connection, instrument_id: u32) -> SqlResult<Vec<(u32, f32)>> {
    let mut stmt = conn.prepare(
        "SELECT param_index, value FROM instrument_vst_params WHERE instrument_id = ?1 ORDER BY param_index"
    )?;
    let values: Vec<(u32, f32)> = stmt.query_map(params![instrument_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.collect::<SqlResult<_>>()?;
    Ok(values)
}

fn load_modulation(
    conn: &Connection,
    instrument_id: u32,
    target_param: &str,
    mod_source: &mut Option<crate::state::instrument::ModSource>,
) -> SqlResult<()> {
    use crate::state::instrument::{ModSource, LfoConfig, EnvConfig};

    let result = conn.query_row(
        "SELECT mod_type, lfo_enabled, lfo_rate, lfo_depth, lfo_shape, lfo_target,
                env_attack, env_decay, env_sustain, env_release,
                source_instrument_id, source_param_name
         FROM instrument_modulations WHERE instrument_id = ?1 AND target_param = ?2",
        params![instrument_id, target_param],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i32>>(1)?,
                row.get::<_, Option<f32>>(2)?,
                row.get::<_, Option<f32>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<f32>>(6)?,
                row.get::<_, Option<f32>>(7)?,
                row.get::<_, Option<f32>>(8)?,
                row.get::<_, Option<f32>>(9)?,
                row.get::<_, Option<u32>>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        },
    ).optional()?;

    if let Some((mod_type, lfo_en, lfo_rate, lfo_depth, lfo_shape, lfo_target,
                 env_a, env_d, env_s, env_r, src_id, src_param)) = result {
        *mod_source = match mod_type.as_str() {
            "Lfo" => Some(ModSource::Lfo(LfoConfig {
                enabled: lfo_en.unwrap_or(0) != 0,
                rate: lfo_rate.unwrap_or(2.0),
                depth: lfo_depth.unwrap_or(0.5),
                shape: decode_lfo_shape(&lfo_shape.unwrap_or_else(|| "Sine".to_string())),
                target: decode_parameter_target(&lfo_target.unwrap_or_else(|| "FilterCutoff".to_string())),
            })),
            "Envelope" => Some(ModSource::Envelope(EnvConfig {
                attack: env_a.unwrap_or(0.01),
                decay: env_d.unwrap_or(0.1),
                sustain: env_s.unwrap_or(0.7),
                release: env_r.unwrap_or(0.3),
            })),
            "InstrumentParam" => {
                if let (Some(id), Some(param)) = (src_id, src_param) {
                    Some(ModSource::InstrumentParam(id, param))
                } else {
                    None
                }
            }
            _ => None,
        };
    }
    Ok(())
}

fn load_sampler_config(
    conn: &Connection,
    instrument_id: u32,
) -> SqlResult<Option<crate::state::sampler::SamplerConfig>> {
    use crate::state::sampler::{SamplerConfig, Slice};

    let result = conn.query_row(
        "SELECT buffer_id, sample_name, loop_mode, pitch_tracking, next_slice_id, selected_slice
         FROM sampler_configs WHERE instrument_id = ?1",
        params![instrument_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, i32>(5)?,
            ))
        },
    ).optional()?;

    let Some((buffer_id, sample_name, loop_mode, pitch_tracking, next_slice_id, selected_slice)) = result else {
        return Ok(None);
    };

    let mut config = SamplerConfig::new();
    config.buffer_id = buffer_id.map(|id| id as u32);
    config.sample_name = sample_name;
    config.loop_mode = loop_mode != 0;
    config.pitch_tracking = pitch_tracking != 0;
    config.set_next_slice_id(next_slice_id);
    config.selected_slice = selected_slice as usize;

    // Slices
    let mut stmt = conn.prepare(
        "SELECT slice_id, start_pos, end_pos, name, root_note FROM sampler_slices WHERE instrument_id = ?1 ORDER BY position"
    )?;
    config.slices = stmt.query_map(params![instrument_id], |row| {
        let mut s = Slice::new(row.get::<_, u32>(0)?, row.get(1)?, row.get(2)?);
        s.name = row.get(3)?;
        s.root_note = row.get::<_, i32>(4)? as u8;
        Ok(s)
    })?.collect::<SqlResult<_>>()?;

    Ok(Some(config))
}

fn load_drum_sequencer(
    conn: &Connection,
    instrument_id: u32,
) -> SqlResult<Option<crate::state::drum_sequencer::DrumSequencerState>> {
    use crate::state::drum_sequencer::*;

    let result = conn.query_row(
        "SELECT current_pattern, next_buffer_id, swing_amount, chain_enabled, step_resolution
         FROM drum_sequencer_state WHERE instrument_id = ?1",
        params![instrument_id],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    ).optional()?;

    let Some((current_pattern, next_buffer_id, swing_amount, chain_enabled, step_resolution_str)) = result else {
        return Ok(None);
    };

    let mut seq = DrumSequencerState::new();
    seq.current_pattern = current_pattern as usize;
    seq.next_buffer_id = next_buffer_id;
    seq.swing_amount = swing_amount;
    seq.chain_enabled = chain_enabled != 0;
    seq.step_resolution = decode_step_resolution(&step_resolution_str);

    // Chain
    let mut chain_stmt = conn.prepare(
        "SELECT pattern_index FROM drum_sequencer_chain WHERE instrument_id = ?1 ORDER BY position"
    )?;
    seq.chain = chain_stmt.query_map(params![instrument_id], |row| {
        Ok(row.get::<_, i32>(0)? as usize)
    })?.collect::<SqlResult<_>>()?;

    // Pads
    let mut pad_stmt = conn.prepare(
        "SELECT pad_index, buffer_id, path, name, level, slice_start, slice_end, reverse, pitch, trigger_instrument_id, trigger_freq
         FROM drum_pads WHERE instrument_id = ?1 ORDER BY pad_index"
    )?;
    let pads: Vec<(usize, Option<i64>, Option<String>, String, f32, f32, f32, i32, i32, Option<i64>, f32)> =
        pad_stmt.query_map(params![instrument_id], |row| {
            Ok((
                row.get::<_, i32>(0)? as usize,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        })?.collect::<SqlResult<_>>()?;

    for (idx, buffer_id, path, name, level, slice_start, slice_end, reverse, pitch, trigger_inst, trigger_freq) in pads {
        if idx < seq.pads.len() {
            seq.pads[idx].buffer_id = buffer_id.map(|id| id as u32);
            seq.pads[idx].path = path;
            seq.pads[idx].name = name;
            seq.pads[idx].level = level;
            seq.pads[idx].slice_start = slice_start;
            seq.pads[idx].slice_end = slice_end;
            seq.pads[idx].reverse = reverse != 0;
            seq.pads[idx].pitch = pitch as i8;
            seq.pads[idx].instrument_id = trigger_inst.map(|id| id as u32);
            seq.pads[idx].trigger_freq = trigger_freq;
        }
    }

    // Patterns
    let mut pat_stmt = conn.prepare(
        "SELECT pattern_index, length FROM drum_patterns WHERE instrument_id = ?1 ORDER BY pattern_index"
    )?;
    let patterns: Vec<(usize, usize)> = pat_stmt.query_map(params![instrument_id], |row| {
        Ok((row.get::<_, i32>(0)? as usize, row.get::<_, i32>(1)? as usize))
    })?.collect::<SqlResult<_>>()?;

    for (pat_idx, length) in patterns {
        if pat_idx < seq.patterns.len() {
            seq.patterns[pat_idx].length = length;
            // Resize step arrays to match length
            for pad_steps in &mut seq.patterns[pat_idx].steps {
                pad_steps.resize(length, DrumStep::default());
            }
        }
    }

    // Steps (only active ones were saved)
    let mut step_stmt = conn.prepare(
        "SELECT pattern_index, pad_index, step_index, velocity, probability, pitch_offset
         FROM drum_steps WHERE instrument_id = ?1"
    )?;
    let steps: Vec<(usize, usize, usize, u8, f32, i8)> = step_stmt.query_map(params![instrument_id], |row| {
        Ok((
            row.get::<_, i32>(0)? as usize,
            row.get::<_, i32>(1)? as usize,
            row.get::<_, i32>(2)? as usize,
            row.get::<_, i32>(3)? as u8,
            row.get::<_, f32>(4)?,
            row.get::<_, i32>(5)? as i8,
        ))
    })?.collect::<SqlResult<_>>()?;

    for (pat_idx, pad_idx, step_idx, velocity, probability, pitch_offset) in steps {
        if pat_idx < seq.patterns.len()
            && pad_idx < seq.patterns[pat_idx].steps.len()
            && step_idx < seq.patterns[pat_idx].steps[pad_idx].len()
        {
            seq.patterns[pat_idx].steps[pad_idx][step_idx] = DrumStep {
                active: true,
                velocity,
                probability,
                pitch_offset,
            };
        }
    }

    // Chopper
    seq.chopper = load_chopper(conn, instrument_id)?;

    Ok(Some(seq))
}

fn load_chopper(
    conn: &Connection,
    instrument_id: u32,
) -> SqlResult<Option<crate::state::drum_sequencer::ChopperState>> {
    use crate::state::drum_sequencer::ChopperState;
    use crate::state::sampler::Slice;

    let result = conn.query_row(
        "SELECT buffer_id, path, name, selected_slice, next_slice_id, duration_secs, waveform_peaks
         FROM chopper_states WHERE instrument_id = ?1",
        params![instrument_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, f32>(5)?,
                row.get::<_, Option<Vec<u8>>>(6)?,
            ))
        },
    ).optional()?;

    let Some((buffer_id, path, name, selected_slice, next_slice_id, duration_secs, peaks_blob)) = result else {
        return Ok(None);
    };

    let waveform_peaks = if let Some(bytes) = peaks_blob {
        bytes.chunks_exact(4).map(|chunk| {
            f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        }).collect()
    } else {
        Vec::new()
    };

    let mut slices_stmt = conn.prepare(
        "SELECT slice_id, start_pos, end_pos, name, root_note FROM chopper_slices WHERE instrument_id = ?1 ORDER BY position"
    )?;
    let slices: Vec<Slice> = slices_stmt.query_map(params![instrument_id], |row| {
        let mut s = Slice::new(row.get::<_, u32>(0)?, row.get(1)?, row.get(2)?);
        s.name = row.get(3)?;
        s.root_note = row.get::<_, i32>(4)? as u8;
        Ok(s)
    })?.collect::<SqlResult<_>>()?;

    Ok(Some(ChopperState {
        buffer_id: buffer_id.map(|id| id as u32),
        path,
        name,
        slices,
        selected_slice: selected_slice as usize,
        next_slice_id,
        waveform_peaks,
        duration_secs,
    }))
}

// ============================================================
// Automation
// ============================================================

fn load_automation(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::automation::{AutomationLane, AutomationPoint};

    session.automation.lanes.clear();

    let mut stmt = conn.prepare(
        "SELECT id, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, enabled, record_armed, min_value, max_value
         FROM automation_lanes ORDER BY id"
    )?;
    let lanes: Vec<(u32, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, i32, i32, f32, f32)> =
        stmt.query_map([], |row| {
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                row.get(4)?, row.get(5)?, row.get(6)?,
                row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?,
            ))
        })?.collect::<SqlResult<_>>()?;

    for (id, target_type, target_inst_id, target_bus_id, _target_effect_id, _target_param_idx, target_extra, enabled, record_armed, min_value, max_value) in lanes {
        let target = decode_automation_target(&target_type, target_inst_id, target_bus_id, target_extra.as_deref());
        let mut lane = AutomationLane::new(id, target);
        lane.enabled = enabled != 0;
        lane.record_armed = record_armed != 0;
        lane.min_value = min_value;
        lane.max_value = max_value;

        // Points
        let mut point_stmt = conn.prepare(
            "SELECT tick, value, curve_type FROM automation_points WHERE lane_id = ?1 ORDER BY tick"
        )?;
        lane.points = point_stmt.query_map(params![id], |row| {
            Ok(AutomationPoint {
                tick: row.get::<_, u32>(0)?,
                value: row.get(1)?,
                curve: decode_curve_type(&row.get::<_, String>(2)?),
            })
        })?.collect::<SqlResult<_>>()?;

        session.automation.lanes.push(lane);
    }

    // Recalculate next_lane_id
    session.automation.recalculate_next_lane_id();

    Ok(())
}

// ============================================================
// MIDI Recording
// ============================================================

fn load_midi_recording(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::midi_recording::{MidiCcMapping, PitchBendConfig};

    let result = conn.query_row(
        "SELECT live_input_instrument, note_passthrough, channel_filter FROM midi_recording_settings WHERE id = 1",
        [],
        |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, i32>(1)?, row.get::<_, Option<i32>>(2)?)),
    ).optional()?;

    if let Some((live_inst, passthrough, channel)) = result {
        session.midi_recording.live_input_instrument = live_inst.map(|v| v as u32);
        session.midi_recording.note_passthrough = passthrough != 0;
        session.midi_recording.channel_filter = channel.map(|v| v as u8);
    }

    // CC mappings
    session.midi_recording.cc_mappings.clear();
    let mut cc_stmt = conn.prepare(
        "SELECT cc_number, channel, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, min_value, max_value
         FROM midi_cc_mappings ORDER BY id"
    )?;
    let cc_rows: Vec<(i32, Option<i32>, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, f32, f32)> =
        cc_stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
                row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?))
        })?.collect::<SqlResult<_>>()?;

    for (cc_number, channel, target_type, target_inst_id, target_bus_id, _target_effect_id, _target_param_idx, target_extra, min_value, max_value) in cc_rows {
        let target = decode_automation_target(&target_type, target_inst_id, target_bus_id, target_extra.as_deref());
        let mut mapping = MidiCcMapping::new(cc_number as u8, target);
        mapping.channel = channel.map(|v| v as u8);
        mapping.min_value = min_value;
        mapping.max_value = max_value;
        session.midi_recording.cc_mappings.push(mapping);
    }

    // Pitch bend configs
    session.midi_recording.pitch_bend_configs.clear();
    let mut pb_stmt = conn.prepare(
        "SELECT target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, center_value, range, sensitivity
         FROM midi_pitch_bend_configs ORDER BY id"
    )?;
    let pb_rows: Vec<(String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, f32, f32, f32)> =
        pb_stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?))
        })?.collect::<SqlResult<_>>()?;

    for (target_type, target_inst_id, target_bus_id, _target_effect_id, _target_param_idx, target_extra, center_value, range, sensitivity) in pb_rows {
        let target = decode_automation_target(&target_type, target_inst_id, target_bus_id, target_extra.as_deref());
        session.midi_recording.pitch_bend_configs.push(PitchBendConfig {
            target,
            center_value,
            range,
            sensitivity,
        });
    }

    Ok(())
}

// ============================================================
// Arrangement
// ============================================================

fn load_arrangement(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::arrangement::*;
    use crate::state::piano_roll::Note;
    use crate::state::automation::{AutomationLane, AutomationPoint};

    let result = conn.query_row(
        "SELECT play_mode, selected_placement, selected_lane, view_start_tick, ticks_per_col, cursor_tick,
                next_clip_id, next_placement_id, next_clip_automation_lane_id
         FROM arrangement_state WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, u32>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u32>(8)?,
            ))
        },
    ).optional()?;

    if let Some((play_mode, sel_placement, sel_lane, view_start, ticks_per_col, cursor, _next_clip, _next_placement, _next_auto_lane)) = result {
        session.arrangement.play_mode = decode_play_mode(&play_mode);
        session.arrangement.selected_placement = sel_placement.map(|v| v as usize);
        session.arrangement.selected_lane = sel_lane as usize;
        session.arrangement.view_start_tick = view_start as u32;
        session.arrangement.ticks_per_col = ticks_per_col as u32;
        session.arrangement.cursor_tick = cursor as u32;
    }

    // Clips
    session.arrangement.clips.clear();
    let mut clip_stmt = conn.prepare("SELECT id, name, instrument_id, length_ticks FROM arrangement_clips ORDER BY id")?;
    let clips: Vec<(u32, String, u32, u32)> = clip_stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, i64>(3)? as u32))
    })?.collect::<SqlResult<_>>()?;

    for (id, name, inst_id, length) in clips {
        let mut clip = Clip { id, name, instrument_id: inst_id, length_ticks: length, notes: Vec::new(), automation_lanes: Vec::new() };

        // Clip notes
        let mut note_stmt = conn.prepare(
            "SELECT tick, duration, pitch, velocity, probability FROM arrangement_clip_notes WHERE clip_id = ?1 ORDER BY position"
        )?;
        clip.notes = note_stmt.query_map(params![id], |row| {
            Ok(Note {
                tick: row.get::<_, u32>(0)?,
                duration: row.get::<_, u32>(1)?,
                pitch: row.get::<_, i32>(2)? as u8,
                velocity: row.get::<_, i32>(3)? as u8,
                probability: row.get::<_, f32>(4)?,
            })
        })?.collect::<SqlResult<_>>()?;

        // Clip automation lanes
        let mut lane_stmt = conn.prepare(
            "SELECT id, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, enabled, record_armed, min_value, max_value
             FROM arrangement_clip_automation_lanes WHERE clip_id = ?1 ORDER BY id"
        )?;
        let lane_rows: Vec<(u32, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, i32, i32, f32, f32)> =
            lane_stmt.query_map(params![id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                    row.get(4)?, row.get(5)?, row.get(6)?,
                    row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?))
            })?.collect::<SqlResult<_>>()?;

        for (lane_id, target_type, target_inst_id, target_bus_id, _eff_id, _param_idx, target_extra, enabled, armed, min_val, max_val) in lane_rows {
            let target = decode_automation_target(&target_type, target_inst_id, target_bus_id, target_extra.as_deref());
            let mut lane = AutomationLane::new(lane_id, target);
            lane.enabled = enabled != 0;
            lane.record_armed = armed != 0;
            lane.min_value = min_val;
            lane.max_value = max_val;

            let mut pt_stmt = conn.prepare(
                "SELECT tick, value, curve_type FROM arrangement_clip_automation_points WHERE lane_id = ?1 ORDER BY tick"
            )?;
            lane.points = pt_stmt.query_map(params![lane_id], |row| {
                Ok(AutomationPoint {
                    tick: row.get::<_, u32>(0)?,
                    value: row.get(1)?,
                    curve: decode_curve_type(&row.get::<_, String>(2)?),
                })
            })?.collect::<SqlResult<_>>()?;

            clip.automation_lanes.push(lane);
        }

        session.arrangement.clips.push(clip);
    }

    // Placements
    session.arrangement.placements.clear();
    let mut place_stmt = conn.prepare(
        "SELECT id, clip_id, instrument_id, start_tick, length_override FROM arrangement_placements ORDER BY id"
    )?;
    session.arrangement.placements = place_stmt.query_map([], |row| {
        Ok(ClipPlacement {
            id: row.get(0)?,
            clip_id: row.get(1)?,
            instrument_id: row.get(2)?,
            start_tick: row.get::<_, i64>(3)? as u32,
            length_override: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
        })
    })?.collect::<SqlResult<_>>()?;

    // Recalculate next IDs
    session.arrangement.recalculate_next_ids();

    Ok(())
}

// ============================================================
// Decoding helpers
// ============================================================

fn decode_key(s: &str) -> crate::state::music::Key {
    use crate::state::music::Key;
    match s {
        "Cs" => Key::Cs, "D" => Key::D, "Ds" => Key::Ds, "E" => Key::E,
        "F" => Key::F, "Fs" => Key::Fs, "G" => Key::G, "Gs" => Key::Gs,
        "A" => Key::A, "As" => Key::As, "B" => Key::B,
        _ => Key::C,
    }
}

fn decode_scale(s: &str) -> crate::state::music::Scale {
    use crate::state::music::Scale;
    match s {
        "Minor" => Scale::Minor, "Dorian" => Scale::Dorian, "Phrygian" => Scale::Phrygian,
        "Lydian" => Scale::Lydian, "Mixolydian" => Scale::Mixolydian, "Aeolian" => Scale::Aeolian,
        "Locrian" => Scale::Locrian, "Pentatonic" => Scale::Pentatonic, "Blues" => Scale::Blues,
        "Chromatic" => Scale::Chromatic,
        _ => Scale::Major,
    }
}

fn decode_source_type(s: &str) -> crate::state::instrument::SourceType {
    use crate::state::instrument::SourceType;
    if let Some(rest) = s.strip_prefix("Custom:") {
        if let Ok(id) = rest.parse::<u32>() {
            return SourceType::Custom(id);
        }
    }
    if let Some(rest) = s.strip_prefix("Vst:") {
        if let Ok(id) = rest.parse::<u32>() {
            return SourceType::Vst(id);
        }
    }
    match s {
        "Saw" => SourceType::Saw, "Sin" => SourceType::Sin, "Sqr" => SourceType::Sqr,
        "Tri" => SourceType::Tri, "Noise" => SourceType::Noise, "Pulse" => SourceType::Pulse,
        "SuperSaw" => SourceType::SuperSaw, "Sync" => SourceType::Sync,
        "Ring" => SourceType::Ring, "FBSin" => SourceType::FBSin,
        "FM" => SourceType::FM, "PhaseMod" => SourceType::PhaseMod,
        "FMBell" => SourceType::FMBell, "FMBrass" => SourceType::FMBrass,
        "Pluck" => SourceType::Pluck, "Formant" => SourceType::Formant,
        "Bowed" => SourceType::Bowed, "Blown" => SourceType::Blown, "Membrane" => SourceType::Membrane,
        "Marimba" => SourceType::Marimba, "Vibes" => SourceType::Vibes,
        "Kalimba" => SourceType::Kalimba, "SteelDrum" => SourceType::SteelDrum,
        "TubularBell" => SourceType::TubularBell, "Glockenspiel" => SourceType::Glockenspiel,
        "Guitar" => SourceType::Guitar, "BassGuitar" => SourceType::BassGuitar,
        "Harp" => SourceType::Harp, "Koto" => SourceType::Koto,
        "Kick" => SourceType::Kick, "Snare" => SourceType::Snare,
        "HihatClosed" => SourceType::HihatClosed, "HihatOpen" => SourceType::HihatOpen,
        "Clap" => SourceType::Clap, "Cowbell" => SourceType::Cowbell,
        "Rim" => SourceType::Rim, "Tom" => SourceType::Tom,
        "Clave" => SourceType::Clave, "Conga" => SourceType::Conga,
        "Choir" => SourceType::Choir, "EPiano" => SourceType::EPiano,
        "Organ" => SourceType::Organ, "BrassStab" => SourceType::BrassStab,
        "Strings" => SourceType::Strings, "Acid" => SourceType::Acid,
        "Gendy" => SourceType::Gendy, "Chaos" => SourceType::Chaos,
        "Additive" => SourceType::Additive, "Wavetable" => SourceType::Wavetable,
        "Granular" => SourceType::Granular,
        "AudioIn" => SourceType::AudioIn, "BusIn" => SourceType::BusIn,
        "PitchedSampler" => SourceType::PitchedSampler, "TimeStretch" => SourceType::TimeStretch,
        "Kit" => SourceType::Kit,
        _ => SourceType::Saw, // fallback
    }
}

fn decode_effect_type(s: &str) -> crate::state::instrument::EffectType {
    use crate::state::instrument::EffectType;
    if let Some(rest) = s.strip_prefix("Vst:") {
        if let Ok(id) = rest.parse::<u32>() {
            return EffectType::Vst(id);
        }
    }
    match s {
        "Delay" => EffectType::Delay, "Reverb" => EffectType::Reverb,
        "Gate" => EffectType::Gate, "TapeComp" => EffectType::TapeComp,
        "SidechainComp" => EffectType::SidechainComp,
        "Chorus" => EffectType::Chorus, "Flanger" => EffectType::Flanger,
        "Phaser" => EffectType::Phaser, "Tremolo" => EffectType::Tremolo,
        "Distortion" => EffectType::Distortion, "Bitcrusher" => EffectType::Bitcrusher,
        "Wavefolder" => EffectType::Wavefolder, "Saturator" => EffectType::Saturator,
        "TiltEq" => EffectType::TiltEq,
        "StereoWidener" => EffectType::StereoWidener, "FreqShifter" => EffectType::FreqShifter,
        "Limiter" => EffectType::Limiter, "PitchShifter" => EffectType::PitchShifter,
        "Vinyl" => EffectType::Vinyl, "Cabinet" => EffectType::Cabinet,
        "GranularDelay" => EffectType::GranularDelay, "GranularFreeze" => EffectType::GranularFreeze,
        "ConvolutionReverb" => EffectType::ConvolutionReverb,
        "Vocoder" => EffectType::Vocoder, "RingMod" => EffectType::RingMod,
        "Autopan" => EffectType::Autopan, "Resonator" => EffectType::Resonator,
        "MultibandComp" => EffectType::MultibandComp, "ParaEq" => EffectType::ParaEq,
        "SpectralFreeze" => EffectType::SpectralFreeze, "Glitch" => EffectType::Glitch,
        "Leslie" => EffectType::Leslie, "SpringReverb" => EffectType::SpringReverb,
        "EnvFollower" => EffectType::EnvFollower,
        "MidSide" => EffectType::MidSide, "Crossfader" => EffectType::Crossfader,
        "Denoise" => EffectType::Denoise, "Autotune" => EffectType::Autotune,
        "WahPedal" => EffectType::WahPedal,
        _ => EffectType::Delay, // fallback
    }
}

fn decode_filter_type(s: &str) -> crate::state::instrument::FilterType {
    use crate::state::instrument::FilterType;
    match s {
        "Hpf" => FilterType::Hpf, "Bpf" => FilterType::Bpf, "Notch" => FilterType::Notch,
        "Comb" => FilterType::Comb, "Allpass" => FilterType::Allpass,
        "Vowel" => FilterType::Vowel, "ResDrive" => FilterType::ResDrive,
        _ => FilterType::Lpf,
    }
}

fn decode_eq_band_type(s: &str) -> crate::state::instrument::EqBandType {
    use crate::state::instrument::EqBandType;
    match s {
        "LowShelf" => EqBandType::LowShelf, "HighShelf" => EqBandType::HighShelf,
        _ => EqBandType::Peaking,
    }
}

fn decode_lfo_shape(s: &str) -> crate::state::instrument::LfoShape {
    use crate::state::instrument::LfoShape;
    match s {
        "Square" => LfoShape::Square, "Saw" => LfoShape::Saw, "Triangle" => LfoShape::Triangle,
        _ => LfoShape::Sine,
    }
}

pub fn decode_parameter_target(s: &str) -> crate::state::instrument::ParameterTarget {
    use crate::state::instrument::ParameterTarget;

    if let Some(rest) = s.strip_prefix("SendLevel:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::SendLevel(idx); }
    }
    if let Some(rest) = s.strip_prefix("EffectParam:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let (Ok(eid), Ok(pidx)) = (parts[0].parse::<u32>(), parts[1].parse::<usize>()) {
                return ParameterTarget::EffectParam(eid, pidx);
            }
        }
    }
    if let Some(rest) = s.strip_prefix("EffectBypass:") {
        if let Ok(eid) = rest.parse::<u32>() { return ParameterTarget::EffectBypass(eid); }
    }
    if let Some(rest) = s.strip_prefix("EqBandFreq:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandFreq(idx); }
    }
    if let Some(rest) = s.strip_prefix("EqBandGain:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandGain(idx); }
    }
    if let Some(rest) = s.strip_prefix("EqBandQ:") {
        if let Ok(idx) = rest.parse::<usize>() { return ParameterTarget::EqBandQ(idx); }
    }
    if let Some(rest) = s.strip_prefix("VstParam:") {
        if let Ok(idx) = rest.parse::<u32>() { return ParameterTarget::VstParam(idx); }
    }

    match s {
        "Level" => ParameterTarget::Level, "Pan" => ParameterTarget::Pan,
        "FilterCutoff" => ParameterTarget::FilterCutoff,
        "FilterResonance" => ParameterTarget::FilterResonance,
        "FilterBypass" => ParameterTarget::FilterBypass,
        "Attack" => ParameterTarget::Attack, "Decay" => ParameterTarget::Decay,
        "Sustain" => ParameterTarget::Sustain, "Release" => ParameterTarget::Release,
        "Pitch" => ParameterTarget::Pitch, "PulseWidth" => ParameterTarget::PulseWidth,
        "Detune" => ParameterTarget::Detune, "FmIndex" => ParameterTarget::FmIndex,
        "WavetablePosition" => ParameterTarget::WavetablePosition,
        "FormantFreq" => ParameterTarget::FormantFreq, "SyncRatio" => ParameterTarget::SyncRatio,
        "Pressure" => ParameterTarget::Pressure, "Embouchure" => ParameterTarget::Embouchure,
        "GrainSize" => ParameterTarget::GrainSize, "GrainDensity" => ParameterTarget::GrainDensity,
        "FbFeedback" => ParameterTarget::FbFeedback, "RingModDepth" => ParameterTarget::RingModDepth,
        "ChaosParam" => ParameterTarget::ChaosParam,
        "AdditiveRolloff" => ParameterTarget::AdditiveRolloff,
        "MembraneTension" => ParameterTarget::MembraneTension,
        "SampleRate" => ParameterTarget::SampleRate, "SampleAmp" => ParameterTarget::SampleAmp,
        "StretchRatio" => ParameterTarget::StretchRatio, "PitchShift" => ParameterTarget::PitchShift,
        "DelayTime" => ParameterTarget::DelayTime, "DelayFeedback" => ParameterTarget::DelayFeedback,
        "ReverbMix" => ParameterTarget::ReverbMix, "GateRate" => ParameterTarget::GateRate,
        "LfoRate" => ParameterTarget::LfoRate, "LfoDepth" => ParameterTarget::LfoDepth,
        "Swing" => ParameterTarget::Swing,
        "HumanizeVelocity" => ParameterTarget::HumanizeVelocity,
        "HumanizeTiming" => ParameterTarget::HumanizeTiming,
        "TimingOffset" => ParameterTarget::TimingOffset,
        "TimeSignature" => ParameterTarget::TimeSignature,
        _ => ParameterTarget::Level, // fallback
    }
}

fn decode_output_target(s: &str) -> crate::state::instrument::OutputTarget {
    use crate::state::instrument::OutputTarget;
    if let Some(rest) = s.strip_prefix("Bus:") {
        if let Ok(id) = rest.parse::<u8>() {
            return OutputTarget::Bus(id);
        }
    }
    OutputTarget::Master
}

fn decode_channel_config(s: &str) -> imbolc_types::ChannelConfig {
    use imbolc_types::ChannelConfig;
    match s {
        "Mono" => ChannelConfig::Mono,
        _ => ChannelConfig::Stereo,
    }
}

fn decode_curve_type(s: &str) -> crate::state::automation::CurveType {
    use crate::state::automation::CurveType;
    match s {
        "Exponential" => CurveType::Exponential,
        "Step" => CurveType::Step,
        "SCurve" => CurveType::SCurve,
        _ => CurveType::Linear,
    }
}

fn decode_play_mode(s: &str) -> crate::state::arrangement::PlayMode {
    use crate::state::arrangement::PlayMode;
    match s {
        "Song" => PlayMode::Song,
        _ => PlayMode::Pattern,
    }
}

fn decode_arp_direction(s: &str) -> crate::state::arpeggiator::ArpDirection {
    use crate::state::arpeggiator::ArpDirection;
    match s {
        "Down" => ArpDirection::Down, "UpDown" => ArpDirection::UpDown,
        "Random" => ArpDirection::Random,
        _ => ArpDirection::Up,
    }
}

fn decode_arp_rate(s: &str) -> crate::state::arpeggiator::ArpRate {
    use crate::state::arpeggiator::ArpRate;
    match s {
        "Quarter" => ArpRate::Quarter, "Sixteenth" => ArpRate::Sixteenth,
        "ThirtySecond" => ArpRate::ThirtySecond,
        _ => ArpRate::Eighth,
    }
}

fn decode_chord_shape(s: &str) -> crate::state::arpeggiator::ChordShape {
    use crate::state::arpeggiator::ChordShape;
    match s {
        "Minor" => ChordShape::Minor, "Seventh" => ChordShape::Seventh,
        "MinorSeventh" => ChordShape::MinorSeventh, "Sus2" => ChordShape::Sus2,
        "Sus4" => ChordShape::Sus4, "PowerChord" => ChordShape::PowerChord,
        "Octave" => ChordShape::Octave,
        _ => ChordShape::Major,
    }
}

fn decode_swing_grid(s: &str) -> imbolc_types::state::groove::SwingGrid {
    use imbolc_types::state::groove::SwingGrid;
    match s {
        "Sixteenths" => SwingGrid::Sixteenths, "Both" => SwingGrid::Both,
        _ => SwingGrid::Eighths,
    }
}

fn decode_step_resolution(s: &str) -> imbolc_types::StepResolution {
    use imbolc_types::StepResolution;
    match s {
        "Quarter" => StepResolution::Quarter, "Eighth" => StepResolution::Eighth,
        "ThirtySecond" => StepResolution::ThirtySecond,
        _ => StepResolution::Sixteenth,
    }
}

pub fn decode_automation_target(
    target_type: &str,
    target_inst_id: Option<i64>,
    target_bus_id: Option<i64>,
    _target_extra: Option<&str>,
) -> crate::state::AutomationTarget {
    use imbolc_types::{AutomationTarget, BusParameter, GlobalParameter, InstrumentParameter};

    match target_type {
        "BusLevel" => {
            AutomationTarget::Bus(target_bus_id.unwrap_or(1) as u8, BusParameter::Level)
        }
        "GlobalBpm" => AutomationTarget::Global(GlobalParameter::Bpm),
        "GlobalTimeSignature" => AutomationTarget::Global(GlobalParameter::TimeSignature),
        _ => {
            // It's an instrument parameter target
            let inst_id = target_inst_id.unwrap_or(0) as u32;
            let param_target = decode_parameter_target(target_type);
            AutomationTarget::Instrument(inst_id, InstrumentParameter::Standard(param_target))
        }
    }
}
