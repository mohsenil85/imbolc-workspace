# Scaling Analysis

Analysis of scaling bottlenecks in Imbolc's local and network architectures,
ranked by impact. Updated Feb 2025 after several optimization passes.

---

## Local Scaling Issues

### High Impact

1. **Full routing rebuild tears down ALL voices** (`audio/engine/routing.rs`).
   Adding/removing an instrument or toggling an effect frees every synth node
   across all instruments, then recreates them. With 30 instruments each having
   3 effects, that's ~150+ node creates in one burst — audible dropout. The
   single-instrument rebuild path (`rebuild_single_instrument_routing`) only
   fires in specific cases.

### Medium Impact

2. **Main thread serialization ceiling**. Event polling, dispatch, undo
   cloning, audio feedback, MIDI, and rendering all happen on one thread in one
   loop iteration. Heavy dispatch + complex pane render could approach the 16ms
   frame budget.

### Resolved

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

### What's Well-Designed Locally

- Lock-free audio thread
- Priority/normal dual channels
- OSC timetags with schedule-ahead
- Targeted param updates for common operations
- Async persistence
- Arrangement cache
- Binary search on sorted notes
- Scoped undo (single-instrument snapshots)

---

## Network (imbolc-net) Scaling Issues

### High Impact

1. **Slow-client poisons the server**. `broadcast()` writes sequentially to
   each client on the main thread. If one client's TCP buffer is full,
   `write_all` blocks, stalling updates to all other clients. No outbound
   queue, no dropping stale updates.

2. **Subsystem-level granularity, not field-level**. `StatePatch` tracks dirty
   at the `session`/`instruments` level. Adding one note sends the entire
   `PianoRollState` (all tracks, all notes). Changing one instrument param
   sends all instruments. As projects grow, these payloads go from ~50KB to
   potentially 500KB+. Per-instrument delta patches mitigate the common case
   (single param tweak), but session-level changes remain coarse.

### Medium Impact

3. **JSON wire format**. Already flagged as a known tradeoff. ~3-5x larger
   than bincode, ~10-50x slower to serialize. Metering at 30Hz x N clients
   adds constant overhead.

4. **Single-threaded server loop**. Everything — accept, poll, dispatch,
   serialize, write — on one thread with 2ms sleep. Hard ceiling for client
   count and action throughput.

5. **No state catchup on reconnect**. After `ReconnectSuccessful`, the server
   doesn't push current state immediately — the client waits for the next
   broadcast cycle, leaving a stale-state gap.

### Resolved

- ~~Per-client JSON re-serialization~~ — **Resolved: serialize-once broadcast.**
  `broadcast()` now calls `serialize_frame()` once and writes the pre-serialized
  bytes to each client via `send_raw()`.

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

### What's Well-Designed in Net

- No locks (ownership by construction)
- Clean reader-thread → mpsc → main-thread pipeline
- Suspension preserving ownership for reconnect
- Dirty flags avoiding no-op broadcasts
- Rate-limited patch broadcasting (~30 Hz)
- Per-instrument delta patches
- Serialize-once broadcast

---

## TODO

Items remaining for future optimization work:

1. **Slow-client mitigation** — add per-client outbound queues with backpressure
   or drop-stale-update semantics so one blocked client doesn't stall others.

2. **Field-level state diffing** — reduce session-level patch granularity.
   E.g., send only the changed notes in a track, not the entire PianoRollState.

3. **Binary wire format** — switch from JSON to bincode or MessagePack for
   smaller payloads and faster serialization. May require a protocol version
   negotiation step.
