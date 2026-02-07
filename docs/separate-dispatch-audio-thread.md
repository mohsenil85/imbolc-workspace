# Separate Dispatch from Audio Thread

**Status:** FUTURE
**Last Updated:** 2025-02-06

## Current State

Everything runs on one thread in a tight loop (`main.rs:88`):

```
poll input (2ms) -> dispatch_action() -> tick_playback() -> tick_drum_sequencer() -> render (~60fps)
```

- `dispatch_action()` mutates `AppState` AND calls `AudioEngine`
  methods directly (~20+ call sites: `spawn_voice`,
  `rebuild_instrument_routing`, `set_source_param`,
  `update_all_instrument_mixer_params`, etc.)
- `tick_playback()` reads piano roll state, calls
  `audio_engine.spawn_voice()` / `release_voice()` /
  `apply_automation()`
- `tick_drum_sequencer()` reads drum patterns, calls
  `audio_engine.play_drum_hit_to_instrument()`
- All audio engine calls are synchronous UDP sends to SuperCollider
  (non-blocking, fire-and-forget)
- `active_notes` is shared between dispatch (PlayNote pushes) and
  playback (decrements/releases)
- Playback timing resolution is limited to ~16ms (frame rate) because
  it ticks in the render loop

## Architecture: MPSC Command Channel + Dedicated Audio Thread

```
Main Thread (UI + Dispatch)              Audio Thread (new)

poll_event (2ms)                         loop {
dispatch -> mutate state                   drain command channel
  -> send AudioCmd via channel              handle commands
poll AudioFeedback                         tick_playback (~1ms resolution)
  -> update playhead, bpm, etc.             tick_drum_sequencer
render UI                                  send feedback (playhead, bpm, status)
                                           sleep(1ms)
                                         }
```

**Why this model:**
- AudioEngine has complex internal state -- exclusive ownership on one
  thread is cleaner than `Arc<Mutex>`
- Commands are variable-sized and infrequent (tens/sec) -- MPSC
  channels are fine, no need for lock-free ring buffers
- No async/tokio needed -- fire-and-forget UDP doesn't benefit from
  async

## State Coupling

The hard part is that dispatch and playback both read/write shared
state:

| State | Dispatch does | Audio thread needs | Strategy |
|-------|--------------|-------------------|----------|
| `InstrumentState` | Add/remove/update instruments, params, level, mute | `spawn_voice`, `rebuild_routing` read it | Clone snapshot on change |
| `SessionState` (buses, master) | Mixer adjust, mute/solo toggle | `rebuild_routing`, `update_mixer_params` | Clone snapshot on change |
| `piano_roll.playhead` | Reset on PlayStop | Advance each tick | Audio thread owns; sends back via feedback |
| `piano_roll.bpm` | Set from FrameEdit | Tick calculation; BPM automation writes it | Audio thread owns; updated by command & feedback |
| `piano_roll.tracks` | Note toggle | Scan for note events | Snapshot on change |
| `active_notes` | PlayNote pushes | Playback decrements/releases | Moves entirely to audio thread |
| `automation.lanes` | Add/remove points | Scan for values during playback | Snapshot on change |
| `drum_sequencer` timing | PlayStop resets | `tick_drum_sequencer` advances | Audio thread owns timing; pattern data via snapshot |

**Feedback (audio -> main):** playhead position, BPM (from
automation), drum step positions, server status, recording
state. Polled via `try_recv()` each frame.

## New Types

**`AudioCmd`** (~20 variants): `SpawnVoice`, `ReleaseAllVoices`,
`PlayDrumHit`, `RebuildRouting`, `UpdateMixerParams`,
`SetSourceParam`, `SetBusMixerParams`, `UpdateState { instruments,
session_snapshot }`, `SetPlaying`, `ResetPlayhead`, `SetBpm`,
`UpdatePianoRollData`, `UpdateAutomationLanes`, `Connect`,
`StartServer`, `StopServer`, `LoadSample`, `StartRecording`,
`StopRecording`, `Shutdown`, etc.

**`AudioFeedback`** (~8 variants): `PlayheadPosition`, `BpmUpdate`,
`DrumSequencerStep`, `ServerStatus`, `RecordingState`,
`RecordingStopped(PathBuf)`, `CompileResult`, `PendingBufferFreed`.

**`AudioHandle`** (main thread interface): holds
`mpsc::Sender<AudioCmd>`, `mpsc::Receiver<AudioFeedback>`, cached
playhead/bpm/is_running/is_recording. Replaces `&mut AudioEngine` in
all dispatch signatures.

**`AudioThread`** (audio thread): owns `AudioEngine`, command
receiver, feedback sender, playback state copies, `active_notes`.

## Migration (5 incremental phases)

### Phase 1: Command layer (no thread yet) [DONE]
Created `AudioCmd`/`AudioFeedback` enums and `AudioHandle` that wraps
`AudioEngine` synchronously. Refactored all ~55
`audio_engine.method()` call sites in `dispatch.rs` and `main.rs` to
`audio_handle.method()`. Behavior is identical -- just routing through
a command layer.

**Files:** created `src/audio/commands.rs`, `src/audio/handle.rs`;
modified `src/dispatch.rs`, `src/main.rs`, `src/audio/mod.rs`

### Phase 2: Move playback into AudioHandle [DONE]
Moved `tick_playback` and `tick_drum_sequencer` into `AudioHandle` as
private methods, exposed via `AudioHandle::tick()`. Moved
`active_notes` into `AudioHandle`. Eliminated `src/playback.rs`.
Main loop now calls only `audio.tick(&mut state, elapsed)`. Feedback
types (`AudioFeedback`) defined but not yet wired -- channel plumbing
deferred to Phase 3.

**Files:** `src/audio/handle.rs`, `src/main.rs`,
`src/dispatch/mod.rs`, `src/dispatch/instrument.rs`,
`src/dispatch/piano_roll.rs`

### Phase 3: Thread separation [DONE]
Split AudioHandle into sender (main thread) + AudioThread (new thread
with 1ms tick loop). AudioHandle wraps MPSC channels and cached state;
AudioThread owns AudioEngine, state snapshots, active_notes, and runs
tick_playback/tick_drum_sequencer at ~1ms resolution. Feedback
(playhead, BPM, server status, recording, compile results) flows back
via AudioFeedback channel, polled each frame in main.rs.

**Files:** `imbolc-core/src/audio/handle.rs`,
`imbolc-core/src/audio/commands.rs`, `src/main.rs`

### Phase 4: State snapshot optimization [DONE]
Added dirty-flag batching via `AudioDirty` struct with 6 boolean flags
(`instruments`, `session`, `piano_roll`, `automation`, `routing`,
`mixer_params`). All ~90 dispatch handler sites set appropriate flags on
`DispatchResult.audio_dirty`. Main loop accumulates flags via
`pending_audio_dirty.merge()` and calls `audio.flush_dirty()` once per
frame. `flush_dirty()` checks each flag individually and only clones the
relevant state snapshots. Type aliases in `snapshot.rs` document the
snapshot types.

**Files:** `imbolc-core/src/action.rs`, `imbolc-core/src/audio/handle.rs`,
`imbolc-core/src/audio/snapshot.rs`, `src/main.rs`,
`src/dispatch/instrument.rs`, `src/dispatch/mixer.rs`,
`src/dispatch/piano_roll.rs`, `src/dispatch/sequencer.rs`,
`src/dispatch/automation.rs`, `src/dispatch/session.rs`,
`src/dispatch/server.rs`

### Phase 5: Server management async responses [DONE]
Refactored `dispatch_server` so Connect/Start/Stop/Restart send
fire-and-forget commands via new async `AudioHandle` helpers. Added
`AudioCmd::RestartServer`, and `AudioThread` now emits
`AudioFeedback::ServerStatus` for lifecycle actions (including
connect/restart synthdef + drum sample loading). ServerPane updates
already happen in the feedback polling loop.

**Files:** `imbolc-core/src/audio/commands.rs`,
`imbolc-core/src/audio/handle.rs`, `imbolc-core/src/dispatch/server.rs`,
`src/main.rs`

## Risks

- **State staleness**: Audio thread operates on snapshots. Param
  change + immediate note play could use stale data. Mitigated by
  sending snapshots eagerly (after each input event, not just end of
  frame).
- **Server startup blocking**: `start_server` sleeps 500ms. Fine on
  the audio thread since it has no other responsibilities during
  startup.
- **Server action refactoring**: `dispatch_server::Restart` is a long
  sequential chain with interleaved UI updates. Becomes a single
  `RestartServer` command; results come back as feedback. Most complex
  part of the migration.
- **Clone cost**: `InstrumentState` with 10-20 instruments is
  <10KB. At 60Hz worst case, ~600KB/s through the channel --
  negligible.
- **Test updates**: Existing tests use `NullOscClient` and call engine
  methods directly. Need a synchronous AudioHandle variant for tests,
  or test through the command interface.

## Verification

1. `cargo test --bin imbolc` -- all existing tests pass after each phase
2. Manual test: start server, play notes, verify timing doesn't
   degrade during UI-heavy operations (rapid pane switching, scrolling
   piano roll during playback)
3. Compare playback timing precision before/after: with thread
   separation, playback should maintain ~1ms tick resolution
   regardless of render load

## Critical Files
- `src/main.rs` -- main loop restructuring
- `src/dispatch.rs` -- ~55 audio_engine call sites to convert
- `src/playback.rs` -- moves into audio thread
- `src/audio/engine.rs` -- AudioEngine moves to audio thread ownership
- `src/audio/osc_client.rs` -- meter/waveform `Arc<Mutex>` stays
  accessible from main thread
