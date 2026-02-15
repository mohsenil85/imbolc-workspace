use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::decoders::*;
use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

pub(super) fn load_session(
    conn: &Connection,
    session: &mut SessionState,
    instruments: &mut InstrumentState,
) -> SqlResult<()> {
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
    instruments.next_id = imbolc_types::InstrumentId::new(row.7);
    instruments.next_sampler_buffer_id = row.8;
    instruments.selected = row.9.map(|v| v as usize);
    instruments.next_layer_group_id = row.10;
    session.humanize.velocity = row.11;
    session.humanize.timing = row.12;
    session.click_track.enabled = row.13 != 0;
    session.click_track.volume = row.14;
    session.click_track.muted = row.15 != 0;

    // Tuning fields (added in schema v13, may not exist in older DBs)
    if let Ok(tuning_row) = conn.query_row(
        "SELECT tuning, ji_flavor FROM session WHERE id = 1",
        [],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ) {
        session.tuning = decode_tuning(&tuning_row.0);
        session.ji_flavor = decode_ji_flavor(&tuning_row.1);
    }

    Ok(())
}

pub(super) fn load_theme(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::state::ThemeColor;

    let result: Option<String> = conn
        .query_row("SELECT name FROM theme WHERE id = 1", [], |row| row.get(0))
        .optional()?;

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
    let c = |i: usize| ThemeColor {
        r: v[i],
        g: v[i + 1],
        b: v[i + 2],
    };

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

pub(super) fn load_musical_settings(
    conn: &Connection,
    session: &mut SessionState,
) -> SqlResult<()> {
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

pub(super) fn load_piano_roll(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::piano_roll::Note;

    // Clear existing tracks
    session.piano_roll.tracks.clear();
    session.piano_roll.track_order.clear();

    // Load tracks ordered by position
    let mut track_stmt =
        conn.prepare("SELECT instrument_id, polyphonic FROM piano_roll_tracks ORDER BY position")?;
    let tracks: Vec<(imbolc_types::InstrumentId, bool)> = track_stmt
        .query_map([], |row| {
            Ok((
                imbolc_types::InstrumentId::new(row.get::<_, u32>(0)?),
                row.get::<_, i32>(1)? != 0,
            ))
        })?
        .collect::<SqlResult<_>>()?;

    for (inst_id, polyphonic) in &tracks {
        session.piano_roll.add_track(*inst_id);
        if let Some(track) = session.piano_roll.tracks.get_mut(inst_id) {
            track.polyphonic = *polyphonic;
        }
    }

    // Load notes
    let mut note_stmt = conn.prepare(
        "SELECT track_instrument_id, tick, duration, pitch, velocity, probability
         FROM piano_roll_notes ORDER BY track_instrument_id, tick",
    )?;
    let notes: Vec<(imbolc_types::InstrumentId, Note)> = note_stmt
        .query_map([], |row| {
            Ok((
                imbolc_types::InstrumentId::new(row.get::<_, u32>(0)?),
                Note {
                    tick: row.get::<_, u32>(1)?,
                    duration: row.get::<_, u32>(2)?,
                    pitch: row.get::<_, i32>(3)? as u8,
                    velocity: row.get::<_, i32>(4)? as u8,
                    probability: row.get::<_, f32>(5)?,
                },
            ))
        })?
        .collect::<SqlResult<_>>()?;

    for (inst_id, note) in notes {
        if let Some(track) = session.piano_roll.tracks.get_mut(&inst_id) {
            track.notes.push(note);
        }
    }

    Ok(())
}

pub(super) fn load_custom_synthdefs(
    conn: &Connection,
    session: &mut SessionState,
) -> SqlResult<()> {
    use crate::state::custom_synthdef::{CustomSynthDef, CustomSynthDefRegistry, ParamSpec};

    let mut registry = CustomSynthDefRegistry::new();

    let mut stmt = conn
        .prepare("SELECT id, name, synthdef_name, source_path FROM custom_synthdefs ORDER BY id")?;
    let synthdefs: Vec<(u32, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<SqlResult<_>>()?;

    for (id, name, synthdef_name, source_path) in synthdefs {
        let mut param_stmt = conn.prepare(
            "SELECT name, default_val, min_val, max_val FROM custom_synthdef_params WHERE synthdef_id = ?1 ORDER BY position"
        )?;
        let params: Vec<ParamSpec> = param_stmt
            .query_map(params![id], |row| {
                Ok(ParamSpec {
                    name: row.get(0)?,
                    default: row.get(1)?,
                    min: row.get(2)?,
                    max: row.get(3)?,
                })
            })?
            .collect::<SqlResult<_>>()?;

        registry.add(CustomSynthDef {
            id: imbolc_types::CustomSynthDefId::new(id),
            name,
            synthdef_name,
            source_path: PathBuf::from(source_path),
            params,
        });
    }

    session.custom_synthdefs = registry;
    Ok(())
}

pub(super) fn load_vst_plugins(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::vst_plugin::{VstParamSpec, VstPlugin, VstPluginKind, VstPluginRegistry};

    let mut registry = VstPluginRegistry::new();

    let mut stmt =
        conn.prepare("SELECT id, name, plugin_path, kind FROM vst_plugins ORDER BY id")?;
    let plugins: Vec<(u32, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<SqlResult<_>>()?;

    for (id, name, plugin_path, kind_str) in plugins {
        let kind = match kind_str.as_str() {
            "Effect" => VstPluginKind::Effect,
            _ => VstPluginKind::Instrument,
        };

        let mut param_stmt = conn.prepare(
            "SELECT param_index, name, default_val, label FROM vst_plugin_params WHERE plugin_id = ?1 ORDER BY position"
        )?;
        let params: Vec<VstParamSpec> = param_stmt
            .query_map(params![id], |row| {
                Ok(VstParamSpec {
                    index: row.get(0)?,
                    name: row.get(1)?,
                    default: row.get(2)?,
                    label: row.get(3)?,
                })
            })?
            .collect::<SqlResult<_>>()?;

        registry.add(VstPlugin {
            id: imbolc_types::VstPluginId::new(id),
            name,
            plugin_path: PathBuf::from(plugin_path),
            kind,
            params,
        });
    }

    session.vst_plugins = registry;
    Ok(())
}
