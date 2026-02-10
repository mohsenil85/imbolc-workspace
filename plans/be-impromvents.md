# Backend Redesign: Lessons Learned & Fresh Approach

**Status:** FUTURE
**Last Updated:** 2025-02-06

This isn't an implementation plan — it's a retrospective analysis of
what we'd change about the audio backend architecture if starting from
scratch, informed by the lessons learned from building the current
system.

---

## What Works Well (Keep These)

### MPSC channel + dedicated audio thread
The thread separation plan (`separate-dispatch-audio-thread.md`) was
well executed across 5 phases. The pattern is correct: UI thread owns
state, audio thread owns the engine, MPSC channels bridge them. Keep
this.

### Action dispatch pattern
Panes never mutate state directly. All mutations flow through
`dispatch_action()`. This is the right architecture for a stateful TUI
app. Keep this.

### OSC bundling for atomicity
Using timestamped OSC bundles for multi-operation commands (voice
spawn, recording start) prevents race conditions. This is required by
SuperCollider's model and is done correctly.

### Group-based execution ordering
The 4-group model (Sources=100, Processing=200, Output=300,
Record=400) maps cleanly to SuperCollider's execution model. This is
the right abstraction level.

---

## The Big Lessons & What We'd Change

### 1. Incremental Graph Updates Instead of Full Rebuild

**The problem:** `rebuild_instrument_routing()` is a 622-line monolith
that tears down every synth node and recreates them all whenever
anything changes — add an effect, toggle a filter, change a source
type. This is the single biggest architectural bottleneck.

**What we'd do instead:**

Model the signal chain as an explicit **directed audio graph** with
typed node slots:

```rust
struct SignalChain {
    source: Slot<SourceNode>,
    lfo: Slot<LfoNode>,
    filter: Slot<FilterNode>,
    eq: Slot<EqNode>,
    effects: Vec<Slot<EffectNode>>,
    output: Slot<OutputNode>,
}

enum SlotChange<T> {
    Added(T),
    Removed,
    ParamChanged { node_id: i32, param: &str, value: f32 },
    Replaced(T),  // e.g. source type changed
    Unchanged,
}
```

Each `Slot` tracks whether its content changed since the last
sync. When we "flush" to SuperCollider, we compute a diff:

- **Added** slot → `/s_new` (create synth, wire buses)
- **Removed** slot → `/n_free`
- **ParamChanged** → `/n_set` (just update the param)
- **Replaced** → `/n_free` old + `/s_new` new
- **Unchanged** → skip

The key insight: **most user actions only change one slot**. Adding a
delay effect doesn't need to recreate the source, filter, EQ, or
output. Changing filter cutoff doesn't need to recreate anything —
just `/n_set`.

This eliminates the "nuke and rebuild" pattern and makes per-parameter
changes essentially free.

**Bus allocation becomes stable** — buses are allocated when a slot is
first populated and freed when it's removed. No reset-and-reallocate
cycle.

### 2. Typed State Diffs Instead of Full Clones

**The problem:** `AudioCmd::UpdateState` clones the entire
`InstrumentState` + `SessionState` across the channel. The dirty flag
system (`AudioDirty`) was a Phase 4 bolt-on that partially mitigates
this, but it's coarse-grained (6 boolean flags for the whole state).

**What we'd do instead:**

Design the state→audio sync as a **diff protocol** from the start:

```rust
enum AudioDiff {
    // Instrument-level
    InstrumentAdded(InstrumentId, InstrumentSpec),
    InstrumentRemoved(InstrumentId),

    // Slot-level
    SourceChanged(InstrumentId, SourceSpec),
    FilterChanged(InstrumentId, Option<FilterSpec>),
    EffectAdded(InstrumentId, usize, EffectSpec),
    EffectRemoved(InstrumentId, usize),
    EffectToggled(InstrumentId, usize, bool),
    LfoChanged(InstrumentId, LfoSpec),

    // Parameter-level (most common)
    ParamSet(InstrumentId, ParamTarget, f32),
    MixerSet(InstrumentId, MixerParams),
    MasterSet(MasterParams),

    // Routing
    OutputTargetChanged(InstrumentId, OutputTarget),
    SendChanged(InstrumentId, u8, f32),

    // Playback data
    NotesUpdated(Vec<TrackSnapshot>),
    AutomationUpdated(Vec<AutomationLane>),
}
```

The dispatch layer produces diffs as a natural byproduct of mutation —
when you call `instrument.filter.cutoff.set(0.5)`, it emits
`ParamSet(id, ParamTarget::FilterCutoff, 0.5)`. The audio thread
applies diffs incrementally to its local state *and* to the engine.

This eliminates the `InstrumentSnapshot` type alias hack (which is
just `InstrumentState` — not a real snapshot) and makes the channel
traffic proportional to what actually changed.

### 3. Backend-Agnostic Audio Trait

**The problem:** The `AudioEngine` is deeply coupled to
SuperCollider's specific model — OSC messages, node IDs, group
ordering, control buses, buffer numbers. This makes testing hard
(NullOscClient is a workaround) and locks us to one backend forever.

**What we'd do instead:**

Define the audio operations as a trait:

```rust
trait AudioBackend {
    type NodeId: Copy;
    type BusId: Copy;
    type BufferId: Copy;

    fn create_synth(&mut self, def: &str, group: Group, params: &[(&str, f32)]) -> Result<Self::NodeId>;
    fn free_node(&mut self, id: Self::NodeId) -> Result<()>;
    fn set_param(&mut self, id: Self::NodeId, param: &str, value: f32) -> Result<()>;
    fn set_params(&mut self, id: Self::NodeId, params: &[(&str, f32)]) -> Result<()>;

    fn alloc_audio_bus(&mut self) -> Self::BusId;
    fn free_audio_bus(&mut self, id: Self::BusId);

    fn load_buffer(&mut self, path: &Path) -> Result<Self::BufferId>;
    fn free_buffer(&mut self, id: Self::BufferId) -> Result<()>;

    fn send_midi(&mut self, node: Self::NodeId, status: u8, data1: u8, data2: u8) -> Result<()>;

    fn start_recording(&mut self, bus: Self::BusId, path: &Path) -> Result<()>;
    fn stop_recording(&mut self) -> Result<()>;
}
```

`SuperColliderBackend` implements this with the current OSC logic. A
`TestBackend` records all calls for assertion. A hypothetical
`NativeBackend` could use `cpal` + Rust DSP crates directly.

The routing module speaks to `dyn AudioBackend` instead of building
raw OSC messages. This is a natural seam that the current code almost
has — `OscClientLike` is close but too low-level (it's an OSC
transport abstraction, not an audio operation abstraction).

### 4. Declarative Instrument Specs

**The problem:** The current `Instrument` struct mixes concerns — it
stores the instrument definition (source type, effects chain, filter
config) *and* the runtime mixer state (level, pan, mute, solo) *and*
routing (output target, sends) *and* persistence metadata (VST state
paths, sampler configs). The routing code must manually interpret all
of this.

**What we'd do instead:**

Separate the instrument **definition** from its **runtime state**:

```rust
/// What the instrument IS — serializable, diffable, pure data
struct InstrumentDef {
    source: SourceDef,
    filter: Option<FilterDef>,
    eq: Option<EqDef>,
    effects: Vec<EffectDef>,
    lfo: Option<LfoDef>,
    envelope: EnvDef,
}

/// How the instrument is configured RIGHT NOW — knobs, routing, mixer
struct InstrumentRuntime {
    level: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    output: OutputTarget,
    sends: Vec<(BusId, f32)>,
}

/// The full instrument combines both
struct Instrument {
    id: InstrumentId,
    name: String,
    def: InstrumentDef,
    runtime: InstrumentRuntime,
}
```

The routing layer takes an `InstrumentDef` and "compiles" it into a
signal chain spec, which is then realized on the backend. When only
`runtime` changes, we know we only need `/n_set` calls, not a graph
rebuild.

### 5. Effect Indexing Done Right

**The problem:** Effects are stored as `Vec<EffectSlot>` where slots
can be enabled/disabled. The audio engine only creates nodes for
*enabled* effects. Automation uses the full-array index, but the
engine uses the enabled-only index. The conversion is a manual loop
that's error-prone:

```rust
let enabled_idx = instrument.effects.iter()
    .take(*effect_idx)
    .filter(|e| e.enabled)
    .count();
```

**What we'd do instead:**

Two approaches, either of which eliminates the mapping:

**Option A: Effect IDs.** Give each effect slot a stable ID (like
`InstrumentId`). The engine maps `EffectId → NodeId`. Automation
targets `EffectId`, not array index. No index translation needed.

**Option B: Separate enabled tracking.** The audio graph only knows
about the effects that exist in it. The "enabled" flag is a UI concept
that means "don't include in graph." The diff system handles this:
toggling an effect emits `EffectAdded` or `EffectRemoved`. The array
index *in the graph* is always dense.

Option A is simpler and more robust.

### 6. Explicit Authority for Shared State

**The problem:** Some state is written by both the UI thread and the
audio thread:
- **Playhead** — UI resets it, audio thread advances it
- **BPM** — UI sets it, automation can modulate it
- **Recording state** — UI starts/stops, audio thread tracks progress

The current design sends feedback back
(`AudioFeedback::PlayheadPosition`) which overwrites UI state. This
works but the ownership is implicit.

**What we'd do instead:**

Make authority explicit in the type system:

```rust
/// State owned by the audio thread, read-only from UI
struct AudioOwned {
    playhead: u32,
    is_recording: bool,
    recording_elapsed: Duration,
    server_status: ServerStatus,
}

/// State owned by the UI thread, sent to audio via commands
struct UiOwned {
    instruments: InstrumentState,
    session: SessionState,
    // ...
}
```

The audio thread publishes `AudioOwned` via a single
`Arc<AtomicCell<AudioOwned>>` or a dedicated feedback channel. The UI
thread reads it but never writes to it. Conversely, the UI thread
never reads back its own state from the audio thread — it just reads
`AudioOwned` for display purposes.

This eliminates the subtle feedback-overwrites-state pattern in
`dispatch/audio_feedback.rs`.

### 7. Persistence as Serialization, Not Manual SQL Mapping

**The problem:** Adding a new field to `Instrument` requires changes
in 5+ places: the struct, dispatch handlers, save function, load
function, schema, and snapshot. There are 50+ specialized save/load
functions. Phase 5C split `instrument.rs` into sub-modules, but the
persistence coupling remains.

**What we'd do instead:**

Use a structured serialization format as the primary persistence
layer:

```rust
#[derive(Serialize, Deserialize)]
struct ProjectFile {
    version: u32,
    session: SessionState,
    instruments: Vec<Instrument>,
}
```

Store the project as a single serialized blob (MessagePack, bincode,
or even JSON) inside a SQLite table, with a version field for
migration. Add new fields with `#[serde(default)]` — no schema
migration needed for additive changes.

For the cases where SQLite's queryability matters (e.g., listing
projects without loading them), store metadata separately:

```sql
CREATE TABLE project (
    id INTEGER PRIMARY KEY,
    name TEXT,
    bpm REAL,
    created_at TEXT,
    data BLOB  -- serialized ProjectFile
);
```

This reduces the 50+ save/load functions to `serde::serialize` +
`serde::deserialize` plus a migration function for breaking changes.

### 8. Voice Allocation as a Proper Subsystem

**The problem:** Voice management is split across `voices.rs`
(spawning, OSC bundle construction), `audio_thread.rs` (tracking,
release), and `playback.rs` (scheduling). Voice stealing is simple
oldest-first. Control bus allocation is per-voice with no pooling.

**What we'd do instead:**

A dedicated `VoiceAllocator` that owns the full voice lifecycle:

```rust
struct VoiceAllocator {
    pools: HashMap<InstrumentId, VoicePool>,
    max_voices_per_instrument: usize,
    control_bus_pool: BusPool,  // reuse freed control buses
}

struct VoicePool {
    active: Vec<Voice>,
    stealing_strategy: StealingStrategy,
}

enum StealingStrategy {
    Oldest,
    LowestVelocity,
    FurthestFromLastNote,
}
```

The allocator handles:
- Bus allocation from a pool (reuse instead of always allocating
  fresh)
- Voice stealing with configurable strategy
- Batch operations (release all, count active)
- Clean separation from the OSC transport layer

### 9. Monitor Data Without Arc<Mutex>

**The problem:** Meter levels, waveforms, spectrum data, and LUFS
measurements flow from SuperCollider's OSC replies through
`Arc<Mutex<T>>` shared between the OSC receive thread and the main
thread. This works but involves contention on every frame render.

**What we'd do instead:**

Use lock-free ring buffers or triple buffering for monitor data:

```rust
struct MonitorData {
    peaks: AtomicF32Pair,  // or triple-buffer
    spectrum: TripleBuffer<[f32; 7]>,
    scope: SpscRingBuffer<f32>,
    per_instrument_levels: DashMap<InstrumentId, AtomicF32>,
}
```

The OSC receive thread writes to the "back" buffer. The UI thread
reads from the "front" buffer. No contention, no mutex. This matters
because monitor data updates at audio rate (potentially thousands of
times per second) while the UI reads at 60fps.

### 10. Error Propagation Instead of Silent Failures

**The problem:** Most OSC operations are fire-and-forget. If a synth
node doesn't exist, `/n_set` silently fails. If `scsynth` crashes
mid-session, stale node IDs accumulate. The only health check is
`try_wait()` on the child process.

**What we'd do instead:**

Implement a **node registry with health tracking**:

```rust
struct NodeRegistry {
    live_nodes: HashSet<NodeId>,
    expected_replies: HashMap<RequestId, Instant>,
}
```

- Track which nodes are believed to be alive
- Send periodic `/status` pings to scsynth
- On `/fail` or `/done` replies, update the registry
- If a node operation fails, flag it for rebuild rather than silently
  continuing
- On scsynth crash detection, invalidate the entire registry and
  trigger a clean rebuild

This doesn't need to be perfect — even a best-effort registry catches
the majority of silent failures.

---

## Architecture Summary: If Starting Over

```
UI Thread                          Audio Thread
─────────                          ────────────
AppState                           AudioOwned (playhead, meters, status)
  │                                     ▲
  │ dispatch_action()                   │ publish via atomic/channel
  │ produces AudioDiff[]                │
  ▼                                     │
AudioDiff channel ──────────────► DiffApplier
                                    │
                                    ├── updates local InstrumentDef cache
                                    ├── computes SignalChain diff
                                    └── applies to AudioBackend trait
                                            │
                                            ▼
                                    SuperColliderBackend (OSC)
                                    or TestBackend (recorded calls)
                                    or NativeBackend (cpal + DSP)
```

**Key properties:**
- State flows one direction (UI → Audio) as typed diffs, not full
  clones
- Authority is explicit: UI owns definitions, audio owns
  playhead/meters
- Graph changes are incremental, not nuke-and-rebuild
- Backend is abstracted behind a trait
- Persistence is serialization, not hand-rolled SQL mapping
- Voice allocation is a proper subsystem with pooling
- Monitor data uses lock-free primitives

---

## What Would NOT Change

- Rust as the language (ownership model is perfect for audio)
- SuperCollider as the primary DSP engine (battle-tested, huge synth
  library)
- ratatui for TUI (it works well)
- OSC as the transport to scsynth (it's the only option)
- The action dispatch pattern (clean, testable, auditable)
- The MPSC channel model for thread communication
- SQLite as the persistence container (just change how we use it)
- The group execution order model (Sources → Processing → Output →
  Record)

---

## Pragmatic vs. Ideal

The current backend **works**. It ships, it plays audio, it records,
it exports. Many of these changes are the kind of thing you only know
to do after building the first version. The most impactful changes to
retrofit would be (in order):

1. **Incremental graph updates** (#1) — eliminates the biggest
   performance cliff
2. **Effect IDs** (#5) — small change, eliminates a class of bugs
3. **Serde persistence** (#7) — eliminates the most maintenance burden
4. **State diff protocol** (#2) — reduces channel traffic, enables #1
5. **Backend trait** (#3) — enables proper testing, future flexibility

The rest (explicit authority, voice allocator, lock-free monitors,
error propagation) are quality-of-life improvements that matter more
as the system grows.

---

## Retrofit Plan: Practical Phases

Based on feasibility analysis of the current codebase, here's the
concrete incremental retrofit order. Each phase is self-contained and
shippable.

### Phase A: Effect IDs (Prerequisite, Low Risk)

**Why first:** Small change, fixes a real bug class (effect
enabled-index mapping), and is a prerequisite for incremental routing.

**Changes:**

1. Add `EffectId` type (u32 counter) to `imbolc-core/src/state/instrument/effect.rs`
2. Add `id: EffectId` field to `EffectSlot`, assign on creation via a
   counter on `Instrument` (like `InstrumentId`)
3. Change `AutomationTarget::EffectParam(InstrumentId, usize, usize)`
   → `EffectParam(InstrumentId, EffectId, usize)` in
   `imbolc-core/src/state/automation.rs`
4. Change `InstrumentNodes.effects: Vec<i32>` →
   `HashMap<EffectId, i32>` in `engine/mod.rs`
5. Replace the 3 manual enabled-index translation sites:
   - `engine/automation.rs:48-51`
   - `audio_thread.rs:426-436`
   - `engine/routing.rs:514-532`
   Each becomes: `nodes.effects.get(&effect_id)`
6. Update persistence: save/load `EffectId` alongside effects
7. Update dispatch: `ToggleEffect`, `MoveEffect`, `RemoveEffect` use
   `EffectId` instead of array index

**Files:** `effect.rs`, `automation.rs`, `engine/mod.rs`,
`engine/automation.rs`, `engine/routing.rs`, `audio_thread.rs`,
`dispatch/instrument.rs`, persistence save/load for effects

**Verification:** `cargo test`, manual: add effects, toggle
enabled/disabled, record automation targeting an effect, save/load

---

### Phase B: Per-Instrument Routing Rebuild (Biggest Win)

**Why second:** Eliminates the nuke-and-rebuild pattern. Most user
actions (add effect, toggle filter, change source) only affect one
instrument.

**Changes:**

1. Add `rebuild_single_instrument(&mut self, id: InstrumentId, state,
   session)` to `engine/routing.rs`:
   - Free only that instrument's nodes from `node_map[id]`
   - Free only that instrument's sends from `send_node_map`
   - Recreate signal chain for that one instrument
   - Recreate sends for that one instrument
   - Preserve bus allocator state (don't reset)

2. Fix `send_node_map` key from `(instrument_idx, bus_id)` →
   `(InstrumentId, bus_id)` — currently position-dependent

3. Make `bus_allocator` persistent across single-instrument rebuilds:
   - Only `reset()` on full rebuild (instrument add/delete) or bus
     config changes
   - Single-instrument rebuild reuses existing bus allocations

4. Add `AudioCmd::RebuildInstrumentRouting(InstrumentId)` to
   `commands.rs`

5. Split `AudioDirty.routing` into:
   - `routing_full: bool` (instrument add/delete, bus changes)
   - `routing_instrument: Option<InstrumentId>` (single instrument
     changed)

6. Update `flush_dirty()` to send targeted rebuild when possible

**Files:** `engine/routing.rs`, `engine/mod.rs`, `commands.rs`,
`handle.rs`, `audio_thread.rs`, `action.rs` (AudioDirty),
`dispatch/instrument.rs`

**Verification:** `cargo test`, manual: add/remove effects during
playback — audio should not glitch on other instruments. Toggle
filter, change source type — only affected instrument rebuilds.

---

### Phase C: Serde Persistence (Maintenance Win)

**Why third:** Independent of A and B. Eliminates the 50+ save/load
function maintenance burden. Every new field becomes `#[serde(default)]`
instead of schema + save + load changes.

**Changes:**

1. Add `#[derive(Serialize, Deserialize)]` to all state types:
   - `Instrument`, `EffectSlot`, `FilterConfig`, `LfoConfig`,
     `EnvConfig`, `SourceType`, etc.
   - `SessionState`, `PianoRollState`, `AutomationState`
   - Use `#[serde(default)]` for optional/new fields

2. Create `ProjectFile` wrapper:
   ```rust
   #[derive(Serialize, Deserialize)]
   struct ProjectFile {
       version: u32,
       session: SessionState,
       instruments: InstrumentState,
   }
   ```

3. New save: serialize `ProjectFile` to MessagePack/bincode, store as
   BLOB in SQLite with metadata (name, bpm, created_at) in separate
   columns for browsing

4. New load: deserialize blob, apply `#[serde(default)]` for missing
   fields

5. Migration: detect old schema (multiple tables) vs new (single
   blob). Old projects load via the existing load functions, then
   re-save in new format on next save.

6. Keep old load functions alive behind a `legacy_load` module until
   all users have migrated. Delete after one release cycle.

**Files:** All state types (add derive macros), new
`persistence/serde_format.rs`, update `persistence/mod.rs`,
add `serde` + `rmp-serde` (or `bincode`) to `Cargo.toml`

**Verification:** `cargo test`, manual: load old project → save →
load again (round-trip). Create new project, save, load. Verify all
fields preserved.

---

### Phase D: Targeted State Diffs for Common Operations (Optional)

**Why fourth:** Builds on Phase B. Further reduces channel traffic for
the most common operations (parameter tweaks, mixer changes). The
incremental mixer diff from `phase4-incremental-mixer-diffs.md` was
already partially done — this extends the pattern.

**Changes:**

1. Add more targeted `AudioCmd` variants:
   - `SetFilterParam(InstrumentId, param, value)` — direct `/n_set`,
     no rebuild
   - `SetEffectParam(InstrumentId, EffectId, param, value)` — direct
     `/n_set`
   - `SetLfoParam(InstrumentId, param, value)` — direct `/n_set`

2. For each, the audio thread applies `/n_set` to the known node ID
   from `InstrumentNodes` (or the `HashMap<EffectId, i32>` from Phase A)

3. Add `AudioDirty` variants for these:
   - `filter_param: Option<(InstrumentId, String, f32)>`
   - `effect_param: Option<(InstrumentId, EffectId, String, f32)>`

4. `flush_dirty()` sends targeted commands when only params changed,
   falls back to full state + rebuild when structure changed

**Files:** `commands.rs`, `handle.rs`, `audio_thread.rs`, `action.rs`,
`dispatch/instrument.rs`

**Verification:** `cargo test`, manual: rapidly adjust filter cutoff
during playback — no audio interruption. Compare behavior with
Phase B routing rebuild (should be smoother).

---

### Phase E: Backend Trait Extraction (Testing Win, Optional)

**Why last:** Largest surface area, but enables proper unit testing of
routing logic without SuperCollider running. Can be done in parallel
with other phases.

**Changes:**

1. Define `AudioBackend` trait in `engine/backend.rs`
2. Implement `ScBackend` wrapping `OscClient` (extract from
   current engine methods)
3. Implement `TestBackend` that records all operations
4. Refactor `AudioEngine` to be generic over `B: AudioBackend`
5. Write routing unit tests using `TestBackend`

**Files:** New `engine/backend.rs`, refactor `engine/mod.rs`,
`engine/routing.rs`, `engine/voices.rs`, new test files

**Verification:** `cargo test` — new tests verify routing produces
expected backend calls. Existing behavior unchanged.

---

### Dependency Graph

```
Phase A (Effect IDs)
    │
    ▼
Phase B (Per-Instrument Routing)
    │
    ▼
Phase D (Targeted Diffs) ←── optional extension

Phase C (Serde Persistence) ←── independent, can run in parallel

Phase E (Backend Trait) ←── independent, can run in parallel
```

Phases A → B are the critical path.
Phases C and E can be done at any time independently.

---

### Phase F: Voice Allocator Extraction (Cleanup Win)

**Why:** Voice management is scattered across `voices.rs` (spawning,
OSC bundle construction), `audio_thread.rs` (tracking, release), and
`playback.rs` (scheduling). Extracting a dedicated struct consolidates
the lifecycle and enables future improvements (pooling, configurable
stealing).

**Changes:**

1. Create `imbolc-core/src/audio/engine/voice_allocator.rs`:
   ```rust
   pub struct VoiceAllocator {
       chains: Vec<VoiceChain>,
       next_voice_audio_bus: i32,
       next_voice_control_bus: i32,
       max_voices_per_instrument: usize,
       control_bus_pool: Vec<i32>,  // freed buses available for reuse
   }
   ```

2. Move into `VoiceAllocator`:
   - `voice_chains` field from `AudioEngine` (mod.rs:106)
   - `next_voice_audio_bus`, `next_voice_control_bus` (mod.rs:107-108)
   - `MAX_VOICES_PER_INSTRUMENT` const (mod.rs:43)
   - `steal_voice_if_needed()` / `steal_score()` (voices.rs:561-642)
   - `cleanup_expired_voices()` logic
   - Control bus triple allocation (voices.rs:47-53)

3. Add bus pooling — when a voice is released and cleaned up, return
   its 3 control buses to a pool. Next `spawn_voice()` checks the pool
   before allocating fresh. Eliminates unbounded control bus growth.

4. `AudioEngine` keeps a `voice_allocator: VoiceAllocator` field.
   `spawn_voice()` calls `self.voice_allocator.allocate(instrument_id)`
   to get bus assignments and a slot, then builds the OSC bundle.
   `release_voice()` calls `self.voice_allocator.release(...)` then
   sends gate=0.

5. Future: configurable `StealingStrategy` per instrument (Oldest,
   LowestVelocity, FurthestFromLastNote). For now, keep Oldest as
   default — just make the strategy a field.

**Files:** New `engine/voice_allocator.rs`, refactor
`engine/voices.rs`, `engine/mod.rs`

**Verification:** `cargo test`, manual: play rapid notes, verify voice
stealing still works. Play 16+ simultaneous notes, verify oldest gets
stolen. Play/stop/play — verify no control bus leaks.

---

### Phase G: Lock-Free Monitor Data (Performance Win)

**Why:** Five `Arc<Mutex<T>>` in `osc_client.rs` are shared between
the OSC receive thread and the UI render thread. While contention is
low today (OSC writes at ~45Hz, UI reads at ~62Hz), it's a
correctness issue — a mutex lock during render can cause frame drops.

**Current shared state** (all in `osc_client.rs`):
- `meter_data: Arc<Mutex<(f32, f32)>>` — peak L/R
- `audio_in_waveforms: Arc<Mutex<HashMap<u32, VecDeque<f32>>>>` —
  per-instrument waveform peaks
- `spectrum_data: Arc<Mutex<[f32; 7]>>` — 7-band spectrum
- `lufs_data: Arc<Mutex<(f32, f32, f32, f32)>>` — LUFS measurements
- `scope_buffer: Arc<Mutex<VecDeque<f32>>>` — oscilloscope ring buffer

**Changes (two sub-phases):**

**G1: RwLock upgrade (minimal, immediate):**
1. Replace all 5 `Arc<Mutex<T>>` with `Arc<RwLock<T>>` in
   `osc_client.rs`
2. OSC receive thread uses `.write().unwrap()`
3. UI thread uses `.read().unwrap()` — multiple concurrent readers
   don't block each other
4. All getter methods in `AudioMonitor` / `OscClient` switch from
   `.lock()` to `.read()`

**G2: Triple-buffer for hot path (optional, if profiling shows need):**
1. Add `triple-buffer` crate to Cargo.toml
2. Replace `meter_data` and `spectrum_data` with triple-buffered
   versions — these are the hottest path (read every frame)
3. OSC thread writes to input buffer, UI thread reads from output
   buffer — zero contention
4. Keep RwLock for `audio_in_waveforms` (HashMap, harder to
   triple-buffer) and `scope_buffer` (VecDeque, variable size)

**Files:** `imbolc-core/src/audio/osc_client.rs`, callers in
`handle.rs` and `main.rs`

**Verification:** `cargo test`, manual: verify meter/spectrum/scope
still update smoothly during playback. Profile with heavy polyphony
to confirm no frame drops.

---

### Phase H: Explicit State Authority (Correctness Win)

**Why:** The audio thread overwrites UI state via
`AudioFeedback::PlayheadPosition`, `BpmUpdate`, and others through
`dispatch/audio_feedback.rs`. This creates implicit bidirectional
state flow and subtle ownership confusion. Most critically, the audio
thread can call `audio.set_playing(false)` which sends a command back
to itself — indirect self-mutation.

**Current bidirectional state:**
| State | UI writes | Audio writes back | Risk |
|-------|-----------|-------------------|------|
| playhead | Reset on stop | Advances every tick | Low (audio authoritative) |
| BPM | User adjustment | Automation modulation | Medium (who wins?) |
| playing | Start/stop | Render completion stops | Medium (race) |
| recording | Start/stop | Elapsed time, completion | Low |
| instrument source | User edit | Render converts to sampler | High (surprise mutation) |
| drum step | User edit | Playback advances | Low |

**Changes:**

1. **Separate AudioHandle cached state from AppState:**
   - `AudioHandle` already caches playhead, BPM, recording state
   - Make these the **canonical read source** for display
   - Stop writing playhead/BPM back into `state.session.piano_roll`
     from `audio_feedback.rs`

2. **Create `AudioReadState` struct on AudioHandle:**
   ```rust
   pub struct AudioReadState {
       pub playhead: u32,
       pub bpm: f32,
       pub is_playing: bool,
       pub is_recording: bool,
       pub recording_elapsed: Duration,
       pub server_status: ServerStatus,
       pub drum_steps: HashMap<InstrumentId, usize>,
   }
   ```
   UI panes read `audio.read_state()` for display instead of
   `state.session.piano_roll.playhead`.

3. **Remove self-mutation in audio_feedback.rs:**
   - `RenderComplete` currently calls `audio.set_playing(false)` and
     `audio.reset_playhead()` (lines 45-46) — this sends commands
     back to the audio thread from inside dispatch
   - Instead: return `DispatchResult { stop_playback: true }` and let
     `main.rs` call `audio.set_playing(false)` explicitly

4. **Instrument source mutation (render → sampler conversion):**
   - Currently `audio_feedback.rs:53-61` directly mutates
     `instrument.source` when render completes
   - Keep this — it's a legitimate state mutation that happens to be
     triggered by audio completion. But wrap it in a proper
     `Action::Instrument(ConvertToSampler { id, buffer_id, path })`
     so it goes through normal dispatch with undo support.

5. **BPM authority:**
   - UI sets BPM → audio thread uses it
   - Automation modulates BPM → audio thread updates its local copy
     AND sends feedback
   - Resolution: automation BPM changes update `AudioReadState.bpm`
     for display but do NOT write to `state.session.piano_roll.bpm`.
     When automation stops, BPM reverts to state value.

**Files:** `audio/handle.rs` (add AudioReadState),
`dispatch/audio_feedback.rs` (remove state overwrites),
`action.rs` (add DispatchResult field), `main.rs` (use
audio.read_state() for display), panes that read playhead/BPM
(piano_roll_pane, mixer_pane, etc.)

**Verification:** `cargo test`, manual: play sequence — playhead
display works. Record automation on BPM — BPM display updates during
playback, reverts when stopped. Render instrument to WAV — playback
stops cleanly, instrument converts to sampler.

---

### Phase I: Error Propagation / Node Registry (Robustness Win)

**Why:** All OSC commands to SuperCollider are fire-and-forget. If
scsynth crashes mid-session, stale node IDs accumulate and all
subsequent `/n_set` calls silently fail. The only detection is
`try_wait()` on the child process.

**Changes:**

1. **Node registry in AudioEngine:**
   ```rust
   struct NodeRegistry {
       live_nodes: HashSet<i32>,
       created_at: HashMap<i32, Instant>,
   }
   ```
   - `create_synth()` adds to registry
   - `free_node()` removes from registry
   - `set_param()` checks registry first — warn if targeting
     unknown node

2. **Server health ping:**
   - Send `/status` to scsynth every 5 seconds from audio thread
   - Parse `/status.reply` in OSC receive thread
   - If no reply within 2 seconds, mark server as potentially dead
   - If `try_wait()` confirms crash → invalidate entire registry →
     emit `AudioFeedback::ServerCrashed`
   - Main thread shows error and offers reconnect

3. **Stale node detection:**
   - On full routing rebuild, compare `node_map` entries against
     registry
   - Log warnings for nodes that should exist but don't appear in
     registry (indicates SC silently freed them)
   - This is diagnostic only — no automatic recovery beyond the
     existing rebuild mechanism

4. **Graceful degradation:**
   - If `send_message()` returns an IO error, mark server as
     disconnected
   - Queue subsequent commands until reconnection
   - On reconnect, trigger full routing rebuild

**Files:** New `engine/node_registry.rs`, update `engine/mod.rs`,
`engine/routing.rs`, `engine/voices.rs`, `audio_thread.rs`,
`osc_client.rs` (status ping), `dispatch/audio_feedback.rs`
(ServerCrashed handling)

**Verification:** `cargo test`, manual: kill scsynth process during
playback — verify error displayed, reconnect works, audio resumes
after rebuild. Verify no panics or hangs on crash.

---

### Updated Dependency Graph

```
Phase A (Effect IDs)
    │
    ▼
Phase B (Per-Instrument Routing)
    │
    ▼
Phase D (Targeted Diffs) ←── optional extension

Phase C (Serde Persistence) ←── independent
Phase E (Backend Trait) ←── independent
Phase F (Voice Allocator) ←── independent
Phase G (Lock-Free Monitor) ←── independent
Phase H (State Authority) ←── independent
Phase I (Node Registry) ←── independent, benefits from Phase B
```

**Recommended execution order:**
1. **A → B** (critical path: incremental routing)
2. **F** (voice allocator: quick cleanup, enables future work)
3. **H** (state authority: correctness fix)
4. **G1** (RwLock: 30 min, immediate win)
5. **C** (serde persistence: maintenance burden)
6. **E** (backend trait: testing infrastructure)
7. **I** (node registry: robustness)
8. **D** (targeted diffs: optimization)
9. **G2** (triple-buffer: only if profiling demands it)
   
