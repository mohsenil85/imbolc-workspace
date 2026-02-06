# Low-Latency/Low-Jitter Rewrite Analysis

**Status:** FUTURE
**Last Updated:** 2025-02-06

## Summary

A rewrite focused on low latency and low jitter is feasible. The current architecture has good foundations but specific bottlenecks cause jitter. A comprehensive test harness can capture behavior for safe validation.

**Target Latencies:**

| Metric | Current (estimated) | Target |
|--------|---------------------|--------|
| MIDI note → OSC send | 0-1ms jitter | <0.5ms p99 |
| Param change → `/n_set` | 0-2ms | <1ms p99 |
| Tick interval stddev | ~200μs | <100μs |
| Monitor read latency | 10-500μs (contention) | <10μs (lock-free) |

---

## What to KEEP

### 1. MPSC Channels (lock-free command/feedback)
- `src/audio/handle.rs:49-50` - Already lock-free, non-blocking sends
- Main→Audio and Audio→Main paths don't contend

### 2. AudioDirty Flags (targeted updates)
- `src/action.rs:453-540` - Fine-grained dirty tracking
- Enables `/n_set` for single params instead of full routing rebuilds
- Pattern: `filter_param`, `effect_param`, `lfo_param` for direct OSC

### 3. OSC Timestamped Bundles with 15ms Lookahead
- `src/audio/engine/mod.rs:43` - `SCHEDULE_LOOKAHEAD_SECS = 0.015`
- Absorbs tick jitter by scheduling ahead of "now"

### 4. Synchronous Dispatch (no async in hot path)
- `src/dispatch/mod.rs:51-112` - Pure state mutation, no awaits
- All mutations deterministic and immediate

### 5. AudioBackend Trait Abstraction
- `src/audio/engine/backend.rs` - Enables testing without SuperCollider

---

## What to CHANGE (Jitter Sources)

### HIGH: RwLock Contention on AudioMonitor
**Location:** `src/audio/osc_client.rs:24-41`

**Problem:** 9 separate RwLocks (meter, spectrum, scope, LUFS, etc.). OSC receive thread writes at 60-100+ Hz, UI thread reads for visualization. Write-heavy contention causes 10-500μs blocking.

**Solution:** Hybrid atomics + triple-buffer

**Use atomics for simple scalar fields:**
```rust
// Bitcast (f32, f32) to u64 for atomic ops
meter_data: AtomicU64,      // (peak_l, peak_r)
sc_cpu: AtomicU32,          // Single f32
osc_latency_ms: AtomicU32,  // Single f32
```

**Use triple-buffer for complex types:**
```rust
pub struct TripleBuffer<T> {
    buffers: [UnsafeCell<T>; 3],
    indices: AtomicU8, // packed: back|middle|front|dirty
}

// Apply to:
audio_in_waveforms: TripleBuffer<HashMap<u32, VecDeque<f32>>>,
spectrum_data: TripleBuffer<[f32; 7]>,
lufs_data: TripleBuffer<(f32, f32, f32, f32)>,
scope_buffer: TripleBuffer<VecDeque<f32>>,
vst_params: TripleBuffer<HashMap<i32, Vec<VstParamReply>>>,
```

**Implementation:** Custom inline (~100 lines) rather than external crate. Keeps dependencies minimal.

### HIGH: 1ms Blocking Timeout Causes Jitter
**Location:** `src/audio/audio_thread.rs:111-141`

**Problem:** `recv_timeout(remaining)` blocks up to 1ms. Commands arriving mid-tick wait for timeout. Voice spawns have 0-1ms jitter based on arrival time.

**Solution:** Priority command channels with crossbeam

**Dependency required:** Add `crossbeam-channel = "0.5"` to Cargo.toml (not currently present)

```rust
use crossbeam_channel::{Receiver, Select};

// Time-critical: SpawnVoice, ReleaseVoice, PlayDrumHit, SetParam
priority_rx: Receiver<PriorityCmd>

// Everything else: UpdateState, RebuildRouting, LoadSample
normal_rx: Receiver<NormalCmd>
```
- Process ALL priority commands immediately (no budget limit)
- Process normal commands with budget (16-32 per tick)
- Reduces tick interval to 0.5ms for finer granularity
- Use `crossbeam::Select` on two channels instead of single `recv_timeout`

### MEDIUM: Full State Snapshot Clones
**Location:** `src/audio/handle.rs:244-249`

**Problem:** `UpdateState` clones entire `InstrumentSnapshot` + `SessionSnapshot`. With many instruments, this is 40-100 KB allocations on hot path.

**Solution:** Selective delta for audio thread only

Full delta tracking would require refactoring all 12 dispatch modules (~3000 lines). Instead, send deltas only for the audio thread:

```rust
pub enum AudioDelta {
    // Time-critical (send immediately, no clone)
    ParamChange { instrument_id: u32, param_path: ParamPath, value: f32 },

    // Structural (batch with full snapshot)
    RoutingChanged,
    InstrumentAdded(u32),
    InstrumentRemoved(u32),
}
```

- Parameter tweaks send small deltas (~70-80% of audio thread updates)
- Full snapshots only for routing changes, undo, project load
- Avoids refactoring dispatch layer

### LOW: Undo Snapshot Cost
**Location:** `src/state/undo.rs:25-34`

**Problem:** Every undoable action clones entire state (40-100 KB). 500 undo levels = 20-50 MB memory pressure.

**Solution:** Chunked per-instrument Arc (simpler than full CoW)

Full Arc-wrapping of all state fields would require:
- Changing all 12 dispatch modules to use `Arc::make_mut()` pattern
- Serialization layer changes (serde + Arc requires extra work)
- Estimated effort: 2-3 weeks

**Simpler alternative:**
```rust
pub struct InstrumentState {
    // Only clone instruments that actually changed
    pub instruments: HashMap<InstrumentId, Arc<Instrument>>,
    pub selected: Option<InstrumentId>,
}
```
- ~60% memory reduction with ~20% of the refactoring effort
- Session state stays as-is (smaller, changes less frequently)

---

## Additional Jitter Sources (Document/Monitor)

### Note Humanization (Intentional)
**Location:** `src/audio/playback.rs:144-147`

```rust
(next_random(rng_state) - 0.5) * 2.0 * humanize_time * 0.02
```

Up to ±20ms intentional jitter for musical feel. Not a bug, but should be:
- Documented in user-facing docs
- Excluded from jitter measurements
- Optionally bypassable for testing

### poll_engine() Per-Tick Overhead
**Location:** `src/audio/audio_thread.rs:698-796`

Every tick runs: voice cleanup, compile polling, server health check (1Hz), status polling, VST param polling (150ms idle timeout), buffer free polling, recording state.

**Mitigation:** Profile and consider moving non-critical polling to separate intervals.

---

## Thread Model (Current vs Proposed)

| Thread | Current | Proposed |
|--------|---------|----------|
| Main | UI, dispatch, feedback poll | Same |
| Audio | 1ms tick, command process, OSC send | 0.5ms tick, priority-first processing |
| OSC Recv | UDP recv, RwLock writes | UDP recv, atomic/triple-buffer writes |
| I/O | File save/load (via io_tx) | Same |

**Key change:** Audio thread uses `crossbeam::Select` on two channels instead of single `recv_timeout`.

---

## Implementation Order

### Phase 1: Low-risk, high-impact
1. Atomic scalars in AudioMonitor (meter_data, sc_cpu, osc_latency_ms)
2. Add `crossbeam-channel` dependency
3. Split priority/normal command channels
4. Reduce tick interval to 0.5ms

### Phase 2: Medium complexity
5. Triple-buffer for complex monitor fields (spectrum, scope, waveforms, lufs, vst_params)
6. AudioDelta for param changes (selective, not full delta)

### Phase 3: Higher complexity (optional)
7. Chunked instrument snapshots with per-instrument Arc
8. Full undo optimization

---

## Test Harness Design

### Yes, we can capture complete behavior

The harness captures:

1. **Action → State mutations** via golden snapshots
2. **Action → AudioDirty flags** via explicit flag tests
3. **Action → AudioCmd sequences** via MockAudioSink (no SuperCollider needed)
4. **Undo/redo correctness** via property tests
5. **Cross-state invariants** via invariant checker

### Core Components

```
MockAudioSink          - Captures AudioCmd without executing
NullAudioHandle        - Pure state testing, no threads
InvariantChecker       - Verifies consistency rules always hold
DispatchCapture        - Serializable snapshot of dispatch results
```

### Benchmark Suite (Add)

```rust
#[bench] fn voice_spawn_latency() { /* MIDI note-on → OSC send */ }
#[bench] fn param_update_latency() { /* knob turn → /n_set */ }
#[bench] fn tick_interval_jitter() { /* stddev of 10,000 ticks */ }
#[bench] fn monitor_read_contention() { /* RwLock wait time */ }
```

**Jitter measurement test:**
- Run 10,000 ticks
- Measure stddev of actual tick intervals
- Fail if >100μs stddev

### Key Test Categories

| Category | Approach | Crate |
|----------|----------|-------|
| Golden snapshots | Serialize dispatch results, compare | `insta` |
| Property tests | Random action sequences + invariants | `proptest` |
| AudioCmd capture | Mock audio sink logs all commands | custom |
| Persistence | Existing round-trip tests | existing |
| Cross-state | Delete instrument cleans up refs | explicit |
| Latency benchmarks | Measure before/after each phase | `criterion` |

### Invariants to Verify

1. Piano roll tracks reference existing instruments
2. Automation lanes reference existing instruments
3. Arrangement placements reference existing clips
4. Instrument IDs are unique
5. Effect IDs are unique within instrument
6. Selection index in bounds
7. Bus IDs valid (1-8)
8. Parameter values in bounds (level 0-1, pan -1 to 1)
9. No singleton layer groups

### Test Organization

```
tests/
├── golden_snapshots/     # insta snapshots for all action types
├── property_tests/       # proptest invariant verification
├── integration/          # cross-state, audio_dirty correctness
├── benchmarks/           # criterion latency benchmarks
└── harness/              # MockAudioSink, NullAudioHandle, invariants
```

---

## Critical Files

| Purpose | Path |
|---------|------|
| RwLock contention | `src/audio/osc_client.rs:24-41` |
| Blocking timeout | `src/audio/audio_thread.rs:111-141` |
| State clones | `src/audio/handle.rs:244-249` |
| AudioDirty flags | `src/action.rs:453-540` |
| Dispatch entry | `src/dispatch/mod.rs:51-112` |
| AudioCmd enum | `src/audio/commands.rs:22-220` |
| Undo system | `src/state/undo.rs` |
| Note humanization | `src/audio/playback.rs:144-147` |
| poll_engine overhead | `src/audio/audio_thread.rs:698-796` |

---

## Verification

1. **Benchmark before/after:**
   - Voice spawn latency (MIDI note-on → OSC send)
   - Parameter update latency (knob turn → `/n_set`)
   - Tick jitter (stddev of tick interval)
   - Monitor read contention (RwLock wait time → 0)

2. **Test harness validation:**
   - Run golden snapshots against both implementations
   - Property tests pass with 10,000+ action sequences
   - All invariants hold

3. **Integration:**
   - Rapid MIDI input (16th notes at 200 BPM)
   - Automation playback with 8 lanes
   - Simultaneous parameter sweeps

4. **Memory profiling:**
   - Confirm undo memory reduction with `cargo instruments` or similar
   - Profile allocation rates during parameter automation
