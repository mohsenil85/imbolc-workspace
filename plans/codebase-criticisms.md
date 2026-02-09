# Codebase Criticisms

Substantive issues worth addressing, filtered for a solo-dev DAW project. Not architecture-astronaut suggestions — real bugs, real duplication, real friction.

---

## 1. AudioDirty last-write-wins loses updates

**Where:** `AudioDirty` struct in `imbolc-types/src/action.rs`

Targeted param fields are `Option<(...)>` to preserve `Copy`:
```rust
pub filter_param: Option<(InstrumentId, FilterParamKind, f32)>,
pub effect_param: Option<(InstrumentId, EffectId, usize, f32)>,
pub lfo_param: Option<(InstrumentId, LfoParamKind, f32)>,
```

If two filter params change in the same tick (cutoff + resonance), only the second survives. Merge uses last-write-wins. In practice mostly safe because UI events are one-per-frame, but a real data loss bug waiting to happen with MIDI CC input or automation playback sending multiple params simultaneously.

**Fix options:**
- Remove `Copy`, use `Vec<Update>` (requires refactoring ~30 call sites)
- Use a small fixed-size array: `[Option<(...)>; 4]` (still Copy, handles reasonable burst)
- Accept the limitation but add a debug assertion that catches overwrites

---

## 2. Action projection has no compile-time link to dispatch

**Where:** `imbolc-core/src/audio/action_projection.rs` vs `imbolc-core/src/dispatch/`

`project_action()` manually reimplements state mutations from dispatch handlers. If you update a dispatch handler and forget the projection, the audio thread silently diverges. No test matrix verifies parity.

**Fix:** Add a test for every projectable action: dispatch it, project it, assert final states match. Could be a macro-generated test matrix.

---

## 3. `is_running()` guards silently swallow operations

**Where:** Throughout `imbolc-core/src/dispatch/instrument/` — playback.rs, effects.rs, eq.rs, etc.

Pattern:
```rust
if audio.is_running() {
    // do the thing
}
// else: silently return DispatchResult::none()
```

User presses a key, nothing happens, no feedback. Should at minimum return a status message.

---

## 4. Effect management code duplicated 3x

**Where:**
- `Instrument` in `imbolc-types/src/state/instrument/mod.rs`
- `MixerBus` in `imbolc-types/src/state/` (bus module)
- `LayerGroupMixer` in `imbolc-types/src/state/` (layer group module)

Each has its own `add_effect()`, `remove_effect()`, `move_effect()`, `effect_by_id()`, `next_effect_id` counter. ~100 lines copy-pasted across three types.

**Fix:** Extract an `EffectChain` struct that all three embed. Consistent by construction, single place to add new effect chain logic.

---

## 5. Instrument is a 28-field god struct

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

A drum pad carries `sampler_config`, `arpeggiator`, `chord_shape`, `vst_param_values`, `vst_state_path`. An audio-in carries effects and filter it'll never use. Every new feature adds another `Option<T>` field. Makes persistence migrations heavier and constructors noisy.

Not urgent — works fine at current scale — but worth noting as friction grows with each new instrument feature.

---

## 6. Current branch has compilation errors on main

**Where:**
- `imbolc-core/src/dispatch/side_effects.rs:266` — calls `set_layer_group_eq_param` which doesn't exist (should be `set_eq_param`)
- `imbolc-core/src/state/persistence/load.rs:268` — missing `eq` field in `LayerGroupMixer` initializer

From in-progress layer-group EQ work that's half-landed.

---

## Priority

| # | Issue | Severity | Effort |
|---|-------|----------|--------|
| 6 | Compilation errors | **High** (blocks build) | Minutes |
| 1 | AudioDirty data loss | **Medium** (latent bug) | Hours |
| 2 | Projection parity | **Medium** (silent correctness) | Hours |
| 3 | Silent `is_running()` | **Low** (UX annoyance) | Small |
| 4 | Effect chain duplication | **Low** (maintenance tax) | Medium |
| 5 | Instrument god struct | **Low** (scaling friction) | Large |
