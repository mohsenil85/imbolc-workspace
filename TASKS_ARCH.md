# Architecture Tasks

Derived from the 16 architecture questions in `plans/questions.md`.

---

## Near-term

### ~~1. Voice allocator /n_end feedback [Q14+Q15]~~ ✓

**Done.** OSC listener receives `/n_end` notifications and feeds them
via crossbeam channel to the audio thread, which removes voices from
the allocator and returns control buses to the pool immediately.
Timer-based `cleanup_expired()` (release_dur + 1.5s) retained as
safety net. 9 tests cover polyphonic voice removal, oneshot bus
return, and unknown node handling.

**Files:** `imbolc-core/src/audio/osc_client.rs`,
`imbolc-core/src/audio/engine/voices.rs`,
`imbolc-core/src/audio/engine/voice_allocator.rs`,
`imbolc-core/src/audio/audio_thread.rs`.

### ~~2. Control-plane vs performance-plane separation [Q3]~~ ✓

**Done.** All heavy control-plane operations moved off the audio thread:

- **Server startup:** `start_server_async()` spawns `scsynth` in a
  background thread; audio thread polls `pending_server_start`.
- **OSC connection:** `connect_with_monitor_async()` runs handshake in a
  background thread; audio thread polls `pending_connect`.
- **SynthDef compilation:** `compile_synthdefs_async()` runs `sclang` in
  a background thread; audio thread polls `poll_compile_result()`.
- **Routing rebuild:** phased state machine
  (`RoutingRebuildPhase`) spreads work across ticks (~0.5ms each).
- **Two-channel dispatch:** priority channel (voice spawn, param
  changes) gets 200µs budget; normal channel (state updates, routing)
  gets 100µs budget — prevents bulk ops from starving playback.

SynthDef *loading* (`LoadSynthDefs`) still runs synchronously on the
audio thread but is fast enough (<100ms) to not cause audible stutter.

**Files:** `imbolc-core/src/audio/audio_thread.rs`,
`imbolc-core/src/audio/engine/server.rs`,
`imbolc-core/src/audio/engine/routing.rs`,
`imbolc-core/src/audio/handle.rs`.
220 tests pass.

### ~~3. Field-level network delta updates [Q9]~~ ✓

**Done.** Instrument-level delta updates fully implemented. Server
tracks per-instrument dirty flags (`DirtyFlags` with
`dirty_instruments: HashSet<InstrumentId>` for targeted edits,
`instruments_structural: bool` for add/delete/select/undo).
`broadcast_state_patch()` sends only changed instruments as
`InstrumentPatch` entries instead of full `InstrumentState` blobs.
Rate-limited at ~30Hz with threshold coalescing (falls back to full
snapshot when >50% of instruments are dirty). Fixed
`Option<Option<T>>` JSON roundtrip bug via `double_option` serde
helper.

**Files:** `imbolc-net/src/server.rs`, `imbolc-net/src/protocol.rs`,
`imbolc-net/src/dirty_flags.rs`, `imbolc-types/src/state/patch.rs`.
78 tests pass (19 unit + 59 integration), including per-instrument
dirty tracking, wire-level patch tests, rate limiting, and threshold
coalescing.

### ~~4. Hybrid undo diffs [Q10]~~ ✓

**Done.** Replaced full-state snapshots with scope-aware `UndoEntry`
variants (`SingleInstrument`, `Session`, `Full`). A single-instrument
param tweak now clones one `Instrument` instead of all 64. Scope
classifier (`undo_scope()`) routes each action to the narrowest scope.
Persistence unaffected (undo history is never persisted).

**Files changed:** `imbolc-core/src/state/undo.rs` (rewrite),
`imbolc-core/src/dispatch/mod.rs` (auto-push + undo/redo arms).
220 tests pass (+3 new scope-aware tests).

---

## Long-term (architectural rewrites)

### ~~5. Event-log architecture [Q1+Q7+Q8]~~ ✓

**Phase 1 (Factor Dispatch) done.** All dispatch functions separated
into pure state mutation + `AudioSideEffect` enum. Dispatchers no
longer call `AudioHandle` methods directly — they push typed effect
variants into a `Vec<AudioSideEffect>`. The top-level
`dispatch_with_audio()` collects effects and applies them after
dispatch returns via `apply_side_effects()`. `audio` param changed
from `&mut AudioHandle` to `&AudioHandle` (read-only for
`is_running()`, `status()` queries).

~30 `AudioSideEffect` variants cover all audio operations: voice
management, transport, samples, mixer, click track, tuner, drums,
automation, EQ, server lifecycle, recording, VST.

**Phase 2 (Action Projection) done.** Audio thread receives
`ForwardAction` commands and applies incremental state mutations via
`action_projection.rs` instead of full-state cloning.

**Phase 3 (Audio Timing Authority) done.** Audio thread is sole
authority on `playing` state (alongside existing `playhead`/`bpm`).
Main thread maintains read-only mirror in `state.audio.playing`
updated via `PlayingChanged` feedback. Dispatch dual-writes both
`state.session.piano_roll.playing` and `state.audio.playing` for
immediate consistency. All dispatch reads migrated to
`state.audio.playing`. UI reads migrated similarly.

**Files (Phase 3):** `imbolc-types/src/audio.rs` (PlayingChanged variant),
`imbolc-core/src/state/audio_feedback.rs`,
`imbolc-core/src/audio/handle.rs`, `imbolc-core/src/audio/audio_thread.rs`,
`imbolc-core/src/dispatch/audio_feedback.rs`,
`imbolc-core/src/dispatch/piano_roll.rs`,
`imbolc-core/src/dispatch/arrangement.rs`,
`imbolc-core/src/dispatch/helpers.rs`,
`imbolc-core/src/dispatch/mixer.rs`,
`imbolc-core/src/dispatch/automation.rs`,
`imbolc-core/src/dispatch/vst_param.rs`,
`imbolc-core/src/dispatch/instrument/crud.rs`,
`imbolc-core/src/dispatch/instrument/effects.rs`,
`imbolc-core/src/dispatch/instrument/eq.rs`,
`imbolc-ui/src/main.rs`,
`imbolc-ui/src/panes/piano_roll_pane/rendering.rs`,
`imbolc-ui/src/panes/waveform_pane.rs`.
594 tests pass.

**Phase 4 (Shared Event Log) done.** Replaced 4 `AudioCmd` state-sync
variants (`ForwardAction`, `FullStateSync`, `UpdatePianoRollData`,
`UpdateAutomationLanes`) with a shared retained event log
(`EventLogWriter`/`EventLogReader`). Main thread appends `Arc<LogEntry>`
entries; audio thread drains them within a 100µs budget. History retained
(10k cap) for future network/debug use. 7 unit tests added.

**Files (Phase 4):** `imbolc-core/src/audio/event_log.rs` (new),
`imbolc-core/src/audio/handle.rs`, `imbolc-core/src/audio/audio_thread.rs`,
`imbolc-core/src/audio/commands.rs`, `imbolc-core/src/dispatch/side_effects.rs`.
601 tests pass.

### 6. Event scheduler with dynamic lookahead [Q5+Q6]

Pre-compute upcoming OSC bundles into a ring buffer. Replace
synchronous 0.5ms ticking with ahead-of-time scheduling via dedicated
sender thread. Compute lookahead dynamically from
buffer_size/sample_rate. Replaces the hardcoded 15ms
`SCHEDULE_LOOKAHEAD_SECS`.

### 7. Modular routing [Q11]

new version planned at targeted-routing-loosening.md
not fullly modular but supporting some/most use cases
---

## Housekeeping

### 8. Prune docs [Q16]

Delete stale plan/design docs from `docs/`. Keep reference docs
(audio-routing, keybindings, sqlite-persistence,
polyphonic-voice-allocation). Per-crate CLAUDE.md files are the living
contracts.

---

## Closed (no action needed)

- **Q2: SC is the answer** -- SuperCollider is the long-term
  backend. Focus on reducing architectural tax.
- **Q4: Network timing** -- Not needed; audio is centralized on
  server.
