//! Network client for Imbolc collaboration.
//!
//! Connects to a server and dispatches actions remotely.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufReader, BufWriter};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use log::{error, info, warn};

use imbolc_types::InstrumentId;

use crate::framing::{read_message, write_message};
use crate::protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo, PrivilegeLevel,
    ServerMessage, SessionToken, StatePatch,
};

/// Metering update from server.
#[derive(Debug, Clone)]
pub struct MeteringUpdate {
    pub playhead: u32,
    pub bpm: f32,
    pub peaks: (f32, f32),
}

/// Messages received from the server via background thread.
enum ServerUpdate {
    State(NetworkState),
    Metering(MeteringUpdate),
    Shutdown,
    Error(String),
    /// An action was rejected by the server (ownership/permission issue).
    ActionRejected(String),
    /// Privilege was granted to this client.
    PrivilegeGranted,
    /// Privilege was denied (held by another client).
    PrivilegeDenied(String),
    /// This client's privilege was revoked.
    PrivilegeRevoked,
    /// Server sent a heartbeat ping — respond with Pong.
    PingReceived,
    /// Connection lost (EOF/error, not graceful shutdown).
    ConnectionLost,
    /// Partial state update (only changed subsystems).
    Patch(StatePatch),
    /// Full state sync.
    FullSync(NetworkState, u64),
}

/// Ownership status for an instrument from this client's perspective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipStatus {
    OwnedByMe,
    OwnedByOther(String),
    Unowned,
}

/// A client that dispatches actions to a remote server.
pub struct RemoteDispatcher {
    state: NetworkState,
    writer: BufWriter<TcpStream>,
    update_rx: Receiver<ServerUpdate>,
    metering: MeteringUpdate,
    server_shutdown: bool,
    /// Our assigned client ID from the server.
    client_id: ClientId,
    /// Instruments we own (can mutate).
    owned_instruments: HashSet<InstrumentId>,
    /// Last rejection reason, if any.
    last_rejection: Option<String>,
    /// Our privilege level.
    privilege: PrivilegeLevel,
    /// Session token for reconnection.
    session_token: SessionToken,
    /// Whether the connection was lost (vs graceful shutdown).
    connection_lost: bool,
    /// Last received sequence number for ordering.
    last_seq: u64,
    /// Server address for reconnection.
    server_addr: String,
    /// Client name for reconnection.
    client_name: String,
}

impl RemoteDispatcher {
    /// Connect to a server and complete the handshake.
    ///
    /// `requested_instruments`: List of instrument IDs to request ownership of.
    /// The server will grant ownership of available (unowned) instruments.
    pub fn connect(
        addr: &str,
        client_name: &str,
        requested_instruments: Vec<InstrumentId>,
    ) -> io::Result<Self> {
        Self::connect_with_options(addr, client_name, requested_instruments, false, None)
    }

    /// Connect with additional options.
    pub fn connect_with_options(
        addr: &str,
        client_name: &str,
        requested_instruments: Vec<InstrumentId>,
        request_privilege: bool,
        reconnect_token: Option<SessionToken>,
    ) -> io::Result<Self> {
        info!("Connecting to server at {}", addr);

        let stream = TcpStream::connect(addr)?;
        let read_stream = stream.try_clone()?;

        let mut writer = BufWriter::new(stream);
        let mut reader = BufReader::new(read_stream.try_clone()?);

        // Send Hello with ownership request
        write_message(&mut writer, &ClientMessage::Hello {
            client_name: client_name.to_string(),
            requested_instruments,
            request_privilege,
            reconnect_token,
        })?;

        // Receive response
        let welcome: ServerMessage = read_message(&mut reader)?;
        let (client_id, granted_instruments, state, privilege, session_token) = match welcome {
            ServerMessage::Welcome { client_id, granted_instruments, state, privilege, session_token } => {
                (client_id, granted_instruments, state, privilege, session_token)
            }
            ServerMessage::ReconnectSuccessful { client_id, restored_instruments, privilege } => {
                // For reconnect, we need to get a fresh state update
                info!("Reconnected as {:?}, restored {} instruments", client_id, restored_instruments.len());
                // Wait for state update
                let state_msg: ServerMessage = read_message(&mut reader)?;
                let state = match state_msg {
                    ServerMessage::StateUpdate { state } => state,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "expected StateUpdate after reconnect",
                        ));
                    }
                };
                (client_id, restored_instruments, state, privilege, SessionToken::new())
            }
            ServerMessage::ReconnectFailed { reason } => {
                return Err(io::Error::new(io::ErrorKind::ConnectionRefused, reason));
            }
            ServerMessage::Error { message } => {
                return Err(io::Error::new(io::ErrorKind::ConnectionRefused, message));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected Welcome message",
                ));
            }
        };

        info!(
            "Connected as client {:?}, granted {} instruments: {:?}, privilege={:?}",
            client_id,
            granted_instruments.len(),
            granted_instruments,
            privilege
        );

        // Start background reader thread
        let (update_tx, update_rx) = mpsc::channel();
        thread::spawn(move || {
            server_reader_thread(read_stream, update_tx);
        });

        Ok(Self {
            state,
            writer,
            update_rx,
            metering: MeteringUpdate {
                playhead: 0,
                bpm: 120.0,
                peaks: (0.0, 0.0),
            },
            server_shutdown: false,
            client_id,
            owned_instruments: granted_instruments.into_iter().collect(),
            last_rejection: None,
            privilege,
            session_token,
            connection_lost: false,
            last_seq: 0,
            server_addr: addr.to_string(),
            client_name: client_name.to_string(),
        })
    }

    /// Reconnect to a server using a saved session token.
    pub fn reconnect(
        addr: &str,
        client_name: &str,
        session_token: SessionToken,
    ) -> io::Result<Self> {
        Self::connect_with_options(addr, client_name, vec![], false, Some(session_token))
    }

    /// Get the current cached state.
    pub fn state(&self) -> &NetworkState {
        &self.state
    }

    /// Get the current metering data.
    pub fn metering(&self) -> &MeteringUpdate {
        &self.metering
    }

    /// Check if the server has shut down.
    pub fn server_shutdown(&self) -> bool {
        self.server_shutdown
    }

    /// Get the assigned client ID.
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// Get the instruments this client owns.
    pub fn owned_instruments(&self) -> &HashSet<InstrumentId> {
        &self.owned_instruments
    }

    /// Check if this client owns a specific instrument.
    pub fn owns(&self, instrument_id: InstrumentId) -> bool {
        self.owned_instruments.contains(&instrument_id)
    }

    /// Get and clear the last rejection reason, if any.
    pub fn take_rejection(&mut self) -> Option<String> {
        self.last_rejection.take()
    }

    /// Check if this client is privileged.
    pub fn is_privileged(&self) -> bool {
        self.privilege == PrivilegeLevel::Privileged
    }

    /// Get the privilege level.
    pub fn privilege(&self) -> PrivilegeLevel {
        self.privilege
    }

    /// Get the session token for reconnection.
    pub fn session_token(&self) -> &SessionToken {
        &self.session_token
    }

    /// Check if the connection was lost (vs graceful shutdown).
    pub fn connection_lost(&self) -> bool {
        self.connection_lost
    }

    /// Get the server address for reconnection.
    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }

    /// Get the client name.
    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    /// Get ownership status for an instrument.
    pub fn ownership_status(&self, instrument_id: InstrumentId) -> OwnershipStatus {
        if self.owned_instruments.contains(&instrument_id) {
            return OwnershipStatus::OwnedByMe;
        }

        if let Some(owner_info) = self.state.ownership.get(&instrument_id) {
            return OwnershipStatus::OwnedByOther(owner_info.client_name.clone());
        }

        OwnershipStatus::Unowned
    }

    /// Get the ownership map from the state.
    pub fn ownership_map(&self) -> &HashMap<InstrumentId, OwnerInfo> {
        &self.state.ownership
    }

    /// Get the privileged client info from state.
    pub fn privileged_client(&self) -> Option<(ClientId, &str)> {
        self.state.privileged_client.as_ref().map(|(id, name)| (*id, name.as_str()))
    }

    /// Send an action to the server.
    pub fn dispatch(&mut self, action: NetworkAction) -> io::Result<()> {
        write_message(&mut self.writer, &ClientMessage::Action(action))
    }

    /// Send a ping to the server.
    pub fn ping(&mut self) -> io::Result<()> {
        write_message(&mut self.writer, &ClientMessage::Ping)
    }

    /// Request privileged status from the server.
    pub fn request_privilege(&mut self) -> io::Result<()> {
        write_message(&mut self.writer, &ClientMessage::RequestPrivilege)
    }

    /// Request a full state sync from the server (desync recovery).
    pub fn request_full_sync(&mut self) -> io::Result<()> {
        write_message(&mut self.writer, &ClientMessage::RequestFullSync)
    }

    /// Poll for updates from the server and apply them to local state.
    /// Returns true if state was updated.
    pub fn poll_updates(&mut self) -> bool {
        let mut state_updated = false;

        loop {
            match self.update_rx.try_recv() {
                Ok(update) => match update {
                    ServerUpdate::State(mut new_state) => {
                        // Update owned instruments from the state ownership map
                        self.owned_instruments.clear();
                        for (&inst_id, owner_info) in &new_state.ownership {
                            if owner_info.client_id == self.client_id {
                                self.owned_instruments.insert(inst_id);
                            }
                        }
                        new_state.instruments.rebuild_index();
                        self.state = new_state;
                        state_updated = true;
                    }
                    ServerUpdate::Metering(m) => {
                        self.metering = m;
                    }
                    ServerUpdate::Shutdown => {
                        info!("Server shutdown received");
                        self.server_shutdown = true;
                    }
                    ServerUpdate::Error(msg) => {
                        warn!("Server error: {}", msg);
                    }
                    ServerUpdate::ActionRejected(reason) => {
                        warn!("Action rejected: {}", reason);
                        self.last_rejection = Some(reason);
                    }
                    ServerUpdate::PrivilegeGranted => {
                        info!("Privilege granted");
                        self.privilege = PrivilegeLevel::Privileged;
                    }
                    ServerUpdate::PrivilegeDenied(held_by) => {
                        warn!("Privilege denied, held by: {}", held_by);
                    }
                    ServerUpdate::PrivilegeRevoked => {
                        info!("Privilege revoked");
                        self.privilege = PrivilegeLevel::Normal;
                    }
                    ServerUpdate::PingReceived => {
                        // Respond to server heartbeat
                        if let Err(e) = write_message(&mut self.writer, &ClientMessage::Pong) {
                            warn!("Failed to send pong: {}", e);
                        }
                    }
                    ServerUpdate::ConnectionLost => {
                        warn!("Connection to server lost");
                        self.connection_lost = true;
                        break;
                    }
                    ServerUpdate::Patch(patch) => {
                        if patch.seq > self.last_seq {
                            self.last_seq = patch.seq;
                            let full_session = patch.session.is_some();
                            if let Some(session) = patch.session {
                                self.state.session = session;
                            }
                            // Granular subsystem patches (only apply if full session wasn't sent)
                            if !full_session {
                                if let Some(pr) = patch.piano_roll {
                                    self.state.session.piano_roll = pr;
                                } else if let Some(track_patches) = patch.piano_roll_track_patches {
                                    for (id, track) in track_patches {
                                        self.state.session.piano_roll.tracks.insert(id, track);
                                    }
                                }
                                if let Some(arr) = patch.arrangement {
                                    self.state.session.arrangement = arr;
                                }
                                if let Some(auto) = patch.automation {
                                    self.state.session.automation = auto;
                                }
                                if let Some(mix) = patch.mixer {
                                    self.state.session.mixer = mix;
                                }
                            }
                            if let Some(instruments) = patch.instruments {
                                self.state.instruments = instruments;
                                self.state.instruments.rebuild_index();
                            } else if let Some(patches) = patch.instrument_patches {
                                for (id, new_instrument) in patches {
                                    if let Some(existing) = self.state.instruments.instrument_mut(id) {
                                        *existing = new_instrument;
                                    }
                                }
                            }
                            if let Some(ownership) = patch.ownership {
                                self.state.ownership = ownership;
                            }
                            if let Some(privileged_client) = patch.privileged_client {
                                self.state.privileged_client = privileged_client;
                            }
                            // Update owned instruments from ownership
                            self.owned_instruments.clear();
                            for (&inst_id, owner_info) in &self.state.ownership {
                                if owner_info.client_id == self.client_id {
                                    self.owned_instruments.insert(inst_id);
                                }
                            }
                            state_updated = true;
                        }
                    }
                    ServerUpdate::FullSync(mut new_state, seq) => {
                        self.last_seq = seq;
                        self.owned_instruments.clear();
                        for (&inst_id, owner_info) in &new_state.ownership {
                            if owner_info.client_id == self.client_id {
                                self.owned_instruments.insert(inst_id);
                            }
                        }
                        new_state.instruments.rebuild_index();
                        self.state = new_state;
                        state_updated = true;
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    warn!("Server connection lost (reader thread exited)");
                    self.connection_lost = true;
                    break;
                }
            }
        }

        state_updated
    }

    /// Send goodbye and disconnect.
    pub fn disconnect(mut self) -> io::Result<()> {
        write_message(&mut self.writer, &ClientMessage::Goodbye)
    }
}

/// Background thread that reads messages from the server.
fn server_reader_thread(
    stream: TcpStream,
    update_tx: mpsc::Sender<ServerUpdate>,
) {
    let mut reader = BufReader::new(stream);

    loop {
        match read_message::<_, ServerMessage>(&mut reader) {
            Ok(msg) => {
                let update = match msg {
                    ServerMessage::Welcome { state, .. } => {
                        // Shouldn't happen after handshake, but handle it
                        ServerUpdate::State(state)
                    }
                    ServerMessage::StateUpdate { state } => {
                        ServerUpdate::State(state)
                    }
                    ServerMessage::Metering { playhead, bpm, peaks } => {
                        ServerUpdate::Metering(MeteringUpdate { playhead, bpm, peaks })
                    }
                    ServerMessage::Shutdown => {
                        info!("Server sent graceful shutdown");
                        let _ = update_tx.send(ServerUpdate::Shutdown);
                        break;
                    }
                    ServerMessage::Ping => {
                        // Server heartbeat — notify main thread to send Pong
                        let _ = update_tx.send(ServerUpdate::PingReceived);
                        continue;
                    }
                    ServerMessage::Pong => {
                        // Response to our Ping — ignore
                        continue;
                    }
                    ServerMessage::Error { message } => {
                        ServerUpdate::Error(message)
                    }
                    ServerMessage::ActionRejected { reason } => {
                        ServerUpdate::ActionRejected(reason)
                    }
                    ServerMessage::PrivilegeGranted => {
                        ServerUpdate::PrivilegeGranted
                    }
                    ServerMessage::PrivilegeDenied { held_by } => {
                        ServerUpdate::PrivilegeDenied(held_by)
                    }
                    ServerMessage::PrivilegeRevoked => {
                        ServerUpdate::PrivilegeRevoked
                    }
                    ServerMessage::ReconnectSuccessful { .. } => {
                        // Should only happen during handshake, not here
                        continue;
                    }
                    ServerMessage::ReconnectFailed { reason } => {
                        ServerUpdate::Error(format!("Reconnect failed: {}", reason))
                    }
                    ServerMessage::StatePatchUpdate { patch } => {
                        ServerUpdate::Patch(patch)
                    }
                    ServerMessage::FullStateSync { state, seq } => {
                        ServerUpdate::FullSync(state, seq)
                    }
                };

                if update_tx.send(update).is_err() {
                    // Receiver dropped, client is shutting down
                    break;
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::UnexpectedEof {
                    error!("Server read error: {}", e);
                }
                // Connection lost (not a graceful shutdown from server)
                let _ = update_tx.send(ServerUpdate::ConnectionLost);
                break;
            }
        }
    }

    info!("Server reader thread exiting");
}
