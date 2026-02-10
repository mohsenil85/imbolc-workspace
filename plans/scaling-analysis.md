# Scaling Analysis

Analysis of scaling bottlenecks in Imbolc's local and network architectures,
ranked by impact. Updated Feb 2026 after dedicated writer thread.

---

## Local Scaling Issues

### Resolved

- ~~Full routing rebuild tears down ALL voices~~ — **Resolved: targeted routing
  rebuild.** Add/delete instrument use `routing_add_instrument` /
  `routing_delete_instrument` flags to build/free only the affected instrument's
  chain. Bus/group effect changes use `routing_bus_processing` to rebuild only
  the bus processing section. `routing_instruments: [Option<InstrumentId>; 4]`
  supports up to 4 concurrent single-instrument rebuilds per frame before
  escalating to full rebuild. Scoped undo (`SingleInstrument`) uses targeted
  flags instead of `AudioDirty::all()`.
  See [targeted-routing-rebuild.md](targeted-routing-rebuild.md).

- ~~Full state clone on every undoable action~~ — **Resolved: scoped undo.**
  `UndoScope::SingleInstrument(id)` clones only the affected instrument for
  parameter tweaks. `Session` scope clones only session state. `Full` scope
  (add/remove instrument) still clones both trees but is rare.

- ~~Full state clone sent to audio thread~~ — **Resolved: incremental audio
  forwarding.** `AudioDirty` flags gate what gets sent. Targeted param updates
  (`filter_param`, `effect_param`, etc.) use `/n_set` OSC messages without
  cloning state. Only structural changes trigger full state clone.

- ~~16-voice-per-instrument hard cap~~ — **Resolved: voice cap increased** to
  24, configurable per source type.

- ~~Arrangement flattening~~ — **Resolved: arrangement cache.** Flattened
  notes are cached and only recomputed when clips or placements change.

- ~~Linear instrument lookups~~ — **Resolved: HashMap index.** `InstrumentState`
  now maintains a `HashMap<InstrumentId, usize>` index for O(1) lookups via
  `instrument()` and `instrument_mut()`, with linear-scan fallback.

- ~~Main thread serialization ceiling~~ — **Resolved: six-part optimization.**
  Event polling, dispatch, undo cloning, audio feedback, MIDI, and rendering all
  ran serially in one loop iteration. Addressed with:
  1. **VecDeque undo stacks** — `remove(0)` at max depth was O(n), now O(1)
     `pop_front()`.
  2. **Fix undo escalation** — param tweaks during playback (without automation
     recording) no longer escalate to `Full` scope. Uses precise
     `automation_recording && playing` check instead of `playing` proxy.
  3. **Undo coalescing** — sequential param tweaks within 500ms share a single
     snapshot (`CoalesceKey` by instrument ID or session). A 30-key sweep
     produces 1 undo entry instead of 30.
  4. **Conditional visualization** — spectrum bands, LUFS, and scope buffer
     (~580 floats) only copied from audio thread when waveform pane is active.
  5. **UI dirty tracking** — `render_needed` flag skips all render work (buffer
     allocation, widget drawing, terminal I/O) when idle with no playback.
  6. **Batch event processing** — drains up to 16 events per loop iteration
     (zero-timeout subsequent polls), preventing intermediate renders during
     rapid input. Combined with coalescing, a 10-event param sweep produces 1
     render and 1 undo snapshot instead of 10 of each.

### What's Well-Designed Locally

- Lock-free audio thread
- Priority/normal dual channels
- OSC timetags with schedule-ahead
- Targeted param updates for common operations
- Async persistence
- Arrangement cache
- Binary search on sorted notes
- Scoped undo (single-instrument snapshots)
- Undo coalescing for parameter sweeps
- Batch event processing (up to 16 per frame)
- UI dirty tracking (zero render cost when idle)
- Conditional visualization data copying

---

## Network (imbolc-net) Scaling Issues

### Remaining

1. **JSON wire format**. ~3-5x larger than bincode, ~10-50x slower to
   serialize. Main cost is in `StatePatch` and `FullStateSync` — these carry
   nested structs with many floats (instrument params, automation points, mixer
   levels). Metering is small (playhead + bpm + peaks) so JSON overhead there
   is low in absolute terms, but it's sent at 30Hz so the per-frame cost adds
   up. Serialization still happens on the main thread (writer thread only does
   I/O), so this is now the main-thread bottleneck for network broadcasts.

2. **No state catchup on reconnect**. After `ReconnectSuccessful`, the client
   waits for the next broadcast cycle to receive state. Gap is at most ~33ms
   (one 30Hz cycle) but could be longer if nothing is dirty. Low impact —
   trivial fix (send `FullStateSync` immediately after reconnect handshake).

3. **Field-level state diffing**. Subsystem-level patches already avoid
   sending unrelated subsystems, but within a subsystem the entire struct is
   sent. E.g., adding one note sends the full `PianoRollState`. Impact scales
   with project complexity — a project with 10k notes in the piano roll sends
   all of them for any single-note edit. Would require delta encoding or
   operation-based sync (CRDTs).

### Resolved

- ~~Slow-client poisons the server~~ — **Resolved: per-client outbox with
  drop policy.** `ClientWriter` uses raw `TcpStream` with 10ms `SO_SNDTIMEO`.
  Per-client `VecDeque<QueuedFrame>` outbox with frame-kind drop policy:
  Metering (keep latest), StatePatch/FullSync (supersede older), Control
  (never drop). Writer thread flushes outboxes each iteration. Clients
  exceeding `MAX_OUTBOX_DEPTH` are suspended via feedback channel.

- ~~Per-client JSON re-serialization~~ — **Resolved: serialize-once broadcast.**
  `broadcast()` serializes once on the main thread, sends `Vec<u8>` to the
  writer thread which writes it to each client.

- ~~No action batching or throttling~~ — **Resolved: rate-limited broadcast.**
  `broadcast_state_patch()` is throttled at ~30 Hz. Dirty flags accumulate
  between broadcasts, coalescing rapid-fire edits into a single patch.

- ~~Double full-state clone per server loop iteration~~ — **Resolved: lazy
  state construction.** `accept_connections()` no longer takes a state param.
  `poll_actions()` takes `(&SessionState, &InstrumentState)` references and
  only builds `NetworkState` during Hello handshakes. The broadcast clone is
  guarded by dirty flags.

- ~~Per-instrument dirty tracking~~ — **Resolved: `DirtyFlags` with targeted
  `dirty_instruments: HashSet<InstrumentId>` and `instruments_structural` flag.**
  Single param tweaks send only the affected instrument. Structural changes
  (add/delete/undo) send full `InstrumentState`. Threshold coalescing sends
  full state when >50% of instruments are dirty.

- ~~Single-threaded server loop~~ — **Resolved: dedicated writer thread.**
  All socket writes moved to a dedicated writer thread. The main thread
  serializes frames once, then sends the bytes via MPSC channel. The writer
  thread owns all client write halves (`ClientWriter`), outboxes, and drop
  policy. Stall detection feeds back to the main thread via a feedback channel.
  Handshake writes (Welcome/ReconnectSuccessful) remain on the main thread
  since they target pending streams before client registration.

- ~~Coarse session-level patch granularity~~ — **Resolved: subsystem-level
  session patches.** `DirtyFlags` now tracks `piano_roll`, `arrangement`,
  `automation`, and `mixer` independently. `StatePatch` has 4 new optional
  subsystem fields. Adding a note sends only `PianoRollState`, not the entire
  `SessionState`. The `session` flag is reserved for rare "remainder" changes
  (BPM, key, registries, undo/redo) that send the full session. Backward-
  compatible: old clients deserialize unknown fields as `None`.

### What's Well-Designed in Net

- No locks (ownership by construction)
- Dedicated writer thread (I/O off main thread)
- Clean reader-thread → mpsc → main-thread → mpsc → writer-thread pipeline
- Suspension preserving ownership for reconnect
- Dirty flags avoiding no-op broadcasts
- Rate-limited patch broadcasting (~30 Hz)
- Per-instrument delta patches
- Serialize-once broadcast
- Per-client outbox with frame-kind drop policy (slow-client isolation)
- Subsystem-level session patches (piano roll, arrangement, automation, mixer)

---

## Recommended Next Steps

Prioritized by effort/impact ratio:

### 1. Reconnect state catchup (quick win)

Send a `FullStateSync` immediately after `ReconnectSuccessful` handshake.
Trivial change — a few lines in the reconnect path of `poll_actions()`.
Eliminates the stale-state gap on reconnect.

### 2. Binary wire format (high impact)

Switch from JSON to bincode for the wire protocol. This is the single biggest
remaining optimization because serialization is now the main-thread bottleneck
for network broadcasts (writer thread handles I/O).

**What it buys:**
- ~3-5x smaller payloads (less channel + socket throughput)
- ~10-50x faster serialization (less main-thread time per broadcast)
- Biggest win on `StatePatch` and `FullStateSync` which carry deeply nested
  structs with many floats (JSON float formatting is notoriously slow)

**Approach:**
- Use `bincode` (already serde-based, so all types work immediately)
- Replace `serde_json` in `framing.rs` — the serialize-once + writer-thread
  architecture means changes are localized to `serialize_frame()`,
  `read_message()`, and `write_message()`
- The length-prefixed framing (`[u32 len][payload]`) stays identical
- No protocol version negotiation needed initially — this is a LAN-only
  protocol with no backward-compatibility requirement across versions
- Reader threads also benefit (faster deserialization of `ClientMessage`)

**Complexity:** Low-medium. The framing layer is already cleanly separated.
Main risk is debugging opaque binary payloads vs readable JSON during
development. Consider keeping a JSON debug mode behind a feature flag.

### 3. Field-level state diffing (complex, diminishing returns)

Reduce within-subsystem patch sizes by sending only changed fields or
operations rather than full subsystem snapshots.

**Where it matters most:** Piano roll (can have thousands of notes), automation
(many points per lane), arrangement (many clip placements).

**Where it doesn't matter:** Mixer (small, flat struct), session remainder
(BPM, key — tiny), metering (already minimal).

**Approach options:**
- **Operation-based**: Send the action itself instead of the resulting state
  diff. Clients apply the same action locally. Simple but requires all actions
  to be deterministic and all clients to have identical dispatch logic.
- **Delta encoding**: Diff old vs new state, send only changed fields. More
  robust but requires a diffing library or custom delta types.
- **CRDT**: Conflict-free replicated data types. Overkill for the current
  single-writer-per-subsystem model but future-proofs for concurrent editing.

Not recommended until binary format is in place — JSON overhead currently
dwarfs the redundant-data overhead.
