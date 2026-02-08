mod blob;
pub mod checkpoint;
pub mod load;
pub mod save;
pub mod schema;
mod tests;

pub use checkpoint::CheckpointInfo;

use std::path::Path;

use rusqlite::{Connection as SqlConnection, Result as SqlResult};

use super::instrument_state::InstrumentState;
use super::session::SessionState;

/// Save project using relational schema.
///
/// Uses WAL mode and an explicit transaction so the write is atomic:
/// if the process crashes mid-save the previous data remains intact.
pub fn save_project(path: &Path, session: &SessionState, instruments: &InstrumentState) -> SqlResult<()> {
    let conn = SqlConnection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;

    let tx = conn.unchecked_transaction()?;
    schema::create_tables(&tx)?;
    save::save_relational(&tx, session, instruments)?;
    tx.commit()?;

    Ok(())
}

/// Load project from relational format (v7+), with fallback to blob format (v1-v2).
pub fn load_project(path: &Path) -> SqlResult<(SessionState, InstrumentState)> {
    let conn = SqlConnection::open(path)?;

    // Check which format this file uses by looking for the schema_version table
    let has_schema_version: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if has_schema_version {
        // Relational format
        let (mut session, instruments) = load::load_relational(&conn)?;
        session.recompute_next_bus_id();
        Ok((session, instruments))
    } else {
        // Legacy blob format — try to load
        load_project_blob(&conn)
    }
}

/// Current blob format version (legacy).
const BLOB_FORMAT_VERSION: i32 = 2;

fn load_project_blob(conn: &SqlConnection) -> SqlResult<(SessionState, InstrumentState)> {
    let (format_version, session_bytes, instrument_bytes): (i32, Vec<u8>, Vec<u8>) = conn.query_row(
        "SELECT format_version, session_data, instrument_data FROM project_blob WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    if format_version > BLOB_FORMAT_VERSION {
        return Err(rusqlite::Error::InvalidParameterName(
            format!("Project format version {} is newer than supported ({})", format_version, BLOB_FORMAT_VERSION),
        ));
    }

    let mut session = blob::deserialize_session(&session_bytes)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let instruments = blob::deserialize_instruments(&instrument_bytes)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

    // Recompute next_bus_id from loaded buses (not persisted, computed on load)
    session.recompute_next_bus_id();

    Ok((session, instruments))
}
