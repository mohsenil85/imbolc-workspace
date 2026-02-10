use rusqlite::{params, Connection, Result as SqlResult, OptionalExtension};

use crate::state::session::SessionState;
use super::decoders::*;

pub(super) fn load_automation(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
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

pub(super) fn load_midi_recording(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
    use crate::state::midi_recording::{MidiCcMapping, PitchBendConfig};

    let result = conn.query_row(
        "SELECT live_input_instrument, note_passthrough, channel_filter FROM midi_recording_settings WHERE id = 1",
        [],
        |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, i32>(1)?, row.get::<_, Option<i32>>(2)?)),
    ).optional()?;

    if let Some((live_inst, passthrough, channel)) = result {
        session.midi_recording.live_input_instrument = live_inst.map(|v| imbolc_types::InstrumentId::new(v as u32));
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

pub(super) fn load_arrangement(conn: &Connection, session: &mut SessionState) -> SqlResult<()> {
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
        let mut clip = Clip { id, name, instrument_id: imbolc_types::InstrumentId::new(inst_id), length_ticks: length, notes: Vec::new(), automation_lanes: Vec::new() };

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
            instrument_id: imbolc_types::InstrumentId::new(row.get::<_, u32>(2)?),
            start_tick: row.get::<_, i64>(3)? as u32,
            length_override: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
        })
    })?.collect::<SqlResult<_>>()?;

    // Recalculate next IDs
    session.arrangement.recalculate_next_ids();

    Ok(())
}
