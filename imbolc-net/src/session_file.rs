//! Session token persistence for reconnection across client restarts.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::protocol::SessionToken;

/// Persisted session info for reconnection.
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedSession {
    pub server_addr: String,
    pub token: SessionToken,
    pub client_name: String,
}

/// Get the path to the session token file.
fn session_file_path() -> Option<PathBuf> {
    dirs_path().map(|d| d.join("session_token.json"))
}

fn dirs_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config").join("imbolc"))
}

/// Save a session token for reconnection.
pub fn save_session(server_addr: &str, token: &SessionToken, client_name: &str) -> io::Result<()> {
    let path = session_file_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot determine config dir"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let saved = SavedSession {
        server_addr: server_addr.to_string(),
        token: token.clone(),
        client_name: client_name.to_string(),
    };

    let json = serde_json::to_string_pretty(&saved)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json)
}

/// Load a saved session token.
pub fn load_session() -> Option<SavedSession> {
    let path = session_file_path()?;
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Clear the saved session token.
pub fn clear_session() {
    if let Some(path) = session_file_path() {
        let _ = fs::remove_file(path);
    }
}
