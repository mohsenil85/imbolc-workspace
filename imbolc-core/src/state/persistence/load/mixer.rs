use rusqlite::{params, Connection, Result as SqlResult, OptionalExtension};

use crate::state::session::SessionState;
use super::{table_exists, load_effects_from};
use super::decoders::*;

pub(super) fn load_mixer(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::MixerBus;

    session.mixer.buses.clear();

    let has_bus_effects = table_exists(conn, "bus_effects")?;

    let mut stmt = conn.prepare("SELECT id, name, level, pan, mute, solo FROM mixer_buses ORDER BY id")?;
    let buses = stmt.query_map([], |row| {
        Ok(MixerBus {
            id: row.get::<_, i32>(0)? as u8,
            name: row.get(1)?,
            level: row.get(2)?,
            pan: row.get(3)?,
            mute: row.get::<_, i32>(4)? != 0,
            solo: row.get::<_, i32>(5)? != 0,
            effects: Vec::new(),
            next_effect_id: 0,
        })
    })?;

    for bus in buses {
        let mut bus = bus?;
        if has_bus_effects {
            bus.effects = load_effects_from(conn, "bus_effects", "bus_effect_params", "bus_effect_vst_params", "bus_id", bus.id as u32)?;
            bus.recalculate_next_effect_id();
        }
        session.mixer.buses.push(bus);
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

pub(super) fn load_layer_group_mixers(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use imbolc_types::LayerGroupMixer;
    use crate::state::instrument::MixerSend;

    session.mixer.layer_group_mixers.clear();

    let has_group_effects = table_exists(conn, "layer_group_effects")?;

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
        let has_group_tap_point = conn.prepare("SELECT tap_point FROM layer_group_sends LIMIT 0").is_ok();
        let group_send_query = if has_group_tap_point {
            "SELECT bus_id, level, enabled, tap_point FROM layer_group_sends WHERE group_id = ?1 ORDER BY bus_id"
        } else {
            "SELECT bus_id, level, enabled FROM layer_group_sends WHERE group_id = ?1 ORDER BY bus_id"
        };
        let mut send_stmt = conn.prepare(group_send_query)?;
        let sends: Vec<MixerSend> = send_stmt.query_map(params![group_id as i32], |row| {
            let tap_point = if has_group_tap_point {
                decode_tap_point(&row.get::<_, String>(3)?)
            } else {
                Default::default()
            };
            Ok(MixerSend {
                bus_id: row.get::<_, i32>(0)? as u8,
                level: row.get(1)?,
                enabled: row.get::<_, i32>(2)? != 0,
                tap_point,
            })
        })?.collect::<SqlResult<_>>()?;

        let mut gm = LayerGroupMixer {
            group_id,
            name,
            level,
            pan,
            mute: mute != 0,
            solo: solo != 0,
            output_target,
            sends,
            effects: Vec::new(),
            next_effect_id: 0,
            eq: None,
        };

        if has_group_effects {
            gm.effects = load_effects_from(conn, "layer_group_effects", "layer_group_effect_params", "layer_group_effect_vst_params", "group_id", group_id)?;
            gm.recalculate_next_effect_id();
        }

        // Load EQ if the table exists
        if table_exists(conn, "layer_group_eq_bands")? {
            let eq_enabled: i32 = conn.query_row(
                "SELECT eq_enabled FROM layer_group_mixers WHERE group_id = ?1",
                [group_id],
                |row| row.get(0),
            ).unwrap_or(0);
            if eq_enabled != 0 {
                let mut eq = crate::state::instrument::EqConfig::default();
                let mut band_stmt = conn.prepare(
                    "SELECT band_index, freq, gain, q, enabled FROM layer_group_eq_bands WHERE group_id = ?1 ORDER BY band_index"
                )?;
                let bands = band_stmt.query_map([group_id], |row| {
                    let band_index: usize = row.get::<_, i32>(0)? as usize;
                    let freq: f32 = row.get(1)?;
                    let gain: f32 = row.get(2)?;
                    let q: f32 = row.get(3)?;
                    let enabled: bool = row.get::<_, i32>(4)? != 0;
                    Ok((band_index, freq, gain, q, enabled))
                })?.collect::<SqlResult<Vec<_>>>()?;
                for (band_index, freq, gain, q, enabled) in bands {
                    if band_index < eq.bands.len() {
                        eq.bands[band_index].freq = freq;
                        eq.bands[band_index].gain = gain;
                        eq.bands[band_index].q = q;
                        eq.bands[band_index].enabled = enabled;
                    }
                }
                gm.eq = Some(eq);
            }
        }

        session.mixer.layer_group_mixers.push(gm);
    }

    Ok(())
}
