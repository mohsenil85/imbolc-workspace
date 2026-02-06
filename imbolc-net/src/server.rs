//! Network server for Imbolc collaboration.
//!
//! Accepts client connections, receives actions, and broadcasts state updates.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use log::{error, info, warn};

use imbolc_types::InstrumentId;

use crate::framing::{read_message, write_message};
use crate::protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo,
    PrivilegeLevel, ServerMessage, SessionToken,
};

/// A connected client with its write half.
struct ClientConnection {
    name: String,
    writer: BufWriter<TcpStream>,
    /// Instruments this client owns (can mutate).
    owned_instruments: HashSet<InstrumentId>,
    /// Session token for reconnection.
    session_token: SessionToken,
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
        })
    }

    /// Accept any pending TCP connections (they become fully connected after Hello handshake).
    pub fn accept_connections(&mut self, _state: &NetworkState) {
        loop {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("Client connecting from {}", addr);

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

        // Clean up expired suspended sessions
        self.cleanup_expired_sessions();

        while let Ok((client_id, msg)) = self.action_rx.try_recv() {
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
                ClientMessage::RequestPrivilege => {
                    self.handle_privilege_request(client_id);
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

    /// Broadcast a state update to all connected clients.
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

    /// Send a message to all connected clients.
    fn broadcast(&mut self, msg: &ServerMessage) {
        let mut disconnected = Vec::new();

        for (id, client) in &mut self.clients {
            if let Err(e) = client.send(msg) {
                warn!("Failed to send to client {:?}: {}", id, e);
                disconnected.push(*id);
            }
        }

        // Remove disconnected clients
        for id in disconnected {
            self.clients.remove(&id);
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
