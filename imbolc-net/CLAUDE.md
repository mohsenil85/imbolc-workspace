# imbolc-net

Network layer for Imbolc multi-client collaboration over LAN.

## Overview

This crate provides client and server components for running networked Imbolc sessions where multiple clients can collaborate on a shared project.

## Architecture

```
                    ┌─────────────────┐
                    │   NetServer     │
                    │  (runs audio)   │
                    └────────┬────────┘
                             │
              TCP (JSON, length-prefixed)
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────┴────┐         ┌────┴────┐         ┌────┴────┐
    │ Client  │         │ Client  │         │ Client  │
    │   UI    │         │   UI    │         │   UI    │
    └─────────┘         └─────────┘         └─────────┘
```

## Key Types

| Type | File | Purpose |
|------|------|---------|
| `NetworkAction` | `protocol.rs` | Serializable subset of `Action` for wire transmission |
| `NetworkState` | `protocol.rs` | State synced from server to clients (session + instruments + ownership) |
| `ClientMessage` | `protocol.rs` | Messages sent from client to server |
| `ServerMessage` | `protocol.rs` | Messages sent from server to clients |
| `NetServer` | `server.rs` | Accepts connections, polls actions, broadcasts state |
| `RemoteDispatcher` | `client.rs` | Connects to server, sends actions, receives updates |
| `OwnerInfo` | `protocol.rs` | Ownership info for an instrument (client_id + name) |
| `PrivilegeLevel` | `protocol.rs` | Normal or Privileged (transport/save/load control) |
| `SessionToken` | `protocol.rs` | Token for reconnecting to a suspended session |
| `DiscoveryServer` | `discovery.rs` | mDNS advertisement for LAN discovery (requires `mdns` feature) |
| `DiscoveryClient` | `discovery.rs` | mDNS browser for finding servers (requires `mdns` feature) |

## Wire Protocol

Length-prefixed JSON over TCP:
```
[u32 length (big-endian)][JSON payload]
```

Messages are defined in `protocol.rs`:
- `ClientMessage::Hello` - Initial handshake with client name, ownership request, privilege request
- `ClientMessage::Action(NetworkAction)` - Action to dispatch on server
- `ClientMessage::RequestPrivilege` - Request privileged status
- `ClientMessage::Goodbye` - Clean disconnect
- `ServerMessage::Welcome { state, privilege, session_token }` - Initial state on connect
- `ServerMessage::StateUpdate { state }` - State update after actions
- `ServerMessage::Metering { playhead, bpm, peaks }` - Real-time metering (~30Hz)
- `ServerMessage::PrivilegeGranted/Denied/Revoked` - Privilege status changes
- `ServerMessage::ReconnectSuccessful/Failed` - Reconnection results

## Usage

Server mode:
```bash
cargo run -p imbolc-ui --features net -- --server
```

Client mode:
```bash
cargo run -p imbolc-ui --features net -- --connect 192.168.1.100:9999
```

Discovery mode (find servers on LAN):
```bash
cargo run -p imbolc-ui --features mdns -- --discover
```

With ownership request:
```bash
cargo run -p imbolc-ui --features net -- --connect 192.168.1.100:9999 --own 1,2,3
```

## Features

| Feature | Description |
|---------|-------------|
| (default) | Base networking without mDNS |
| `mdns` | mDNS/Bonjour discovery for LAN servers |

## What's Synced vs Local

| Synced (NetworkState) | Client-Local |
|----------------------|--------------|
| SessionState | Clipboard |
| InstrumentState | Undo/Redo history |
| Ownership map | Navigation (active pane) |
| Privileged client info | Layer stack (UI modes) |
| | MIDI connections |

## Ownership & Privilege

- **Ownership**: Each client owns specific instruments (assigned at connection or requested later)
- **Privilege**: One client has privileged status to control transport, save, load
- Ownership is displayed in the UI with indicators: `[ME]`, `[Joe]`, etc.
- Privileged actions (transport, save, load, bus control) require privilege status

## Reconnection

- Clients receive a `SessionToken` on connect
- If disconnected, they have 60 seconds to reconnect with their token
- Ownership and privilege are restored on successful reconnection

## Build & Test

```bash
cargo build -p imbolc-net
cargo test -p imbolc-net
cargo build -p imbolc-net --features mdns  # With mDNS discovery
cargo test -p imbolc-net --features mdns
```

## Deployment Scenarios

See [docs/network-scenarios.md](../docs/network-scenarios.md) for detailed deployment scenarios:
- Solo local (laptop only)
- Solo with MIDI/audio interface
- Two laptops (casual jam)
- Pro setup (dedicated server)

Key principle: **Audio never traverses the network.** All audio I/O is cabled to the server's audio interface. Only control data (actions, state) goes over the LAN.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Wire format | JSON | Debuggable; switch to bincode later for performance |
| State sync | Full broadcast | Simple; <100KB on LAN is fine |
| TCP vs UDP | TCP | Reliability matters; latency budget generous |
| Reconnect window | 60 seconds | Balance between UX and stale reservations |
| Privilege model | Single privileged | Simple; multi-admin adds complexity |

## Future Work

- State diffing (delta updates instead of full broadcast)
- Per-client undo on server
- VST state sync (binary blobs)
- Network latency compensation for transport
