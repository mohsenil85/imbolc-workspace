//! Network server for Imbolc collaboration.
//!
//! Accepts client connections, receives actions, and broadcasts state updates.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use log::{error, info, warn};

use imbolc_types::{
    AutomationAction, AutomationLaneId, BusAction, BusId, InstrumentAction, InstrumentId,
    PianoRollAction, SessionState, VstParamAction,
};

use crate::framing::{read_message, serialize_frame, write_message};
use crate::protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo,
    PrivilegeLevel, ServerMessage, SessionToken, StatePatch,
};

/// What kind of frame is being queued — determines the drop policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FrameKind {
    /// Transient metering data — always droppable.
    Metering,
    /// Incremental state patch — superseded by newer patches or full syncs.
    StatePatch,
    /// Full state sync — supersedes all patches and older syncs.
    FullSync,
    /// Control messages (ping, shutdown, privilege) — never dropped.
    Control,
}

/// A queued frame awaiting delivery to a slow client.
struct QueuedFrame {
    data: Vec<u8>,
    /// Bytes already written (for partial write resume).
    offset: usize,
    kind: FrameKind,
}

/// Maximum number of frames in a client's outbox before declaring it stalled.
const MAX_OUTBOX_DEPTH: usize = 8;

/// Write timeout for client sockets (10ms).
const WRITE_TIMEOUT: Duration = Duration::from_millis(10);

/// Client metadata — stays on the main thread.
struct ClientInfo {
    name: String,
    /// Instruments this client owns (can mutate).
    owned_instruments: HashSet<InstrumentId>,
    /// Session token for reconnection.
    session_token: SessionToken,
    /// Last time we received any message from this client.
    last_seen: Instant,
}

/// Client write half — owned by the writer thread.
struct ClientWriter {
    stream: TcpStream,
    /// Per-client outbox for frames that couldn't be fully written.
    outbox: VecDeque<QueuedFrame>,
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
    /// Per-track piano roll edits (e.g. ToggleNote on a specific track).
    pub dirty_piano_roll_tracks: HashSet<InstrumentId>,
    /// Structural piano roll changes: metadata, loop settings, time sig, etc.
    pub piano_roll_structural: bool,
    /// Arrangement subsystem changed.
    pub arrangement: bool,
    /// Per-lane automation edits.
    pub dirty_automation_lanes: HashSet<AutomationLaneId>,
    /// Structural automation changes: add/remove lane, select, recording toggle.
    pub automation_structural: bool,
    /// Per-bus mixer edits.
    pub dirty_mixer_buses: HashSet<BusId>,
    /// Structural mixer changes: add/remove bus, layer groups, full mixer actions.
    pub mixer_structural: bool,
    /// Session "remainder" — musical settings, registries, rare changes, undo/redo.
    /// When set, sends the full SessionState (which includes all subsystems).
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
    ///
    /// `session` is needed to resolve piano roll track indices to `InstrumentId`.
    /// Pass `None` in unit tests — PianoRoll actions will fall back to structural.
    pub fn mark_from_action(
        &mut self,
        action: &NetworkAction,
        session: Option<&SessionState>,
    ) {
        match action {
            NetworkAction::Instrument(a) => {
                match a.target_instrument_id() {
                    Some(id) => {
                        // Delete changes the instrument Vec structurally
                        if matches!(a, InstrumentAction::Delete(_)) {
                            self.instruments_structural = true;
                            // Instrument deletion also affects piano roll track list
                            self.piano_roll_structural = true;
                        } else {
                            self.dirty_instruments.insert(id);
                        }
                    }
                    None => {
                        // Add, Select*, PlayNote, PlayDrumPad — structural
                        self.instruments_structural = true;
                        // Instrument addition also affects piano roll track list
                        if matches!(a, InstrumentAction::Add(_)) {
                            self.piano_roll_structural = true;
                        }
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
            NetworkAction::PianoRoll(pr) => {
                self.mark_piano_roll_action(pr, session);
            }
            NetworkAction::Arrangement(_) => {
                self.arrangement = true;
            }
            NetworkAction::Automation(a) => {
                self.mark_automation_action(a);
            }
            NetworkAction::Bus(a) => {
                self.mark_bus_action(a);
            }
            NetworkAction::LayerGroup(_) => {
                self.mixer_structural = true;
            }
            NetworkAction::Session(_) | NetworkAction::Server(_) | NetworkAction::Chopper(_) => {
                self.session = true;
            }
            NetworkAction::Mixer(_) => {
                self.mixer_structural = true;
                self.instruments_structural = true;
            }
            NetworkAction::Midi(_) => {
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

    /// Resolve a piano roll track index to an `InstrumentId` using session state.
    /// Falls back to structural if session is unavailable or index is out of bounds.
    fn resolve_track_id(
        &mut self,
        track: usize,
        session: Option<&SessionState>,
    ) {
        match session.and_then(|s| s.piano_roll.track_order.get(track).copied()) {
            Some(id) => { self.dirty_piano_roll_tracks.insert(id); }
            None => self.piano_roll_structural = true,
        }
    }

    /// Mark piano roll dirty flags based on the specific PianoRollAction variant.
    fn mark_piano_roll_action(
        &mut self,
        action: &PianoRollAction,
        session: Option<&SessionState>,
    ) {
        match action {
            // Per-track note edits
            PianoRollAction::ToggleNote { track, .. } => {
                self.resolve_track_id(*track, session);
            }
            PianoRollAction::DeleteNotesInRegion { track, .. } => {
                self.resolve_track_id(*track, session);
            }
            PianoRollAction::PasteNotes { track, .. } => {
                self.resolve_track_id(*track, session);
            }
            PianoRollAction::TogglePolyMode(track) => {
                self.resolve_track_id(*track, session);
            }
            // Recording: per-track if recording, otherwise audio-only (no flag)
            PianoRollAction::PlayNote { track, .. } => {
                if session.is_some_and(|s| s.piano_roll.recording) {
                    self.resolve_track_id(*track, session);
                }
            }
            PianoRollAction::PlayNotes { track, .. } => {
                if session.is_some_and(|s| s.piano_roll.recording) {
                    self.resolve_track_id(*track, session);
                }
            }
            // Metadata / structural changes
            PianoRollAction::ToggleLoop
            | PianoRollAction::SetLoopStart(_)
            | PianoRollAction::SetLoopEnd(_)
            | PianoRollAction::CycleTimeSig
            | PianoRollAction::AdjustSwing(_) => {
                self.piano_roll_structural = true;
            }
            // Audio-only / transient — no state change to broadcast
            PianoRollAction::PlayStop
            | PianoRollAction::PlayStopRecord
            | PianoRollAction::ReleaseNote { .. }
            | PianoRollAction::ReleaseNotes { .. }
            | PianoRollAction::BounceToWav
            | PianoRollAction::ExportStems
            | PianoRollAction::CancelExport
            | PianoRollAction::RenderToWav(_) => {}
            // Clipboard-only — no state change
            PianoRollAction::CopyNotes { .. } => {}
        }
    }

    /// Mark automation dirty flags based on the specific AutomationAction variant.
    fn mark_automation_action(&mut self, action: &AutomationAction) {
        match action {
            // Per-lane edits
            AutomationAction::AddPoint(id, ..)
            | AutomationAction::RemovePoint(id, ..)
            | AutomationAction::MovePoint(id, ..)
            | AutomationAction::SetCurveType(id, ..)
            | AutomationAction::ClearLane(id)
            | AutomationAction::ToggleLaneEnabled(id)
            | AutomationAction::ToggleLaneArm(id)
            | AutomationAction::DeletePointsInRange(id, ..)
            | AutomationAction::PastePoints(id, ..) => {
                self.dirty_automation_lanes.insert(*id);
            }
            // Clipboard-only — no state change
            AutomationAction::CopyPoints(..) => {}
            // Structural changes
            AutomationAction::AddLane(_)
            | AutomationAction::RemoveLane(_)
            | AutomationAction::SelectLane(_)
            | AutomationAction::ToggleRecording
            | AutomationAction::ArmAllLanes
            | AutomationAction::DisarmAllLanes
            | AutomationAction::RecordValue(..) => {
                self.automation_structural = true;
            }
        }
    }

    /// Mark mixer dirty flags based on the specific BusAction variant.
    fn mark_bus_action(&mut self, action: &BusAction) {
        match action {
            // Per-bus edits
            BusAction::Rename(id, ..)
            | BusAction::AddEffect(id, ..)
            | BusAction::RemoveEffect(id, ..)
            | BusAction::MoveEffect(id, ..)
            | BusAction::ToggleEffectBypass(id, ..)
            | BusAction::AdjustEffectParam(id, ..) => {
                self.dirty_mixer_buses.insert(*id);
            }
            // Structural changes
            BusAction::Add | BusAction::Remove(_) => {
                self.mixer_structural = true;
            }
        }
    }

    fn any(&self) -> bool {
        !self.dirty_piano_roll_tracks.is_empty()
            || self.piano_roll_structural
            || self.arrangement
            || !self.dirty_automation_lanes.is_empty()
            || self.automation_structural
            || !self.dirty_mixer_buses.is_empty()
            || self.mixer_structural
            || self.session
            || !self.dirty_instruments.is_empty()
            || self.instruments_structural
            || self.ownership
            || self.privileged_client
    }

    fn clear(&mut self) {
        self.dirty_piano_roll_tracks.clear();
        self.piano_roll_structural = false;
        self.arrangement = false;
        self.dirty_automation_lanes.clear();
        self.automation_structural = false;
        self.dirty_mixer_buses.clear();
        self.mixer_structural = false;
        self.session = false;
        self.dirty_instruments.clear();
        self.instruments_structural = false;
        self.ownership = false;
        self.privileged_client = false;
    }
}

impl ClientWriter {
    /// Try to write a frame directly; queue the remainder on partial write or timeout.
    fn send_frame(&mut self, data: &[u8], kind: FrameKind) -> io::Result<()> {
        // First try a direct write
        match self.stream.write(data) {
            Ok(n) if n == data.len() => {
                // Wrote everything — still need to push through to the OS
                Ok(())
            }
            Ok(n) => {
                // Partial write — queue the remainder
                self.queue_frame(data[n..].to_vec(), kind);
                Ok(())
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock
                || e.kind() == io::ErrorKind::TimedOut =>
            {
                // Timeout — queue the whole frame
                self.queue_frame(data.to_vec(), kind);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Apply the drop policy and push a frame into the outbox.
    fn queue_frame(&mut self, data: Vec<u8>, kind: FrameKind) {
        match kind {
            FrameKind::Metering => {
                // Drop all pending (unstarted) metering frames — keep only latest
                self.outbox.retain(|f| f.kind != FrameKind::Metering || f.offset > 0);
            }
            FrameKind::StatePatch => {
                // Drop all unstarted StatePatch and FullSync frames
                self.outbox.retain(|f| {
                    f.offset > 0
                        || (f.kind != FrameKind::StatePatch && f.kind != FrameKind::FullSync)
                });
            }
            FrameKind::FullSync => {
                // Drop all unstarted StatePatch and FullSync frames
                self.outbox.retain(|f| {
                    f.offset > 0
                        || (f.kind != FrameKind::StatePatch && f.kind != FrameKind::FullSync)
                });
            }
            FrameKind::Control => {
                // Never drop control frames, and don't drop anything else
            }
        }
        self.outbox.push_back(QueuedFrame {
            data,
            offset: 0,
            kind,
        });
    }

    /// Drain the outbox by writing queued frames. Returns Ok(true) if outbox is empty.
    fn flush_outbox(&mut self) -> io::Result<bool> {
        while let Some(front) = self.outbox.front_mut() {
            let remaining = &front.data[front.offset..];
            match self.stream.write(remaining) {
                Ok(0) => {
                    // Connection closed
                    return Err(io::Error::new(io::ErrorKind::WriteZero, "write returned 0"));
                }
                Ok(n) => {
                    front.offset += n;
                    if front.offset >= front.data.len() {
                        self.outbox.pop_front();
                    } else {
                        // Partial write — stop for now
                        return Ok(false);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
                {
                    return Ok(false);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(true)
    }

    /// True if the outbox exceeds the maximum depth threshold.
    fn is_stalled(&self) -> bool {
        self.outbox.len() > MAX_OUTBOX_DEPTH
    }
}

// ── Writer thread protocol ──────────────────────────────────────

/// Commands sent from the main thread to the writer thread.
enum WriterCommand {
    /// Register a new client's write half after handshake.
    AddClient { client_id: ClientId, stream: TcpStream },
    /// Remove a client (suspended/disconnected).
    RemoveClient { client_id: ClientId },
    /// Broadcast pre-serialized frame to all clients.
    Broadcast { frame: Vec<u8>, kind: FrameKind },
    /// Send pre-serialized frame to one client.
    SendTo { client_id: ClientId, frame: Vec<u8>, kind: FrameKind },
    /// Block until all pending commands are processed (test sync).
    Flush { done: Sender<()> },
    /// Inject dummy frames into outboxes (test helper).
    InjectFrames { count: usize },
    /// Shut down.
    Shutdown,
}

/// Feedback from the writer thread to the main thread.
enum WriterFeedback {
    /// Client write failed or outbox overflowed — main thread should suspend.
    ClientStalled { client_id: ClientId },
}

/// The writer thread function — owns all client write halves.
fn writer_thread(
    cmd_rx: Receiver<WriterCommand>,
    feedback_tx: Sender<WriterFeedback>,
) {
    let mut writers: HashMap<ClientId, ClientWriter> = HashMap::new();

    loop {
        // Drain all pending commands, deferring Flush responses
        let mut got_command = false;
        let mut pending_flushes: Vec<Sender<()>> = Vec::new();
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    got_command = true;
                    match cmd {
                        WriterCommand::AddClient { client_id, stream } => {
                            writers.insert(client_id, ClientWriter {
                                stream,
                                outbox: VecDeque::new(),
                            });
                        }
                        WriterCommand::RemoveClient { client_id } => {
                            writers.remove(&client_id);
                        }
                        WriterCommand::Broadcast { frame, kind } => {
                            let mut stalled = Vec::new();
                            for (&id, writer) in &mut writers {
                                // Try to drain any pending outbox first
                                if !writer.outbox.is_empty()
                                    && writer.flush_outbox().is_err()
                                {
                                    stalled.push(id);
                                    continue;
                                }
                                // Send the new frame
                                if writer.send_frame(&frame, kind).is_err() {
                                    stalled.push(id);
                                    continue;
                                }
                                // Check stall threshold
                                if writer.is_stalled() {
                                    stalled.push(id);
                                }
                            }
                            for id in stalled {
                                writers.remove(&id);
                                let _ = feedback_tx.send(WriterFeedback::ClientStalled { client_id: id });
                            }
                        }
                        WriterCommand::SendTo { client_id, frame, kind } => {
                            if let Some(writer) = writers.get_mut(&client_id) {
                                if writer.send_frame(&frame, kind).is_err() {
                                    writers.remove(&client_id);
                                    let _ = feedback_tx.send(WriterFeedback::ClientStalled { client_id });
                                }
                            }
                        }
                        WriterCommand::Flush { done } => {
                            // Defer response until after outbox flush pass
                            pending_flushes.push(done);
                        }
                        WriterCommand::InjectFrames { count } => {
                            for writer in writers.values_mut() {
                                for _ in 0..count {
                                    writer.outbox.push_back(QueuedFrame {
                                        data: vec![0u8; 64 * 1024],
                                        offset: 0,
                                        kind: FrameKind::Control,
                                    });
                                }
                            }
                        }
                        WriterCommand::Shutdown => {
                            return;
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // One pass of outbox flushing across all writers
        let mut stalled = Vec::new();
        for (&id, writer) in &mut writers {
            if !writer.outbox.is_empty() {
                match writer.flush_outbox() {
                    Err(_) => {
                        stalled.push(id);
                    }
                    Ok(_) => {
                        if writer.is_stalled() {
                            stalled.push(id);
                        }
                    }
                }
            }
        }
        for id in stalled {
            writers.remove(&id);
            let _ = feedback_tx.send(WriterFeedback::ClientStalled { client_id: id });
        }

        // Respond to deferred flush requests (after outbox pass)
        for done in pending_flushes.drain(..) {
            let _ = done.send(());
        }

        // Sleep briefly if no commands were processed to avoid spinning
        if !got_command {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

/// A pending connection awaiting Hello handshake.
struct PendingConnection {
    stream: TcpStream,
}

/// Network server that accepts client connections and coordinates actions.
pub struct NetServer {
    listener: TcpListener,
    /// Fully connected clients (completed Hello handshake) — metadata only.
    clients: HashMap<ClientId, ClientInfo>,
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
    /// Channel to send commands to the writer thread.
    writer_tx: Sender<WriterCommand>,
    /// Channel to receive feedback from the writer thread.
    writer_feedback_rx: Receiver<WriterFeedback>,
    /// Handle to the writer thread (joined on drop).
    writer_handle: Option<JoinHandle<()>>,
}

impl NetServer {
    /// Bind the server to an address.
    pub fn bind(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let (action_tx, action_rx) = mpsc::channel();

        // Spawn the writer thread
        let (writer_tx, writer_rx) = mpsc::channel();
        let (feedback_tx, feedback_rx) = mpsc::channel();
        let writer_handle = thread::spawn(move || {
            writer_thread(writer_rx, feedback_tx);
        });

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
            last_patch_broadcast: Instant::now() - Duration::from_secs(1),
            writer_tx,
            writer_feedback_rx: feedback_rx,
            writer_handle: Some(writer_handle),
        })
    }

    /// Accept any pending TCP connections (they become fully connected after Hello handshake).
    pub fn accept_connections(&mut self) {
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

                    // Set write timeout so slow clients don't block the server
                    if let Err(e) = stream.set_write_timeout(Some(WRITE_TIMEOUT)) {
                        error!("Failed to set write timeout: {}", e);
                        continue;
                    }

                    // Start reader thread
                    let action_tx = self.action_tx.clone();
                    thread::spawn(move || {
                        client_reader_thread(client_id, read_stream, action_tx);
                    });

                    // Store as pending (will become full client on Hello)
                    self.pending.insert(client_id, PendingConnection { stream });

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
    ///
    /// Takes references to session and instrument state — only builds a full
    /// `NetworkState` during Hello handshakes (rare).
    pub fn poll_actions(
        &mut self,
        session: &imbolc_types::SessionState,
        instruments: &imbolc_types::InstrumentState,
    ) -> Vec<(ClientId, NetworkAction)> {
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
                                if let Err(e) = write_message(&mut pending.stream, &msg) {
                                    error!("Failed to send reconnect success to {:?}: {}", client_id, e);
                                    continue;
                                }

                                // Send current state so the client isn't stale
                                let net_state = NetworkState {
                                    session: session.clone(),
                                    instruments: instruments.clone(),
                                    ownership: self.build_ownership_map(),
                                    privileged_client: self.privileged_client_info(),
                                };
                                let state_msg = ServerMessage::StateUpdate { state: net_state };
                                if let Err(e) = write_message(&mut pending.stream, &state_msg) {
                                    error!("Failed to send state to reconnecting {:?}: {}", client_id, e);
                                    continue;
                                }

                                // Send the write half to the writer thread
                                let _ = self.writer_tx.send(WriterCommand::AddClient {
                                    client_id,
                                    stream: pending.stream,
                                });

                                self.clients.insert(client_id, ClientInfo {
                                    name: suspended.client_name,
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
                                let _ = write_message(&mut pending.stream, &msg);
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

                        // Build NetworkState on demand (only during Hello handshake)
                        let net_state = NetworkState {
                            session: session.clone(),
                            instruments: instruments.clone(),
                            ownership: self.build_ownership_map(),
                            privileged_client: self.privileged_client_info(),
                        };

                        // Send Welcome with granted instruments
                        let welcome = ServerMessage::Welcome {
                            client_id,
                            granted_instruments: granted.clone(),
                            state: net_state,
                            privilege,
                            session_token: session_token.clone(),
                        };
                        if let Err(e) = write_message(&mut pending.stream, &welcome) {
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

                        // Send the write half to the writer thread
                        let _ = self.writer_tx.send(WriterCommand::AddClient {
                            client_id,
                            stream: pending.stream,
                        });

                        // Promote to full client (metadata only)
                        self.clients.insert(client_id, ClientInfo {
                            name: client_name.clone(),
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
                        self.send_to_client(client_id, &ServerMessage::ActionRejected { reason: reason.clone() });
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
                    self.send_to_client(client_id, &ServerMessage::Pong);
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
                self.send_to_client(client_id, &ServerMessage::PrivilegeGranted);
                return;
            }

            // Revoke from current holder
            let current_name = self.clients.get(&current)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "unknown".into());

            self.send_to_client(current, &ServerMessage::PrivilegeRevoked);

            info!(
                "Privilege transferred from {:?} '{}' to {:?}",
                current, current_name, client_id
            );
        }

        // Grant to new client
        self.privileged_client = Some(client_id);
        self.send_to_client(client_id, &ServerMessage::PrivilegeGranted);

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

            // Tell the writer thread to drop this client's write half
            let _ = self.writer_tx.send(WriterCommand::RemoveClient { client_id });

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

            // Ping remaining clients via broadcast
            self.broadcast(&ServerMessage::Ping, FrameKind::Control);
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
            let _ = self.writer_tx.send(WriterCommand::RemoveClient { client_id });
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
            NetworkAction::Bus(_) | NetworkAction::LayerGroup(_) => {
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
    pub fn mark_dirty(&mut self, action: &NetworkAction, session: &SessionState) {
        self.dirty.mark_from_action(action, Some(session));
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

        // If dirty.session is set, send full SessionState (includes all subsystems).
        // Otherwise, send only the specific subsystems that changed.
        let (
            session, piano_roll, piano_roll_track_patches, arrangement,
            automation, automation_lane_patches, mixer, mixer_bus_patches,
        ) = if self.dirty.session {
                (Some(state.session.clone()), None, None, None, None, None, None, None)
            } else {
                // Piano roll: threshold coalescing (same pattern as instruments)
                let pr_total = state.session.piano_roll.tracks.len();
                let use_full_pr = self.dirty.piano_roll_structural
                    || (pr_total > 0
                        && self.dirty.dirty_piano_roll_tracks.len() > pr_total / 2);

                let pr_full = if use_full_pr
                    && (self.dirty.piano_roll_structural
                        || !self.dirty.dirty_piano_roll_tracks.is_empty())
                {
                    Some(state.session.piano_roll.clone())
                } else {
                    None
                };

                let pr_patches = if !use_full_pr
                    && !self.dirty.dirty_piano_roll_tracks.is_empty()
                {
                    let mut patches = HashMap::new();
                    for &id in &self.dirty.dirty_piano_roll_tracks {
                        if let Some(track) = state.session.piano_roll.tracks.get(&id) {
                            patches.insert(id, track.clone());
                        }
                    }
                    if patches.is_empty() { None } else { Some(patches) }
                } else {
                    None
                };

                // Automation: threshold coalescing (same pattern as instruments)
                let auto_total = state.session.automation.lanes.len();
                let use_full_auto = self.dirty.automation_structural
                    || (auto_total > 0
                        && self.dirty.dirty_automation_lanes.len() > auto_total / 2);

                let auto_full = if use_full_auto
                    && (self.dirty.automation_structural
                        || !self.dirty.dirty_automation_lanes.is_empty())
                {
                    Some(state.session.automation.clone())
                } else {
                    None
                };

                let auto_patches = if !use_full_auto
                    && !self.dirty.dirty_automation_lanes.is_empty()
                {
                    let mut patches = HashMap::new();
                    for &id in &self.dirty.dirty_automation_lanes {
                        if let Some(lane) = state.session.automation.lane(id) {
                            patches.insert(id, lane.clone());
                        }
                    }
                    if patches.is_empty() { None } else { Some(patches) }
                } else {
                    None
                };

                // Mixer: threshold coalescing (same pattern as instruments)
                let mixer_total = state.session.mixer.buses.len();
                let use_full_mixer = self.dirty.mixer_structural
                    || (mixer_total > 0
                        && self.dirty.dirty_mixer_buses.len() > mixer_total / 2);

                let mixer_full = if use_full_mixer
                    && (self.dirty.mixer_structural
                        || !self.dirty.dirty_mixer_buses.is_empty())
                {
                    Some(state.session.mixer.clone())
                } else {
                    None
                };

                let mixer_patches = if !use_full_mixer
                    && !self.dirty.dirty_mixer_buses.is_empty()
                {
                    let mut patches = HashMap::new();
                    for &id in &self.dirty.dirty_mixer_buses {
                        if let Some(bus) = state.session.mixer.bus(id) {
                            patches.insert(id, bus.clone());
                        }
                    }
                    if patches.is_empty() { None } else { Some(patches) }
                } else {
                    None
                };

                (
                    None,
                    pr_full,
                    pr_patches,
                    if self.dirty.arrangement { Some(state.session.arrangement.clone()) } else { None },
                    auto_full,
                    auto_patches,
                    mixer_full,
                    mixer_patches,
                )
            };

        let patch = StatePatch {
            session,
            piano_roll,
            piano_roll_track_patches,
            arrangement,
            automation,
            automation_lane_patches,
            mixer,
            mixer_bus_patches,
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
        self.broadcast(&msg, FrameKind::StatePatch);
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
        self.broadcast(&msg, FrameKind::FullSync);
        self.dirty.clear();
        self.last_full_sync = Instant::now();
        self.force_full_sync = false;
    }

    /// Check if any dirty flags are set (useful for callers to avoid building state when clean).
    pub fn has_dirty_flags(&self) -> bool {
        self.dirty.any()
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
        self.broadcast(&msg, FrameKind::StatePatch);
    }

    /// Broadcast metering data to all connected clients.
    pub fn broadcast_metering(&mut self, playhead: u32, bpm: f32, peaks: (f32, f32)) {
        let msg = ServerMessage::Metering {
            playhead,
            bpm,
            peaks,
        };
        self.broadcast(&msg, FrameKind::Metering);
    }

    /// Broadcast a shutdown message to all clients.
    pub fn broadcast_shutdown(&mut self) {
        self.broadcast(&ServerMessage::Shutdown, FrameKind::Control);
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
        self.last_patch_broadcast = Instant::now() - Duration::from_millis(100);
    }

    /// Inject large dummy frames into all clients' outboxes (for testing stall detection).
    ///
    /// Sends a command to the writer thread to push `count` large Control frames (64KB each)
    /// into each client's outbox.
    pub fn inject_outbox_frames(&mut self, count: usize) {
        let _ = self.writer_tx.send(WriterCommand::InjectFrames { count });
    }

    /// Process feedback from the writer thread (stalled clients).
    ///
    /// Call once per server loop iteration, before `accept_connections()`.
    pub fn process_writer_feedback(&mut self) {
        while let Ok(feedback) = self.writer_feedback_rx.try_recv() {
            match feedback {
                WriterFeedback::ClientStalled { client_id } => {
                    warn!("Writer thread reports client {:?} stalled, suspending", client_id);
                    self.suspend_client(client_id);
                }
            }
        }
    }

    /// Block until the writer thread has processed all pending commands.
    ///
    /// Used by tests to synchronize after broadcasts.
    pub fn flush_writer(&self) {
        let (tx, rx) = mpsc::channel();
        let _ = self.writer_tx.send(WriterCommand::Flush { done: tx });
        let _ = rx.recv_timeout(Duration::from_secs(5));
    }

    /// Send a message to a specific client via the writer thread.
    fn send_to_client(&self, client_id: ClientId, msg: &ServerMessage) {
        match serialize_frame(msg) {
            Ok(frame) => {
                let _ = self.writer_tx.send(WriterCommand::SendTo {
                    client_id,
                    frame,
                    kind: FrameKind::Control,
                });
            }
            Err(e) => {
                error!("Failed to serialize message for {:?}: {}", client_id, e);
            }
        }
    }

    /// Send a message to all connected clients via the writer thread.
    ///
    /// Serializes the message once, then sends the pre-serialized frame to the
    /// writer thread for fan-out delivery. The writer thread handles outbox
    /// queuing and stall detection.
    fn broadcast(&mut self, msg: &ServerMessage, kind: FrameKind) {
        let frame = match serialize_frame(msg) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to serialize broadcast message: {}", e);
                return;
            }
        };

        let _ = self.writer_tx.send(WriterCommand::Broadcast { frame, kind });
    }
}

impl Drop for NetServer {
    fn drop(&mut self) {
        let _ = self.writer_tx.send(WriterCommand::Shutdown);
        if let Some(handle) = self.writer_handle.take() {
            let _ = handle.join();
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
            d.mark_from_action(action, None);
            assert!(d.instruments_structural, "instruments_structural for {:?}", action);
            assert!(!d.session, "session clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_instrument_targeted_actions() {
        // VstParam and targeted InstrumentAction go into dirty_instruments
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::VstParam(VstParamAction::SetParam(InstrumentId::new(0), VstTarget::Source, 0, 0.5)),
            NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(InstrumentId::new(5), 0.1)),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action, None);
            assert!(
                !d.dirty_instruments.is_empty(),
                "dirty_instruments should be non-empty for {:?}", action
            );
            assert!(!d.instruments_structural, "instruments_structural false for {:?}", action);
            assert!(!d.session, "session clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_session_remainder_actions() {
        // These actions mark the "session" (remainder) flag — sends full SessionState
        let cases: Vec<NetworkAction> = vec![
            NetworkAction::Session(SessionAction::Save),
            NetworkAction::Server(ServerAction::Connect),
            NetworkAction::Chopper(ChopperAction::LoadSample),
        ];
        for action in &cases {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action, None);
            assert!(d.session, "session dirty for {:?}", action);
            assert!(!instruments_dirty(&d), "instruments clean for {:?}", action);
        }
    }

    #[test]
    fn dirty_subsystem_actions() {
        // PianoRoll metadata → piano_roll_structural (without session, falls back to structural)
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::PianoRoll(PianoRollAction::ToggleLoop), None);
        assert!(d.piano_roll_structural, "piano_roll_structural dirty for ToggleLoop");
        assert!(!d.session, "session clean for PianoRoll");

        // PianoRoll audio-only → no flags
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::PianoRoll(PianoRollAction::PlayStop), None);
        assert!(!d.any(), "PlayStop should not dirty anything");

        // Arrangement → arrangement flag
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Arrangement(ArrangementAction::TogglePlayMode), None);
        assert!(d.arrangement, "arrangement dirty");
        assert!(!d.session, "session clean for Arrangement");

        // Automation (structural) → automation_structural flag
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Automation(AutomationAction::AddLane(
            AutomationTarget::Instrument(InstrumentId::new(0), InstrumentParameter::Standard(ParameterTarget::Level)),
        )), None);
        assert!(d.automation_structural, "automation_structural dirty");
        assert!(!d.session, "session clean for Automation");

        // Bus (structural) → mixer_structural flag
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Bus(BusAction::Add), None);
        assert!(d.mixer_structural, "mixer_structural dirty for Bus::Add");
        assert!(!d.session, "session clean for Bus");
    }

    #[test]
    fn dirty_mixer_marks_mixer_and_instruments() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Mixer(MixerAction::Move(1)), None);
        assert!(d.mixer_structural, "mixer_structural dirty for Mixer");
        assert!(d.instruments_structural, "instruments_structural for Mixer");
        assert!(!d.session, "session clean for Mixer");
    }

    #[test]
    fn dirty_midi_marks_session_and_instruments() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Midi(MidiAction::ConnectPort(0)), None);
        assert!(d.session, "session dirty for Midi");
        assert!(d.instruments_structural, "instruments_structural for Midi");
    }

    #[test]
    fn dirty_undo_redo_mark_both() {
        for action in &[NetworkAction::Undo, NetworkAction::Redo] {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action, None);
            assert!(d.session, "session dirty for {:?}", action);
            assert!(d.instruments_structural, "instruments_structural for {:?}", action);
        }
    }

    #[test]
    fn dirty_noop_actions() {
        for action in &[NetworkAction::None, NetworkAction::Quit] {
            let mut d = DirtyFlags::default();
            d.mark_from_action(action, None);
            assert!(!d.any(), "no flags dirty for {:?}", action);
        }
    }

    // ── Targeted vs structural ──────────────────────────────────────

    #[test]
    fn dirty_instrument_targeted_vs_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(InstrumentId::new(5), 0.1),
        ), None);
        assert_eq!(d.dirty_instruments, HashSet::from([InstrumentId::new(5)]));
        assert!(!d.instruments_structural);
    }

    #[test]
    fn dirty_instrument_delete_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(InstrumentAction::Delete(InstrumentId::new(5))), None);
        assert!(d.instruments_structural);
        assert!(d.piano_roll_structural, "instrument delete should also mark piano_roll_structural");
    }

    #[test]
    fn dirty_instrument_add_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::Add(SourceType::Saw),
        ), None);
        assert!(d.instruments_structural);
        assert!(d.dirty_instruments.is_empty());
        assert!(d.piano_roll_structural, "instrument add should also mark piano_roll_structural");
    }

    #[test]
    fn dirty_vst_param_is_targeted() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::VstParam(
            VstParamAction::SetParam(InstrumentId::new(3), VstTarget::Source, 0, 0.5),
        ), None);
        assert_eq!(d.dirty_instruments, HashSet::from([InstrumentId::new(3)]));
        assert!(!d.instruments_structural);
    }

    #[test]
    fn dirty_undo_is_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Undo, None);
        assert!(d.instruments_structural);
    }

    #[test]
    fn dirty_accumulated_instruments() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(InstrumentId::new(2), 0.1),
        ), None);
        d.mark_from_action(&NetworkAction::Instrument(
            InstrumentAction::AdjustFilterCutoff(InstrumentId::new(7), 0.2),
        ), None);
        assert_eq!(d.dirty_instruments, HashSet::from([InstrumentId::new(2), InstrumentId::new(7)]));
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
            (|d: &mut DirtyFlags| { d.dirty_piano_roll_tracks.insert(InstrumentId::new(0)); }) as fn(&mut DirtyFlags),
            |d: &mut DirtyFlags| d.piano_roll_structural = true,
            |d: &mut DirtyFlags| d.arrangement = true,
            |d: &mut DirtyFlags| { d.dirty_automation_lanes.insert(0); },
            |d: &mut DirtyFlags| d.automation_structural = true,
            |d: &mut DirtyFlags| { d.dirty_mixer_buses.insert(BusId::new(1)); },
            |d: &mut DirtyFlags| d.mixer_structural = true,
            |d: &mut DirtyFlags| d.session = true,
            |d: &mut DirtyFlags| { d.dirty_instruments.insert(InstrumentId::new(0)); },
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
            dirty_piano_roll_tracks: HashSet::from([InstrumentId::new(0), InstrumentId::new(1)]),
            piano_roll_structural: true,
            arrangement: true,
            dirty_automation_lanes: HashSet::from([0, 1]),
            automation_structural: true,
            dirty_mixer_buses: HashSet::from([BusId::new(1), BusId::new(2)]),
            mixer_structural: true,
            session: true,
            dirty_instruments: HashSet::from([InstrumentId::new(0), InstrumentId::new(1), InstrumentId::new(2)]),
            instruments_structural: true,
            ownership: true,
            privileged_client: true,
        };
        d.clear();
        assert!(!d.any());
        assert!(d.dirty_piano_roll_tracks.is_empty());
        assert!(!d.piano_roll_structural);
        assert!(!d.arrangement);
        assert!(d.dirty_automation_lanes.is_empty());
        assert!(!d.automation_structural);
        assert!(d.dirty_mixer_buses.is_empty());
        assert!(!d.mixer_structural);
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
        d.mark_from_action(&NetworkAction::Server(ServerAction::Connect), None);
        assert!(d.session);
        assert!(!instruments_dirty(&d));
        // Second: instruments (structural) — session stays dirty
        d.mark_from_action(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)), None);
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
            d.mark_from_action(a, None);
        }
        assert!(!d.ownership);
        assert!(!d.privileged_client);
    }

    // ── Automation & mixer targeted vs structural ──────────────────

    #[test]
    fn dirty_automation_targeted_vs_structural() {
        // AddPoint → per-lane
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Automation(
            AutomationAction::AddPoint(42, 100, 0.5),
        ), None);
        assert!(d.dirty_automation_lanes.contains(&42));
        assert!(!d.automation_structural);

        // AddLane → structural
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Automation(AutomationAction::AddLane(
            AutomationTarget::Instrument(InstrumentId::new(0), InstrumentParameter::Standard(ParameterTarget::Level)),
        )), None);
        assert!(d.dirty_automation_lanes.is_empty());
        assert!(d.automation_structural);
    }

    #[test]
    fn dirty_automation_copypoints_is_noop() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Automation(
            AutomationAction::CopyPoints(1, 0, 100),
        ), None);
        assert!(!d.any(), "CopyPoints should not dirty anything");
    }

    #[test]
    fn dirty_mixer_bus_targeted_vs_structural() {
        // Rename → per-bus
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Bus(
            BusAction::Rename(BusId::new(3), "FX".into()),
        ), None);
        assert!(d.dirty_mixer_buses.contains(&BusId::new(3)));
        assert!(!d.mixer_structural);

        // Add → structural
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Bus(BusAction::Add), None);
        assert!(d.dirty_mixer_buses.is_empty());
        assert!(d.mixer_structural);

        // Remove → structural
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::Bus(BusAction::Remove(BusId::new(1))), None);
        assert!(d.dirty_mixer_buses.is_empty());
        assert!(d.mixer_structural);
    }

    #[test]
    fn dirty_layer_group_is_mixer_structural() {
        let mut d = DirtyFlags::default();
        d.mark_from_action(&NetworkAction::LayerGroup(
            imbolc_types::LayerGroupAction::AddEffect(0, imbolc_types::EffectType::Delay),
        ), None);
        assert!(d.mixer_structural);
        assert!(d.dirty_mixer_buses.is_empty());
    }

    // ── Outbox drop policy ──────────────────────────────────────────

    /// Helper: create a QueuedFrame with given kind and offset.
    fn make_frame(kind: FrameKind, offset: usize) -> QueuedFrame {
        QueuedFrame {
            data: vec![0u8; 100],
            offset,
            kind,
        }
    }

    /// Helper: create a ClientWriter with an outbox (no real socket needed for unit tests).
    fn make_test_writer(outbox: VecDeque<QueuedFrame>) -> ClientWriter {
        // Use a loopback TCP connection — we only test the outbox logic, not actual I/O
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).unwrap();
        let _ = listener.accept().unwrap();
        ClientWriter {
            stream,
            outbox,
        }
    }

    #[test]
    fn drop_policy_metering() {
        let mut outbox = VecDeque::new();
        outbox.push_back(make_frame(FrameKind::Metering, 0));
        outbox.push_back(make_frame(FrameKind::Metering, 0));
        outbox.push_back(make_frame(FrameKind::Metering, 0));
        let mut client = make_test_writer(outbox);
        // Queue a new metering frame — should drop the 3 pending ones
        client.queue_frame(vec![0u8; 50], FrameKind::Metering);
        assert_eq!(client.outbox.len(), 1, "only the new metering frame should remain");
        assert_eq!(client.outbox[0].data.len(), 50, "should be the new frame");
    }

    #[test]
    fn drop_policy_state_patch_superseded() {
        let mut outbox = VecDeque::new();
        outbox.push_back(make_frame(FrameKind::StatePatch, 0));
        outbox.push_back(make_frame(FrameKind::Control, 0)); // control: not dropped
        outbox.push_back(make_frame(FrameKind::FullSync, 0));
        let mut client = make_test_writer(outbox);
        client.queue_frame(vec![0u8; 50], FrameKind::StatePatch);
        // StatePatch drops unstarted StatePatch + FullSync, keeps Control
        assert_eq!(client.outbox.len(), 2);
        assert_eq!(client.outbox[0].kind, FrameKind::Control);
        assert_eq!(client.outbox[1].kind, FrameKind::StatePatch);
    }

    #[test]
    fn drop_policy_full_sync_supersedes_patches() {
        let mut outbox = VecDeque::new();
        outbox.push_back(make_frame(FrameKind::StatePatch, 0));
        outbox.push_back(make_frame(FrameKind::StatePatch, 0));
        outbox.push_back(make_frame(FrameKind::FullSync, 0));
        let mut client = make_test_writer(outbox);
        client.queue_frame(vec![0u8; 50], FrameKind::FullSync);
        // FullSync drops all unstarted patches + older full syncs
        assert_eq!(client.outbox.len(), 1);
        assert_eq!(client.outbox[0].kind, FrameKind::FullSync);
        assert_eq!(client.outbox[0].data.len(), 50);
    }

    #[test]
    fn drop_policy_preserves_partial_writes() {
        let mut outbox = VecDeque::new();
        outbox.push_back(make_frame(FrameKind::StatePatch, 10)); // offset > 0 = partial
        outbox.push_back(make_frame(FrameKind::Metering, 5)); // offset > 0 = partial
        outbox.push_back(make_frame(FrameKind::StatePatch, 0)); // unstarted
        let mut client = make_test_writer(outbox);
        // Queue FullSync — should drop only the unstarted StatePatch
        client.queue_frame(vec![0u8; 50], FrameKind::FullSync);
        assert_eq!(client.outbox.len(), 3);
        assert_eq!(client.outbox[0].kind, FrameKind::StatePatch);
        assert_eq!(client.outbox[0].offset, 10); // preserved
        assert_eq!(client.outbox[1].kind, FrameKind::Metering);
        assert_eq!(client.outbox[1].offset, 5); // preserved
        assert_eq!(client.outbox[2].kind, FrameKind::FullSync);
    }

    #[test]
    fn drop_policy_control_never_dropped() {
        let mut outbox = VecDeque::new();
        outbox.push_back(make_frame(FrameKind::Control, 0));
        outbox.push_back(make_frame(FrameKind::Control, 0));
        outbox.push_back(make_frame(FrameKind::Control, 0));
        let mut client = make_test_writer(outbox);
        client.queue_frame(vec![0u8; 50], FrameKind::Control);
        // Control frames are never dropped
        assert_eq!(client.outbox.len(), 4);
    }

    #[test]
    fn is_stalled_threshold() {
        let mut outbox = VecDeque::new();
        // Fill to exactly MAX_OUTBOX_DEPTH
        for _ in 0..MAX_OUTBOX_DEPTH {
            outbox.push_back(make_frame(FrameKind::Control, 0));
        }
        let client = make_test_writer(outbox);
        assert!(!client.is_stalled(), "at threshold should not be stalled");

        let mut outbox2 = VecDeque::new();
        for _ in 0..=MAX_OUTBOX_DEPTH {
            outbox2.push_back(make_frame(FrameKind::Control, 0));
        }
        let client2 = make_test_writer(outbox2);
        assert!(client2.is_stalled(), "above threshold should be stalled");
    }
}
