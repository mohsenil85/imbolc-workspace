//! Shared event log: retained, cursor-readable record of state changes.
//!
//! Replaces the 4 state-sync AudioCmd variants (ForwardAction, FullStateSync,
//! UpdatePianoRollData, UpdateAutomationLanes) with a canonical log that both
//! delivers entries to the audio thread and retains history for network/debug.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError};

use imbolc_types::Action;

use super::snapshot::{AutomationSnapshot, InstrumentSnapshot, PianoRollSnapshot, SessionSnapshot};
use imbolc_types::InstrumentId;

/// A single entry in the event log.
#[derive(Debug)]
pub struct LogEntry {
    /// Monotonically increasing sequence number.
    pub seq: u64,
    /// The payload.
    pub kind: LogEntryKind,
}

/// Payload variants for log entries.
#[derive(Debug)]
pub enum LogEntryKind {
    /// Projectable action (replaces AudioCmd::ForwardAction).
    Action {
        action: Box<Action>,
        rebuild_routing: bool,
        rebuild_instrument_routing: [Option<InstrumentId>; 4],
        add_instrument_routing: Option<InstrumentId>,
        delete_instrument_routing: Option<InstrumentId>,
        rebuild_bus_processing: bool,
        mixer_dirty: bool,
    },
    /// Full state checkpoint (replaces AudioCmd::FullStateSync).
    Checkpoint {
        instruments: InstrumentSnapshot,
        session: SessionSnapshot,
        piano_roll: PianoRollSnapshot,
        automation_lanes: AutomationSnapshot,
        rebuild_routing: bool,
    },
    /// Song mode piano roll flattening (replaces AudioCmd::UpdatePianoRollData).
    PianoRollUpdate(PianoRollSnapshot),
    /// Song mode automation flattening (replaces AudioCmd::UpdateAutomationLanes).
    AutomationUpdate(AutomationSnapshot),
}

/// Maximum number of retained history entries before trimming.
const DEFAULT_HISTORY_CAPACITY: usize = 10_000;

/// Main-thread writer: appends entries and retains history.
pub struct EventLogWriter {
    tx: Sender<Arc<LogEntry>>,
    history: Vec<Arc<LogEntry>>,
    next_seq: u64,
    history_capacity: usize,
}

impl EventLogWriter {
    /// Create a paired (writer, reader).
    pub fn new() -> (Self, EventLogReader) {
        Self::with_capacity(DEFAULT_HISTORY_CAPACITY)
    }

    /// Create a paired (writer, reader) with a custom history capacity.
    pub fn with_capacity(history_capacity: usize) -> (Self, EventLogReader) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let writer = Self {
            tx,
            history: Vec::new(),
            next_seq: 0,
            history_capacity,
        };
        let reader = EventLogReader { rx };
        (writer, reader)
    }

    /// Append an entry to the log: sends to the audio thread and retains in history.
    pub fn append(&mut self, kind: LogEntryKind) {
        let entry = Arc::new(LogEntry {
            seq: self.next_seq,
            kind,
        });
        self.next_seq += 1;

        // Send to audio thread (fire-and-forget — if receiver is gone, just log)
        if let Err(e) = self.tx.send(Arc::clone(&entry)) {
            log::warn!(target: "audio::event_log", "event log entry dropped (seq {}): {}", entry.seq, e);
        }

        // Retain in history
        self.history.push(entry);

        // Trim history if over capacity
        if self.history.len() > self.history_capacity {
            let excess = self.history.len() - self.history_capacity;
            self.history.drain(..excess);
        }
    }

    /// Read access to retained history.
    pub fn history(&self) -> &[Arc<LogEntry>] {
        &self.history
    }

    /// Current sequence counter (next entry will get this seq).
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// Audio-thread reader: drains entries from the channel.
pub struct EventLogReader {
    rx: Receiver<Arc<LogEntry>>,
}

impl EventLogReader {
    /// Drain available entries within a time budget.
    /// Returns entries in order. Stops when the channel is empty or budget is exhausted.
    pub fn drain(&self, budget: Duration) -> Vec<Arc<LogEntry>> {
        let start = Instant::now();
        let mut entries = Vec::new();

        loop {
            if start.elapsed() >= budget {
                break;
            }
            match self.rx.try_recv() {
                Ok(entry) => entries.push(entry),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dummy_action_kind() -> LogEntryKind {
        LogEntryKind::Action {
            action: Box::new(Action::Undo),
            rebuild_routing: false,
            rebuild_instrument_routing: [None; 4],
            add_instrument_routing: None,
            delete_instrument_routing: None,
            rebuild_bus_processing: false,
            mixer_dirty: false,
        }
    }

    #[test]
    fn append_and_drain() {
        let (mut writer, reader) = EventLogWriter::new();

        writer.append(make_dummy_action_kind());
        writer.append(make_dummy_action_kind());

        let entries = reader.drain(Duration::from_millis(10));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].seq, 0);
        assert_eq!(entries[1].seq, 1);
    }

    #[test]
    fn sequence_monotonicity() {
        let (mut writer, reader) = EventLogWriter::new();

        for _ in 0..100 {
            writer.append(make_dummy_action_kind());
        }

        let entries = reader.drain(Duration::from_millis(50));
        assert_eq!(entries.len(), 100);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.seq, i as u64);
        }
    }

    #[test]
    fn history_retained() {
        let (mut writer, _reader) = EventLogWriter::new();

        writer.append(make_dummy_action_kind());
        writer.append(make_dummy_action_kind());

        assert_eq!(writer.history().len(), 2);
        assert_eq!(writer.history()[0].seq, 0);
        assert_eq!(writer.history()[1].seq, 1);
    }

    #[test]
    fn history_trimming() {
        let (mut writer, _reader) = EventLogWriter::with_capacity(5);

        for _ in 0..10 {
            writer.append(make_dummy_action_kind());
        }

        assert_eq!(writer.history().len(), 5);
        // Oldest entries should have been trimmed; remaining are seq 5..9
        assert_eq!(writer.history()[0].seq, 5);
        assert_eq!(writer.history()[4].seq, 9);
    }

    #[test]
    fn arc_sharing() {
        let (mut writer, reader) = EventLogWriter::new();

        writer.append(make_dummy_action_kind());

        // Same Arc is in both history and the channel
        let history_arc = Arc::clone(&writer.history()[0]);
        let drained = reader.drain(Duration::from_millis(10));
        assert_eq!(drained.len(), 1);
        assert!(Arc::ptr_eq(&history_arc, &drained[0]));
    }

    #[test]
    fn drain_empty_returns_empty() {
        let (_writer, reader) = EventLogWriter::new();
        let entries = reader.drain(Duration::from_millis(10));
        assert!(entries.is_empty());
    }

    #[test]
    fn next_seq_advances() {
        let (mut writer, _reader) = EventLogWriter::new();

        assert_eq!(writer.next_seq(), 0);
        writer.append(make_dummy_action_kind());
        assert_eq!(writer.next_seq(), 1);
        writer.append(make_dummy_action_kind());
        assert_eq!(writer.next_seq(), 2);
    }
}
