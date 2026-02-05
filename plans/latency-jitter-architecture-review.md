# Latency/Jitter Review Plan

## Goals
- Make drum sequencer sample-accurate under load.
- Reduce scheduler jitter and late event delivery.
- Move blocking server start/restart off the scheduling thread.
- Ensure playback stops cleanly before any blocking server operations.
- Preserve local-only SuperCollider assumptions (no network clock sync).

## Findings (from code review)

1) **Drum sequencer drops intermediate steps.** When the accumulator advances multiple steps in one tick, the `while` loop in `drum_tick.rs:44-60` advances `current_step` but only the final position is played (line 62: `last_played_step` guard). All intermediate steps are silently skipped. Hits use `offset_secs: 0.0`. This is the highest-impact bug — it drops beats under load.

2) **Audio thread drains all queued commands before ticking** (`audio_thread.rs:127-139`). The `drain_remaining_commands` loop has no cap. Individual commands are cheap, but commands that trigger heavy inline work (e.g., `RebuildRouting`, `UpdateState` with large snapshots) can delay the tick.

3) **Server start/restart blocks the audio thread.** `start_server_with_devices` (`server.rs:122`) calls `thread::sleep(Duration::from_millis(500))` inline. `RestartServer` (`audio_thread.rs:178-207`) chains stop + start (with sleep) + connect + `load_synthdefs_and_samples` synchronously. `Connect` (`audio_thread.rs:143-157`) calls `load_synthdefs_and_samples` synchronously. **Note:** `compile_synthdefs_async` already uses a background thread — the blocking concern there is limited to the synthdef reload in `poll_compile_result` which sends `/d_loadDir` (fast OSC message, not a real problem).

4) **No fixed scheduling lookahead.** Piano roll notes already use sub-tick offsets (`playback.rs:134`: `offset = ticks_from_old * secs_per_tick`) sent as timed OSC bundles via the backend. This provides per-event timing within a tick. However, all offsets are relative to "now" — if a tick runs late, events are delivered late with no compensation. A fixed lookahead (e.g., 15ms) would give the server a consistent scheduling horizon. The automation bundle (`automation.rs:359`) always uses `offset_secs: 0.0`, which is a separate gap.

5) **Arpeggiator lacks lateness compensation.** `arpeggiator_tick.rs:57-128` correctly handles multiple steps per tick with cumulative `step_offset` (line 127). The issue is the same as Finding 4: offsets are relative to "now" with no lookahead or late-tick compensation.

6) ~~Voice cleanup overhead.~~ **Dropped.** `voice_allocator.rs:178-187` is a simple `retain` with a time check. With `MAX_VOICES_PER_INSTRUMENT = 16` and typical instrument counts, this scans ~100-300 voices — negligible cost. Not worth throttling.

### Additional issues found

7) **Compile errors in `server.rs`, `recording.rs`, `routing.rs`** — references to `self.client` where the field is `self.backend`. These must be fixed before any latency work can be tested. (Visible in LSP diagnostics.)

8) **Unbounded MPSC command channel** (`handle.rs:59`). Under burst conditions, commands accumulate without backpressure. Not a latency root cause, but worth noting.

## Constraints / Decisions (per user answers)
- Drum sequencer must be sample-accurate.
- Server start/restart must move off the scheduling thread.
- If a user triggers server operations, playback should stop first.
- SuperCollider is always local (no network latency calibration needed).

## Plan (priority order)

### 0) Fix compile errors (prerequisite)
Replace `self.client` references with `self.backend` in `server.rs`, `recording.rs`, and `routing.rs`. Without this, nothing else compiles or can be tested.

**Files:** `imbolc-core/src/audio/engine/server.rs`, `imbolc-core/src/audio/engine/recording.rs`, `imbolc-core/src/audio/engine/routing.rs`

### 1) Make drum sequencer sample-accurate (highest impact)
Model after the piano roll's range-scan approach (`playback.rs:46-98`):
- Record `old_step` before the accumulator advance loop.
- After advancing, iterate each step in `[old_step+1 .. new_step]`.
- Compute per-step offset: `step_index_from_old * step_duration_secs`.
- Send each hit as a timed bundle with that offset.
- Apply swing/humanize offsets on top of the per-step base offset.

**File:** `imbolc-core/src/audio/drum_tick.rs`

### 2) Introduce a fixed scheduling lookahead
- Define `SCHEDULE_LOOKAHEAD_SECS: f64 = 0.015` (15ms).
- Voice spawn (`voices.rs:228`): `send_bundle(messages, offset_secs + LOOKAHEAD)`.
- Voice release (`voices.rs:498`): add lookahead to gate=0 bundle offset.
- Drum hits (`voices.rs:631`): add lookahead to oneshot bundle offset.
- Automation bundle (`automation.rs:359`): use `LOOKAHEAD` instead of `0.0`.
- Clamp all offsets at `>= 0.0` (defensive).
- **Do NOT apply to live/manual triggers** (MIDI keyboard, UI preview) — only sequenced playback. The playback path already passes computed offsets; live triggers pass `0.0` and should remain immediate.

**Files:** `imbolc-core/src/audio/engine/voices.rs`, `imbolc-core/src/audio/engine/automation.rs`, `imbolc-core/src/audio/drum_tick.rs`

### 3) Move blocking server operations off the audio thread
- `StartServer`/`RestartServer`: spawn the server process and sleep on a dedicated thread. Audio thread sets status to "Starting" and continues ticking. Completion signaled via feedback channel.
- `Connect`: OSC client creation is fast, but `load_synthdefs_and_samples` should move to a background task with completion feedback.
- When these operations start, dispatch "stop playback" first so the scheduling loop is idle but still responsive.

**Files:** `imbolc-core/src/audio/audio_thread.rs`, `imbolc-core/src/audio/engine/server.rs`

### 4) Cap command drain per tick (lower priority)
Change `drain_remaining_commands` (`audio_thread.rs:127-139`) to process at most N commands per tick (e.g., 64). Remaining commands picked up on next tick. This prevents burst scenarios from starving the scheduler.

Alternative: address the root cause by not sending redundant state updates. The `flush_dirty` system already batches, but rapid UI interactions can still generate many incremental commands.

**File:** `imbolc-core/src/audio/audio_thread.rs`

### 5) Add scheduling instrumentation (optional, for validation)
Track and log per-tick: tick duration, command queue depth, late-tick delta (actual elapsed - expected 1ms). Helps validate the other changes and catch future regressions.

**File:** `imbolc-core/src/audio/audio_thread.rs`

## Next Steps
- Fix compile errors (step 0) first so the build is green.
- Implement steps 1-3 in a feature branch.
- Stress test: 200+ BPM, dense 16-step drum patterns with all pads active, automation sweeps on multiple parameters.
- Compare drum hit timing (log OSC bundle timestamps) before and after step 1.
- Verify server restart doesn't stall playback after step 3.
- Add a regression test for drum step timing (simulate late ticks, verify scheduled offsets).
- Run `cargo test` to catch regressions.
