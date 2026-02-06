# Phase 4: Incremental State Diffs for Mixer

**Status:** FUTURE
**Last Updated:** 2025-02-06

## Problem

When a single mixer knob turns, `flush_dirty` clones the entire `InstrumentState` + `SessionState` across the MPSC channel (via `UpdateState`), then sends `UpdateMixerParams`. These types contain all instrument definitions, effects chains, samples, etc. — far more data than needed for a mixer parameter change.

## Goal

When only `mixer_params` is dirty (no instrument/session/routing changes), send lightweight targeted commands instead of full state snapshots. The engine update itself is unchanged — the optimization is in avoiding the large clone.

## Current Code

### `handle.rs` `flush_dirty` (lines 112-133):
```rust
let needs_state = dirty.instruments || dirty.session || dirty.routing || dirty.mixer_params;
if needs_state {
    self.update_state(&state.instruments, &state.session);  // FULL CLONE
}
// ...
if dirty.mixer_params {
    let _ = self.send_cmd(AudioCmd::UpdateMixerParams);
}
```

Problem: `mixer_params` is included in `needs_state`, so any mixer change triggers a full snapshot clone.

### `audio_thread.rs` `handle_cmd` (lines 181-206):
```rust
AudioCmd::UpdateState { instruments, session } => {
    self.apply_state_update(instruments, session);  // replaces local copies
}
AudioCmd::UpdateMixerParams => {
    let _ = self.engine.update_all_instrument_mixer_params(&self.instruments, &self.session);
}
```

### `routing.rs` `update_all_instrument_mixer_params` (lines 542-559):
Iterates all instruments, computes effective mute (including solo logic: `any_solo && !instrument.solo`), sets level/mute/pan via OSC.

---

## Changes

### 1. `imbolc-core/src/audio/commands.rs` — Add two new commands

After `SetEqParam` (around line 92), add:

```rust
SetInstrumentMixerParams {
    instrument_id: InstrumentId,
    level: f32,
    pan: f32,
    mute: bool,
    solo: bool,
},
SetMasterParams {
    level: f32,
    mute: bool,
},
```

These carry only the mixer-relevant fields (~20 bytes per instrument vs hundreds for a full `Instrument`).

### 2. `imbolc-core/src/audio/handle.rs` — Split `flush_dirty` logic

Change `flush_dirty` (line 112) to:

```rust
pub fn flush_dirty(&mut self, state: &AppState, dirty: AudioDirty) {
    if !dirty.any() {
        return;
    }

    let needs_full_state = dirty.instruments || dirty.session || dirty.routing;
    if needs_full_state {
        self.update_state(&state.instruments, &state.session);
    }
    if dirty.piano_roll {
        self.update_piano_roll_data(&state.session.piano_roll);
    }
    if dirty.automation {
        self.update_automation_lanes(&state.session.automation.lanes);
    }
    if dirty.routing {
        let _ = self.send_cmd(AudioCmd::RebuildRouting);
    }
    if dirty.mixer_params {
        if needs_full_state {
            // Full state already sent — just trigger the engine update
            let _ = self.send_cmd(AudioCmd::UpdateMixerParams);
        } else {
            // Mixer-only change: send targeted updates (no full clone)
            self.send_mixer_params_incremental(state);
        }
    }
}
```

Add a new private method:

```rust
fn send_mixer_params_incremental(&self, state: &AppState) {
    let _ = self.send_cmd(AudioCmd::SetMasterParams {
        level: state.session.master_level,
        mute: state.session.master_mute,
    });
    for inst in &state.instruments.instruments {
        let _ = self.send_cmd(AudioCmd::SetInstrumentMixerParams {
            instrument_id: inst.id,
            level: inst.level,
            pan: inst.pan,
            mute: inst.mute,
            solo: inst.solo,
        });
    }
    // After all fields are updated on the audio thread, trigger engine apply
    let _ = self.send_cmd(AudioCmd::UpdateMixerParams);
}
```

Key: the `needs_state` calculation on line 117 must **remove** `dirty.mixer_params` — change from:
```rust
let needs_state = dirty.instruments || dirty.session || dirty.routing || dirty.mixer_params;
```
to:
```rust
let needs_full_state = dirty.instruments || dirty.session || dirty.routing;
```

### 3. `imbolc-core/src/audio/audio_thread.rs` — Handle new commands

In `handle_cmd` (the match block starting around line 102), add handlers for the two new commands. Place them near `UpdateMixerParams` (around line 204):

```rust
AudioCmd::SetMasterParams { level, mute } => {
    self.session.master_level = level;
    self.session.master_mute = mute;
}
AudioCmd::SetInstrumentMixerParams { instrument_id, level, pan, mute, solo } => {
    if let Some(inst) = self.instruments.instruments.iter_mut().find(|i| i.id == instrument_id) {
        inst.level = level;
        inst.pan = pan;
        inst.mute = mute;
        inst.solo = solo;
    }
}
```

These update the audio thread's local state copies. The subsequent `UpdateMixerParams` command (sent by `send_mixer_params_incremental`) will call `engine.update_all_instrument_mixer_params` using the now-updated local copies.

The existing `UpdateMixerParams` handler remains unchanged:
```rust
AudioCmd::UpdateMixerParams => {
    let _ = self.engine.update_all_instrument_mixer_params(&self.instruments, &self.session);
}
```

---

## Files Modified (summary)

1. `imbolc-core/src/audio/commands.rs` — add `SetInstrumentMixerParams` and `SetMasterParams` to `AudioCmd`
2. `imbolc-core/src/audio/handle.rs` — rewrite `flush_dirty` to avoid full clone on mixer-only changes; add `send_mixer_params_incremental`
3. `imbolc-core/src/audio/audio_thread.rs` — handle new commands by updating local mixer fields

**No new files.** Three files modified.

## Important Notes

- `InstrumentSnapshot` is a type alias for `InstrumentState` (defined in `snapshot.rs:5`). The audio thread's `self.instruments` field is typed `InstrumentSnapshot`.
- `SessionSnapshot` is a type alias for `SessionState` (defined in `snapshot.rs:6`). The audio thread's `self.session` field is typed `SessionSnapshot`.
- The `instruments` field on both types has a `.instruments` sub-field which is `Vec<Instrument>`.
- Solo logic (when any instrument has solo=true, all non-solo'd instruments are muted) is handled inside `engine.update_all_instrument_mixer_params`. No need to change that.
- `master_level` and `master_mute` are fields on `SessionState` (session.rs lines 67-68).
- Instrument mixer fields: `level: f32`, `pan: f32`, `mute: bool`, `solo: bool` (instrument.rs lines 1062-1066).

## Verification

1. `cargo build` — compiles with no errors
2. `cargo test` — all tests pass
3. Manual: rapidly adjust mixer level during playback — no stuttering or latency difference
