# imbolc-net

Network layer for Imbolc multi-client collaboration over LAN.

## Overview

This crate provides client and server components for running shared Imbolc sessions.
The server runs the audio engine and holds the authoritative state; clients send actions
and receive state updates. **Audio never traverses the network** — only control/state data.

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
| `NetworkState` | `protocol.rs` | Canonical state synced from server to clients |
| `StatePatch` | `protocol.rs` | Partial state update (dirty subsystem patches + deltas) |
| `ClientMessage` | `protocol.rs` | Messages sent from client to server |
| `ServerMessage` | `protocol.rs` | Messages sent from server to clients |
| `NetServer` | `server.rs` | Accepts connections, validates actions, broadcasts updates |
| `RemoteDispatcher` | `client.rs` | Connects to server, sends actions, applies updates |
| `DirtyFlags` | `server.rs` | Tracks which subsystems changed since last broadcast |
| `OwnerInfo` | `protocol.rs` | Ownership info for an instrument (client_id + name) |
| `PrivilegeLevel` | `protocol.rs` | Normal or Privileged (transport/save/load control) |
| `SessionToken` | `protocol.rs` | Token for reconnecting to a suspended session |
| `DiscoveryServer` | `discovery.rs` | mDNS advertisement for LAN discovery (requires `mdns`) |
| `DiscoveryClient` | `discovery.rs` | mDNS browser for finding servers (requires `mdns`) |

## Wire Protocol

Length-prefixed JSON over TCP:
```
[u32 length (big-endian)][JSON payload]
```

Messages are defined in `protocol.rs`:
- `ClientMessage::Hello` — handshake (name, ownership request, privilege request, reconnect token)
- `ClientMessage::Action(NetworkAction)` — action to dispatch on server
- `ClientMessage::RequestPrivilege` — request privileged status
- `ClientMessage::RequestFullSync` — request full state resync
- `ClientMessage::Ping` / `ClientMessage::Pong` — keepalive
- `ClientMessage::Goodbye` — clean disconnect
- `ServerMessage::Welcome` — assigns `client_id`, grants ownership, sends initial state
- `ServerMessage::StatePatchUpdate` — partial update (dirty subsystems + deltas)
- `ServerMessage::FullStateSync` — full snapshot fallback or on request
- `ServerMessage::Metering` — real-time metering (~30 Hz)
- `ServerMessage::PrivilegeGranted/Denied/Revoked` — privilege transitions
- `ServerMessage::ActionRejected` — authorization failure
- `ServerMessage::ReconnectSuccessful/Failed` — reconnection results
- `ServerMessage::Ping` / `ServerMessage::Pong` — keepalive
- `ServerMessage::Error` — server-side errors
- `ServerMessage::Shutdown` — graceful shutdown

## Sync Model

- The server maintains a `DirtyFlags` set and increments a monotonically increasing `seq`.
- Patches are rate-limited to ~30 Hz.
- Each patch includes only the dirty subsystems: session remainder (`SessionState`) or granular subsystems (piano roll, arrangement, automation, mixer).
- Instrument updates are either full `InstrumentState` for structural changes or per-instrument deltas for targeted edits.
- If more than half the instruments are dirty or a structural change occurs, the server sends a full `InstrumentState`.
- A `FullStateSync` is sent every 30 seconds or on `RequestFullSync`.
- Clients discard out-of-order patches by tracking `seq`.

## What's Synced vs Local

| Synced (NetworkState) | Client-Local |
|----------------------|--------------|
| SessionState | Clipboard |
| InstrumentState | Undo/Redo history |
| Ownership map | Navigation (active pane) |
| Privileged client info | Layer stack (UI modes) |
| | MIDI connections |

## Ownership & Privilege

- **Ownership**: Each client owns specific instruments (assigned at connection or requested later).
- **Privilege**: A single client has privileged status for transport, session, and bus controls.
- Unauthorized actions are rejected with `ServerMessage::ActionRejected`.

## Heartbeat & Reconnection

- Server sends `Ping` every 5 seconds; clients respond with `Pong`.
- Clients missing 3 heartbeats (~15s) are suspended.
- Suspended sessions are retained for 60 seconds via a `SessionToken`.
- Reconnecting restores ownership and privilege if the token is still valid.

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

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Wire format | JSON | Debuggable; switch to binary later for performance |
| State sync | Dirty-flag patches + periodic full sync | Scales to more instruments while preserving recovery path |
| TCP vs UDP | TCP | Reliability matters; latency budget is generous |
| Reconnect window | 60 seconds | Balance between UX and stale reservations |
| Privilege model | Single privileged | Simple; multi-admin adds complexity |

## Future Work

- Binary serialization (bincode) for lower overhead
- Per-client undo on the server
- VST state sync for binary blobs
- Transport latency compensation
