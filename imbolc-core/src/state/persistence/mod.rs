mod blob;
mod tests;

use std::path::Path;

use rusqlite::{Connection as SqlConnection, Result as SqlResult};

use super::instrument_state::InstrumentState;
use super::session::SessionState;

/// Save project as MessagePack blobs in SQLite.
///
/// Uses WAL mode and an explicit transaction so the write is atomic:
/// if the process crashes mid-save the previous data remains intact.
pub fn save_project(path: &Path, session: &SessionState, instruments: &InstrumentState) -> SqlResult<()> {
    let conn = SqlConnection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;

    let session_bytes = blob::serialize_session(session)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let instrument_bytes = blob::serialize_instruments(instruments)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_blob (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            format_version INTEGER NOT NULL,
            session_data BLOB NOT NULL,
            instrument_data BLOB NOT NULL
        );"
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO project_blob (id, format_version, session_data, instrument_data) VALUES (1, ?1, ?2, ?3)",
        rusqlite::params![BLOB_FORMAT_VERSION, session_bytes, instrument_bytes],
    )?;
    tx.commit()?;

    Ok(())
}

/// Load project from blob format
pub fn load_project(path: &Path) -> SqlResult<(SessionState, InstrumentState)> {
    let conn = SqlConnection::open(path)?;
    load_project_blob(&conn)
}

/// Current blob format version. Increment when the serialized schema changes.
const BLOB_FORMAT_VERSION: i32 = 1;

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
