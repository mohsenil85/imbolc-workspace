# Project Steady Pulse - Realtime Latency and Jitter Hardening Plan

**Status:** FUTURE
**Last Updated:** 2026-02-07

## Mission
- Reduce audio scheduling jitter under load.
- Remove thread-contention hazards in monitor and command paths.
- Prevent long-tail latency spikes from control-path work.
- Preserve existing behavior while making timing more deterministic.

## Success Metrics
- Audio thread tick jitter (p99): <= 1.0 ms under stress.
- Priority command latency (note on/off, drum hit, live param): <= 2.0 ms p99.
- No unsafe multi-writer access patterns in lock-free monitor structures.
- Automation evaluation cost: O(log n) or amortized O(1) per lane per tick.
- No blocking startup or device discovery work on realtime-sensitive loops.

## Findings This Plan Addresses
1. `vst_params` uses `TripleBufferHandle` with operations from more than one thread (`imbolc-core/src/audio/osc_client.rs`, `imbolc-core/src/audio/triple_buffer.rs` contract).
2. Many dispatch paths still escalate to full instrument sync (`audio_dirty.instruments = true`) for realtime tweaks (`imbolc-core/src/dispatch/*`), which drives `AudioCmd::UpdateState` cloning in `imbolc-core/src/audio/handle.rs`.
3. Automation lookup is linear in lane point count per tick (`imbolc-types/src/state/automation.rs` `value_at`).
4. Routing rebuild/update work is executed inline on the audio thread (`imbolc-core/src/audio/audio_thread.rs`).
5. Adaptive draining can still monopolize loop time under bursty command traffic (`imbolc-core/src/audio/audio_thread.rs`).
6. Device enumeration via `system_profiler` is expensive and currently uncached (`imbolc-core/src/audio/devices.rs`).
7. MIDI event timestamps are captured but not used in live event scheduling (`imbolc-core/src/midi/mod.rs`, `imbolc-ui/src/midi_dispatch.rs`, `imbolc-ui/src/main.rs`).
8. Priority queue includes bulk mixer updates that are not all equally time-critical (`imbolc-core/src/audio/commands.rs`).
9. Engine polling work runs every loop iteration (~0.5 ms cadence) with mixed criticality (`imbolc-core/src/audio/audio_thread.rs`).
10. Click downbeat classification can be wrong when multiple beats are crossed in one tick (`imbolc-core/src/audio/click_tick.rs`).

## Phase 0 - Baseline Instrumentation (Prerequisite)
### Changes
- Add realtime telemetry from the audio thread:
  - Tick delta (actual vs target).
  - Per-loop command counts (priority and normal).
  - Loop time spent in `poll_engine`.
  - Queue depth snapshots.
- Emit telemetry at low frequency (1 Hz) via `AudioFeedback`.
- Add a stress harness for burst command scenarios.

### Files
- `imbolc-core/src/audio/audio_thread.rs`
- `imbolc-core/src/audio/commands.rs`
- `imbolc-ui/src/main.rs`

### Exit Criteria
- Baseline numbers captured for idle and stress workloads.
- Telemetry can be toggled without affecting release behavior.

## Phase 1 - Correctness and Safety First
### Changes
- Remove cross-thread writer ambiguity for `vst_params`:
  - Keep single-writer semantics for triple buffer fields, or
  - Replace `vst_params` with a structure designed for multi-producer access.
- Fix click downbeat logic so each crossed beat computes its own beat index.

### Files
- `imbolc-core/src/audio/osc_client.rs`
- `imbolc-core/src/audio/triple_buffer.rs`
- `imbolc-core/src/audio/audio_thread.rs`
- `imbolc-core/src/audio/click_tick.rs`

### Exit Criteria
- No shared field relies on unsafe single-writer assumptions across threads.
- Multi-beat catch-up click tests pass with correct downbeat detection.

## Phase 2 - Command Path Stabilization
### Changes
- Reclassify command priority:
  - Keep note on/off, drum hits, and tight param updates on priority.
  - Move bulk mixer/state maintenance traffic off priority queue.
- Replace pure count-based draining with combined count + time budget per loop.
- Add coalescing for redundant normal commands (`UpdateState`, mixer refresh, routing refresh).

### Files
- `imbolc-core/src/audio/commands.rs`
- `imbolc-core/src/audio/audio_thread.rs`
- `imbolc-core/src/audio/handle.rs`

### Exit Criteria
- Under synthetic burst load, tick jitter remains inside target.
- Priority command latency is stable and insensitive to normal queue spikes.

## Phase 3 - Reduce Full-State Sync Pressure
### Changes
- Expand targeted audio-dirty paths for high-frequency edits.
- Avoid `UpdateState` clones for common realtime adjustments where direct audio commands are sufficient.
- Preserve full snapshots for structural changes only (instrument add/remove, deep routing changes, project load).

### Files
- `imbolc-types/src/action.rs`
- `imbolc-core/src/dispatch/helpers.rs`
- `imbolc-core/src/dispatch/instrument/groove.rs`
- `imbolc-core/src/dispatch/mixer.rs`
- `imbolc-core/src/audio/handle.rs`
- `imbolc-core/src/audio/audio_thread.rs`

### Exit Criteria
- Continuous knob and groove edits no longer trigger full instrument snapshot clones.
- Measurable reduction in allocation volume and command payload size.

## Phase 4 - Automation and Sequencing Efficiency
### Changes
- Replace linear `value_at` scan with indexed lookup:
  - Binary search for random access path.
  - Optional rolling cursor for monotonic playhead progression.
- Add loop-wrap handling for cursor reset correctness.
- Keep current interpolation behavior intact.

### Files
- `imbolc-types/src/state/automation.rs`
- `imbolc-core/src/audio/playback.rs`

### Exit Criteria
- Automation evaluation cost scales sublinearly with point count.
- Dense multi-lane automation sessions maintain stable tick timing.

## Phase 5 - Polling and Startup Jitter Cleanup
### Changes
- Split `poll_engine` work into scheduled cadences by criticality.
- Cache or async-refresh device enumeration results instead of synchronous `system_profiler` calls in hot control flow.
- Use MIDI timestamps to compute event offsets for live note scheduling.
- Replace fixed 2 ms UI/network sleeps with event-driven wait plus bounded fallback polling.

### Files
- `imbolc-core/src/audio/audio_thread.rs`
- `imbolc-core/src/audio/devices.rs`
- `imbolc-core/src/midi/mod.rs`
- `imbolc-ui/src/midi_dispatch.rs`
- `imbolc-ui/src/main.rs`
- `imbolc-ui/src/network.rs`

### Exit Criteria
- Startup and reconnect operations do not produce visible control-loop stalls.
- Live MIDI timing jitter improves in repeatable measurements.

## Phase 6 - Validation and Rollout
### Validation Matrix
- Idle baseline.
- High BPM dense sequencing.
- Heavy automation playback.
- Burst control input (MIDI CC + UI parameter spam).
- Server restart/connect while project is active.

### Required Checks
- `cargo check`
- `cargo test -p imbolc-core --lib`
- Stress benchmark/report from Phase 0 harness.

### Rollout
- Land per phase behind small, reviewable PRs.
- Compare telemetry before and after each phase.
- Stop after any phase that regresses jitter or behavior until corrected.

## Implementation Order
1. Phase 0 instrumentation
2. Phase 1 correctness/safety
3. Phase 2 command path stabilization
4. Phase 3 state sync reduction
5. Phase 4 automation efficiency
6. Phase 5 polling/startup cleanup
7. Phase 6 validation and rollout
