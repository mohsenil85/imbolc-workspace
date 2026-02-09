//! Network protocol types for Imbolc collaboration.
//!
//! Defines the wire protocol for client-server communication.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use imbolc_types::{
    ArrangementAction, AutomationAction, BusAction, ChopperAction, InstrumentAction,
    InstrumentId, InstrumentState, MidiAction, MixerAction, PianoRollAction, SequencerAction,
    ServerAction, SessionAction, SessionState, VstParamAction,
};

/// Unique identifier for a connected client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

impl ClientId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Information about an instrument's owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerInfo {
    pub client_id: ClientId,
    pub client_name: String,
}

/// Privilege level for a connected client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrivilegeLevel {
    #[default]
    Normal,
    Privileged,
}

/// Session token for reconnection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionToken(pub String);

impl SessionToken {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self(format!("{:x}-{:x}", nanos, rand_u64()))
    }
}

impl Default for SessionToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple pseudo-random u64 for token generation.
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // Simple xorshift
    let mut x = seed ^ 0x1234567890abcdef;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Serializable subset of `Action` for network transmission.
///
/// Excludes client-local variants like:
/// - `Nav(NavAction)` — pane switching is per-client
/// - `AudioFeedback` — server-internal
/// - `PushLayer/PopLayer` — client-local UI layers
/// - `ExitPerformanceMode` — client-local
/// - `SaveAndQuit` — handled locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkAction {
    None,
    Quit,
    Instrument(InstrumentAction),
    Mixer(MixerAction),
    PianoRoll(PianoRollAction),
    Arrangement(ArrangementAction),
    Server(ServerAction),
    Session(SessionAction),
    Sequencer(SequencerAction),
    Chopper(ChopperAction),
    Automation(AutomationAction),
    Midi(MidiAction),
    Bus(BusAction),
    VstParam(VstParamAction),
    Undo,
    Redo,
}

/// State that syncs from server to clients.
///
/// NOT synced (client-local):
/// - `AudioFeedbackState` — server audio thread only
/// - `MidiConnectionState` — local hardware
/// - `UndoHistory` — per-client
/// - `Clipboard` — per-client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub session: SessionState,
    pub instruments: InstrumentState,
    /// Map of instrument IDs to their owners.
    pub ownership: HashMap<InstrumentId, OwnerInfo>,
    /// The privileged client (if any) who can control transport/save/load.
    pub privileged_client: Option<(ClientId, String)>,
}

/// A partial state update containing only changed subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePatch {
    pub session: Option<SessionState>,
    pub instruments: Option<InstrumentState>,
    pub ownership: Option<HashMap<InstrumentId, OwnerInfo>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "double_option")]
    pub privileged_client: Option<Option<(ClientId, String)>>,
    pub seq: u64,
}

/// Serde helper for `Option<Option<T>>` over JSON.
///
/// Plain JSON cannot distinguish `None` (field absent = "no change") from
/// `Some(None)` (field present as `null` = "changed to nothing").
/// This module uses field-skipping to preserve the distinction:
///   - outer `None`        → field omitted from JSON
///   - `Some(None)`        → `"field": null`
///   - `Some(Some(value))` → `"field": value`
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        match value {
            Some(inner) => inner.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        // This is only called when the field is present in JSON.
        // Missing fields get `None` via #[serde(default)].
        let inner = Option::<T>::deserialize(deserializer)?;
        Ok(Some(inner))
    }
}

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Initial handshake with client name and ownership request.
    Hello {
        client_name: String,
        /// Instruments the client wants to own (may be partially granted).
        requested_instruments: Vec<InstrumentId>,
        /// Request privileged status (transport/save/load control).
        request_privilege: bool,
        /// Token for reconnecting to a previous session.
        reconnect_token: Option<SessionToken>,
    },
    /// Action to dispatch on the server.
    Action(NetworkAction),
    /// Clean disconnection.
    Goodbye,
    /// Keepalive ping (client-initiated).
    Ping,
    /// Response to server-initiated heartbeat ping.
    Pong,
    /// Request privileged status.
    RequestPrivilege,
    /// Request a full state sync (desync recovery).
    RequestFullSync,
}

/// Messages sent from server to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Initial state on connection with ownership confirmation.
    Welcome {
        /// The client's assigned ID.
        client_id: ClientId,
        /// Instruments the client was granted ownership of.
        granted_instruments: Vec<InstrumentId>,
        /// Initial state snapshot.
        state: NetworkState,
        /// Client's privilege level.
        privilege: PrivilegeLevel,
        /// Session token for reconnection.
        session_token: SessionToken,
    },
    /// State update after action dispatch.
    StateUpdate { state: NetworkState },
    /// Real-time metering data (sent at ~30Hz).
    Metering {
        playhead: u32,
        bpm: f32,
        peaks: (f32, f32),
    },
    /// Server is shutting down.
    Shutdown,
    /// Server-initiated heartbeat ping.
    Ping,
    /// Response to client-initiated Ping.
    Pong,
    /// Error message.
    Error { message: String },
    /// Action was rejected due to authorization failure.
    ActionRejected { reason: String },
    /// Privilege was granted to this client.
    PrivilegeGranted,
    /// Privilege request was denied (already held by another).
    PrivilegeDenied { held_by: String },
    /// This client's privilege was revoked (given to another).
    PrivilegeRevoked,
    /// Reconnection was successful.
    ReconnectSuccessful {
        client_id: ClientId,
        restored_instruments: Vec<InstrumentId>,
        privilege: PrivilegeLevel,
    },
    /// Reconnection failed (token expired or invalid).
    ReconnectFailed { reason: String },
    /// Partial state update (only changed subsystems).
    StatePatchUpdate { patch: StatePatch },
    /// Full state sync (periodic fallback or on request).
    FullStateSync { state: NetworkState, seq: u64 },
}
