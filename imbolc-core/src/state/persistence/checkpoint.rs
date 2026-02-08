use std::path::Path;

use rusqlite::{params, Connection, DatabaseName, Result as SqlResult};
use rusqlite::session::Session;

use super::blob;
use super::schema;
use super::save;
use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

/// Data tables to include in session diffs (all tables except metadata/checkpoint tables).
const DIFF_TABLES: &[&str] = &[
    "session", "theme", "instruments", "instrument_source_params",
    "instrument_effects", "instrument_effect_params", "instrument_sends",
    "instrument_modulations", "instrument_filter_extra_params",
    "instrument_eq_bands", "instrument_vst_params", "effect_vst_params",
    "mixer_buses", "mixer_master", "musical_settings", "piano_roll_tracks",
    "piano_roll_notes", "sampler_configs", "sampler_slices", "vst_plugins",
    "vst_plugin_params", "automation_lanes", "automation_points",
    "custom_synthdefs", "custom_synthdef_params", "drum_sequencer_state",
    "drum_sequencer_chain", "drum_pads", "drum_patterns", "drum_steps",
    "chopper_states", "chopper_slices", "midi_recording_settings",
    "midi_cc_mappings", "midi_pitch_bend_configs", "arrangement_state",
    "arrangement_clips", "arrangement_clip_notes", "arrangement_placements",
    "arrangement_clip_automation_lanes", "arrangement_clip_automation_points",
];

/// Metadata for a checkpoint, returned by list operations.
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    pub id: i64,
    pub label: String,
    pub created_at: String,
    pub parent_id: Option<i64>,
}

/// Compute a binary changeset between old state (from a previous checkpoint) and
/// the current relational tables in `conn`.
///
/// Returns `None` if the states are identical.
fn compute_changeset(
    conn: &Connection,
    old_session: &SessionState,
    old_instruments: &InstrumentState,
) -> SqlResult<Option<Vec<u8>>> {
    // 1. Write the old state to a temp file
    let tmp = tempfile::NamedTempFile::new()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let tmp_path = tmp.path().to_owned();

    {
        let tmp_conn = Connection::open(&tmp_path)?;
        schema::create_tables(&tmp_conn)?;
        save::save_relational(&tmp_conn, old_session, old_instruments)?;
    }

    // 2. ATTACH the temp file as "old" on the main connection
    conn.execute(
        "ATTACH DATABASE ?1 AS old",
        params![tmp_path.to_str().unwrap_or("")],
    )?;

    // 3. Create a session and diff each table
    let result = (|| -> SqlResult<Option<Vec<u8>>> {
        let mut session = Session::new(conn)?;
        session.attach(None)?;

        for table in DIFF_TABLES {
            session.diff(DatabaseName::Attached("old"), table)?;
        }

        if session.is_empty() {
            return Ok(None);
        }

        let mut output = Vec::new();
        session.changeset_strm(&mut output)?;
        Ok(Some(output))
    })();

    // 4. Always detach, even on error
    let _ = conn.execute("DETACH DATABASE old", []);

    result
}

/// Create a named checkpoint from the current state.
///
/// Serializes session and instrument state as blobs and stores them
/// in the checkpoints table. If a parent checkpoint exists, computes
/// and stores a binary changeset. Returns the new checkpoint's ID.
pub fn create_checkpoint(
    path: &Path,
    label: &str,
    session: &SessionState,
    instruments: &InstrumentState,
) -> SqlResult<i64> {
    let session_blob = blob::serialize_session(session)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let instrument_blob = blob::serialize_instruments(instruments)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

    let mut conn = Connection::open(path)?;
    schema::create_tables(&conn)?;

    // Find the most recent checkpoint to set as parent
    let parent_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM checkpoints ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    conn.execute(
        "INSERT INTO checkpoints (label, created_at, parent_id, session_blob, instrument_blob)
         VALUES (?1, datetime('now'), ?2, ?3, ?4)",
        params![label, parent_id, session_blob, instrument_blob],
    )?;

    let checkpoint_id = conn.last_insert_rowid();

    // Compute and store changeset if there's a parent
    if let Some(parent) = parent_id {
        // Load the parent's blobs and deserialize to old state
        let (parent_session_blob, parent_instrument_blob): (Vec<u8>, Vec<u8>) = conn.query_row(
            "SELECT session_blob, instrument_blob FROM checkpoints WHERE id = ?1",
            params![parent],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let old_session = blob::deserialize_session(&parent_session_blob)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        let old_instruments = blob::deserialize_instruments(&parent_instrument_blob)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

        // Write current (new) state to relational tables inside a savepoint,
        // compute the changeset, then roll back the relational data.
        let changeset_bytes = {
            let mut sp = conn.savepoint()?;
            save::save_relational(&sp, session, instruments)?;
            let bytes = compute_changeset(&sp, &old_session, &old_instruments)?;
            sp.rollback()?;
            bytes
        };

        // Store the changeset (outside the savepoint so it persists)
        if let Some(bytes) = changeset_bytes {
            conn.execute(
                "INSERT INTO checkpoint_changesets (checkpoint_id, changeset) VALUES (?1, ?2)",
                params![checkpoint_id, bytes],
            )?;
        }
    }

    Ok(checkpoint_id)
}

/// Restore session and instrument state from a checkpoint.
pub fn restore_checkpoint(
    path: &Path,
    checkpoint_id: i64,
) -> SqlResult<(SessionState, InstrumentState)> {
    let conn = Connection::open(path)?;

    let (session_blob, instrument_blob): (Vec<u8>, Vec<u8>) = conn.query_row(
        "SELECT session_blob, instrument_blob FROM checkpoints WHERE id = ?1",
        params![checkpoint_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut session = blob::deserialize_session(&session_blob)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let instruments = blob::deserialize_instruments(&instrument_blob)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

    session.recompute_next_bus_id();

    Ok((session, instruments))
}

/// List all checkpoints, newest first.
pub fn list_checkpoints(path: &Path) -> SqlResult<Vec<CheckpointInfo>> {
    let conn = Connection::open(path)?;

    // Check if checkpoints table exists
    let has_table: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='checkpoints'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !has_table {
        return Ok(vec![]);
    }

    let mut stmt = conn.prepare(
        "SELECT id, label, created_at, parent_id FROM checkpoints ORDER BY id DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(CheckpointInfo {
            id: row.get(0)?,
            label: row.get(1)?,
            created_at: row.get(2)?,
            parent_id: row.get(3)?,
        })
    })?;

    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }
    Ok(checkpoints)
}

/// Delete a checkpoint by ID.
pub fn delete_checkpoint(path: &Path, checkpoint_id: i64) -> SqlResult<()> {
    let conn = Connection::open(path)?;

    // Delete associated changesets first (FK)
    conn.execute(
        "DELETE FROM checkpoint_changesets WHERE checkpoint_id = ?1",
        params![checkpoint_id],
    )?;

    // Update children that reference this checkpoint as parent
    conn.execute(
        "UPDATE checkpoints SET parent_id = NULL WHERE parent_id = ?1",
        params![checkpoint_id],
    )?;

    conn.execute(
        "DELETE FROM checkpoints WHERE id = ?1",
        params![checkpoint_id],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::instrument::SourceType;
    use tempfile::NamedTempFile;

    fn save_empty_project(path: &Path) {
        let session = SessionState::new();
        let instruments = InstrumentState::new();
        crate::state::persistence::save_project(path, &session, &instruments).unwrap();
    }

    #[test]
    fn create_and_list_checkpoints() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        let session = SessionState::new();
        let instruments = InstrumentState::new();

        let id1 = create_checkpoint(path, "First", &session, &instruments).unwrap();
        let id2 = create_checkpoint(path, "Second", &session, &instruments).unwrap();

        let list = list_checkpoints(path).unwrap();
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].id, id2);
        assert_eq!(list[0].label, "Second");
        assert_eq!(list[0].parent_id, Some(id1));
        assert_eq!(list[1].id, id1);
        assert_eq!(list[1].label, "First");
        assert_eq!(list[1].parent_id, None);
    }

    #[test]
    fn restore_checkpoint_restores_state() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        // Create state with specific values
        let mut session = SessionState::new();
        session.bpm = 140;
        let mut instruments = InstrumentState::new();
        instruments.add_instrument(SourceType::Saw);
        instruments.add_instrument(SourceType::Sin);

        let cp_id = create_checkpoint(path, "Snapshot", &session, &instruments).unwrap();

        // Restore and verify
        let (loaded_session, loaded_instruments) = restore_checkpoint(path, cp_id).unwrap();
        assert_eq!(loaded_session.bpm, 140);
        assert_eq!(loaded_instruments.instruments.len(), 2);
    }

    #[test]
    fn delete_checkpoint_removes_entry() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        let session = SessionState::new();
        let instruments = InstrumentState::new();

        let id1 = create_checkpoint(path, "First", &session, &instruments).unwrap();
        let id2 = create_checkpoint(path, "Second", &session, &instruments).unwrap();

        delete_checkpoint(path, id1).unwrap();

        let list = list_checkpoints(path).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id2);
        // Parent should be cleared since id1 was deleted
        assert_eq!(list[0].parent_id, None);
    }

    #[test]
    fn list_checkpoints_empty_db() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        let list = list_checkpoints(path).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_checkpoints_no_table() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        // Create a bare SQLite DB without our schema
        let conn = Connection::open(path).unwrap();
        conn.execute("CREATE TABLE dummy (id INTEGER PRIMARY KEY)", []).unwrap();
        drop(conn);

        let list = list_checkpoints(path).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn changeset_stored_between_checkpoints() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        // Checkpoint A: empty state
        let session_a = SessionState::new();
        let instruments_a = InstrumentState::new();
        let _id_a = create_checkpoint(path, "A", &session_a, &instruments_a).unwrap();

        // Checkpoint B: modified state
        let mut session_b = SessionState::new();
        session_b.bpm = 200;
        let mut instruments_b = InstrumentState::new();
        instruments_b.add_instrument(SourceType::Saw);
        let id_b = create_checkpoint(path, "B", &session_b, &instruments_b).unwrap();

        // Verify changeset row exists
        let conn = Connection::open(path).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoint_changesets WHERE checkpoint_id = ?1",
            params![id_b],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1, "Expected changeset to be stored for checkpoint B");
    }

    #[test]
    fn changeset_empty_for_identical_states() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        let session = SessionState::new();
        let instruments = InstrumentState::new();

        let _id1 = create_checkpoint(path, "First", &session, &instruments).unwrap();
        let id2 = create_checkpoint(path, "Second", &session, &instruments).unwrap();

        // Verify NO changeset row — identical states
        let conn = Connection::open(path).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoint_changesets WHERE checkpoint_id = ?1",
            params![id2],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0, "Expected no changeset for identical states");
    }

    #[test]
    fn changeset_captures_differences() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        save_empty_project(path);

        // Checkpoint A: empty
        let session_a = SessionState::new();
        let instruments_a = InstrumentState::new();
        let _id_a = create_checkpoint(path, "A", &session_a, &instruments_a).unwrap();

        // Checkpoint B: with instruments
        let mut session_b = SessionState::new();
        session_b.bpm = 180;
        let mut instruments_b = InstrumentState::new();
        instruments_b.add_instrument(SourceType::Sin);
        instruments_b.add_instrument(SourceType::Sqr);
        let id_b = create_checkpoint(path, "B", &session_b, &instruments_b).unwrap();

        // Verify changeset blob is non-empty
        let conn = Connection::open(path).unwrap();
        let changeset_blob: Vec<u8> = conn.query_row(
            "SELECT changeset FROM checkpoint_changesets WHERE checkpoint_id = ?1",
            params![id_b],
            |row| row.get(0),
        ).unwrap();
        assert!(!changeset_blob.is_empty(), "Changeset should contain diff data");
    }
}
