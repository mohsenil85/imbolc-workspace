#![allow(dead_code)]
//! Test harness utilities for imbolc-net integration tests.

use std::io::{BufReader, BufWriter};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use imbolc_net::framing::{read_message, write_message};
use imbolc_net::protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, ServerMessage, SessionToken,
};
use imbolc_net::server::NetServer;
use imbolc_types::{InstrumentId, InstrumentState, SessionState};

/// Build a test NetworkState from a server.
pub fn make_test_state(server: &NetServer) -> NetworkState {
    NetworkState {
        session: SessionState::new(),
        instruments: InstrumentState::new(),
        ownership: server.build_ownership_map(),
        privileged_client: server.privileged_client_info(),
    }
}

/// Build a test NetworkState with N instruments.
pub fn make_test_state_with_instruments(server: &NetServer, count: u32) -> NetworkState {
    use imbolc_types::SourceType;
    let mut instruments = InstrumentState::new();
    for _i in 0..count {
        instruments.add_instrument(SourceType::Saw);
    }
    NetworkState {
        session: SessionState::new(),
        instruments,
        ownership: server.build_ownership_map(),
        privileged_client: server.privileged_client_info(),
    }
}

/// Drive the server (accept + poll) until the expected client count is reached, or timeout.
pub fn drive_until_clients(
    server: &mut NetServer,
    state: &NetworkState,
    expected: usize,
    timeout: Duration,
) {
    let start = Instant::now();
    while Instant::now().duration_since(start) < timeout {
        server.process_writer_feedback();
        server.accept_connections();
        server.poll_actions(&state.session, &state.instruments);
        if server.client_count() >= expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "Timed out waiting for {} clients (have {})",
        expected,
        server.client_count()
    );
}

/// Drive the server (accept + poll) and return any actions received.
pub fn drive_and_collect_actions(
    server: &mut NetServer,
    state: &NetworkState,
    timeout: Duration,
) -> Vec<(ClientId, NetworkAction)> {
    let start = Instant::now();
    let mut all_actions = Vec::new();
    while Instant::now().duration_since(start) < timeout {
        server.accept_connections();
        let actions = server.poll_actions(&state.session, &state.instruments);
        all_actions.extend(actions);
        if !all_actions.is_empty() {
            return all_actions;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    all_actions
}

/// A raw TCP client for protocol-level tests.
///
/// Because the tests are single-threaded, handshakes must be split:
/// 1. `send_hello()` — sends the Hello message (non-blocking)
/// 2. Drive the server with `drive_until_clients()` — server processes Hello, sends Welcome
/// 3. `recv()` — client receives the Welcome
pub struct RawClient {
    pub reader: BufReader<TcpStream>,
    pub writer: BufWriter<TcpStream>,
}

impl RawClient {
    /// Connect to a server via TCP.
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let read_stream = stream.try_clone()?;
        Ok(Self {
            reader: BufReader::new(read_stream),
            writer: BufWriter::new(stream),
        })
    }

    /// Send a client message.
    pub fn send(&mut self, msg: &ClientMessage) -> std::io::Result<()> {
        write_message(&mut self.writer, msg)
    }

    /// Receive a server message.
    pub fn recv(&mut self) -> std::io::Result<ServerMessage> {
        read_message(&mut self.reader)
    }

    /// Send Hello (without waiting for the response).
    pub fn send_hello(
        &mut self,
        name: &str,
        instruments: Vec<InstrumentId>,
        privilege: bool,
    ) -> std::io::Result<()> {
        self.send(&ClientMessage::Hello {
            client_name: name.to_string(),
            requested_instruments: instruments,
            request_privilege: privilege,
            reconnect_token: None,
        })
    }

    /// Send Hello with reconnect token (without waiting for the response).
    pub fn send_reconnect(&mut self, name: &str, token: SessionToken) -> std::io::Result<()> {
        self.send(&ClientMessage::Hello {
            client_name: name.to_string(),
            requested_instruments: vec![],
            request_privilege: false,
            reconnect_token: Some(token),
        })
    }
}
