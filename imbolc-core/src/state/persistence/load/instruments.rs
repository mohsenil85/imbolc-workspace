use std::path::PathBuf;

use rusqlite::{params, Connection, Result as SqlResult, OptionalExtension};

use imbolc_types::BusId;
use crate::state::instrument_state::InstrumentState;
use super::{table_exists, load_params, load_effects_from};
use super::decoders::*;

pub(super) fn load_instruments(conn: &Connection, instruments: &mut InstrumentState) -> SqlResult<()> {
    use crate::state::instrument::*;
    use crate::state::arpeggiator::ArpeggiatorConfig;
    use imbolc_types::ProcessingStage;
    use imbolc_types::state::groove::GrooveConfig;

    instruments.instruments.clear();

    let has_layer_octave_offset = conn.prepare("SELECT layer_octave_offset FROM instruments LIMIT 0").is_ok();

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
        let mut inst = Instrument::new(imbolc_types::InstrumentId::new(r.id), source);
        inst.name = r.name;
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
        inst.next_effect_id = imbolc_types::EffectId::new(r.next_effect_id);
        inst.arpeggiator = arpeggiator;
        inst.chord_shape = chord_shape;
        inst.vst_state_path = r.vst_state_path.map(PathBuf::from);
        inst.groove = groove;

        // Layer octave offset (backward compat: column may not exist in old databases)
        if has_layer_octave_offset {
            let offset: i32 = conn.query_row(
                "SELECT layer_octave_offset FROM instruments WHERE id = ?1",
                params![r.id],
                |row| row.get(0),
            ).unwrap_or(0);
            inst.layer_octave_offset = offset.clamp(-4, 4) as i8;
        }

        // Source params
        inst.source_params = load_params(conn, "instrument_source_params", "instrument_id", r.id)?;

        // Load effects for chain assembly
        let mut effects = load_effects(conn, r.id)?;
        let mut filter = filter;
        let mut eq = eq;

        // Build processing_chain: use persisted ordering if available, else legacy fallback
        inst.processing_chain.clear();
        if table_exists(conn, "instrument_processing_chain")? {
            let mut ord_stmt = conn.prepare(
                "SELECT stage_type, effect_id FROM instrument_processing_chain \
                 WHERE instrument_id = ?1 ORDER BY position"
            )?;
            let ordering: Vec<(String, Option<u32>)> = ord_stmt.query_map(params![r.id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<u32>>(1)?))
            })?.collect::<SqlResult<_>>()?;

            if ordering.is_empty() {
                // No chain rows — use legacy order (filter → eq → effects)
                if let Some(f) = filter.take() {
                    inst.processing_chain.push(ProcessingStage::Filter(f));
                }
                if let Some(e) = eq.take() {
                    inst.processing_chain.push(ProcessingStage::Eq(e));
                }
            } else {
                for (stage_type, eff_id) in &ordering {
                    match stage_type.as_str() {
                        "filter" => {
                            if let Some(f) = filter.take() {
                                inst.processing_chain.push(ProcessingStage::Filter(f));
                            }
                        }
                        "eq" => {
                            if let Some(e) = eq.take() {
                                inst.processing_chain.push(ProcessingStage::Eq(e));
                            }
                        }
                        "effect" => {
                            if let Some(eid) = eff_id {
                                if let Some(idx) = effects.iter().position(|e| e.id == imbolc_types::EffectId::new(*eid)) {
                                    inst.processing_chain.push(
                                        ProcessingStage::Effect(effects.remove(idx))
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Legacy fallback: filter → eq → effects
            if let Some(f) = filter.take() {
                inst.processing_chain.push(ProcessingStage::Filter(f));
            }
            if let Some(e) = eq.take() {
                inst.processing_chain.push(ProcessingStage::Eq(e));
            }
        }
        // Append any effects not covered by ordering (defensive)
        for effect in effects {
            inst.processing_chain.push(ProcessingStage::Effect(effect));
        }

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

fn load_effects(conn: &Connection, instrument_id: u32) -> SqlResult<Vec<crate::state::instrument::EffectSlot>> {
    load_effects_from(conn, "instrument_effects", "instrument_effect_params", "effect_vst_params", "instrument_id", instrument_id)
}

fn load_sends(conn: &Connection, instrument_id: u32) -> SqlResult<std::collections::BTreeMap<BusId, crate::state::instrument::MixerSend>> {
    use crate::state::instrument::MixerSend;

    // Try with tap_point column first; fall back for old schemas
    let has_tap_point = conn.prepare("SELECT tap_point FROM instrument_sends LIMIT 0").is_ok();
    let query = if has_tap_point {
        "SELECT bus_id, level, enabled, tap_point FROM instrument_sends WHERE instrument_id = ?1 ORDER BY bus_id"
    } else {
        "SELECT bus_id, level, enabled FROM instrument_sends WHERE instrument_id = ?1 ORDER BY bus_id"
    };
    let mut stmt = conn.prepare(query)?;
    let sends = stmt.query_map(params![instrument_id], |row| {
        let tap_point = if has_tap_point {
            decode_tap_point(&row.get::<_, String>(3)?)
        } else {
            Default::default()
        };
        let send = MixerSend {
            bus_id: BusId::new(row.get::<_, i32>(0)? as u8),
            level: row.get(1)?,
            enabled: row.get::<_, i32>(2)? != 0,
            tap_point,
        };
        Ok((send.bus_id, send))
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
                    Some(ModSource::InstrumentParam(imbolc_types::InstrumentId::new(id), param))
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
            seq.pads[idx].instrument_id = trigger_inst.map(|id| imbolc_types::InstrumentId::new(id as u32));
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
