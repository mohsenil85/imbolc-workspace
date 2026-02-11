use std::path::PathBuf;

use rusqlite::{params, Connection, Result as SqlResult};

use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

mod arrangement;
pub(crate) mod decoders;
mod instruments;
mod mixer;
mod session;

/// Load project state from relational tables.
pub fn load_relational(conn: &Connection) -> SqlResult<(SessionState, InstrumentState)> {
    let mut session = SessionState::new();
    let mut instruments = InstrumentState::new();

    session::load_session(conn, &mut session, &mut instruments)?;
    session::load_theme(conn, &mut session)?;
    mixer::load_mixer(conn, &mut session)?;
    mixer::load_layer_group_mixers(conn, &mut session)?;
    session::load_musical_settings(conn, &mut session)?;
    session::load_piano_roll(conn, &mut session)?;
    session::load_custom_synthdefs(conn, &mut session)?;
    session::load_vst_plugins(conn, &mut session)?;
    instruments::load_instruments(conn, &mut instruments)?;
    arrangement::load_automation(conn, &mut session)?;
    arrangement::load_midi_recording(conn, &mut session)?;
    arrangement::load_arrangement(conn, &mut session)?;

    // Recompute derived state
    session.recompute_next_bus_id();
    instruments.rebuild_index();

    Ok((session, instruments))
}

pub(super) fn table_exists(conn: &Connection, name: &str) -> SqlResult<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub(super) fn load_params(
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
    let params: Vec<Param> = stmt
        .query_map(params![id], |row| {
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

            Ok(Param {
                name,
                value,
                min,
                max,
            })
        })?
        .collect::<SqlResult<_>>()?;

    Ok(params)
}

pub(super) fn load_effects_from(
    conn: &Connection,
    effects_table: &str,
    params_table: &str,
    vst_table: &str,
    owner_col: &str,
    owner_id: u32,
) -> SqlResult<Vec<crate::state::instrument::EffectSlot>> {
    use crate::state::instrument::EffectSlot;

    let sql = format!(
        "SELECT effect_id, effect_type, enabled, vst_state_path FROM {} WHERE {} = ?1 ORDER BY position",
        effects_table, owner_col
    );
    let mut stmt = conn.prepare(&sql)?;
    let effect_rows: Vec<(u32, String, i32, Option<String>)> = stmt
        .query_map(params![owner_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<SqlResult<_>>()?;

    let mut effects = Vec::new();
    for (effect_id, effect_type_str, enabled, vst_state) in effect_rows {
        let effect_type = decoders::decode_effect_type(&effect_type_str);
        let mut slot = EffectSlot::new(imbolc_types::EffectId::new(effect_id), effect_type);
        slot.enabled = enabled != 0;
        slot.vst_state_path = vst_state.map(PathBuf::from);

        // Effect params
        slot.params = load_effect_params_from(conn, params_table, owner_col, owner_id, effect_id)?;

        // Effect VST param values
        let vst_sql = format!(
            "SELECT param_index, value FROM {} WHERE {} = ?1 AND effect_id = ?2 ORDER BY param_index",
            vst_table, owner_col
        );
        let mut vst_stmt = conn.prepare(&vst_sql)?;
        slot.vst_param_values = vst_stmt
            .query_map(params![owner_id, effect_id], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, f32>(1)?))
            })?
            .collect::<SqlResult<_>>()?;

        effects.push(slot);
    }

    Ok(effects)
}

pub(super) fn load_effect_params_from(
    conn: &Connection,
    table: &str,
    owner_col: &str,
    owner_id: u32,
    effect_id: u32,
) -> SqlResult<Vec<crate::state::param::Param>> {
    use crate::state::param::{Param, ParamValue};

    let sql = format!(
        "SELECT param_name, param_value_type, param_value_float, param_value_int, param_value_bool, param_min, param_max
         FROM {} WHERE {} = ?1 AND effect_id = ?2 ORDER BY position",
        table, owner_col
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<Param> = stmt
        .query_map(params![owner_id, effect_id], |row| {
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

            Ok(Param {
                name,
                value,
                min,
                max,
            })
        })?
        .collect::<SqlResult<_>>()?;

    Ok(params)
}
