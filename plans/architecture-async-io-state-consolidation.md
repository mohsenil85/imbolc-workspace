# Architectural Refactoring Plan

**Status:** FUTURE
**Last Updated:** 2025-02-06

Six issues ordered by priority. Phases are sequenced for dependency and risk: high-impact/low-risk first.

Recommended order: **1 → 2 → 3 → 4 → 5 → 6**

Phases 1, 2, 3 have no mutual dependencies and can be interleaved.

---

## Phase 1: Async I/O for Save/Load/Import

**Issue:** Save/Load/ImportCustomSynthDef block the UI thread with SQLite + filesystem + synthdef compilation.
**Strategy:** `std::thread::spawn` for I/O, completion via new MPSC channel. Same pattern as existing audio thread.

### Changes

**`imbolc-core/src/action.rs`** — Add `IoFeedback` enum:
```rust
pub enum IoFeedback {
    SaveComplete { result: Result<String, String> },
    LoadComplete { result: Result<(SessionState, InstrumentState, String), String> },
    ImportSynthDefComplete { result: Result<(CustomSynthDef, String), String> },
}
```

**`imbolc-core/src/dispatch/session.rs`** — Replace sync I/O with thread spawns:
- `dispatch_session` gains `io_tx: &Sender<IoFeedback>` parameter
- `Save`: clone state, spawn thread, do SQLite write there, send `IoFeedback::SaveComplete`
- `Load`: spawn thread, do SQLite read there, send `IoFeedback::LoadComplete`
- `ImportCustomSynthDef`: spawn thread for file read + parse + copy + compile, send result back
- Each case immediately returns a "Saving..."/"Loading..." status

**`imbolc-core/src/dispatch/mod.rs`** — Pass `io_tx` through `dispatch_action` to session handler.

**`src/main.rs`** — Create `(io_tx, io_rx)` channel. After audio feedback drain, drain `io_rx`:
- `SaveComplete`: set project name, show status
- `LoadComplete`: replace `state.session`/`state.instruments`, set all audio dirty flags, queue VST restores
- `ImportSynthDefComplete`: add synthdef to state, send audio load command

### Files
- `imbolc-core/src/action.rs` — add `IoFeedback`
- `imbolc-core/src/dispatch/session.rs` — replace sync handlers
- `imbolc-core/src/dispatch/mod.rs` — thread `io_tx`
- `src/main.rs` — create channel, drain feedback

### Test
- `cargo test` passes
- Manual: Ctrl+S during playback produces no audio hiccup or UI freeze

---

## Phase 2: Consolidate State Mutation into Dispatch

**Issue:** State mutated in 3 places (dispatch, audio feedback drain, global action handler).
**Strategy:** Wrap audio feedback in `Action::AudioFeedback(...)`, dispatch it like any other action.

### Changes

**`imbolc-core/src/action.rs`** — Add:
```rust
pub enum Action {
    // ... existing ...
    AudioFeedback(AudioFeedbackAction),
}

pub enum AudioFeedbackAction {
    PlayheadPosition(u32),
    BpmUpdate(f32),
    DrumSequencerStep { instrument_id: InstrumentId, step: usize },
    ServerStatus { status: ServerStatus, message: String, server_running: bool },
    RecordingState { is_recording: bool, elapsed_secs: u64 },
    RecordingStopped(PathBuf),
    CompileResult(Result<String, String>),
    PendingBufferFreed,
    VstParamsDiscovered { ... },
    VstStateSaved { ... },
}

pub enum SessionAction {
    // ... existing ...
    ToggleMasterMute,
}
```

**`imbolc-core/src/dispatch/audio_feedback.rs`** (new) — All feedback→state logic extracted from main.rs:
- `dispatch_audio_feedback(action, state, audio) -> DispatchResult`
- Handles all variants: playhead, BPM, drum step, recording, VST params, etc.
- `PendingBufferFreed` computes waveform peaks and stores in `state.recorded_waveform_peaks`

**`imbolc-core/src/dispatch/mod.rs`** — Wire `Action::AudioFeedback` to new handler.

**`imbolc-core/src/dispatch/session.rs`** — Handle `ToggleMasterMute`.

**`imbolc-core/src/state/mod.rs`** — Add `recorded_waveform_peaks: Option<Vec<f32>>` to `AppState`.

**`src/main.rs`** — Replace 100-line feedback drain block with:
```rust
for feedback in audio.drain_feedback() {
    let action = Action::AudioFeedback(feedback.into());
    let r = dispatch::dispatch_action(&action, &mut state, &mut audio);
    pending_audio_dirty.merge(r.audio_dirty);
    apply_dispatch_result(r, &mut state, &mut panes, &mut app_frame);
}
```
Move `master_mute` from `handle_global_action` to return `Action::Session(SessionAction::ToggleMasterMute)`.

### Files
- `imbolc-core/src/action.rs` — add enums
- `imbolc-core/src/dispatch/audio_feedback.rs` — new handler
- `imbolc-core/src/dispatch/mod.rs` — wire up
- `imbolc-core/src/dispatch/session.rs` — ToggleMasterMute
- `imbolc-core/src/state/mod.rs` — waveform peaks field
- `src/main.rs` — simplify feedback drain + global handler

### Test
- `cargo test` passes
- Unit test: `dispatch_audio_feedback(PlayheadPosition(100), ...)` → verify `state.session.piano_roll.playhead == 100`
- Manual: playback, recording, VST discovery all behave identically

---

## Phase 3: Extract Audio Thread + Sorted Notes

**Issue (6):** `handle.rs` is 1372 lines mixing AudioHandle, AudioThread, playback, drum sequencer, arpeggiator.
**Issue (4):** `tick_playback` does O(n) full scan of all notes every 1ms tick.

### 3A: File Decomposition

Extract from `handle.rs`:

| New file | Content |
|----------|---------|
| `audio_thread.rs` | `AudioThread` struct, `run()`, `drain_commands()`, `handle_cmd()` |
| `playback.rs` | `tick_playback()` as standalone function |
| `drum_tick.rs` | `tick_drum_sequencer()` as standalone function |
| `arpeggiator_tick.rs` | `tick_arpeggiator()` as standalone function |

`handle.rs` retains only `AudioHandle` (~250 lines).

Each tick function takes the fields it needs as parameters (avoids partial self-borrow issues). Pure mechanical extraction.

### 3B: Sorted Notes + Binary Search

**`imbolc-core/src/state/piano_roll.rs`** — Keep `notes` sorted by tick:
- `toggle_note`: use `partition_point` for insertion position
- Add `insert_note_sorted` helper

**`imbolc-core/src/audio/playback.rs`** — Replace linear scan:
```rust
// Before (O(n)):
for note in &track.notes {
    if note.tick >= scan_start && note.tick < scan_end { ... }
}
// After (O(log n + k)):
let start = track.notes.partition_point(|n| n.tick < scan_start);
let end = track.notes.partition_point(|n| n.tick < scan_end);
for note in &track.notes[start..end] { ... }
```

**`imbolc-core/src/state/persistence/load.rs`** — Sort notes after loading (safety).

### Files
- `imbolc-core/src/audio/handle.rs` — keep AudioHandle only
- `imbolc-core/src/audio/audio_thread.rs` — new
- `imbolc-core/src/audio/playback.rs` — new, with binary search
- `imbolc-core/src/audio/drum_tick.rs` — new
- `imbolc-core/src/audio/arpeggiator_tick.rs` — new
- `imbolc-core/src/audio/mod.rs` — add module declarations
- `imbolc-core/src/state/piano_roll.rs` — sorted insertion
- `imbolc-core/src/state/persistence/load.rs` — sort after load

### Test
- `cargo test` passes
- Add test: insert notes in random tick order, verify sorted
- Manual: playback timing unchanged

---

## Phase 4: Incremental State Diffs for Mixer

**Issue:** `flush_dirty` clones entire `InstrumentState + SessionState` even for a single mixer knob turn.
**Strategy:** When only `mixer_params` is dirty, send targeted commands instead of full snapshot.

### Changes

**`imbolc-core/src/audio/commands.rs`** — Add:
```rust
AudioCmd::SetInstrumentMixerParams {
    instrument_id: InstrumentId,
    level: f32, pan: f32, mute: bool, solo: bool,
},
AudioCmd::SetMasterParams {
    level: f32, mute: bool,
},
```

**`imbolc-core/src/audio/handle.rs`** — Smarter `flush_dirty`:
```rust
let needs_full_state = dirty.instruments || dirty.session || dirty.routing;
if needs_full_state {
    self.update_state(...);  // full clone
} else if dirty.mixer_params {
    self.send_mixer_params_only(state);  // no clone — targeted commands
}
```

**`imbolc-core/src/audio/audio_thread.rs`** — Handle `SetInstrumentMixerParams` and `SetMasterParams` by updating local mixer state and calling engine methods.

### Scope Note

This captures the most common case (mixer knob turns are the most frequent dirty flag during normal use). Full diff-based updates for instrument params, effects, etc. would require a much larger change to the dirty tracking system and is deferred.

### Files
- `imbolc-core/src/audio/commands.rs` — add commands
- `imbolc-core/src/audio/handle.rs` — split flush_dirty logic
- `imbolc-core/src/audio/audio_thread.rs` — handle new commands

### Test
- `cargo test` passes
- Manual: rapidly adjust mixer level during playback — no stuttering or latency

---

## Phase 5: Code Organization (Mega-Files)

**Issue:** `main.rs`, persistence `load.rs`/`save.rs`, `instrument.rs` centralize unrelated responsibilities.

### 5A: Extract global action handler from main.rs

**`src/global_actions.rs`** (new) — Move `handle_global_action`, `select_instrument`, `sync_piano_roll_to_selection`, `sync_instrument_edit`, `sync_pane_layer`, `InstrumentSelectMode` from main.rs.

### 5B: Split persistence into domain modules

```
persistence/
  save/
    instruments.rs, mixer.rs, piano_roll.rs, automation.rs, sequencer.rs, plugins.rs
  load/
    instruments.rs, mixer.rs, piano_roll.rs, automation.rs, sequencer.rs, plugins.rs
```

### 5C: Split instrument.rs into submodules

```
state/instrument/
  mod.rs, source_type.rs, filter.rs, effect.rs, lfo.rs, envelope.rs
```

All pure mechanical extraction — no behavioral changes.

### Files
- `src/main.rs` — shrink by extracting to `src/global_actions.rs`
- `imbolc-core/src/state/persistence/` — split save.rs and load.rs
- `imbolc-core/src/state/instrument.rs` → `instrument/` module

### Test
- `cargo test` passes (no behavioral change)

---

## Phase 6: Action String Validation (Quick Win)

**Issue:** Stringly-typed actions fail silently at runtime.
**Strategy:** Validate at startup instead of full type-level refactor (which would require rewriting all 20 panes + keybinding format).

### Changes

**`src/ui/keybindings.rs`** — Add startup validation:
```rust
pub fn validate_keybindings(known: &[&str], layers: &[Layer]) {
    for binding in all_bindings(layers) {
        if !known.contains(&binding.action) {
            eprintln!("Warning: unknown action '{}'", binding.action);
        }
    }
}
```

Collect `KNOWN_ACTIONS` from all pane keymaps + global handler strings.

Replace `panic!` on unknown key names with `eprintln!` warning + skip.

### Why Not Full Type Refactor

Converting to enum-based actions would require:
- Changing TOML format (breaking user configs) or adding conversion layer
- Rewriting `Pane::handle_action` signature and all 20+ implementations
- Per-pane action enums or a mega-enum
- High effort relative to benefit — the current system works, failures are benign (`Action::None`)

### Files
- `src/ui/keybindings.rs` — add validation, soften panics

### Test
- `cargo test` passes
- Add keybinding with typo, verify warning printed at startup

---

## Verification Plan

After each phase:
1. `cargo build` — compiles
2. `cargo test --bin imbolc` — unit tests pass
3. `cargo test` — all tests pass
4. Manual smoke test: launch app, play sequence, save/load, adjust mixer, verify no regressions
