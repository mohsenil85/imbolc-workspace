# Targeted Routing Loosening

## Context

The full modular routing refactor (TASKS_ARCH #7) would be the largest rewrite in the codebase — touching every crate — for power-user features that affect ~5% of sessions. Instead, three targeted changes address the real pain points at a fraction of the cost.

**Sidechain support is already implemented** (`SidechainComp` effect type + `sc_bus` param + routing.rs bus lookup), so this plan covers the two phases that need work.

---

## Phase A: Flexible Effect Chain Ordering

**Problem:** Signal chain is hardcoded as source → filter → EQ → effects → output. Can't put distortion before filter, or EQ after reverb.

**Approach:** Unify filter, EQ, and effects into `processing_chain: Vec<ProcessingStage>`. Keep filter/EQ as distinct types (not EffectType variants) because they have fundamentally different param structures (FilterConfig uses ModulatedParam, EQ uses fixed 12-band array, LFO targets filter specifically).

### A1. Types (`imbolc-types/src/state/instrument/mod.rs`)

- Add `ProcessingStage` enum: `Filter(FilterConfig)`, `Eq(EqConfig)`, `Effect(EffectSlot)`
- Replace `filter: Option<FilterConfig>`, `eq: Option<EqConfig>`, `effects: Vec<EffectSlot>` with `processing_chain: Vec<ProcessingStage>`
- Add convenience accessors (`filter()`, `filter_mut()`, `eq()`, `eq_mut()`, `effects()`) that scan the chain — minimizes churn in existing code
- Add `move_stage(index, direction)`, `filter_chain_index()`, `eq_chain_index()`
- Update `toggle_filter()` / `toggle_eq()` to insert/remove from chain (filter defaults to front, EQ defaults to after filter)
- Update `add_effect()` to push `ProcessingStage::Effect(...)` to end of chain

### A2. Navigation helpers (`imbolc-types/src/state/instrument/mod.rs`)

- Change `InstrumentSection` enum: `Source`, `Processing(usize)`, `Lfo`, `Envelope`
  - `Processing(idx)` identifies a chain stage by its index
- Rewrite `instrument_row_count()`: sum source rows + per-stage rows (dynamically) + LFO + envelope
- Rewrite `instrument_section_for_row()` and `instrument_row_info()`: iterate chain stages with cumulative offsets

### A3. Actions (`imbolc-types/src/action.rs`)

- Add `InstrumentAction::MoveStage(InstrumentId, usize, i8)` for moving any stage
- Update `InstrumentUpdate` to use `processing_chain` instead of separate fields
- Existing `MoveEffect` stays as convenience — finds effect's chain index internally

### A4. Dispatch (`imbolc-core/src/dispatch/instrument/`)

- **filter.rs**: Replace `instrument.filter` with `instrument.filter_mut()` (convenience accessor)
- **eq.rs**: Replace `instrument.eq` with `instrument.eq_mut()`
- **effects.rs**: `add_effect` pushes to chain; `move_effect` operates on chain index; `remove_effect` scans chain
- **crud.rs**: `handle_update` copies `processing_chain` instead of three separate fields
- Add `handle_move_stage` in effects.rs (or new stage.rs)

### A5. Audio routing (`imbolc-core/src/audio/engine/routing.rs`)

Replace the three sequential blocks (filter, EQ, effects) in `build_instrument_chain` with a single loop:

```
for stage in instrument.processing_chain.iter() {
    match stage {
        Filter(f) => { /* existing filter routing code, using current_bus */ }
        Eq(eq)    => { /* existing EQ routing code, using current_bus */ }
        Effect(e) => { /* existing per-effect routing code, using current_bus */ }
    }
}
```

`InstrumentNodes` keeps its existing fields (filter, eq, effects maps). Real-time param updates (`set_filter_param`, `set_effect_param`) look up nodes by type/id, not position — unchanged.

### A6. UI (`imbolc-ui/src/panes/instrument_edit_pane/`)

- **mod.rs**: Replace `filter`, `eq`, `effects` fields with `processing_chain`; update `set_instrument()`, `refresh_instrument()`
- **rendering.rs**: Render Source → loop over `processing_chain` stages → LFO → Envelope (dynamic order)
- **input.rs**: Section nav uses `Processing(idx)` and steps through chain; add Ctrl+Up/Down to move stages; context-sensitive keys based on stage type at cursor
- **editing.rs**: `adjust_value` matches on `Processing(idx)` then dispatches by stage type

### A7. Persistence (`imbolc-core/src/state/persistence/`)

- Add `instrument_processing_chain` table: `(instrument_id, position, stage_type, effect_id)`
- **Save**: After saving filter/EQ/effects in existing tables, also save chain order
- **Load**: If `instrument_processing_chain` exists, assemble chain from order table; otherwise fall back to legacy order (filter → EQ → effects)
- Bump `SCHEMA_VERSION`

### A8. Network (`imbolc-net`)

- `Instrument` serde changes propagate automatically (breaking protocol change, acceptable pre-1.0)
- `StatePatch` / `DirtyFlags` unchanged (operate at instrument level)

---

## Phase B: Effects on Mixer Buses

**Problem:** Buses are dumb mixers — no processing. Can't have a reverb bus, compression bus, or EQ on the mix bus.

**Approach:** Add `effects: Vec<EffectSlot>` to `MixerBus` (and `LayerGroupMixer`). Build effect chains before bus output synths in routing. No LFO modulation on bus effects initially.

### B1. Types (`imbolc-types/src/state/mixer.rs`)

- Add to `MixerBus`: `effects: Vec<EffectSlot>`, `next_effect_id: EffectId`
- Add to `LayerGroupMixer`: same two fields
- Add effect CRUD methods mirroring `Instrument`: `add_effect`, `remove_effect`, `move_effect`, `effect_by_id`, `effect_by_id_mut`
- Default: empty effects, `next_effect_id: 0`

### B2. Actions (`imbolc-types/src/action.rs`)

- Extend `BusAction`:
  - `AddEffect(u8, EffectType)`
  - `RemoveEffect(u8, EffectId)`
  - `MoveEffect(u8, EffectId, i8)`
  - `ToggleEffectBypass(u8, EffectId)`
  - `AdjustEffectParam(u8, EffectId, usize, f32)`
- Add `bus_effect_param: Option<(u8, EffectId, usize, f32)>` to `AudioDirty`
- Add `LayerGroupAction` enum with same effect variants (keyed by `u32` group_id)
- Wire `Action::LayerGroup(LayerGroupAction)` into main `Action` enum

### B3. Dispatch (`imbolc-core/src/dispatch/bus.rs`)

- Handle new `BusAction` effect variants (mirror instrument effects.rs pattern)
- Effect CRUD sets `routing = true`; param adjust sets `bus_effect_param` for targeted `/n_set`
- Undo: effect CRUD is undoable with `UndoScope::Session`; param adjust is not (real-time tweak)
- Create `dispatch/layer_group.rs` for layer group effect dispatch

### B4. Audio routing (`imbolc-core/src/audio/engine/routing.rs`)

- Add to `AudioEngine`: `bus_effect_node_map: HashMap<(u8, EffectId), i32>`, `bus_effect_order: HashMap<u8, Vec<EffectId>>` (and layer group equivalents)
- In `BuildOutputs` phase, before creating `imbolc_bus_out`:
  - Loop through `bus.effects`, create effect synths in `GROUP_PROCESSING` with bus-threaded intermediate buses
  - Feed final bus into `imbolc_bus_out` as before
- Add `set_bus_effect_param()` method for targeted updates
- Handle `bus_effect_param` in audio thread dirty processing
- Clear bus effect maps in `TearDown`
- Extract shared effect param wiring logic (SidechainComp bus translation, etc.) into a helper to avoid duplication with instrument effect chain building

### B5. Persistence (`imbolc-core/src/state/persistence/`)

- Add tables: `bus_effects`, `bus_effect_params`, `bus_effect_vst_params` (and layer group equivalents)
- Extend `save_mixer` / `load_mixer` to handle bus effects
- Use `#[serde(default)]` on new fields for backward-compatible blob deserialization
- Bump `SCHEMA_VERSION` (can share the bump with Phase A if done together)

### B6. UI (`imbolc-ui/src/panes/mixer_pane/`)

- Add bus detail view: when bus is selected and user presses Enter, show effects list with params (simpler than instrument detail — no filter/LFO/sends sections)
- Input handling: navigate effect params, add/remove/move effects, toggle bypass, adjust params
- Emit `Action::Bus(BusAction::...)` instead of `Action::Instrument(InstrumentAction::...)`
- Extend add-effect pane to accept an `EffectTarget` context (`Instrument(id)`, `Bus(id)`, `LayerGroup(id)`) so it returns the right action type

---

## Implementation Order

Recommended: Phase B first — it's self-contained, doesn't touch the Instrument struct, and delivers the highest-value feature (reverb/compression buses). Phase A is larger and more invasive.

1. Phase B (bus effects) — ~3 working sessions
2. Phase A (flexible chain) — ~5 working sessions

They share no code dependencies, so Phase A can't break Phase B. Bus effects stay as plain `Vec<EffectSlot>` (buses don't have filter/EQ, so no need for the unified chain enum).

---

## Verification

### Phase A
- `cargo test -p imbolc-types` — navigation helpers, chain manipulation
- `cargo test -p imbolc-core` — dispatch handlers, persistence round-trip, routing
- `cargo test -p imbolc-ui --bin imbolc-ui` — pane rendering, section navigation
- Manual: create instrument, add filter, move it after an effect, verify audio chain reflects the new order

### Phase B
- `cargo test -p imbolc-types` — bus effect CRUD methods
- `cargo test -p imbolc-core` — bus dispatch, persistence round-trip, routing builds effect chain for buses
- `cargo test -p imbolc-ui --bin imbolc-ui` — bus detail view rendering
- Manual: create bus, add reverb effect, send an instrument to the bus, verify reverb applies

### Both
- `cargo test` — all 594+ tests pass
- No new warnings beyond pre-existing dead_code in style.rs
