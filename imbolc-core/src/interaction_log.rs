//! Append-only JSONL interaction logs for debugging and replay.
//!
//! Two independent log files:
//! - **UI log** (`~/.local/share/imbolc/ui.jsonl`) — navigation, layer push/pop, pane switches
//! - **Domain log** (`~/.local/share/imbolc/domain.jsonl`) — all state-mutating actions + effects
//!
//! Each log is tailable via `tail -f` for real-time observation.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use imbolc_types::{DispatchResult, DomainAction, UiAction};

/// Log directory: `~/.local/share/imbolc/`
fn log_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("imbolc")
    } else {
        PathBuf::from(".")
    }
}

/// Append-only JSONL writer for interaction logs.
pub struct InteractionLog {
    writer: BufWriter<File>,
    session_start: Instant,
}

#[derive(Serialize)]
struct SessionHeader {
    event: &'static str,
    epoch_ms: u128,
    pid: u32,
}

#[derive(Serialize)]
struct UiLogEntry<'a> {
    t_ms: u128,
    pane: &'a str,
    action: &'a UiAction,
}

#[derive(Serialize)]
struct DomainLogEntry<'a> {
    t_ms: u128,
    pane: &'a str,
    action: &'a DomainAction,
    effects: Vec<String>,
    undoable: bool,
}

/// Deserialized domain log entry for replay.
#[derive(Deserialize)]
struct ReplayEntry {
    #[allow(dead_code)]
    t_ms: Option<u128>,
    action: Option<DomainAction>,
    // session headers have `event` instead of `action`
    #[allow(dead_code)]
    event: Option<String>,
}

impl InteractionLog {
    fn open(filename: &str) -> Option<Self> {
        let dir = log_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        let path = dir.join(filename);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        let session_start = Instant::now();
        let mut writer = BufWriter::new(file);

        let header = SessionHeader {
            event: "session_start",
            epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            pid: std::process::id(),
        };
        if let Ok(json) = serde_json::to_string(&header) {
            let _ = writeln!(writer, "{}", json);
            let _ = writer.flush();
        }

        Some(Self {
            writer,
            session_start,
        })
    }

    /// Open the UI interaction log (`~/.local/share/imbolc/ui.jsonl`).
    pub fn ui() -> Option<Self> {
        Self::open("ui.jsonl")
    }

    /// Open the domain interaction log (`~/.local/share/imbolc/domain.jsonl`).
    pub fn domain() -> Option<Self> {
        Self::open("domain.jsonl")
    }

    /// Log a UI-layer action.
    pub fn log_ui(&mut self, pane: &str, action: &UiAction) {
        let entry = UiLogEntry {
            t_ms: self.session_start.elapsed().as_millis(),
            pane,
            action,
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = writeln!(self.writer, "{}", json);
            let _ = self.writer.flush();
        }
    }

    /// Log a domain action and its dispatch result.
    ///
    /// Filters out `AudioFeedback` actions (high-frequency audio thread feedback, not user actions).
    pub fn log_domain(&mut self, pane: &str, action: &DomainAction, result: &DispatchResult) {
        if matches!(action, DomainAction::AudioFeedback(_)) {
            return;
        }
        let effects: Vec<String> = result
            .audio_effects
            .iter()
            .map(|e| format!("{:?}", e))
            .collect();
        let undoable = !result.audio_effects.is_empty();
        let entry = DomainLogEntry {
            t_ms: self.session_start.elapsed().as_millis(),
            pane,
            action,
            effects,
            undoable,
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = writeln!(self.writer, "{}", json);
            let _ = self.writer.flush();
        }
    }
}

/// Error type for domain log replay.
#[derive(Debug)]
pub enum ReplayError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl From<std::io::Error> for ReplayError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for ReplayError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Replay a domain log file to derive final state.
///
/// Reads each line, deserializes the `DomainAction`, and applies reducible actions
/// to a fresh `AppState`. Session headers and non-reducible actions are skipped.
///
/// Usage for deriving smoke tests from real sessions:
/// ```bash
/// cp ~/.local/share/imbolc/domain.jsonl imbolc-core/tests/fixtures/my_scenario.jsonl
/// ```
pub fn replay_domain_log(path: &Path) -> Result<crate::state::AppState, ReplayError> {
    use imbolc_types::reduce;

    let mut state = crate::state::AppState::new();
    let file = File::open(path)?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        // Try to parse as a replay entry; skip session headers
        let entry: ReplayEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // Skip unparseable lines
        };
        if let Some(action) = entry.action {
            // Skip AudioFeedback during replay
            if matches!(&action, DomainAction::AudioFeedback(_)) {
                continue;
            }
            if reduce::is_reducible(&action) {
                reduce::reduce_action(&action, &mut state.instruments, &mut state.session);
            }
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn replay_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.jsonl");
        File::create(&path).unwrap();

        let state = replay_domain_log(&path).unwrap();
        assert!(state.instruments.instruments.is_empty());
    }

    #[test]
    fn replay_with_session_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("header_only.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"event":"session_start","epoch_ms":1739290222000,"pid":12345}}"#
        )
        .unwrap();

        let state = replay_domain_log(&path).unwrap();
        assert!(state.instruments.instruments.is_empty());
    }

    #[test]
    fn replay_add_instrument() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("add_instrument.jsonl");
        let mut f = File::create(&path).unwrap();
        // Session header
        writeln!(
            f,
            r#"{{"event":"session_start","epoch_ms":1739290222000,"pid":12345}}"#
        )
        .unwrap();
        // Add a Saw instrument
        writeln!(
            f,
            r#"{{"t_ms":100,"pane":"instrument","action":{{"Instrument":{{"Add":"Saw"}}}},"effects":["RebuildInstruments"],"undoable":true}}"#
        )
        .unwrap();

        let state = replay_domain_log(&path).unwrap();
        assert_eq!(state.instruments.instruments.len(), 1);
    }

    #[test]
    fn replay_skips_audio_feedback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("feedback.jsonl");
        let mut f = File::create(&path).unwrap();
        // AudioFeedback should be filtered (not logged normally, but test the skip)
        writeln!(
            f,
            r#"{{"t_ms":50,"pane":"server","action":{{"AudioFeedback":{{"PlayheadPosition":0}}}},"effects":[],"undoable":false}}"#
        )
        .unwrap();

        let state = replay_domain_log(&path).unwrap();
        assert!(state.instruments.instruments.is_empty());
    }

    #[test]
    fn replay_skips_unparseable_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad_lines.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "not valid json at all").unwrap();
        writeln!(f).unwrap();
        writeln!(
            f,
            r#"{{"t_ms":100,"pane":"instrument","action":{{"Instrument":{{"Add":"Sin"}}}},"effects":[],"undoable":true}}"#
        )
        .unwrap();

        let state = replay_domain_log(&path).unwrap();
        assert_eq!(state.instruments.instruments.len(), 1);
    }
}
