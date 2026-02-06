# Tasks Done

Historical record of completed work. Consolidated from NEXT_STEPS.md, REFACTOR1.md, REFACTOR2.md, FEATURES.md, SAMPLER.md, and UNWIRED.md.

---

## Foundation (Phases 1-10)

**Source:** NEXT_STEPS.md

### Phase 1-6: Foundation
- Ratatui UI with Graphics abstraction
- State management (strips, connections, piano roll)
- SQLite persistence
- SuperCollider OSC integration
- Bus-based audio routing

### Phase 7: Strip Architecture
- Instrument strips replacing modular rack
- Source -> Filter -> Effects -> Output chain
- ADSR envelopes
- Level/pan per strip

### Phase 8: Piano Roll
- Multi-track note editing
- Playback engine with transport
- Voice allocation (polyphony)

### Phase 9: LFO & Effects
- LFO module with shape selection
- 15 modulation targets defined
- Gate effect (tremolo/hard gate)
- FilterCutoff modulation wired

### Phase 10: Audio Input
- AudioIn source type
- Hardware input monitoring
- Test tone for debugging

---

## Bug Fixes

**Source:** REFACTOR1.md Part 1

### Automation node index calculation — FIXED
Originally quick-fixed by adding `strip.lfo.enabled` checks to positional index calculations. Properly fixed by the StripNodes refactor — `apply_automation()` uses named fields instead of positional indexing.

### Filter resonance parameter name mismatch — FIXED
Changed `"res"` to `"resonance"` in the FilterResonance automation handler.

### Sampler automation is a no-op — FIXED
Added `source_node: i32` and `spawn_time: Instant` to `VoiceChain`. Both `spawn_voice()` and `spawn_sampler_voice()` store the source node ID. SamplerRate and SamplerAmp automation handlers use `voice.source_node`.

### SetStripParam action is a stub — FIXED
Implemented the dispatch handler to update `source_params` in state and call `AudioEngine::set_source_param()`.

---

## Iteration Artifact Cleanup

**Source:** REFACTOR1.md Part 2

### Naming mismatch between docs and code — FIXED
Rewrote CLAUDE.md, `docs/architecture.md`, and `docs/ai-coding-affordances.md` to use current Strip-based naming. Removed all references to `RackState`, `RackPane`, `ModuleId`.

### `rebuild_routing()` backward-compat alias — FIXED
Removed the dead wrapper method from `AudioEngine`.

### Unused `_polyphonic` parameter — FIXED
Removed from `spawn_voice()` and cleaned up all call sites.

### Global dead code suppression — FIXED
Removed global `#![allow(dead_code)]` from `main.rs`. Removed 4 truly dead items. Added targeted `#[allow(dead_code)]` annotations on intentional API surface across 15 files. Zero warnings.

### Piano keyboard mapping duplicated 3 times — FIXED
Extracted `PianoKeyboard` struct into `src/ui/piano_keyboard.rs`. All three panes delegate to it. Removed ~200 lines of duplicated code.

### PushPane/PopPane actions not implemented — FIXED
Implemented proper pane stack in `PaneManager` with `stack: Vec<usize>`. Help pane and file browser use push/pop for modal behavior.

### SemanticColor enum unused — FIXED
Removed from `src/ui/style.rs` and its re-export from `src/ui/mod.rs`.

### Keymap::merge() never called — FIXED
Removed the unused method.

---

## Audio Engine Architecture

**Source:** REFACTOR1.md Part 3

### Structured node map — DONE
Replaced `HashMap<StripId, Vec<i32>>` with `HashMap<StripId, StripNodes>` using named fields (`source`, `lfo`, `filter`, `effects`, `output`). Eliminated all positional index calculations.

### Richer voice tracking — DONE
Added `source_node: i32` and `spawn_time: Instant` to `VoiceChain`. Enables direct `/n_set` for sampler automation and proper oldest-voice stealing via `min_by_key`.

### Configurable release cleanup — DONE
Replaced hardcoded 5-second group free with `strip.amp_envelope.release + 1.0s` margin.

### Mixer bus allocation through BusAllocator — DONE
Replaced hardcoded `bus_audio_base = 200` with `bus_allocator.get_or_alloc_audio_bus()` using sentinel StripIds. Prevents collisions.

### Stop rebuilding full graph for mixer changes — DONE
Added `update_all_strip_mixer_params()` for level/mute/solo/pan. Replaced 4 `rebuild_strip_routing()` calls.

---

## UI Engine Architecture

**Source:** REFACTOR1.md Part 4

### Extract state from panes — DONE
Moved `StripState` into top-level `AppState` owned by `main.rs`. Pane trait passes `&AppState` to `handle_input()` and `render()`. Eliminated frame-by-frame cloning. Action dispatch moved to `src/dispatch.rs`.

### Split the Action enum — DONE
Split flat 50+ variant enum into domain-specific sub-enums: `NavAction`, `StripAction`, `MixerAction`, `PianoRollAction`, `ServerAction`, `SessionAction`. Dispatch restructured into domain functions.

### Extract piano keyboard utility — DONE
Created `src/ui/piano_keyboard.rs` with `PianoKeyboard` struct and `PianoLayout` enum.

### Implement proper pane stack — DONE
Added `stack: Vec<usize>` to `PaneManager`. `push_to()` saves current index and switches; `pop()` restores. Help pane and file browser use push/pop.

---

## REFACTOR2 Completed Items

**Source:** REFACTOR2.md

### "Jump back" — DONE (R2 #5)
Backtick/tilde navigate back/forward through pane history.

### OSC screen consolidated — DONE (R2 #8)
Strip editing consolidated into a single `StripEditPane`.

### ESC exits piano/insert mode directly — DONE (R2 #23)
All three panes check `piano.is_active()` first. Escape in piano mode calls `piano.handle_escape()` without propagating. Priority chain (insert mode > piano mode > pane navigation) in place.

### Pane markup language — SKIPPED (R2 #25)
Skipped per user decision — too aspirational for current scope.

### MIDI note 0 name verified correct (R2 #10, partial)
`note_name(0)` produces "C-1" which is the standard convention. No fix needed. (The BPM display removal from the same item remains as a task.)

---

## FEATURES Completed Items

**Source:** FEATURES.md

### Number key pane navigation — DONE (FEATURES #1)
`1` = Strip, `2` = Piano Roll, `3` = Sequencer, `4` = Mixer, `5` = Server. Help moved to `?` key (global). Already implemented.

### Refactor main.rs — PARTIALLY DONE (FEATURES #6)
Action dispatch extracted to `dispatch.rs`, playback tick logic extracted to `playback.rs`. Further extraction of main.rs remains as a task.

### Linter cleanup — PARTIALLY DONE (FEATURES #7)
Global `#![allow(dead_code)]` removed, 4 dead items removed, targeted annotations added. Further dead code cleanup remains as a task (see UNWIRED inventory).

---

## Recent Feature Work

### Drum Sequencer — DONE
Implemented 16-step drum sequencer with per-pad samples, velocity, pattern length cycling, playback tick integration, and UI controls.

**Files:** `src/panes/sequencer_pane.rs`, `src/state/drum_sequencer.rs`, `src/playback.rs`, `src/dispatch.rs`, `src/ui/pane.rs`

### Sample Chopper Pane — DONE
Implemented sample chopper with waveform peaks, slice editing, and pad assignment for drum machines.

**Files:** `src/panes/sample_chopper_pane.rs`, `src/state/drum_sequencer.rs`, `src/dispatch.rs`, `src/ui/pane.rs`, `Cargo.toml`

### Audio Device Selection in Server Pane — DONE (core)
Server pane lists input/output devices, persists selection to `audio_devices.json`, and starts scsynth with the selected devices.

**Files:** `src/panes/server_pane.rs`, `src/audio/devices.rs`, `src/audio/engine.rs`

### Frame Master Meter — DONE
Added master level meter rendering in the frame border.

**Files:** `src/ui/frame.rs`

### Server Status UI — DONE
Server pane displays connection status and messages; frame no longer renders a bottom console.

**Files:** `src/panes/server_pane.rs`, `src/ui/frame.rs`
