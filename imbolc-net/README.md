# imbolc-net

Network/collaboration layer for Imbolc DAW.

## Summary

- Single audio server, multiple clients.
- Control data over TCP (length-prefixed JSON); audio stays local to the server.
- Server is authoritative; clients send actions and receive state updates.

## Sync Model

- Actions arrive as `NetworkAction` and are validated for ownership/privilege.
- State updates are sent as `StatePatchUpdate` (dirty subsystem patches + per-instrument deltas).
- Patches are rate-limited (~30 Hz) and ordered by a monotonically increasing `seq`.
- Periodic `FullStateSync` (every 30s or on request) heals drift and recovers desyncs.

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

## See Also

- `CLAUDE.md` — architecture and protocol details
- `../plans/network-scenarios.md` — deployment scenarios and assumptions
