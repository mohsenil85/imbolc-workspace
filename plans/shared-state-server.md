# Plan: Shared State Server (Multi-Client DAW)

Multiple imbolc TUI frontends connect over LAN to a shared state server that owns the SuperCollider backend.

## Architecture

```
  Client A (TUI)          Client B (TUI)
  ┌──────────────┐        ┌──────────────┐
  │ Panes render │        │ Panes render │
  │ &AppState    │        │ &AppState    │
  │ (local copy) │        │ (local copy) │
  └──────┬───────┘        └──────┬───────┘
         │ TCP (actions)         │
         v                       v
  ┌─── Server (imbolc --server) ──────────┐
  │  Actions → ownership check → dispatch │
  │  AppState → AudioHandle → SC          │
  │  Registers (shared named buffers)     │
  │  Broadcasts state + metering → clients│
  └───────────────────────────────────────┘
                    │ OSC/UDP
                    v
              SuperCollider
```

**Key principles:**
- Server owns all state and audio. Clients are view+input terminals.
- Panes, rendering, cursor/selection remain client-local.
- Instruments have **ownership** — only the claiming client can modify.
- **Bookmarks** — each client has its own 1-10 number-key→instrument mappings.
- **Registers** provide named shared buffers for exchanging musical data.

## Collaboration Model

### Instrument Ownership

Each instrument can be **claimed** by one client at a time. The owner has exclusive write access.

- **Claim**: Client sends `ClaimInstrument(id)` — server grants if unclaimed
- **Release**: Client sends `ReleaseInstrument(id)` — server marks unclaimed
- **Auto-release on disconnect**: Server releases all of a disconnecting client's instruments
- **Scope**: Ownership covers the instrument and everything downstream — its params, notes on its track, automation lanes targeting its params, effects chain, sends _from_ it
- **Shared resources**: Transport (play/stop/BPM/loop), master mixer, bus configuration remain globally editable by any client (these are session-level, not instrument-level)
- **Creating instruments**: Creator auto-owns. Deleting requires ownership.

Server rejects actions targeting instruments owned by another client (returns an `OwnershipDenied` message).

### Undo/Redo (Per-Client, Scoped to Owned Instruments)

- Server maintains a separate undo stack per client
- Each stack only contains actions that client made on instruments they owned
- Undo reverses your own most recent action — never touches another client's work
- If you release an instrument, your undo entries for it are discarded (you gave up ownership)
- This keeps undo predictable: you can only undo what you did, on things you own

### Per-Client Instrument Bookmarks

Currently `InstrumentState.selected` is a global `Option<usize>` index and number keys 1-10 select by list position. In multi-client, each musician needs independent selection and number-key mappings.

**Client-local state (never sent to server):**
- `selected: Option<InstrumentId>` — which instrument this client is viewing/editing
- `bookmarks: [Option<InstrumentId>; 10]` — maps number keys 1-10 to instrument IDs

**How it works:**
- Pressing `1` → looks up `bookmarks[0]` → gets `InstrumentId` → sets client-local `selected`
- Before rendering, client patches `state.instruments.selected` in its local AppState copy to match
- Panes call `state.instruments.selected_instrument()` unchanged — no pane modifications needed
- Selection is instant (no server round-trip, purely client-local)

**Bookmark management:**
- Auto-bookmark on instrument claim (fills next free slot)
- Manual override: `Ctrl+1` to assign current instrument to slot 1
- Bookmarks persist client-side even after releasing ownership (for easy re-claim)

**Implementation:** `select_instrument()` in `src/global_actions.rs` currently dispatches `Action::Instrument(Select(idx))` which mutates server state. In client mode, this becomes a purely local operation that updates the client's `selected` field and syncs pane state — no network message needed.

### Registers (Shared Named Buffers)

Named storage slots (`a`-`z` or string keys) on the server. Any client can read or write.

**Content types:**
- **Note sequences** — piano roll selections (pitch, time, duration, velocity)
- **Drum patterns** — sequencer steps (hits, accents, per-step velocity)
- **Instrument presets** — full instrument config (source, filter, effects, envelope, LFO, mixer params)
- **Automation curves** — lane segments (control points, curve types)
- **Keybinding snippets** — custom keymap fragments to share workflows

**Actions:**
- `YankToRegister { name: String, content: RegisterContent }` — store data
- `PasteFromRegister { name: String, target: ... }` — retrieve and apply
- `ListRegisters` → server responds with register names + content summaries
- `ClearRegister { name: String }`

**RegisterContent enum:**
```rust
#[derive(Serialize, Deserialize)]
enum RegisterContent {
    Notes(Vec<ClipboardNote>),
    DrumPattern { steps: Vec<DrumStep>, length: u32 },
    InstrumentPreset(Instrument),
    AutomationCurve { points: Vec<AutomationPoint>, curve: CurveType },
    Keybindings(String),  // TOML fragment
}
```

Registers persist in server memory for the session. Could optionally save to disk.

### Import/Export & Preset Library

Multi-client makes import/export a first-class workflow. Each musician has their own preset library on their machine and needs to move instruments/patterns in and out of shared sessions.

**Preset format** (`.imbolc-preset`): Self-contained SQLite file containing:
- Instrument definition (MessagePack blob, same format as persistence)
- Embedded sample data (audio bytes stored as BLOBs, keyed by original filename)
- SynthDef source if custom
- Fully portable — works on any machine without external file dependencies

**Preset library:** `~/.config/imbolc/presets/` on each musician's machine. Browsable from a client-local preset browser pane.

**Import to server:**
1. Client reads `.imbolc-preset` from local disk
2. Client sends `ImportPreset { instrument: Instrument, samples: Vec<(String, Vec<u8>)>, synthdef: Option<String> }` to server
3. Server writes sample files to its local sample directory, updates paths in instrument
4. Server creates instrument, auto-owns to importer, auto-bookmarks
5. Server loads samples into SC buffers

**Export from server:**
1. Client sends `ExportInstrument(InstrumentId)` (must own it)
2. Server gathers the instrument + all referenced sample files + synthdef source
3. Server sends back the full bundle
4. Client writes `.imbolc-preset` to their local preset library

**Session snapshot (local backup):**
1. Client sends `RequestSessionSnapshot`
2. Server sends full `SessionState + InstrumentState` + all sample data
3. Client writes it locally as a `.imbolc` SQLite file using existing persistence code
4. Acts as a fork point — can be opened standalone later

**What's exportable:**
- Instrument chains (source + filter + effects + envelope + LFO + mixer + samples)
- Drum patterns (as part of instrument preset, or standalone via registers)
- Note sequences (via registers)
- Automation curves (via registers)
- Keybinding snippets (via registers)
- Full session (snapshot)

**Relationship to Registers:**
- Registers = ephemeral shared buffers, session-scoped, lightweight (no file I/O)
- Presets = persistent files, disk-scoped, self-contained (embed samples)
- Both carry similar content types but serve different purposes (quick sharing vs. permanent library)

## Protocol

- **TCP with length-prefixed framing** (`[u32 length][msgpack payload]`)
- **Client → Server:** `ClientMessage` (Hello, DispatchAction, ClaimInstrument, ReleaseInstrument, YankToRegister, PasteFromRegister, ListRegisters, ImportPreset, ExportInstrument, RequestSessionSnapshot, Goodbye, Ping)
- **Server → Client:** `ServerMessage` (Welcome, StateUpdate, Metering @30Hz, OwnershipUpdate, RegisterUpdate, OwnershipDenied, PresetData, SessionSnapshot, ClientEvent, Shutdown, Pong)
- **Serialization:** MessagePack via `rmp-serde` (already a dependency)

## Network Action Design

A `NetworkAction` enum — serializable subset of `Action` excluding:
- Nav/Layer variants (`&'static str`, client-local)
- `AudioFeedback` (server-internal, contains non-serializable channels)
- Performance-mode-only variants (client-local state)

All content action sub-enums (InstrumentAction, MixerAction, PianoRollAction, etc.) are already fully serializable except `ClipboardNote` which needs `Serialize, Deserialize` added.

## State Sync Strategy

- **On connect:** Full `SessionState + InstrumentState` snapshot + ownership map + register list
- **On action:** Full snapshot of changed sub-states (typically <100KB, fine for LAN)
- **Metering:** Separate lightweight message at 30Hz (playhead, peaks, spectrum, BPM — ~100 bytes)
- **Ownership changes:** Broadcast `OwnershipUpdate { instrument_id, owner: Option<ClientId> }` to all clients
- **Register changes:** Broadcast `RegisterUpdate { name, content_summary }` to all clients
- **Delta sync deferred** to a future phase

## New Modules

```
imbolc-core/src/net/
  mod.rs          — re-exports
  protocol.rs     — ClientMessage, ServerMessage, NetworkAction, MeteringUpdate,
                    RegisterContent, OwnershipMap
  framing.rs      — length-prefixed read/write over TcpStream
  server.rs       — ImbolcServer (listener, dispatch loop, ownership, registers, broadcast)
  client.rs       — ImbolcClient (connect, send actions, receive state)
  ownership.rs    — OwnershipManager (claim/release/check, per-client undo stacks)
  registers.rs    — RegisterStore (named buffers, content types)
```

## Files to Modify

| File | Change |
|------|--------|
| `imbolc-core/src/action.rs` | Add `Serialize, Deserialize` to all action sub-enums |
| `imbolc-core/src/state/piano_roll.rs` | Add `Serialize, Deserialize` to `ClipboardNote` |
| `imbolc-core/src/state/instrument.rs` | Ownership field or tracked externally |
| `imbolc-core/Cargo.toml` | No new deps needed |
| `src/main.rs` | CLI arg parsing (`--server`/`--connect`), branch into run modes |

## Implementation (Single Pass)

Server is **headless** — logs to stdout, no TUI. Can run in tmux/background.

### Step 1: Serializable Actions + Protocol Types
- Add `Serialize, Deserialize` to all action sub-enums in `action.rs`
- Add `Serialize, Deserialize` to `ClipboardNote`
- Create `imbolc-core/src/net/` with all module files
- Define `NetworkAction`, `ClientMessage`, `ServerMessage`, `MeteringUpdate`, `RegisterContent`
- Implement `NetworkAction::into_action()` and `NetworkAction::from_action()`
- Implement `framing.rs` (length-prefixed read/write)
- Write round-trip serialization tests

### Step 2: Ownership + Registers
- `OwnershipManager`: tracks `HashMap<InstrumentId, ClientId>`, per-client undo stacks
- Ownership check gate in server dispatch loop (reject actions on unowned instruments)
- `RegisterStore`: `HashMap<String, RegisterContent>`, yank/paste/list/clear
- Unit tests for ownership claim/release/disconnect and register CRUD

### Step 3: Server Mode
- CLI arg parsing in `main.rs` (no clap — just `std::env::args()`)
- `ImbolcServer` in `net/server.rs`:
  - TCP listener on background thread, accepts connections
  - Per-client reader thread → MPSC channel
  - Headless main loop: drain messages → ownership check → `dispatch_action()` → broadcast state
  - Register operations handled inline
  - Metering broadcast at 30Hz from `AudioMonitor`
  - Per-client undo stacks: Undo/Redo pulls from the requesting client's stack
- Server reuses `dispatch_action()` and `AudioHandle` unchanged

### Step 4: Client Mode
- `ImbolcClient` in `net/client.rs`: connect, Hello/Welcome handshake
- `client_run()` in `main.rs`:
  - Replaces `dispatch_action()` with `client.send_action()`
  - No local AudioHandle — panes render against state copy updated from network
  - Nav/Layer/selection/cursor remain client-local
  - Client-local action filtering before network send
  - Receives and displays ownership state (highlight which instruments you own)
  - Register UI: yank selections to named registers, paste from them
- Graceful disconnect handling (display message, exit)

### Step 5: Import/Export & Presets
- Preset file format: SQLite with instrument blob + embedded sample BLOBs
- `ImportPreset` message: client reads local `.imbolc-preset`, sends instrument + samples to server
- `ExportInstrument` message: server bundles instrument + samples, sends to client
- `RequestSessionSnapshot`: server sends full state, client writes local `.imbolc` file
- Preset browser: client-local pane that lists `~/.config/imbolc/presets/`

### Future (Deferred)
- Connection status + client list indicator in Frame
- Visual ownership indicators (color-code instruments by owner)
- Delta sync (instrument-level deltas, sequence numbers)
- Client MIDI → NetworkAction translation
- mDNS service discovery
- Register persistence to disk

## Key Design Decisions

1. **No optimistic updates** — Input polls at 2ms, render at 16ms (60fps). LAN round-trip (~2-4ms) fits within one render frame, so network dispatch feels identical to local dispatch
2. **Server serializes all actions** through one thread (same as current single-threaded dispatch)
3. **Instrument ownership prevents conflicts** — no need for merge resolution
4. **Per-client undo stacks** — undo only touches your own work on your instruments
5. **Registers are explicitly shared** — no implicit clipboard sharing, always intentional
6. **Transport is shared** — play/stop/BPM/loop editable by anyone (session-level)
7. **No auth** — LAN tool for trusted collaborators
8. **No new dependencies** — stdlib TCP + existing rmp-serde

## Risks

| Risk | Mitigation |
|------|------------|
| `&'static str` in Action | NetworkAction excludes Nav/Layer variants (client-local) |
| AudioFeedback not serializable | Excluded from NetworkAction (server-internal) |
| Node ID conflicts | Only server talks to SC — no conflict possible |
| File paths are client-local | Save/Load/Import operate on server filesystem |
| MIDI on client machines | Deferred; future: translate MIDI → NetworkAction |
| Live playing latency | LAN <5ms acceptable; local monitoring synth if needed later |
| Ownership contention | First-come-first-served; UI shows who owns what |
| Undo after releasing instrument | Entries discarded on release — clean slate |

## Verification

1. **Serialization:** `cargo test` — round-trip tests for every NetworkAction variant, RegisterContent variant
2. **Ownership:** Unit tests — claim/release/reject/disconnect scenarios
3. **Server:** Start `imbolc --server`, verify SC boots, connect via raw TCP, send Hello, receive Welcome with state + empty ownership map
4. **Client:** Start server on machine A, `imbolc --connect <A>` on machine B — UI renders, claim an instrument, make changes, verify state propagates
5. **Multi-client:** Two clients — claim different instruments, verify independent editing, shared transport, register yank/paste between clients
6. **Undo:** Client A edits instrument 1, Client B edits instrument 2, each undoes independently without affecting the other
