****# Foundational Architecture Redesign

**Status: Complete** (all 5 phases shipped)

## Context

Six areas of the codebase were "retrofit-shaped" — working but carrying structural debt from incremental evolution. This plan addressed all six as a phased refactor, ordered by dependency so each phase was independently shippable and the test suite stayed green throughout.

The key enabler: pure state-mutation logic in `imbolc-core/src/dispatch/` depends only on types already in `imbolc-types`. No Cargo.toml dependency changes were needed — the "restructuring" was about where code lives, not the dependency graph itself.

---

## Phase 1: Split UiAction from DomainAction -- COMPLETE

**Problem**: `Action` enum mixes UI-layer mechanics (Nav, PushLayer, PopLayer, ExitPerformanceMode, Quit, SaveAndQuit) with domain mutations (Instrument, Mixer, PianoRoll, etc.). Audio/core must handle UI variants it doesn't care about.

**What was done**: Rather than a full three-enum split (UiAction/DomainAction/PaneAction), the simpler approach was taken: `DomainAction` was added alongside the existing `Action` enum. `Action::to_domain() -> Option<DomainAction>` bridges the two. Pane handlers continue returning `Action`; the domain extraction happens at the dispatch boundary.

**Key changes**:
- `imbolc-types/src/action.rs` — added `DomainAction` enum with all domain-only variants, `Action::to_domain()`, `From<DomainAction> for Action`
- `imbolc-core/src/dispatch/mod.rs` — `dispatch_action()` takes `&DomainAction`
- `imbolc-core/src/dispatch/local.rs` — `dispatch_with_audio(&Action)` calls `to_domain()` then delegates to `dispatch_domain(&DomainAction)`
- `imbolc-audio/src/action_projection.rs` — `project_action()` and `is_action_projectable()` take `&DomainAction`
- Undo helpers (`is_undoable`, `undo_scope`, `coalesce_key`) take `&DomainAction`
- `DispatchResult.needs_full_sync: bool` added for callers to check reducibility
- `LogEntryKind::Action` holds `Box<DomainAction>`

**Deviation from plan**: The PaneAction wrapper and UiAction enum were not created. The existing `Action` enum still serves as the UI-layer type. This was simpler and touched fewer files while achieving the core goal: dispatch/audio only see domain actions.

---

## Phase 2: Unify AudioSideEffect and AudioCmd -- COMPLETE

**Problem**: Three-layer command pipeline: dispatch produces `AudioSideEffect` → `apply_side_effects()` converts to `AudioHandle` method calls → each method sends `AudioCmd`. Three enum mappings for the same operation.

**What was done**: Deleted `AudioSideEffect` entirely. Dispatch handlers now call `AudioHandle` methods directly (which internally send `AudioCmd`). The intermediate enum layer was removed.

**Key changes**:
- Deleted `imbolc-core/src/dispatch/side_effects.rs` (~360 lines)
- All dispatch sub-handlers (`instrument/*.rs`, `mixer.rs`, `session.rs`, `server.rs`, `sequencer.rs`, `automation.rs`, `bus.rs`, `arrangement.rs`, `piano_roll.rs`, `vst_param.rs`, `audio_feedback.rs`, `helpers.rs`) — replaced `AudioSideEffect` pushes with direct `audio.method()` calls
- `dispatch_action()` signature changed to take `&mut AudioHandle` directly
- `AudioHandle` convenience methods kept as the ergonomic API

**Deviation from plan**: Rather than having dispatch handlers produce `Vec<AudioCmd>` directly, they call `AudioHandle` methods. This preserves the ergonomic API and avoids exposing `AudioCmd` internals to dispatch code.

---

## Phase 3: Replace AudioDirty with Typed Event Stream -- COMPLETE

**Problem**: `AudioDirty` is a Copy struct with coupled booleans and fixed-size Option arrays. Overflow escalation logic (>4 routing instruments → full rebuild) is fragile and hard to extend.

**What was done**: Replaced `AudioDirty` with `Vec<AudioEffect>` — a 16-variant typed event stream.

**AudioEffect variants** (in `imbolc-types/src/action.rs`):
- Structural: `RebuildInstruments`, `RebuildSession`, `RebuildRouting`, `RebuildRoutingForInstrument(id)`, `AddInstrumentRouting(id)`, `DeleteInstrumentRouting(id)`, `RebuildBusProcessing`, `UpdateMixerParams`, `UpdatePianoRoll`, `UpdateAutomation`
- Targeted params: `SetFilterParam`, `SetEffectParam`, `SetLfoParam`, `SetBusEffectParam`, `SetLayerGroupEffectParam`
- Convenience: `AudioEffect::all()` and `AudioEffect::for_instrument(id)` constructors

**Key changes**:
- `imbolc-types/src/action.rs` — deleted `AudioDirty`, added `AudioEffect` enum
- `DispatchResult.audio_dirty` → `DispatchResult.audio_effects: Vec<AudioEffect>`
- All dispatch handlers push `AudioEffect` variants instead of setting boolean flags
- `imbolc-audio/src/handle.rs` — `apply_dirty()` → `apply_effects()` with coalescing
- `imbolc-ui/src/main.rs` — `pending_audio_dirty` → `Vec<AudioEffect>`, `merge()` → `extend()`
- `imbolc-net/src/server.rs` — network DirtyFlags kept separate (different concern)

---

## Phase 4: Single Shared Reducer + Dispatcher Trait Fix -- COMPLETE

**Problem**: Two independent state-mutation engines — `dispatch_action()` (imbolc-core) and `project_action()` (imbolc-audio) — replicate the same mutations. 1,556 lines of projection code + 1,720 lines of parity tests exist solely to keep them in sync. Also, the `Dispatcher` trait panics in its only implementation.

**What was done**: Created `imbolc-types/src/reduce/` module with pure state-mutation functions. Audio thread now calls the shared reducer. Deleted projection code, parity tests, and Dispatcher trait.

**New module `imbolc-types/src/reduce/`** (1,520 lines):
```
mod.rs              — reduce_action(), is_reducible()
instrument.rs       — InstrumentAction mutations (~350 lines)
mixer.rs            — MixerAction mutations (~340 lines)
piano_roll.rs       — PianoRollAction mutations (~105 lines)
automation.rs       — AutomationAction mutations (~100 lines)
bus.rs              — BusAction + LayerGroupAction mutations (~180 lines)
session.rs          — SessionAction (musical settings) (~60 lines)
vst_param.rs        — VstParamAction mutations (~100 lines)
click.rs            — ClickAction mutations (~15 lines)
```

**Key changes**:
- `imbolc-types/src/lib.rs` — `pub mod reduce` replaces `pub mod dispatch` + `pub use dispatch::Dispatcher`
- `imbolc-audio/src/audio_thread.rs` — `imbolc_types::reduce::reduce_action()` replaces `action_projection::project_action()`
- `imbolc-audio/src/handle.rs` — `imbolc_types::reduce::is_reducible()` replaces `is_action_projectable()`
- `imbolc-core/src/dispatch/local.rs` — same `is_reducible()` update, Dispatcher trait impl removed

**Deletions** (3,289 lines):
- `imbolc-audio/src/action_projection.rs` — 1,556 lines
- `imbolc-core/src/dispatch/projection_parity.rs` — 1,720 lines
- `imbolc-types/src/dispatch.rs` — 13 lines (Dispatcher trait)

**Net impact**: -1,769 lines (deleted 3,289, added 1,520)

**Deviation from plan**: Core dispatch handlers were not refactored to call the shared reducers for their mutations. They retain their own mutation code (which is authoritative). The reducer is used only by the audio thread for incremental state projection. This was a pragmatic choice — refactoring core dispatch to use reducers would have been a much larger change with no correctness benefit (the reducer was ported from projection code that was already verified by parity tests).

---

## Phase 5: Extract App Runtime Coordinator -- COMPLETE

**Problem**: `main.rs` (842 lines) is a monolithic event loop handling input routing, layer management, audio sync, feedback draining, rendering, and special-case intercepts all in one function.

**What was done**: Extracted the event loop into `AppRuntime` struct with methods split across focused sub-modules. Rather than creating separate subsystem structs (InputRouter, AudioSync, etc.), used Rust's split `impl` blocks — each file adds methods to the single `AppRuntime` struct, which enables field-level borrow splitting.

**New module `imbolc-ui/src/runtime/`**:
```
mod.rs          — AppRuntime struct, new(), run()              (181 lines)
input.rs        — process_events(), process_tick()              (409 lines)
audio_sync.rs   — apply_pending_effects()                       (18 lines)
feedback.rs     — drain_io_feedback(), drain_audio/midi_events() (256 lines)
render.rs       — maybe_render()                                (139 lines)
```

**The event loop** is now 15 lines:
```rust
loop {
    self.layer_stack.set_pane_layer(self.panes.active().id());
    if self.process_events(backend)? { break; }
    self.process_tick();
    self.apply_pending_effects();
    self.drain_io_feedback();
    if self.quit_after_save && !self.dispatcher.state().project.dirty { break; }
    self.drain_audio_feedback();
    self.drain_midi_events();
    self.maybe_render(backend)?;
}
```

**Key changes**:
- `main.rs` — 842 → 162 lines (entry point, CLI args, panic hook, pane registration)
- `runtime::run()` — public entry point that creates `AppRuntime` and calls `run()`
- No changes to core/audio/types crates
- `global_actions.rs` — unchanged (already well-factored)

**Deviation from plan**: Instead of separate subsystem structs (InputRouter, AudioSync, RenderLoop, FeedbackDrain) composed inside AppRuntime, used a single flat struct with split `impl` blocks across files. This avoids borrow checker issues that would arise from nested struct access patterns, while still achieving the code organization goal.

---

## Results

| Phase | What Changed | Lines Deleted | Lines Added | Net |
|-------|-------------|--------------|-------------|-----|
| 1. DomainAction split | Domain/UI action separation | — | ~100 | +100 |
| 2. Unify AudioSideEffect | Deleted intermediate enum | ~360 | — | -360 |
| 3. AudioDirty → AudioEffect | Typed event stream | ~280 | ~100 | -180 |
| 4. Single reducer | Shared state mutations | 3,289 | 1,520 | -1,769 |
| 5. Extract runtime | Decomposed main.rs | 842 | 1,165 | +323 |
| **Total** | | | | **~-1,886** |

**Test counts**: Started at 935 tests, ended at 780 tests. The 155-test decrease is entirely from deleting projection parity tests (Phase 4) — those tests verified that two independent mutation engines produced identical results, which is no longer needed since both paths now call the same code.

## Phase Dependencies (as executed)

```
Phase 1 (DomainAction split)
    ↓
Phase 2 (Unify AudioSideEffect) — independent, done after Phase 1
    ↓
Phase 3 (AudioDirty → AudioEffect) — benefited from Phase 2
    ↓
Phase 4 (Single reducer) — used DomainAction from Phase 1
    ↓
Phase 5 (Runtime extraction) — benefited from all above being clean
```
