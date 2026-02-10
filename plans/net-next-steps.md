# Plan: Network Next Steps

Three incremental improvements to imbolc-net, ordered by effort/impact ratio.

---

## 1. Reconnect State Catchup

### Problem

After `ReconnectSuccessful`, the server sends no follow-up state. The client
(`client.rs:135-142`) blocks on `read_message()` expecting a `StateUpdate`
message, but the server never sends one — the client waits until a normal
broadcast cycle produces something. This creates a stale-state gap that can
last from one tick (~2ms if dirty) up to 30 seconds (next periodic full sync
if nothing is dirty).

### Fix

In `server.rs` `poll_actions()`, immediately after writing `ReconnectSuccessful`
to the pending stream (line ~613), build a `NetworkState` and write a
`StateUpdate` to the same stream before passing it to the writer thread.

This mirrors the Welcome handshake path, which already sends initial state on
the pending stream before `WriterCommand::AddClient`.

```
Current:
  write ReconnectSuccessful → AddClient → (client waits...)

After:
  write ReconnectSuccessful → write StateUpdate → AddClient → (client ready)
```

### Changes

| File | Change |
|------|--------|
| `server.rs` | After `ReconnectSuccessful` write, build `NetworkState` from the `session`/`instruments` refs already available in `poll_actions()`, write `StateUpdate { state }` to `pending.stream` |
| `reconnection.rs` | Add assertion: after `ReconnectSuccessful`, next message is `StateUpdate` |

No changes to `client.rs` — it already expects this message.

### Scope

~10 lines of code. One new test assertion.

---

## 2. Binary Wire Format

### Problem

JSON serialization is the main-thread bottleneck for network broadcasts (writer
thread handles I/O). `StatePatch` and `FullStateSync` carry deeply nested
structs with many floats. JSON float formatting is slow, and the text
representation is 3-5x larger than binary. Serialization cost scales with
state complexity and broadcast rate (30 Hz for patches, plus metering).

### Approach

Replace `serde_json` with `bincode` in the framing layer. The length-prefixed
frame format (`[u32 BE length][payload]`) stays identical — only the codec
between header and payload changes.

**Why bincode:**
- All types already derive `Serialize`/`Deserialize`
- `Option<Option<T>>` encodes natively (the custom `double_option` serde
  module in `protocol.rs:148-180` becomes unnecessary)
- Deterministic output (no field ordering surprises)
- Well-maintained, widely used in Rust networking

**Why not MessagePack:** bincode is more compact for typed data (no field name
overhead, no type tags for known schemas). MessagePack's self-describing nature
is wasted here since both sides share the same Rust types.

### Changes

| File | Change |
|------|--------|
| `Cargo.toml` | Add `bincode = "2"`. Keep `serde_json` (used by `session_file.rs` for human-readable persistence) |
| `framing.rs:11` | `write_message()`: `serde_json::to_vec(msg)` → `bincode::serde::encode_to_vec(msg, bincode::config::standard())` |
| `framing.rs:27` | `serialize_frame()`: same swap |
| `framing.rs:62` | `read_message()`: `serde_json::from_slice()` → `bincode::serde::decode_from_slice(&payload, bincode::config::standard())` |
| `framing.rs` tests | Update roundtrip tests. Remove `serialize_frame_matches_write_message` size assertion (binary output differs) |
| `protocol.rs:148-180` | Remove `double_option` module. `Option<Option<T>>` works natively with bincode. Remove `#[serde(with = "double_option")]` from `StatePatch.privileged_client`. Can also remove `skip_serializing_if` annotations (they're no-ops with bincode but harmless) |
| `protocol_roundtrip.rs` | Tests should pass unchanged after removing `double_option` references. Verify `StatePatch` edge cases (all-None, privileged_client None vs Some(None) vs Some(Some(..))) |

### What Stays JSON

`session_file.rs` — human-readable session persistence to disk. Separate
concern from the wire protocol. No reason to change it.

### Risk

**Debugging opaque payloads.** JSON was nice for `tcpdump` inspection. Mitigate
with logging: add a `log::trace!` in `write_message` that logs the message
type (variant name) without serializing the full payload.

**No backward compatibility.** LAN-only protocol, both sides are always the
same binary. No version negotiation needed. If we ever need it, add a 4-byte
magic prefix to the stream (not per-frame — once at connection start).

### Scope

~30 lines changed across 3 files. Test updates are mechanical.

### Verification

1. `cargo test -p imbolc-net` — all tests pass
2. `cargo build -p imbolc-ui --features net` — compiles
3. Manual: server + 2 clients, state sync, metering, reconnect all work

---

## 3. Field-Level State Diffing

### Problem

Subsystem-level patches avoid sending unrelated subsystems, but within a
subsystem the entire struct is sent. Adding one note sends the full
`PianoRollState` (all tracks, all notes). Impact scales with project size —
a project with 10k notes sends all of them for any single-note edit.

### Where It Matters

| Subsystem | Current payload | Grows with | Delta value |
|-----------|----------------|------------|-------------|
| Piano roll | Full `PianoRollState` | Tracks x notes | **High** |
| Arrangement | Full `ArrangementState` | Clips x placements | **High** |
| Automation | Full `AutomationState` | Lanes x points | **Medium** |
| Mixer | Full `MixerState` | Buses x effects | **Low** (small, flat) |
| Instruments | Already has per-instrument deltas | — | Already addressed |

### Approach: Operation-Based Sync

Instead of diffing state snapshots, forward the `NetworkAction` itself to
clients alongside the `StatePatch`. Clients that receive the action before
the state diverges can apply it locally (optimistic), then reconcile with
the authoritative patch when it arrives. Clients that missed the action
(due to drop policy) fall back to the full subsystem patch.

This is a hybrid model:
- **Actions** provide the fine-grained delta (free — already serialized)
- **Patches** provide the authoritative reconciliation (existing mechanism)
- **FullStateSync** provides the periodic safety net (existing mechanism)

### Protocol Changes

```rust
// New: action echo for optimistic apply
ServerMessage::ActionEcho {
    action: NetworkAction,
    origin: ClientId,       // so clients can skip their own echoed actions
    seq: u64,               // for ordering against patches
}
```

The server sends `ActionEcho` immediately when it processes a client action
(before the next rate-limited patch). Clients apply the action optimistically
to their local state copy. When the next `StatePatchUpdate` arrives, clients
compare `seq` to know which actions are already reflected in the patch.

### Subsystem-Level Deltas (Alternative)

If operation-based sync is too complex, a simpler approach for piano roll
and arrangement:

```rust
// Delta variants for StatePatch
pub struct PianoRollDelta {
    pub added_notes: Vec<(InstrumentId, Note)>,
    pub removed_notes: Vec<(InstrumentId, NoteId)>,
    pub modified_notes: Vec<(InstrumentId, Note)>,
    pub metadata_changed: bool,  // bpm, time sig, loop bounds
    pub full: Option<PianoRollState>,  // fallback when delta is larger than full
}
```

The server tracks per-note changes within the dirty window and sends deltas
when they're smaller than the full subsystem state. Falls back to full state
when too many changes accumulate (similar to the existing 50% instrument
threshold).

### Changes

| File | Change |
|------|--------|
| `protocol.rs` | Add `ActionEcho` variant to `ServerMessage`. Or: add delta types for piano roll / arrangement |
| `server.rs` | After `poll_actions()` processes a valid action, broadcast `ActionEcho`. Or: track per-subsystem deltas in `DirtyFlags` |
| `client.rs` | Handle `ActionEcho`: apply action to local state, track seq. Or: handle delta types in patch application |
| `imbolc-types` | If delta approach: add `NoteId`, delta structs |

### Prerequisites

**Do item 2 (binary format) first.** JSON overhead currently dwarfs
redundant-data overhead. With bincode, a `PianoRollState` with 1000 notes is
~40 KB. A single-note delta would be ~100 bytes. The 400x reduction only
matters once serialization cost is proportional to payload size (which JSON
breaks because of float formatting overhead).

### Scope

Medium-large. Operation-based sync is ~200 lines but requires careful seq
tracking. Subsystem deltas are ~300 lines but more mechanical. Recommend
starting with operation-based sync for piano roll only, then expanding.

### Verification

1. `cargo test -p imbolc-net` — all tests pass
2. Benchmark: measure patch sizes before/after with a 1000-note project
3. Manual: rapid note editing with 2 clients, verify no drift
