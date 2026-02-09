//! Network server for Imbolc collaboration.
//!
//! Accepts client connections, receives actions, and broadcasts state updates.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufReader, BufWriter};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use log::{error, info, warn};

use imbolc_types::{InstrumentAction, InstrumentId, VstParamAction};

use crate::framing::{read_message, write_message};
use crate::protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo,
    PrivilegeLevel, ServerMessage, SessionToken, StatePatch,
};

/// A connected client with its write half.
struct ClientConnection {
    name: String,
    writer: BufWriter<TcpStream>,
    /// Instruments this client owns (can mutate).
    owned_instruments: HashSet<InstrumentId>,
    /// Session token for reconnection.
    session_token: SessionToken,
    /// Last time we received any message from this client.
    last_seen: Instant,
}

/// A suspended session awaiting reconnection.
struct SuspendedSession {
    client_name: String,
    owned_instruments: HashSet<InstrumentId>,
    was_privileged: bool,
    disconnected_at: Instant,
}

/// How long to keep a suspended session before expiring it.
const RECONNECT_WINDOW_SECS: u64 = 60;

/// Minimum interval between patch broadcasts (~30 Hz).
const PATCH_BROADCAST_INTERVAL_MS: u128 = 33;

/// Tracks which subsystems have changed since last broadcast.
#[derive(Debug, Default)]
pub struct DirtyFlags {
    pub session: bool,
    /// Per-instrument targeted edits (e.g. filter cutoff on instrument 5).
    pub dirty_instruments: HashSet<InstrumentId>,
    /// Structural instrument changes: add, delete, select, undo/redo, mixer, etc.
    pub instruments_structural: bool,
    pub ownership: bool,
    pub privileged_client: bool,
}

impl DirtyFlags {
    /// Mark dirty flags based on the action variant.
    pub fn mark_from_action(&mut self, action: &NetworkAction) {
        match action {
            NetworkAction::Instrument(a) => {
                match a.target_instrument_id() {
                    Some(id) => {
                        // Delete changes the instrument Vec structurally
                        if matches!(a, InstrumentAction::Delete(_)) {
                            self.instruments_structural = true;
                        } else {
                            self.dirty_instruments.insert(id);
                        }
                    }
                    None => {
                        // Add, Select*, PlayNote, PlayDrumPad — structural
                        self.instruments_structural = true;
                    }
                }
            }
            NetworkAction::VstParam(a) => {
                let id = match a {
                    VstParamAction::SetParam(id, ..)
                    | VstParamAction::AdjustParam(id, ..)
                    | VstParamAction::ResetParam(id, ..)
                    | VstParamAction::DiscoverParams(id, ..)
                    | VstParamAction::SaveState(id, ..) => *id,
                };
                self.dirty_instruments.insert(id);
            }
            NetworkAction::Sequencer(_) => {
                self.instruments_structural = true;
            }
            NetworkAction::PianoRoll(_)
            | NetworkAction::Arrangement(_)
            | NetworkAction::Automation(_)
            | NetworkAction::Session(_)
            | NetworkAction::Server(_)
            | NetworkAction::Bus(_)
            | NetworkAction::Chopper(_) => {
                self.session = true;
            }
            NetworkAction::Mixer(_) | NetworkAction::Midi(_) => {
                self.session = true;
                self.instruments_structural = true;
            }
            NetworkAction::Undo | NetworkAction::Redo => {
                self.session = true;
                self.instruments_structural = true;
            }
            NetworkAction::None | NetworkAction::Quit => {}
        }
    }

    fn any(&self) -> bool {
        self.session
            || !self.dirty_instruments.is_empty()
            || self.instruments_structural
            || self.ownership
            || self.privileged_client
    }

    fn clear(&mut self) {
        self.session = false;
        self.dirty_instruments.clear();
        self.instruments_structural = false;
        self.ownership = false;
        self.privileged_client = false;
    }
}

impl ClientConnection {
    fn send(&mut self, msg: &ServerMessage) -> io::Result<()> {
        write_message(&mut self.writer, msg)
    }
}

/// A pending connection awaiting Hello handshake.
struct PendingConnection {
    writer: BufWriter<TcpStream>,
}

/// Network server that accepts client connections and coordinates actions.
pub struct NetServer {
    listener: TcpListener,
    /// Fully connected clients (completed Hello handshake).
    clients: HashMap<ClientId, ClientConnection>,
    /// Clients awaiting Hello message.
    pending: HashMap<ClientId, PendingConnection>,
    action_rx: Receiver<(ClientId, ClientMessage)>,
    action_tx: Sender<(ClientId, ClientMessage)>,
    next_client_id: u64,
    /// Tracks which instruments are owned by which client.
    /// An instrument can only be owned by one client at a time.
    ownership: HashMap<InstrumentId, ClientId>,
    /// The client with privileged status (transport/save/load control).
    privileged_client: Option<ClientId>,
    /// Suspended sessions awaiting reconnection.
    suspended_sessions: HashMap<SessionToken, SuspendedSession>,
    /// Last time we sent heartbeat pings.
    last_heartbeat: Instant,
    /// Dirty flags for state diffing.
    dirty: DirtyFlags,
    /// Sequence number for state patches.
    seq: u64,
    /// Force a full sync on next broadcast.
    force_full_sync: bool,
    /// Last time a full sync was sent.
    last_full_sync: Instant,
    /// Last time a patch broadcast was sent (for rate limiting).
    last_patch_broadcast: Instant,
}

impl NetServer {
    /// Bind the server to an address.
    pub fn bind(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let (action_tx, action_rx) = mpsc::channel();

        info!("NetServer listening on {}", addr);

        Ok(Self {
            listener,
            clients: HashMap::new(),
            pending: HashMap::new(),
            action_rx,
            action_tx,
            next_client_id: 0,
            ownership: HashMap::new(),
            privileged_client: None,
            suspended_sessions: HashMap::new(),
            last_heartbeat: Instant::now(),
            dirty: DirtyFlags::default(),
            seq: 0,
            force_full_sync: false,
            last_full_sync: Instant::now(),
            last_patch_broadcast: Instant::now() - std::time::Duration::from_secs(1),
        })
    }

    /// Accept any pending TCP connections (they become fully connected after Hello handshake).
    pub fn accept_connections(&mut self, _state: &NetworkState) {
        loop {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("Client connecting from {}", addr);

                    // Accepted streams may inherit nonblocking from the listener (macOS/BSD).
                    // The reader thread needs blocking I/O.
                    if let Err(e) = stream.set_nonblocking(false) {
                        error!("Failed to set stream to blocking: {}", e);
                        continue;
                    }

                    let client_id = ClientId::new(self.next_client_id);
                    self.next_client_id += 1;

                    // Clone stream for reader thread
                    let read_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to clone stream: {}", e);
                            continue;
                        }
                    };

                    // Set up writer (don't send Welcome yet — wait for Hello)
                    let writer = BufWriter::new(stream);

                    // Start reader thread
                    let action_tx = self.action_tx.clone();
                    thread::spawn(move || {
                        client_reader_thread(client_id, read_stream, action_tx);
                    });

                    // Store as pending (will become full client on Hello)
                    self.pending.insert(client_id, PendingConnection { writer });

                    info!("Client {:?} TCP connected from {}, awaiting Hello", client_id, addr);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No pending connections
                    break;
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                    break;
                }
            }
        }
    }

    /// Poll for client messages, returning any NetworkActions received.
    /// Must call with current state for Hello handshake.
    pub fn poll_actions(&mut self, state: &NetworkState) -> Vec<(ClientId, NetworkAction)> {
        let mut actions = Vec::new();

        while let Ok((client_id, msg)) = self.action_rx.try_recv() {
            // Update last_seen for any message from a connected client
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.last_seen = Instant::now();
            }

            match msg {
                ClientMessage::Hello {
                    client_name,
                    requested_instruments,
                    request_privilege,
                    reconnect_token,
                } => {
                    // Check for reconnection
                    if let Some(token) = reconnect_token {
                        if let Some(suspended) = self.suspended_sessions.remove(&token) {
                            // Valid reconnection
                            if let Some(mut pending) = self.pending.remove(&client_id) {
                                // Restore ownership
                                for &id in &suspended.owned_instruments {
                                    self.ownership.insert(id, client_id);
                                }

                                // Restore privilege if they had it
                                let privilege = if suspended.was_privileged {
                                    self.privileged_client = Some(client_id);
                                    PrivilegeLevel::Privileged
                                } else {
                                    PrivilegeLevel::Normal
                                };

                                let session_token = SessionToken::new();
                                let msg = ServerMessage::ReconnectSuccessful {
                                    client_id,
                                    restored_instruments: suspended.owned_instruments.iter().copied().collect(),
                                    privilege,
                                };
                                if let Err(e) = write_message(&mut pending.writer, &msg) {
                                    error!("Failed to send reconnect success to {:?}: {}", client_id, e);
                                    continue;
                                }

                                self.clients.insert(client_id, ClientConnection {
                                    name: suspended.client_name,
                                    writer: pending.writer,
                                    owned_instruments: suspended.owned_instruments,
                                    session_token,
                                    last_seen: Instant::now(),
                                });

                                info!("Client {:?} reconnected successfully", client_id);
                                continue;
                            }
                        } else {
                            // Token invalid or expired
                            if let Some(mut pending) = self.pending.remove(&client_id) {
                                let msg = ServerMessage::ReconnectFailed {
                                    reason: "Session expired or invalid token".into(),
                                };
                                let _ = write_message(&mut pending.writer, &msg);
                            }
                            continue;
                        }
                    }

                    // Normal handshake: move from pending to clients
                    if let Some(mut pending) = self.pending.remove(&client_id) {
                        // Assign ownership for requested instruments that aren't already owned
                        let granted: Vec<InstrumentId> = requested_instruments
                            .into_iter()
                            .filter(|id| !self.ownership.contains_key(id))
                            .collect();

                        // Record ownership
                        for &id in &granted {
                            self.ownership.insert(id, client_id);
                        }

                        // Handle privilege request
                        let privilege = if request_privilege && self.privileged_client.is_none() {
                            self.privileged_client = Some(client_id);
                            PrivilegeLevel::Privileged
                        } else {
                            PrivilegeLevel::Normal
                        };

                        let session_token = SessionToken::new();

                        // Send Welcome with granted instruments
                        let welcome = ServerMessage::Welcome {
                            client_id,
                            granted_instruments: granted.clone(),
                            state: state.clone(),
                            privilege,
                            session_token: session_token.clone(),
                        };
                        if let Err(e) = write_message(&mut pending.writer, &welcome) {
                            error!("Failed to send welcome to {:?}: {}", client_id, e);
                            // Clean up ownership we just assigned
                            for id in &granted {
                                self.ownership.remove(id);
                            }
                            if self.privileged_client == Some(client_id) {
                                self.privileged_client = None;
                            }
                            continue;
                        }

                        // Promote to full client
                        self.clients.insert(client_id, ClientConnection {
                            name: client_name.clone(),
                            writer: pending.writer,
                            owned_instruments: granted.iter().copied().collect(),
                            session_token,
                            last_seen: Instant::now(),
                        });

                        info!(
                            "Client {:?} '{}' completed handshake, granted {} instruments, privilege={:?}",
                            client_id,
                            client_name,
                            granted.len(),
                            privilege
                        );
                    } else if let Some(client) = self.clients.get_mut(&client_id) {
                        // Already connected — just update name (shouldn't happen normally)
                        warn!("Client {:?} sent Hello after already connected", client_id);
                        client.name = client_name;
                    }
                }
                ClientMessage::Action(action) => {
                    // Validate ownership before accepting action
                    if let Err(reason) = self.validate_action(client_id, &action) {
                        if let Some(client) = self.clients.get_mut(&client_id) {
                            if let Err(e) = client.send(&ServerMessage::ActionRejected { reason: reason.clone() }) {
                                warn!("Failed to send rejection to {:?}: {}", client_id, e);
                            }
                        }
                        warn!("Action from {:?} rejected: {}", client_id, reason);
                        continue;
                    }
                    actions.push((client_id, action));
                }
                ClientMessage::Goodbye => {
                    info!("Client {:?} disconnected gracefully", client_id);
                    self.suspend_client(client_id);
                }
                ClientMessage::Ping => {
                    if let Some(client) = self.clients.get_mut(&client_id) {
                        if let Err(e) = client.send(&ServerMessage::Pong) {
                            warn!("Failed to send pong to {:?}: {}", client_id, e);
                        }
                    }
                }
                ClientMessage::Pong => {
                    // Client responded to server heartbeat — last_seen updated below
                }
                ClientMessage::RequestPrivilege => {
                    self.handle_privilege_request(client_id);
                }
                ClientMessage::RequestFullSync => {
                    // Will be handled by the caller (server loop sends full state)
                    info!("Client {:?} requested full sync", client_id);
                    self.force_full_sync = true;
                }
            }
        }

        actions
    }

    /// Handle a privilege request from a client.
    fn handle_privilege_request(&mut self, client_id: ClientId) {
        if let Some(current) = self.privileged_client {
            if current == client_id {
                // Already privileged
                if let Some(client) = self.clients.get_mut(&client_id) {
                    let _ = client.send(&ServerMessage::PrivilegeGranted);
                }
                return;
            }

            // Revoke from current holder
            let current_name = self.clients.get(&current)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "unknown".into());

            if let Some(old_client) = self.clients.get_mut(&current) {
                let _ = old_client.send(&ServerMessage::PrivilegeRevoked);
            }

            info!(
                "Privilege transferred from {:?} '{}' to {:?}",
                current, current_name, client_id
            );
        }

        // Grant to new client
        self.privileged_client = Some(client_id);
        if let Some(client) = self.clients.get_mut(&client_id) {
            let _ = client.send(&ServerMessage::PrivilegeGranted);
        }

        info!("Client {:?} granted privilege", client_id);
    }

    /// Suspend a client session for potential reconnection.
    fn suspend_client(&mut self, client_id: ClientId) {
        // Remove from pending if handshake incomplete
        self.pending.remove(&client_id);

        // Move to suspended if they have a session
        if let Some(client) = self.clients.remove(&client_id) {
            let was_privileged = self.privileged_client == Some(client_id);

            // Clear privilege (will be restored on reconnect)
            if was_privileged {
                self.privileged_client = None;
            }

            // Create suspended session
            self.suspended_sessions.insert(client.session_token.clone(), SuspendedSession {
                client_name: client.name.clone(),
                owned_instruments: client.owned_instruments.clone(),
                was_privileged,
                disconnected_at: Instant::now(),
            });

            // Keep ownership reserved in the ownership map
            // (don't release it, so others can't claim it during reconnect window)

            info!(
                "Client {:?} '{}' suspended, {} instruments reserved for reconnection",
                client_id,
                client.name,
                client.owned_instruments.len()
            );
        }
    }

    /// Heartbeat tick: ping clients, detect dead connections, clean up expired sessions.
    /// Call this from the main loop (e.g. at ~30Hz or on each iteration).
    pub fn tick_heartbeat(&mut self) {
        let now = Instant::now();

        // Send pings every 5 seconds
        if now.duration_since(self.last_heartbeat).as_secs() >= 5 {
            self.last_heartbeat = now;

            // Collect dead clients (no response for 15s = 3 missed beats)
            let dead: Vec<ClientId> = self.clients
                .iter()
                .filter(|(_, c)| now.duration_since(c.last_seen).as_secs() > 15)
                .map(|(&id, _)| id)
                .collect();

            for id in dead {
                warn!("Client {:?} timed out (no heartbeat response), suspending", id);
                self.suspend_client(id);
            }

            // Ping remaining clients
            let ping = ServerMessage::Ping;
            let mut disconnected = Vec::new();
            for (&id, client) in &mut self.clients {
                if let Err(e) = client.send(&ping) {
                    warn!("Failed to ping client {:?}: {}", id, e);
                    disconnected.push(id);
                }
            }
            for id in disconnected {
                self.suspend_client(id);
            }
        }

        // Clean up expired suspended sessions
        self.cleanup_expired_sessions();
    }

    /// Clean up expired suspended sessions.
    fn cleanup_expired_sessions(&mut self) {
        let now = Instant::now();
        let expired: Vec<SessionToken> = self.suspended_sessions
            .iter()
            .filter(|(_, s)| now.duration_since(s.disconnected_at).as_secs() > RECONNECT_WINDOW_SECS)
            .map(|(t, _)| t.clone())
            .collect();

        for token in expired {
            if let Some(session) = self.suspended_sessions.remove(&token) {
                // Now release the ownership
                for id in &session.owned_instruments {
                    self.ownership.remove(id);
                }
                info!(
                    "Suspended session for '{}' expired, releasing {} instruments",
                    session.client_name,
                    session.owned_instruments.len()
                );
            }
        }
    }

    /// Remove a client and release their ownership.
    #[allow(dead_code)]
    fn remove_client(&mut self, client_id: ClientId) {
        // Remove from pending if handshake incomplete
        self.pending.remove(&client_id);

        // Remove from clients and release ownership
        if let Some(client) = self.clients.remove(&client_id) {
            for id in client.owned_instruments {
                self.ownership.remove(&id);
            }
            info!("Client {:?} '{}' removed, ownership released", client_id, client.name);
        }
    }

    /// Validate that a client is authorized to perform an action.
    fn validate_action(&self, client_id: ClientId, action: &NetworkAction) -> Result<(), String> {
        match action {
            // Per-instrument actions — require ownership
            NetworkAction::Instrument(a) => {
                if let Some(id) = a.target_instrument_id() {
                    if !self.is_owner(client_id, id) {
                        return Err(format!("You don't own instrument {}", id));
                    }
                }
            }
            NetworkAction::PianoRoll(a) => {
                if let Some(id) = a.target_instrument_id() {
                    if !self.is_owner(client_id, id) {
                        return Err(format!("You don't own track for instrument {}", id));
                    }
                }
            }
            NetworkAction::Sequencer(a) => {
                // Sequencer actions target the currently selected instrument
                // We'd need state to check this — for now, allow (revisit later)
                let _ = a;
            }

            // Privileged actions — require privilege
            NetworkAction::Server(_) => {
                if !self.is_privileged(client_id) {
                    return Err("Transport controls require privilege (use 'Request Privilege')".into());
                }
            }
            NetworkAction::Session(_) => {
                if !self.is_privileged(client_id) {
                    return Err("Session controls require privilege (use 'Request Privilege')".into());
                }
            }
            NetworkAction::Bus(_) => {
                if !self.is_privileged(client_id) {
                    return Err("Bus controls require privilege (use 'Request Privilege')".into());
                }
            }

            // Allow for now (revisit later for finer-grained control)
            NetworkAction::Mixer(_) => {}
            NetworkAction::Automation(_) => {}
            NetworkAction::Arrangement(_) => {}
            NetworkAction::Chopper(_) => {}
            NetworkAction::Midi(_) => {}
            NetworkAction::VstParam(_) => {}
            NetworkAction::Undo | NetworkAction::Redo => {}
            NetworkAction::None | NetworkAction::Quit => {}
        }
        Ok(())
    }

    /// Check if a client has privileged status.
    fn is_privileged(&self, client_id: ClientId) -> bool {
        self.privileged_client == Some(client_id)
    }

    /// Check if a client owns an instrument.
    fn is_owner(&self, client_id: ClientId, instrument_id: InstrumentId) -> bool {
        self.ownership.get(&instrument_id) == Some(&client_id)
    }

    /// Get the owner of an instrument, if any.
    #[allow(dead_code)]
    fn owned_by(&self, instrument_id: InstrumentId) -> Option<ClientId> {
        self.ownership.get(&instrument_id).copied()
    }

    /// Mark dirty flags for a dispatched action.
    pub fn mark_dirty(&mut self, action: &NetworkAction) {
        self.dirty.mark_from_action(action);
    }

    /// Mark ownership as dirty (call on connect/disconnect).
    pub fn mark_ownership_dirty(&mut self) {
        self.dirty.ownership = true;
        self.dirty.privileged_client = true;
    }

    /// Broadcast only changed subsystems as a patch.
    ///
    /// Uses per-instrument delta patches when only a few instruments changed,
    /// falling back to full `InstrumentState` for structural changes or when
    /// more than half the instruments are dirty. Rate-limited to ~30 Hz.
    pub fn broadcast_state_patch(&mut self, state: &NetworkState) {
        if !self.dirty.any() {
            return;
        }

        // Rate limit: skip if last broadcast was too recent (dirty flags persist)
        let now = Instant::now();
        if now.duration_since(self.last_patch_broadcast).as_millis()
            < PATCH_BROADCAST_INTERVAL_MS
        {
            return;
        }

        self.seq += 1;

        // Threshold coalescing: if structural or >half of instruments are dirty, send full state
        let total = state.instruments.instruments.len();
        let use_full_instruments = self.dirty.instruments_structural
            || (total > 0 && self.dirty.dirty_instruments.len() > total / 2);

        let instruments = if use_full_instruments {
            Some(state.instruments.clone())
        } else {
            None
        };

        let instrument_patches = if !use_full_instruments
            && !self.dirty.dirty_instruments.is_empty()
        {
            let mut patches = HashMap::new();
            for &id in &self.dirty.dirty_instruments {
                if let Some(inst) = state.instruments.instrument(id) {
                    patches.insert(id, inst.clone());
                }
            }
            if patches.is_empty() {
                None
            } else {
                Some(patches)
            }
        } else {
            None
        };

        let patch = StatePatch {
            session: if self.dirty.session {
                Some(state.session.clone())
            } else {
                None
            },
            instruments,
            instrument_patches,
            ownership: if self.dirty.ownership {
                Some(state.ownership.clone())
            } else {
                None
            },
            privileged_client: if self.dirty.privileged_client {
                Some(state.privileged_client.clone())
            } else {
                None
            },
            seq: self.seq,
        };

        let msg = ServerMessage::StatePatchUpdate { patch };
        self.broadcast(&msg);
        self.dirty.clear();
        self.last_patch_broadcast = now;
    }

    /// Broadcast full state to all clients (periodic fallback).
    pub fn broadcast_full_sync(&mut self, state: &NetworkState) {
        self.seq += 1;
        let msg = ServerMessage::FullStateSync {
            state: state.clone(),
            seq: self.seq,
        };
        self.broadcast(&msg);
        self.dirty.clear();
        self.last_full_sync = Instant::now();
        self.force_full_sync = false;
    }

    /// Check if a full sync should be sent (every 30s or on request).
    pub fn needs_full_sync(&self) -> bool {
        self.force_full_sync
            || Instant::now().duration_since(self.last_full_sync).as_secs() >= 30
    }

    /// Broadcast a state update to all connected clients (legacy full broadcast).
    pub fn broadcast_state(&mut self, state: &NetworkState) {
        let msg = ServerMessage::StateUpdate {
            state: state.clone(),
        };
        self.broadcast(&msg);
    }

    /// Broadcast metering data to all connected clients.
    pub fn broadcast_metering(&mut self, playhead: u32, bpm: f32, peaks: (f32, f32)) {
        let msg = ServerMessage::Metering {
            playhead,
            bpm,
            peaks,
        };
        self.broadcast(&msg);
    }

    /// Broadcast a shutdown message to all clients.
    pub fn broadcast_shutdown(&mut self) {
        self.broadcast(&ServerMessage::Shutdown);
    }

    /// Get the number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get the number of pending (not yet handshaked) connections.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get the local address the server is bound to.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Build the ownership map for NetworkState.
    pub fn build_ownership_map(&self) -> HashMap<InstrumentId, OwnerInfo> {
        self.ownership
            .iter()
            .filter_map(|(&inst_id, &client_id)| {
                self.clients.get(&client_id).map(|client| {
                    (inst_id, OwnerInfo {
                        client_id,
                        client_name: client.name.clone(),
                    })
                })
            })
            .collect()
    }

    /// Get the privileged client info (if any).
    pub fn privileged_client_info(&self) -> Option<(ClientId, String)> {
        self.privileged_client.and_then(|id| {
            self.clients.get(&id).map(|c| (id, c.name.clone()))
        })
    }

    /// Reset the rate limiter so the next `broadcast_state_patch` is not throttled.
    pub fn reset_rate_limit(&mut self) {
        self.last_patch_broadcast = Instant::now() - std::time::Duration::from_millis(100);
    }

    /// Send a message to all connected clients.
    fn broadcast(&mut self, msg: &ServerMessage) {
        let mut disconnected = Vec::new();

        for (id, client) in &mut self.clients {
            if let Err(e) = client.send(msg) {
                warn!("Failed to send to client {:?}: {}", id, e);
                disconnected.push(*id);
            }
        }

        // Suspend disconnected clients (preserves ownership for reconnection)
        for id in disconnected {
            self.suspend_client(id);
        }
    }
}

/// Background thread that reads messages from a client and sends to the action channel.
fn client_reader_thread(
    client_id: ClientId,
    stream: TcpStream,
    action_tx: Sender<(ClientId, ClientMessage)>,
) {
    let mut reader = BufReader::new(stream);

    loop {
        match read_message::<_, ClientMessage>(&mut reader) {
            Ok(msg) => {
                let is_goodbye = matches!(msg, ClientMessage::Goodbye);
                if action_tx.send((client_id, msg)).is_err() {
                    // Receiver dropped, server is shutting down
                    break;
                }
                if is_goodbye {
                    break;
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::UnexpectedEof {
                    warn!("Client {:?} read error: {}", client_id, e);
                }
                // Send implicit goodbye on disconnect
                let _ = action_tx.send((client_id, ClientMessage::Goodbye));
                break;
            }
        }
    }

    info!("Client {:?} reader thread exiting", client_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::NetworkAction;
    use imbolc_types::{
        ArrangementAction, AutomationAction, AutomationTarget, BusAction, ChopperAction,
        InstrumentAction, InstrumentParameter, MidiAction, MixerAction, ParameterTarget,
        PianoRollAction, SequencerAction, ServerAction, SessionAction, SourceType,
        VstParamAction, VstTarget,
    };

    /// Helper: check that dirty flags indicate instruments are dirty in some way
    /// (either targeted or structural).
    fn instruments_dirty(d: &DirtyFlags) -> bool {
        !d.dirty_instruments.is_empty() || d.instruments_structural
    }

    // ── DirtyFlags::mark_from_action ────────────────────────────────

    #[test]
    fn dirty_instrument_structural_actions() {
        // Add and Sequencer are structural
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)),
            NetworkAction::Sequencer(SequencerAction::ToggleStep(0, 0)),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(d.instruments_structural, "instruments_structural for {:?}", action);
            assert!(!d.session, "session clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_instrument_targeted_actions() {
        // VstParam and targeted InstrumentAction go into dirty_instruments
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::VstParam(VstParamAction::SetParam(0, VstTarget::Source, 0, 0.5)),
            NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(5, 0.1)),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(
                !d.dirty_instruments.is_empty(),
                "dirty_instruments should be non-empty for {:?}", action
            );
            assert!(!d.instruments_structural, "instruments_structural false for {:?}", action);
            assert!(!d.session, "session clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_session_actions() {
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::PianoRoll(PianoRollAction::PlayStop),
            NetworkAction::Arrangement(ArrangementAction::TogglePlayMode),
            NetworkAction::Automation(AutomationAction::AddLane(
                AutomationTarget::Instrument(0, InstrumentParameter::Standard(ParameterTarget::Level)),
            )),
            NetworkAction::Session(SessionAction::Save),
            NetworkAction::Server(ServerAction::Connect),
            NetworkAction::Bus(BusAction::Add),
            NetworkAction::Chopper(ChopperAction::LoadSample),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(d.session, "session dirty for {:?}", action);
            assert!(!instruments_dirty(&d), "instruments clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_mixer_and_midi_mark_both() {
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::Mixer(MixerAction::Move(1)),
            NetworkAction::Midi(MidiAction::ConnectPort(0)),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(d.session, "session dirty for {:?}", action);
            assert!(d.instruments_structural, "instruments_structural for {:?}", action);
        }
    }

    #[test]
    fn dirty_undo_redo_mark_both() {
        for action in &[NetworkAction::Undo, NetworkAction::Redo] {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(d.session, "session dirty for {:?}", action);
            assert!(d.instruments_structural, "instruments_structural for {:?}", action);
        }
    }

    #[test]
    fn dirty_noop_actions() {
        for action in &[NetworkAction::None, NetworkAction::Quit] {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action);
            assert!(!d.any(), "no flags dirty for {:?}", action);
        }
    }

    // ── Targeted vs structural ──────────────────────────────────────

    #[test]
    fn dirty_instrument_targeted_vs_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(5, 0.1),
        ));
        assert_eq!(d.dirty_instruments, HashSet::from([5]));
        assert!(!d.instruments_structural);
    }

    #[test]
    fn dirty_instrument_delete_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(InstrumentAction::Delete(5)));
        assert!(d.instruments_structural);
    }

    #[test]
    fn dirty_instrument_add_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::Add(SourceType::Saw),
        ));
        assert!(d.instruments_structural);
        assert!(d.dirty_instruments.is_empty());
    }

    #[test]
    fn dirty_vst_param_is_targeted() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::VstParam(
            VstParamAction::SetParam(3, VstTarget::Source, 0, 0.5),
        ));
        assert_eq!(d.dirty_instruments, HashSet::from([3]));
        assert!(!d.instruments_structural);
    }

    #[test]
    fn dirty_undo_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Undo);
        assert!(d.instruments_structural);
    }

    #[test]
    fn dirty_accumulated_instruments() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(2, 0.1),
        ));
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(7, 0.2),
        ));
        assert_eq!(d.dirty_instruments, HashSet::from([2, 7]));
        assert!(!d.instruments_structural);
    }

    // ── DirtyFlags::any / clear ─────────────────────────────────────

    #[test]
    fn any_false_when_default() {
        assert!(!DirtyFlags::default().any());
    }

    #[test]
    fn any_true_for_each_flag() {
        for setter in [
            (|d: &mut DirtyFlags| d.session = true) as fn(&mut DirtyFlags),
            |d: &mut DirtyFlags| { d.dirty_instruments.insert(0); },
            |d: &mut DirtyFlags| d.instruments_structural = true,
            |d: &mut DirtyFlags| d.ownership = true,
            |d: &mut DirtyFlags| d.privileged_client = true,
        ] {
            let mut d = DirtyFlags::default();
            setter(&mut d);
            assert!(d.any());
        }
    }

    #[test]
    fn clear_resets_all() {
        let mut d = DirtyFlags {
            session: true,
            dirty_instruments: HashSet::from([0, 1, 2]),
            instruments_structural: true,
            ownership: true,
            privileged_client: true,
        };
        d.clear();
        assert!(!d.any());
        assert!(!d.session);
        assert!(d.dirty_instruments.is_empty());
        assert!(!d.instruments_structural);
        assert!(!d.ownership);
        assert!(!d.privileged_client);
    }

    // ── Accumulation (OR semantics) ─────────────────────────────────

    #[test]
    fn multiple_actions_accumulate() {
        let mut d = DirtyFlags::default();
        // First: session only
        d.mark_from_action(&NetworkAction::Server(ServerAction::Connect));
        assert!(d.session);
        assert!(!instruments_dirty(&d));
        // Second: instruments (structural) — session stays dirty
        d.mark_from_action(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
        assert!(d.session);
        assert!(d.instruments_structural);
    }

    #[test]
    fn ownership_not_set_by_actions() {
        // ownership is only set by mark_ownership_dirty(), never by mark_from_action()
        let all: Vec<NetworkAction> = vec![
            NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)),
            NetworkAction::Mixer(MixerAction::Move(1)),
            NetworkAction::Undo,
            NetworkAction::Server(ServerAction::Connect),
        ];
        let mut d = DirtyFlags::default();
        for a in &all {
            d.mark_from_action(a);
        }
        assert!(!d.ownership);
        assert!(!d.privileged_client);
    }
}
