# Targeted Routing Loosening

## Context

The full modular routing refactor (TASKS_ARCH #7) would be the largest rewrite in the codebase — touching every crate — for power-user features that affect ~5% of sessions. Instead, three targeted changes address the real pain points at a fraction of the cost.

**Sidechain support is already implemented** (`SidechainComp` effect type + `sc_bus` param + routing.rs bus lookup), so this plan covers the two phases that need work.

---

## Phase A: Flexible Effect Chain Ordering

**Problem:** Signal chain is hardcoded as source → filter → EQ → effects → output. Can't put distortion before filter, or EQ after reverb.

**Approach:** Unify filter, EQ, and effects into `processing_chain: Vec<ProcessingStage>`. Keep filter/EQ as distinct types (not EffectType variants) because they have fundamentally different param structures (FilterConfig uses ModulatedParam, EQ uses fixed 12-band array). LFO modulation targets a specific filter by chain index when multiple filters exist.

### Behavioral Guarantees

- **Multiple Filters allowed.** Each `toggle_filter()` call inserts a new Filter stage at index 0 (or removes the one at cursor). Useful for serial filter stacking.
- **At most one EQ.** Enforced in `toggle_eq()` (refuse if chain already contains an EQ) and validated in `processing_chain` helpers. Convenience accessors `eq()`/`eq_mut()` return the single instance.
- Toggling a stage off removes it; toggling on inserts at its default position. Relative order of other stages is preserved.
- `move_stage()` is a stable reorder — no other stage indices shift unexpectedly.
- Pre-insert sends tap `source_out_bus` (before any processing stage, regardless of chain order). Post-insert sends tap the final bus after all processing stages.

### A1. Types (`imbolc-types/src/state/instrument/mod.rs`)

- Add `ProcessingStage` enum: `Filter(FilterConfig)`, `Eq(EqConfig)`, `Effect(EffectSlot)`
- Replace `filter: Option<FilterConfig>`, `eq: Option<EqConfig>`, `effects: Vec<EffectSlot>` with `processing_chain: Vec<ProcessingStage>`
- Add convenience accessors that scan the chain — minimizes churn in existing code:
  - `filters()` / `filters_mut()` → iterators over all Filter stages (multiple allowed)
  - `eq()` / `eq_mut()` → `Option<&EqConfig>` / `Option<&mut EqConfig>` (single instance)
  - `effects()` → iterator over Effect stages
- Add `move_stage(index, direction)`, `filter_chain_index()`, `eq_chain_index()`
- Update `toggle_filter()`: inserts a new `Filter(FilterConfig::default())` at index 0 (or at cursor position if UI provides one). Multiple filters are allowed — serial stacking is a valid use case.
- Update `toggle_eq()`: inserts EQ after the last Filter if any exist, else index 0. **Enforces single-instance** — if chain already contains an EQ, toggle removes it instead of inserting a second.
- Update `add_effect()` to push `ProcessingStage::Effect(...)` to end of chain

### A2. Navigation helpers (`imbolc-types/src/state/instrument/mod.rs`)

- Change `InstrumentSection` enum: `Source`, `Processing(usize)`, `Lfo`, `Envelope`
  - `Processing(idx)` identifies a chain stage by its index
- Rewrite `instrument_row_count()`: sum source rows + per-stage rows (dynamically) + LFO + envelope
- Rewrite `instrument_section_for_row()` and `instrument_row_info()`: iterate chain stages with cumulative offsets
- Add table-driven tests for `instrument_section_for_row()` and `instrument_row_info()` covering chains with different stage orders, empty chains, and chains with all three stage types

### A3. Actions (`imbolc-types/src/action.rs`)

- Add `InstrumentAction::MoveStage(InstrumentId, usize, i8)` for moving any stage
- Update `InstrumentUpdate` to use `processing_chain` instead of separate fields
- Remove `MoveEffect` — `MoveStage` subsumes it. Having two ways to move effects is a maintenance burden with no user-facing benefit; callers can find the chain index via `effect_chain_index(id)`.

### A4. Dispatch (`imbolc-core/src/dispatch/instrument/`)

- **filter.rs**: Filter dispatch needs a target index since multiple filters can exist. Current single-filter dispatch uses `instrument.filter` — replace with `instrument.filter_at_mut(chain_idx)` or route through `Processing(idx)` from the action.
- **eq.rs**: Replace `instrument.eq` with `instrument.eq_mut()`
- **effects.rs**: `add_effect` pushes to chain; `remove_effect` scans chain
- **crud.rs**: `handle_update` copies `processing_chain` instead of three separate fields
- Add `handle_move_stage` in effects.rs (or new stage.rs). `MoveStage` is undoable with `UndoScope::Session` (structural change, undo feels natural).
- **Routing rebuild trigger:** Any stage insert/remove/move must set `routing = true` in dirty flags, same as existing effect add/remove. This is critical for correctness — the audio engine must rebuild the SC node chain to reflect the new order.

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

**Node ordering:** `InstrumentNodes::all_node_ids()` currently returns nodes in fixed order (source → lfo → filter → eq → effects → output). With flexible ordering, the processing segment must return nodes in actual chain order. SC node ordering within a group determines signal flow — wrong order means audio glitches. Update `all_node_ids()` to iterate `processing_chain` for the middle segment.

**Send tap points:** Pre-insert sends tap `source_out_bus`, which is before all processing stages regardless of chain order. Add a comment in routing.rs confirming this intent so readers don't confuse "pre-insert" with "before filter specifically."

### A6. UI (`imbolc-ui/src/panes/instrument_edit_pane/`)

- **mod.rs**: Replace `filter`, `eq`, `effects` fields with `processing_chain`; update `set_instrument()`, `refresh_instrument()`
- **rendering.rs**: Render Source → loop over `processing_chain` stages → LFO → Envelope (dynamic order)
- **input.rs**: Section nav uses `Processing(idx)` and steps through chain; add Ctrl+Up/Down to move stages; context-sensitive keys based on stage type at cursor
- **editing.rs**: `adjust_value` matches on `Processing(idx)` then dispatches by stage type
- **Cursor stability after moves:** When `MoveStage` shifts chain indices, the UI must follow the moved stage. After dispatching the move, recalculate `selected_row` from the stage's new chain index (not old row number). Otherwise the cursor stays on the same row and points at a different stage.

### A7. Persistence (`imbolc-core/src/state/persistence/`)

- Add `instrument_processing_chain` table: `(instrument_id, position, stage_type, effect_id)`
- Add `instrument_eq_bands` table: `(instrument_id, band_index, freq, gain, q)` — EQ band data is not currently persisted (only `eq_enabled` flag exists). This plan owns closing that gap.
- **Save**: Save filter/EQ/effects in their respective tables, plus chain order in `instrument_processing_chain`, plus EQ band data in `instrument_eq_bands`
- **Load**: If `instrument_processing_chain` exists, assemble chain from order table; otherwise fall back to legacy order (filter → EQ → effects). Use `#[serde(default)]` on `processing_chain` for backward-compatible blob deserialization.
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
- Add `bus_effect_params: Vec<(u8, EffectId, usize, f32)>` to `AudioDirty` — Vec not Option, since rapid automation or multi-param controls can emit multiple changes per tick. If this proves unnecessary, can simplify to Option later (easier to narrow than widen).
- Add `LayerGroupAction` enum with same effect variants (keyed by `u32` group_id). Consider a shared `MixerEffectAction<K>` generic over the key type (`u8` for buses, `u32` for layer groups) to avoid duplicating five identical variants.
- Wire `Action::LayerGroup(LayerGroupAction)` into main `Action` enum

### B3. Dispatch (`imbolc-core/src/dispatch/bus.rs`)

- Handle new `BusAction` effect variants (mirror instrument effects.rs pattern)
- Effect CRUD sets `routing = true`; param adjust pushes to `bus_effect_params` for targeted `/n_set`
- Undo: effect CRUD is undoable with `UndoScope::Session`; param adjust is not (real-time tweak)
- Create `dispatch/layer_group.rs` for layer group effect dispatch. If using the generic `MixerEffectAction<K>` approach from B2, share dispatch logic via a trait-bounded handler function to avoid duplicating the same match arms.

### B4. Audio routing (`imbolc-core/src/audio/engine/routing.rs`)

- Add to `AudioEngine`: `bus_effect_node_map: HashMap<(u8, EffectId), i32>` (and layer group equivalent). **Drop `bus_effect_order`** — the canonical order already lives in `bus.effects`; a separate map risks divergence. Read order from the state when building the chain.
- **Bus effects need their own SC group** (e.g., `GROUP_BUS_PROCESSING`) to avoid node ordering conflicts with instrument chains in `GROUP_PROCESSING`. Bus effects must run after all instrument chains have mixed into the bus.
- In `BuildOutputs` phase, before creating `imbolc_bus_out`:
  - Loop through `bus.effects`, create effect synths in `GROUP_BUS_PROCESSING` with bus-threaded intermediate buses
  - Feed final bus into `imbolc_bus_out` as before
- **SidechainComp on buses:** Decide how `sc_bus` resolves. The instrument path uses routing.rs bus lookup to find another instrument's output bus. On buses, sidechain sources could be other buses or instrument outputs. Call out the supported sources explicitly (recommend: instrument outputs only for now, same lookup path).
- Add `set_bus_effect_param()` method for targeted updates
- Handle `bus_effect_params` in audio thread dirty processing
- Clear bus effect maps in `TearDown`
- Extract shared effect param wiring logic (SidechainComp bus translation, etc.) into a helper to avoid duplication with instrument effect chain building

### ~~B5. Persistence (`imbolc-core/src/state/persistence/`)~~ ✓

**Done.** Bumped `SCHEMA_VERSION` 8→9. Added 6 tables: `bus_effects`,
`bus_effect_params`, `bus_effect_vst_params`, `layer_group_effects`,
`layer_group_effect_params`, `layer_group_effect_vst_params`. Extracted
generic `save_effects_to()` / `load_effects_from()` /
`load_effect_params_from()` helpers (instrument effects refactored to
delegate). `table_exists()` check provides backward compat with v8
schemas. `recalculate_next_effect_id()` on load. 2 new round-trip tests
(`round_trip_bus_effects`, `round_trip_layer_group_effects`). 258 tests
pass, 0 warnings.

**Files:** `imbolc-core/src/state/persistence/schema.rs`,
`save.rs`, `load.rs`, `tests.rs`.

### B6. UI (`imbolc-ui/src/panes/mixer_pane/`)

- Add bus detail view: when bus is selected and user presses Enter, show effects list with params (simpler than instrument detail — no filter/LFO/sends sections)
- Input handling: navigate effect params, add/remove/move effects, toggle bypass, adjust params
- Emit `Action::Bus(BusAction::...)` instead of `Action::Instrument(InstrumentAction::...)`
- Extend add-effect pane to accept an `EffectTarget` context (`Instrument(id)`, `Bus(id)`, `LayerGroup(id)`) so it returns the right action type

---

## Migration Notes

Both phases add new fields to persisted structs. To avoid breakage on older project files or network messages:

- **Serde:** `#[serde(default)]` on `processing_chain`, `MixerBus::effects`, `LayerGroupMixer::effects`. Missing fields deserialize as empty vecs.
- **SQLite:** New tables (`instrument_processing_chain`, `instrument_eq_bands`, `bus_effects`, etc.) are created on schema migration. Loading checks for table existence — if absent, falls back to legacy layout (Phase A: filter → EQ → effects; Phase B: no bus effects).
- **Network:** Breaking protocol change, acceptable pre-1.0. Clients on mismatched versions will fail to deserialize `Instrument`/`MixerBus` — the existing connection error path handles this (disconnect + log).
- **Schema version:** Single bump covers both phases if shipped together; otherwise one bump per phase.

---

## Implementation Order

Recommended: Phase B first — it's self-contained, doesn't touch the Instrument struct, and delivers the highest-value feature (reverb/compression buses). Phase A is larger and more invasive.

1. Phase B (bus effects) — ~3 working sessions
2. Phase A (flexible chain) — ~5 working sessions

They share no code dependencies, so Phase A can't break Phase B. Bus effects stay as plain `Vec<EffectSlot>` (buses don't have filter/EQ, so no need for the unified chain enum).

---

## Verification

### Phase A
- `cargo test -p imbolc-types` — navigation helpers, chain manipulation, row mapping (table-driven)
- `cargo test -p imbolc-core` — dispatch handlers, persistence round-trip (including EQ bands), routing
- `cargo test -p imbolc-ui --bin imbolc-ui` — pane rendering, section navigation, cursor stability after moves
- Manual: create instrument, add filter, move it after an effect, verify audio chain reflects the new order

### Phase B
- `cargo test -p imbolc-types` — bus effect CRUD methods
- `cargo test -p imbolc-core` — bus dispatch, persistence round-trip, routing builds effect chain for buses
- `cargo test -p imbolc-ui --bin imbolc-ui` — bus detail view rendering
- Manual: create bus, add reverb effect, send an instrument to the bus, verify reverb applies

### Both
- `cargo test` — all tests pass
- No new warnings beyond pre-existing dead_code in style.rs
