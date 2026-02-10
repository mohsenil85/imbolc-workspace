# Scaling Analysis

Analysis of scaling bottlenecks in Imbolc's local and network architectures,
ranked by impact. Updated Feb 2026 after field-level state diffing.

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

1. **Automation/arrangement/mixer field-level diffing.** The piano roll and
   instruments already have per-item delta patches, but automation lanes,
   arrangement clips, and mixer buses still send the full subsystem struct on
   any change. Lower priority — these subsystems are typically small. When
   needed, the pattern is proven: add `dirty_<entity>: HashSet<Id>` +
   `<subsystem>_structural: bool` to `DirtyFlags`, add a
   `<subsystem>_<entity>_patches` field to `StatePatch`, and apply threshold
   coalescing at >50%.

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
  session patches.** `DirtyFlags` tracks piano roll, arrangement, automation,
  and mixer independently. `StatePatch` has optional subsystem fields. The
  `session` flag is reserved for rare "remainder" changes (BPM, key,
  registries, undo/redo) that send the full session.

- ~~JSON wire format~~ — **Resolved: binary message format.** Switched from
  JSON (`serde_json`) to bincode for the wire protocol. ~3-5x smaller payloads,
  ~10-50x faster serialization. Changes localized to `framing.rs`
  (`serialize_frame()`, `read_message()`, `write_message()`). Length-prefixed
  framing (`[u32 len][payload]`) unchanged.

- ~~No state catchup on reconnect~~ — **Resolved: reconnect catchup.**
  Server sends `FullStateSync` immediately after `ReconnectSuccessful`
  handshake. Client receives up-to-date state without waiting for the next
  broadcast cycle.

- ~~Field-level state diffing (piano roll / instruments)~~ — **Resolved:
  per-item delta patches.** Both instruments and piano roll tracks use the
  same pattern: `dirty_<items>: HashSet<InstrumentId>` for targeted edits,
  `<subsystem>_structural: bool` for add/delete/metadata changes. `StatePatch`
  carries either the full subsystem or a `HashMap<InstrumentId, T>` of changed
  items (mutually exclusive). Threshold coalescing sends full state when >50%
  of items are dirty. Piano roll actions are matched per-variant: note edits
  resolve the track index to `InstrumentId` via `track_order`, metadata changes
  (loop, time sig, swing) are structural, and audio-only actions (PlayStop,
  ReleaseNote) set no flags at all.

### What's Well-Designed in Net

- No locks (ownership by construction)
- Dedicated writer thread (I/O off main thread)
- Clean reader-thread → mpsc → main-thread → mpsc → writer-thread pipeline
- Suspension preserving ownership for reconnect
- Dirty flags avoiding no-op broadcasts
- Rate-limited patch broadcasting (~30 Hz)
- Per-instrument delta patches with threshold coalescing
- Per-track piano roll delta patches with threshold coalescing
- Serialize-once broadcast
- Per-client outbox with frame-kind drop policy (slow-client isolation)
- Subsystem-level session patches (piano roll, arrangement, automation, mixer)
- Binary wire format (bincode)
- Immediate state catchup on reconnect

---

## Potential Future Work

Low priority — all high-impact issues are resolved. Listed for reference:

### Automation per-lane deltas

`dirty_automation_lanes: HashSet<AutomationLaneId>` + `automation_structural`.
Simpler than piano roll — `AutomationAction` variants already carry lane IDs
directly. Add `automation_lane_patches: Option<HashMap<AutomationLaneId,
AutomationLane>>` to `StatePatch`. Only worth doing if automation lanes grow
large (many points per lane).

### Arrangement per-entity deltas

Two entity types (clips, placements). Lower priority — arrangement state is
typically small.

### Mixer per-bus deltas

`mixer_bus_patches: Option<HashMap<u8, MixerBus>>`. Also typically small
payloads.
