use rusqlite::{params, Connection, Result as SqlResult};

use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

use super::schema::{self, SCHEMA_VERSION};

/// Save project state to relational tables. Performs DELETE-all + INSERT-current atomically.
pub fn save_relational(
    conn: &Connection,
    session: &SessionState,
    instruments: &InstrumentState,
) -> SqlResult<()> {
    schema::delete_all_data(conn)?;

    // Schema version
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, datetime('now'))",
        params![SCHEMA_VERSION],
    )?;

    save_session(conn, session, instruments)?;
    save_theme(conn, session)?;
    save_instruments(conn, instruments)?;
    save_mixer(conn, session)?;
    save_layer_group_mixers(conn, session)?;
    save_musical_settings(conn, session)?;
    save_piano_roll(conn, session)?;
    save_automation(conn, session)?;
    save_custom_synthdefs(conn, session)?;
    save_vst_plugins(conn, session)?;
    save_midi_recording(conn, session)?;
    save_arrangement(conn, session)?;

    Ok(())
}

// ============================================================
// Session
// ============================================================

fn save_session(
    conn: &Connection,
    session: &SessionState,
    instruments: &InstrumentState,
) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO session (id, bpm, time_sig_num, time_sig_denom, key, scale, tuning_a4, snap,
            next_instrument_id, next_sampler_buffer_id, selected_instrument, next_layer_group_id,
            humanize_velocity, humanize_timing,
            click_enabled, click_volume, click_muted)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            session.bpm,
            session.time_signature.0,
            session.time_signature.1,
            format!("{:?}", session.key),
            format!("{:?}", session.scale),
            session.tuning_a4,
            session.snap as i32,
            instruments.next_id,
            instruments.next_sampler_buffer_id,
            instruments.selected.map(|s| s as i64),
            instruments.next_layer_group_id,
            session.humanize.velocity,
            session.humanize.timing,
            session.click_track.enabled as i32,
            session.click_track.volume,
            session.click_track.muted as i32,
        ],
    )?;
    Ok(())
}

fn save_theme(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    let t = &session.theme;
    conn.execute(
        "INSERT INTO theme (id, name,
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
            armed_r, armed_g, armed_b)
         VALUES (1, ?1,
            ?2,?3,?4, ?5,?6,?7, ?8,?9,?10, ?11,?12,?13, ?14,?15,?16,
            ?17,?18,?19, ?20,?21,?22, ?23,?24,?25, ?26,?27,?28,
            ?29,?30,?31, ?32,?33,?34, ?35,?36,?37, ?38,?39,?40,
            ?41,?42,?43, ?44,?45,?46, ?47,?48,?49, ?50,?51,?52,
            ?53,?54,?55, ?56,?57,?58, ?59,?60,?61, ?62,?63,?64,
            ?65,?66,?67, ?68,?69,?70, ?71,?72,?73, ?74,?75,?76,
            ?77,?78,?79, ?80,?81,?82)",
        params![
            t.name,
            t.background.r, t.background.g, t.background.b,
            t.foreground.r, t.foreground.g, t.foreground.b,
            t.border.r, t.border.g, t.border.b,
            t.selection_bg.r, t.selection_bg.g, t.selection_bg.b,
            t.selection_fg.r, t.selection_fg.g, t.selection_fg.b,
            t.muted.r, t.muted.g, t.muted.b,
            t.error.r, t.error.g, t.error.b,
            t.warning.r, t.warning.g, t.warning.b,
            t.success.r, t.success.g, t.success.b,
            t.osc_color.r, t.osc_color.g, t.osc_color.b,
            t.filter_color.r, t.filter_color.g, t.filter_color.b,
            t.env_color.r, t.env_color.g, t.env_color.b,
            t.lfo_color.r, t.lfo_color.g, t.lfo_color.b,
            t.fx_color.r, t.fx_color.g, t.fx_color.b,
            t.sample_color.r, t.sample_color.g, t.sample_color.b,
            t.midi_color.r, t.midi_color.g, t.midi_color.b,
            t.audio_in_color.r, t.audio_in_color.g, t.audio_in_color.b,
            t.meter_low.r, t.meter_low.g, t.meter_low.b,
            t.meter_mid.r, t.meter_mid.g, t.meter_mid.b,
            t.meter_high.r, t.meter_high.g, t.meter_high.b,
            t.waveform_gradient[0].r, t.waveform_gradient[0].g, t.waveform_gradient[0].b,
            t.waveform_gradient[1].r, t.waveform_gradient[1].g, t.waveform_gradient[1].b,
            t.waveform_gradient[2].r, t.waveform_gradient[2].g, t.waveform_gradient[2].b,
            t.waveform_gradient[3].r, t.waveform_gradient[3].g, t.waveform_gradient[3].b,
            t.playing.r, t.playing.g, t.playing.b,
            t.recording.r, t.recording.g, t.recording.b,
            t.armed.r, t.armed.g, t.armed.b,
        ],
    )?;
    Ok(())
}

// ============================================================
// Instruments
// ============================================================

fn save_instruments(conn: &Connection, instruments: &InstrumentState) -> SqlResult<()> {
    let mut inst_stmt = conn.prepare(
        "INSERT INTO instruments (
            id, name, position, source_type,
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
            groove_timing_offset_ms, groove_time_sig_num, groove_time_sig_denom,
            layer_octave_offset)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48)",
    )?;

    for (pos, inst) in instruments.instruments.iter().enumerate() {
        let source_type = encode_source_type(&inst.source);
        let output_target = encode_output_target(&inst.output_target);
        let channel_config = format!("{:?}", inst.channel_config);

        let (filter_type, filter_cutoff, filter_cutoff_min, filter_cutoff_max,
             filter_resonance, filter_resonance_min, filter_resonance_max, filter_enabled) =
            if let Some(ref f) = inst.filter {
                (
                    Some(format!("{:?}", f.filter_type)),
                    Some(f.cutoff.value), Some(f.cutoff.min), Some(f.cutoff.max),
                    Some(f.resonance.value), Some(f.resonance.min), Some(f.resonance.max),
                    f.enabled as i32,
                )
            } else {
                (None, None, None, None, None, None, None, 1)
            };

        let eq_enabled = inst.eq.as_ref().map(|eq| eq.enabled as i32);

        let chord_shape = inst.chord_shape.as_ref().map(|cs| format!("{:?}", cs));

        let vst_state = inst.vst_state_path.as_ref().map(|p| p.to_string_lossy().to_string());

        let groove = &inst.groove;
        let groove_time_sig_num = groove.time_signature.map(|(n, _)| n as i32);
        let groove_time_sig_denom = groove.time_signature.map(|(_, d)| d as i32);
        let groove_swing_grid = groove.swing_grid.as_ref().map(|g| format!("{:?}", g));

        inst_stmt.execute(params![
            inst.id,
            inst.name,
            pos as i32,
            source_type,
            filter_type,
            filter_cutoff, filter_cutoff_min, filter_cutoff_max,
            filter_resonance, filter_resonance_min, filter_resonance_max,
            filter_enabled,
            inst.lfo.enabled as i32,
            inst.lfo.rate,
            inst.lfo.depth,
            format!("{:?}", inst.lfo.shape),
            encode_parameter_target(&inst.lfo.target),
            inst.amp_envelope.attack,
            inst.amp_envelope.decay,
            inst.amp_envelope.sustain,
            inst.amp_envelope.release,
            inst.polyphonic as i32,
            inst.level,
            inst.pan,
            inst.mute as i32,
            inst.solo as i32,
            inst.active as i32,
            output_target,
            channel_config,
            inst.convolution_ir_path.as_deref(),
            inst.layer_group,
            inst.next_effect_id,
            eq_enabled,
            inst.arpeggiator.enabled as i32,
            format!("{:?}", inst.arpeggiator.direction),
            format!("{:?}", inst.arpeggiator.rate),
            inst.arpeggiator.octaves,
            inst.arpeggiator.gate,
            chord_shape,
            vst_state,
            groove.swing_amount,
            groove_swing_grid,
            groove.humanize_velocity,
            groove.humanize_timing,
            groove.timing_offset_ms,
            groove_time_sig_num,
            groove_time_sig_denom,
            inst.layer_octave_offset as i32,
        ])?;

        // Source params
        save_params(conn, "instrument_source_params", "instrument_id", inst.id, &inst.source_params)?;

        // Effects
        save_effects(conn, inst.id, &inst.effects)?;

        // Sends
        for send in &inst.sends {
            conn.execute(
                "INSERT INTO instrument_sends (instrument_id, bus_id, level, enabled, tap_point)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![inst.id, send.bus_id, send.level, send.enabled as i32, encode_tap_point(send.tap_point)],
            )?;
        }

        // Filter modulations
        if let Some(ref f) = inst.filter {
            save_modulation(conn, inst.id, "cutoff", &f.cutoff.mod_source)?;
            save_modulation(conn, inst.id, "resonance", &f.resonance.mod_source)?;

            // Filter extra params
            save_params(conn, "instrument_filter_extra_params", "instrument_id", inst.id, &f.extra_params)?;
        }

        // EQ bands
        if let Some(ref eq) = inst.eq {
            for (i, band) in eq.bands.iter().enumerate() {
                conn.execute(
                    "INSERT INTO instrument_eq_bands (instrument_id, band_index, band_type, freq, gain, q, enabled)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        inst.id, i as i32, format!("{:?}", band.band_type),
                        band.freq, band.gain, band.q, band.enabled as i32,
                    ],
                )?;
            }
        }

        // VST param values
        for (param_idx, value) in &inst.vst_param_values {
            conn.execute(
                "INSERT INTO instrument_vst_params (instrument_id, param_index, value)
                 VALUES (?1, ?2, ?3)",
                params![inst.id, param_idx, value],
            )?;
        }

        // Sampler config
        if let Some(ref config) = inst.sampler_config {
            save_sampler_config(conn, inst.id, config)?;
        }

        // Drum sequencer
        if let Some(ref seq) = inst.drum_sequencer {
            save_drum_sequencer(conn, inst.id, seq)?;
        }
    }

    Ok(())
}

fn save_params(
    conn: &Connection,
    table: &str,
    id_col: &str,
    id: u32,
    params: &[crate::state::param::Param],
) -> SqlResult<()> {
    let sql = format!(
        "INSERT INTO {} ({}, position, param_name, param_value_type, param_value_float, param_value_int, param_value_bool, param_min, param_max)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        table, id_col
    );
    let mut stmt = conn.prepare(&sql)?;

    for (pos, p) in params.iter().enumerate() {
        let (vtype, vf, vi, vb) = encode_param_value(&p.value);
        stmt.execute(params![id, pos as i32, p.name, vtype, vf, vi, vb, p.min, p.max])?;
    }
    Ok(())
}

fn save_effects(conn: &Connection, instrument_id: u32, effects: &[crate::state::instrument::EffectSlot]) -> SqlResult<()> {
    save_effects_to(conn, "instrument_effects", "instrument_effect_params", "effect_vst_params", "instrument_id", instrument_id as u32, effects)
}

fn save_effects_to(
    conn: &Connection,
    effects_table: &str,
    params_table: &str,
    vst_table: &str,
    owner_col: &str,
    owner_id: u32,
    effects: &[crate::state::instrument::EffectSlot],
) -> SqlResult<()> {
    for (pos, effect) in effects.iter().enumerate() {
        let effect_type = encode_effect_type(&effect.effect_type);
        let vst_state = effect.vst_state_path.as_ref().map(|p| p.to_string_lossy().to_string());

        let sql = format!(
            "INSERT INTO {} ({}, effect_id, position, effect_type, enabled, vst_state_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            effects_table, owner_col
        );
        conn.execute(&sql, params![owner_id, effect.id, pos as i32, effect_type, effect.enabled as i32, vst_state])?;

        // Effect params
        let param_sql = format!(
            "INSERT INTO {} ({}, effect_id, position, param_name, param_value_type, param_value_float, param_value_int, param_value_bool, param_min, param_max)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params_table, owner_col
        );
        let mut stmt = conn.prepare(&param_sql)?;
        for (ppos, p) in effect.params.iter().enumerate() {
            let (vtype, vf, vi, vb) = encode_param_value(&p.value);
            stmt.execute(params![owner_id, effect.id, ppos as i32, p.name, vtype, vf, vi, vb, p.min, p.max])?;
        }

        // Effect VST param values
        let vst_sql = format!(
            "INSERT INTO {} ({}, effect_id, param_index, value)
             VALUES (?1, ?2, ?3, ?4)",
            vst_table, owner_col
        );
        for (param_idx, value) in &effect.vst_param_values {
            conn.execute(&vst_sql, params![owner_id, effect.id, param_idx, value])?;
        }
    }
    Ok(())
}

fn save_modulation(
    conn: &Connection,
    instrument_id: u32,
    target_param: &str,
    mod_source: &Option<crate::state::instrument::ModSource>,
) -> SqlResult<()> {
    use crate::state::instrument::ModSource;

    if let Some(ms) = mod_source {
        match ms {
            ModSource::Lfo(lfo) => {
                conn.execute(
                    "INSERT INTO instrument_modulations (instrument_id, target_param, mod_type,
                        lfo_enabled, lfo_rate, lfo_depth, lfo_shape, lfo_target)
                     VALUES (?1, ?2, 'Lfo', ?3, ?4, ?5, ?6, ?7)",
                    params![
                        instrument_id, target_param,
                        lfo.enabled as i32, lfo.rate, lfo.depth,
                        format!("{:?}", lfo.shape),
                        encode_parameter_target(&lfo.target),
                    ],
                )?;
            }
            ModSource::Envelope(env) => {
                conn.execute(
                    "INSERT INTO instrument_modulations (instrument_id, target_param, mod_type,
                        env_attack, env_decay, env_sustain, env_release)
                     VALUES (?1, ?2, 'Envelope', ?3, ?4, ?5, ?6)",
                    params![
                        instrument_id, target_param,
                        env.attack, env.decay, env.sustain, env.release,
                    ],
                )?;
            }
            ModSource::InstrumentParam(src_id, param_name) => {
                conn.execute(
                    "INSERT INTO instrument_modulations (instrument_id, target_param, mod_type,
                        source_instrument_id, source_param_name)
                     VALUES (?1, ?2, 'InstrumentParam', ?3, ?4)",
                    params![instrument_id, target_param, src_id, param_name],
                )?;
            }
        }
    }
    Ok(())
}

fn save_sampler_config(
    conn: &Connection,
    instrument_id: u32,
    config: &crate::state::sampler::SamplerConfig,
) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO sampler_configs (instrument_id, buffer_id, sample_name, loop_mode, pitch_tracking, next_slice_id, selected_slice)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            instrument_id,
            config.buffer_id.map(|id| id as i64),
            config.sample_name.as_deref(),
            config.loop_mode as i32,
            config.pitch_tracking as i32,
            config.next_slice_id(),
            config.selected_slice as i32,
        ],
    )?;

    for (pos, slice) in config.slices.iter().enumerate() {
        conn.execute(
            "INSERT INTO sampler_slices (instrument_id, slice_id, position, start_pos, end_pos, name, root_note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                instrument_id, slice.id, pos as i32,
                slice.start, slice.end, slice.name, slice.root_note as i32,
            ],
        )?;
    }
    Ok(())
}

fn save_drum_sequencer(
    conn: &Connection,
    instrument_id: u32,
    seq: &crate::state::drum_sequencer::DrumSequencerState,
) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO drum_sequencer_state (instrument_id, current_pattern, next_buffer_id, swing_amount, chain_enabled, step_resolution)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            instrument_id,
            seq.current_pattern as i32,
            seq.next_buffer_id,
            seq.swing_amount,
            seq.chain_enabled as i32,
            format!("{:?}", seq.step_resolution),
        ],
    )?;

    // Chain
    for (pos, &pattern_idx) in seq.chain.iter().enumerate() {
        conn.execute(
            "INSERT INTO drum_sequencer_chain (instrument_id, position, pattern_index)
             VALUES (?1, ?2, ?3)",
            params![instrument_id, pos as i32, pattern_idx as i32],
        )?;
    }

    // Pads
    for (pad_idx, pad) in seq.pads.iter().enumerate() {
        conn.execute(
            "INSERT INTO drum_pads (instrument_id, pad_index, buffer_id, path, name, level, slice_start, slice_end, reverse, pitch, trigger_instrument_id, trigger_freq)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                instrument_id, pad_idx as i32,
                pad.buffer_id.map(|id| id as i64),
                pad.path.as_deref(),
                pad.name,
                pad.level,
                pad.slice_start,
                pad.slice_end,
                pad.reverse as i32,
                pad.pitch as i32,
                pad.instrument_id.map(|id| id as i64),
                pad.trigger_freq,
            ],
        )?;
    }

    // Patterns and steps
    for (pat_idx, pattern) in seq.patterns.iter().enumerate() {
        conn.execute(
            "INSERT INTO drum_patterns (instrument_id, pattern_index, length)
             VALUES (?1, ?2, ?3)",
            params![instrument_id, pat_idx as i32, pattern.length as i32],
        )?;

        // Only save active steps (sparse)
        for (pad_idx, pad_steps) in pattern.steps.iter().enumerate() {
            for (step_idx, step) in pad_steps.iter().enumerate() {
                if step.active {
                    conn.execute(
                        "INSERT INTO drum_steps (instrument_id, pattern_index, pad_index, step_index, velocity, probability, pitch_offset)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            instrument_id, pat_idx as i32, pad_idx as i32, step_idx as i32,
                            step.velocity as i32, step.probability, step.pitch_offset as i32,
                        ],
                    )?;
                }
            }
        }
    }

    // Chopper
    if let Some(ref chopper) = seq.chopper {
        let peaks_blob: Option<Vec<u8>> = if chopper.waveform_peaks.is_empty() {
            None
        } else {
            // Store as raw f32 bytes
            let mut bytes = Vec::with_capacity(chopper.waveform_peaks.len() * 4);
            for &peak in &chopper.waveform_peaks {
                bytes.extend_from_slice(&peak.to_le_bytes());
            }
            Some(bytes)
        };

        conn.execute(
            "INSERT INTO chopper_states (instrument_id, buffer_id, path, name, selected_slice, next_slice_id, duration_secs, waveform_peaks)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                instrument_id,
                chopper.buffer_id.map(|id| id as i64),
                chopper.path.as_deref(),
                chopper.name,
                chopper.selected_slice as i32,
                chopper.next_slice_id,
                chopper.duration_secs,
                peaks_blob,
            ],
        )?;

        for (pos, slice) in chopper.slices.iter().enumerate() {
            conn.execute(
                "INSERT INTO chopper_slices (instrument_id, slice_id, position, start_pos, end_pos, name, root_note)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    instrument_id, slice.id, pos as i32,
                    slice.start, slice.end, slice.name, slice.root_note as i32,
                ],
            )?;
        }
    }

    Ok(())
}

// ============================================================
// Mixer
// ============================================================

fn save_mixer(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    for bus in &session.mixer.buses {
        conn.execute(
            "INSERT INTO mixer_buses (id, name, level, pan, mute, solo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![bus.id as i32, bus.name, bus.level, bus.pan, bus.mute as i32, bus.solo as i32],
        )?;

        save_effects_to(conn, "bus_effects", "bus_effect_params", "bus_effect_vst_params", "bus_id", bus.id as u32, &bus.effects)?;
    }

    conn.execute(
        "INSERT INTO mixer_master (id, level, mute) VALUES (1, ?1, ?2)",
        params![session.mixer.master_level, session.mixer.master_mute as i32],
    )?;
    Ok(())
}

fn save_layer_group_mixers(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    for gm in &session.mixer.layer_group_mixers {
        let output_target = encode_output_target(&gm.output_target);
        conn.execute(
            "INSERT INTO layer_group_mixers (group_id, name, level, pan, mute, solo, output_target)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                gm.group_id as i32, gm.name, gm.level, gm.pan,
                gm.mute as i32, gm.solo as i32, output_target,
            ],
        )?;

        for send in &gm.sends {
            conn.execute(
                "INSERT INTO layer_group_sends (group_id, bus_id, level, enabled, tap_point)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![gm.group_id as i32, send.bus_id as i32, send.level, send.enabled as i32, encode_tap_point(send.tap_point)],
            )?;
        }

        save_effects_to(conn, "layer_group_effects", "layer_group_effect_params", "layer_group_effect_vst_params", "group_id", gm.group_id, &gm.effects)?;
    }
    Ok(())
}

// ============================================================
// Musical Settings / Piano Roll
// ============================================================

fn save_musical_settings(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    let pr = &session.piano_roll;
    conn.execute(
        "INSERT INTO musical_settings (id, bpm, time_sig_num, time_sig_denom, ticks_per_beat, loop_start, loop_end, looping, swing_amount)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            pr.bpm,
            pr.time_signature.0,
            pr.time_signature.1,
            pr.ticks_per_beat,
            pr.loop_start,
            pr.loop_end,
            pr.looping as i32,
            pr.swing_amount,
        ],
    )?;
    Ok(())
}

fn save_piano_roll(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    let pr = &session.piano_roll;
    let mut note_id: i64 = 0;

    for (pos, &inst_id) in pr.track_order.iter().enumerate() {
        if let Some(track) = pr.tracks.get(&inst_id) {
            conn.execute(
                "INSERT INTO piano_roll_tracks (instrument_id, position, polyphonic)
                 VALUES (?1, ?2, ?3)",
                params![inst_id, pos as i32, track.polyphonic as i32],
            )?;

            for note in &track.notes {
                conn.execute(
                    "INSERT INTO piano_roll_notes (id, track_instrument_id, tick, duration, pitch, velocity, probability)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        note_id, inst_id,
                        note.tick as i64, note.duration as i64,
                        note.pitch as i32, note.velocity as i32,
                        note.probability,
                    ],
                )?;
                note_id += 1;
            }
        }
    }
    Ok(())
}

// ============================================================
// Automation
// ============================================================

fn save_automation(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    for lane in &session.automation.lanes {
        let (target_type, target_inst_id, target_bus_id, target_effect_id, target_param_idx, target_extra) =
            encode_automation_target(&lane.target);

        conn.execute(
            "INSERT INTO automation_lanes (id, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, enabled, record_armed, min_value, max_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                lane.id, target_type, target_inst_id, target_bus_id,
                target_effect_id, target_param_idx, target_extra,
                lane.enabled as i32, lane.record_armed as i32,
                lane.min_value, lane.max_value,
            ],
        )?;

        for point in &lane.points {
            conn.execute(
                "INSERT INTO automation_points (lane_id, tick, value, curve_type)
                 VALUES (?1, ?2, ?3, ?4)",
                params![lane.id, point.tick as i64, point.value, format!("{:?}", point.curve)],
            )?;
        }
    }
    Ok(())
}

// ============================================================
// Custom SynthDefs
// ============================================================

fn save_custom_synthdefs(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    for synth in &session.custom_synthdefs.synthdefs {
        conn.execute(
            "INSERT INTO custom_synthdefs (id, name, synthdef_name, source_path)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                synth.id, synth.name, synth.synthdef_name,
                synth.source_path.to_string_lossy().to_string(),
            ],
        )?;

        for (pos, param) in synth.params.iter().enumerate() {
            conn.execute(
                "INSERT INTO custom_synthdef_params (synthdef_id, position, name, default_val, min_val, max_val)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![synth.id, pos as i32, param.name, param.default, param.min, param.max],
            )?;
        }
    }
    Ok(())
}

// ============================================================
// VST Plugins
// ============================================================

fn save_vst_plugins(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    for plugin in &session.vst_plugins.plugins {
        conn.execute(
            "INSERT INTO vst_plugins (id, name, plugin_path, kind)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                plugin.id, plugin.name,
                plugin.plugin_path.to_string_lossy().to_string(),
                format!("{:?}", plugin.kind),
            ],
        )?;

        for (pos, param) in plugin.params.iter().enumerate() {
            conn.execute(
                "INSERT INTO vst_plugin_params (plugin_id, position, param_index, name, default_val, label)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    plugin.id, pos as i32, param.index,
                    param.name, param.default, param.label.as_deref(),
                ],
            )?;
        }
    }
    Ok(())
}

// ============================================================
// MIDI Recording
// ============================================================

fn save_midi_recording(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    let midi = &session.midi_recording;
    conn.execute(
        "INSERT INTO midi_recording_settings (id, live_input_instrument, note_passthrough, channel_filter)
         VALUES (1, ?1, ?2, ?3)",
        params![
            midi.live_input_instrument.map(|id| id as i64),
            midi.note_passthrough as i32,
            midi.channel_filter.map(|ch| ch as i32),
        ],
    )?;

    for (idx, cc) in midi.cc_mappings.iter().enumerate() {
        let (target_type, target_inst_id, target_bus_id, target_effect_id, target_param_idx, target_extra) =
            encode_automation_target(&cc.target);
        conn.execute(
            "INSERT INTO midi_cc_mappings (id, cc_number, channel, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, min_value, max_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                idx as i64, cc.cc_number as i32,
                cc.channel.map(|ch| ch as i32),
                target_type, target_inst_id, target_bus_id,
                target_effect_id, target_param_idx, target_extra,
                cc.min_value, cc.max_value,
            ],
        )?;
    }

    for (idx, pb) in midi.pitch_bend_configs.iter().enumerate() {
        let (target_type, target_inst_id, target_bus_id, target_effect_id, target_param_idx, target_extra) =
            encode_automation_target(&pb.target);
        conn.execute(
            "INSERT INTO midi_pitch_bend_configs (id, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, center_value, range, sensitivity)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                idx as i64,
                target_type, target_inst_id, target_bus_id,
                target_effect_id, target_param_idx, target_extra,
                pb.center_value, pb.range, pb.sensitivity,
            ],
        )?;
    }
    Ok(())
}

// ============================================================
// Arrangement
// ============================================================

fn save_arrangement(conn: &Connection, session: &SessionState) -> SqlResult<()> {
    let arr = &session.arrangement;

    conn.execute(
        "INSERT INTO arrangement_state (id, play_mode, selected_placement, selected_lane, view_start_tick, ticks_per_col, cursor_tick, next_clip_id, next_placement_id, next_clip_automation_lane_id)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            format!("{:?}", arr.play_mode),
            arr.selected_placement.map(|s| s as i64),
            arr.selected_lane as i64,
            arr.view_start_tick as i64,
            arr.ticks_per_col as i64,
            arr.cursor_tick as i64,
            arr.next_clip_id(),
            arr.next_placement_id(),
            arr.next_clip_automation_lane_id(),
        ],
    )?;

    // Clips
    for clip in &arr.clips {
        conn.execute(
            "INSERT INTO arrangement_clips (id, name, instrument_id, length_ticks)
             VALUES (?1, ?2, ?3, ?4)",
            params![clip.id, clip.name, clip.instrument_id, clip.length_ticks as i64],
        )?;

        // Clip notes
        for (pos, note) in clip.notes.iter().enumerate() {
            conn.execute(
                "INSERT INTO arrangement_clip_notes (clip_id, position, tick, duration, pitch, velocity, probability)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    clip.id, pos as i32,
                    note.tick as i64, note.duration as i64,
                    note.pitch as i32, note.velocity as i32,
                    note.probability,
                ],
            )?;
        }

        // Clip automation lanes
        for lane in &clip.automation_lanes {
            let (target_type, target_inst_id, target_bus_id, target_effect_id, target_param_idx, target_extra) =
                encode_automation_target(&lane.target);
            conn.execute(
                "INSERT INTO arrangement_clip_automation_lanes (id, clip_id, target_type, target_instrument_id, target_bus_id, target_effect_id, target_param_idx, target_extra, enabled, record_armed, min_value, max_value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    lane.id, clip.id, target_type, target_inst_id, target_bus_id,
                    target_effect_id, target_param_idx, target_extra,
                    lane.enabled as i32, lane.record_armed as i32,
                    lane.min_value, lane.max_value,
                ],
            )?;

            for point in &lane.points {
                conn.execute(
                    "INSERT INTO arrangement_clip_automation_points (lane_id, tick, value, curve_type)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![lane.id, point.tick as i64, point.value, format!("{:?}", point.curve)],
                )?;
            }
        }
    }

    // Placements
    for placement in &arr.placements {
        conn.execute(
            "INSERT INTO arrangement_placements (id, clip_id, instrument_id, start_tick, length_override)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                placement.id, placement.clip_id, placement.instrument_id,
                placement.start_tick as i64,
                placement.length_override.map(|l| l as i64),
            ],
        )?;
    }

    Ok(())
}

// ============================================================
// Encoding helpers
// ============================================================

fn encode_source_type(source: &crate::state::instrument::SourceType) -> String {
    use crate::state::instrument::SourceType;
    match source {
        SourceType::Custom(id) => format!("Custom:{}", id),
        SourceType::Vst(id) => format!("Vst:{}", id),
        other => format!("{:?}", other),
    }
}

fn encode_effect_type(effect: &crate::state::instrument::EffectType) -> String {
    use crate::state::instrument::EffectType;
    match effect {
        EffectType::Vst(id) => format!("Vst:{}", id),
        other => format!("{:?}", other),
    }
}

fn encode_tap_point(tap_point: crate::state::instrument::SendTapPoint) -> &'static str {
    use crate::state::instrument::SendTapPoint;
    match tap_point {
        SendTapPoint::PreInsert => "PreInsert",
        SendTapPoint::PostInsert => "PostInsert",
    }
}

fn encode_output_target(target: &crate::state::instrument::OutputTarget) -> String {
    use crate::state::instrument::OutputTarget;
    match target {
        OutputTarget::Master => "Master".to_string(),
        OutputTarget::Bus(id) => format!("Bus:{}", id),
    }
}

pub fn encode_parameter_target(target: &crate::state::instrument::ParameterTarget) -> String {
    use crate::state::instrument::ParameterTarget;
    match target {
        ParameterTarget::SendLevel(idx) => format!("SendLevel:{}", idx),
        ParameterTarget::EffectParam(eid, pidx) => format!("EffectParam:{}:{}", eid, pidx),
        ParameterTarget::EffectBypass(eid) => format!("EffectBypass:{}", eid),
        ParameterTarget::EqBandFreq(idx) => format!("EqBandFreq:{}", idx),
        ParameterTarget::EqBandGain(idx) => format!("EqBandGain:{}", idx),
        ParameterTarget::EqBandQ(idx) => format!("EqBandQ:{}", idx),
        ParameterTarget::VstParam(idx) => format!("VstParam:{}", idx),
        other => format!("{:?}", other),
    }
}

pub fn encode_automation_target(
    target: &crate::state::AutomationTarget,
) -> (String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    use imbolc_types::{AutomationTarget, BusParameter, GlobalParameter, InstrumentParameter};
    use crate::state::instrument::ParameterTarget;

    match target {
        AutomationTarget::Instrument(inst_id, InstrumentParameter::Standard(param_target)) => {
            let target_extra = match param_target {
                ParameterTarget::EffectParam(eid, pidx) => Some(format!("{}:{}", eid, pidx)),
                ParameterTarget::EffectBypass(eid) => Some(format!("{}", eid)),
                ParameterTarget::SendLevel(idx) => Some(format!("{}", idx)),
                ParameterTarget::EqBandFreq(idx) => Some(format!("{}", idx)),
                ParameterTarget::EqBandGain(idx) => Some(format!("{}", idx)),
                ParameterTarget::EqBandQ(idx) => Some(format!("{}", idx)),
                ParameterTarget::VstParam(idx) => Some(format!("{}", idx)),
                _ => None,
            };
            let param_name = encode_parameter_target(param_target);
            (param_name, Some(*inst_id as i64), None, None, None, target_extra)
        }
        AutomationTarget::Bus(bus_id, BusParameter::Level) => {
            ("BusLevel".to_string(), None, Some(*bus_id as i64), None, None, None)
        }
        AutomationTarget::Global(GlobalParameter::Bpm) => {
            ("GlobalBpm".to_string(), None, None, None, None, None)
        }
        AutomationTarget::Global(GlobalParameter::TimeSignature) => {
            ("GlobalTimeSignature".to_string(), None, None, None, None, None)
        }
    }
}

fn encode_param_value(value: &crate::state::param::ParamValue) -> (&str, Option<f64>, Option<i64>, Option<i32>) {
    use crate::state::param::ParamValue;
    match value {
        ParamValue::Float(v) => ("Float", Some(*v as f64), None, None),
        ParamValue::Int(v) => ("Int", None, Some(*v as i64), None),
        ParamValue::Bool(v) => ("Bool", None, None, Some(*v as i32)),
    }
}
