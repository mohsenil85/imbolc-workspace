# Targeted Routing Loosening

## Context

The full modular routing refactor (Q11 in questions.md) would be the largest
rewrite in the codebase — touching every crate — for power-user
features that affect ~5% of sessions. Instead, three targeted changes
address the real pain points at a fraction of the cost.

**Sidechain support is already implemented** (`SidechainComp` effect
type + `sc_bus` param + routing.rs bus lookup), so this plan covers
the two phases that need work.

---

## Phase A: Flexible Effect Chain Ordering

**Problem:** Signal chain is hardcoded as source → filter → EQ →
effects → output. Can't put distortion before filter, or EQ after
reverb.

**Approach:** Unify filter, EQ, and effects into `processing_chain:
Vec<ProcessingStage>`. Keep filter/EQ as distinct types (not
EffectType variants) because they have fundamentally different param
structures (FilterConfig uses ModulatedParam, EQ uses fixed 12-band
array). LFO modulation targets a specific filter by chain index when
multiple filters exist.

### Behavioral Guarantees

- **Multiple Filters allowed.** Each `toggle_filter()` call inserts a
  new Filter stage at index 0 (or removes the one at cursor). Useful
  for serial filter stacking.
- **At most one EQ.** Enforced in `toggle_eq()` (refuse if chain
  already contains an EQ) and validated in `processing_chain`
  helpers. Convenience accessors `eq()`/`eq_mut()` return the single
  instance.
- Toggling a stage off removes it; toggling on inserts at its default
  position. Relative order of other stages is preserved.
- `move_stage()` is a stable reorder — no other stage indices shift
  unexpectedly.
- Pre-insert sends tap `source_out_bus` (before any processing stage,
  regardless of chain order). Post-insert sends tap the final bus
  after all processing stages.

### ~~A1. Types (`imbolc-types/src/state/instrument/mod.rs`)~~ ✓

- Add `ProcessingStage` enum: `Filter(FilterConfig)`, `Eq(EqConfig)`,
  `Effect(EffectSlot)`
- Replace `filter: Option<FilterConfig>`, `eq: Option<EqConfig>`,
  `effects: Vec<EffectSlot>` with `processing_chain:
  Vec<ProcessingStage>`
- Add convenience accessors that scan the chain — minimizes churn in
  existing code:
  - `filters()` / `filters_mut()` → iterators over all Filter stages
    (multiple allowed)
  - `eq()` / `eq_mut()` → `Option<&EqConfig>` / `Option<&mut
    EqConfig>` (single instance)
  - `effects()` → iterator over Effect stages
- Add `move_stage(index, direction)`, `filter_chain_index()`,
  `eq_chain_index()`
- Update `toggle_filter()`: inserts a new
  `Filter(FilterConfig::default())` at index 0 (or at cursor position
  if UI provides one). Multiple filters are allowed — serial stacking
  is a valid use case.
- Update `toggle_eq()`: inserts EQ after the last Filter if any exist,
  else index 0. **Enforces single-instance** — if chain already
  contains an EQ, toggle removes it instead of inserting a second.
- Update `add_effect()` to push `ProcessingStage::Effect(...)` to end
  of chain

### ~~A2. Navigation helpers (`imbolc-types/src/state/instrument/mod.rs`)~~ ✓

- Change `InstrumentSection` enum: `Source`, `Processing(usize)`,
  `Lfo`, `Envelope`
  - `Processing(idx)` identifies a chain stage by its index
- Rewrite `instrument_row_count()`: sum source rows + per-stage rows
  (dynamically) + LFO + envelope
- Rewrite `instrument_section_for_row()` and `instrument_row_info()`:
  iterate chain stages with cumulative offsets
- Add table-driven tests for `instrument_section_for_row()` and
  `instrument_row_info()` covering chains with different stage orders,
  empty chains, and chains with all three stage types

### ~~A3. Actions (`imbolc-types/src/action.rs`)~~ ✓

**Done.** Added `MoveStage`, updated `InstrumentUpdate`, removed
`InstrumentAction::MoveEffect` (variant, dispatch handler in
`effects.rs`, action projection arm, `Instrument::move_effect()`
method and its test). Mixer pane (`mixer_pane/input.rs`) and GUI
(`effect_slot.rs`, `instrument_editor.rs`) now use
`effect_chain_index()` + `MoveStage`. `BusAction::MoveEffect` and
`LayerGroupAction::MoveEffect` stay — buses and layer groups have
flat `effects: Vec<EffectSlot>`, no `processing_chain`.

### ~~A4. Dispatch (`imbolc-core/src/dispatch/instrument/`)~~ ✓

**Done.** All dispatch handlers migrated from direct field access to
accessor methods and `processing_chain` manipulation.

- **filter.rs**: `instrument.filter` → `instrument.filter_mut()`,
  `instrument.set_filter()`, `instrument.toggle_filter()`
- **eq.rs**: `instrument.eq` → `instrument.eq_mut()`,
  `instrument.toggle_eq()`
- **effects.rs**: `add_effect` pushes to chain via
  `instrument.add_effect()`; `remove_effect` via
  `instrument.remove_effect()`; `effect_by_id_mut()` for param access
- **crud.rs**: `handle_update` copies `processing_chain` instead of
  three separate fields
- **action_projection.rs**: Same accessor migration for audio thread
  state copy
- **automation.rs**: `inst.effects` → `inst.effects()`, `inst.eq` →
  `inst.eq()`

`MoveStage` dispatch handler already existed (added in A1–A3).
Routing rebuild triggers unchanged (existing `routing = true` flags).

### ~~A5. Audio routing (`imbolc-core/src/audio/engine/routing.rs`)~~ ✓

**Done.** Replaced the three sequential blocks (filter, EQ, effects)
in `build_instrument_chain` with a single `processing_chain` loop:

```rust
for stage in instrument.processing_chain.iter() {
    match stage {
        ProcessingStage::Filter(f) => { /* filter routing, current_bus chaining */ }
        ProcessingStage::Eq(eq)    => { /* EQ routing, current_bus chaining */ }
        ProcessingStage::Effect(e) => { /* per-effect routing, current_bus chaining */ }
    }
}
```

`InstrumentNodes` keeps existing fields (filter, eq, effects maps).
Real-time param updates look up nodes by type/id — unchanged.

**Node ordering:** Added `ProcessingNodeRef` enum (`Filter`, `Eq`,
`Effect(EffectId)`) and `processing_order: Vec<ProcessingNodeRef>` to
`InstrumentNodes`. `all_node_ids()` iterates `processing_order` for
the middle segment, ensuring SC node ordering matches the actual
chain.

**Send tap points:** Pre-insert sends tap `source_out_bus` (before all
processing stages regardless of chain order). Post-insert sends tap
the final bus after all processing.

**Files:** `engine/routing.rs` (chain loop, node ordering),
`engine/mod.rs` (`ProcessingNodeRef`, `InstrumentNodes` update,
`all_node_ids()`), `engine/mod.rs` tests (updated `set_filter` calls).

### ~~A6. UI (`imbolc-ui/src/panes/instrument_edit_pane/`)~~ ✓

**Done.** Full refactor complete — replaced local `filter`/`eq`/`effects`
fields with `processing_chain: Vec<ProcessingStage>`. Removed the local
`Section` enum and all bridging helpers (`build_processing_chain()`,
`map_section()`, `map_row_info()`, `effect_row_info()`). All four pane
files rewritten:

- **mod.rs**: `set_instrument()` / `refresh_instrument()` clone
  `instrument.processing_chain` directly. Nav helpers delegate to
  type-level functions with no bridging. Dynamic `tab_index()` /
  `set_tab_index()` encoding (0=Source, 1..=N=Processing, N+1=Lfo,
  N+2=Envelope). New `row_for_processing_stage()` for cursor stability
  after MoveStage. Dynamic `visual_overhead()` from chain content.
- **editing.rs**: All match blocks use `InstrumentSection::Processing(i)`
  dispatching to `ProcessingStage::Filter/Eq/Effect`. `emit_update()`
  uses `self.processing_chain.clone()` directly.
- **input.rs**: All match blocks rewritten. New handlers: `MoveStageUp`
  (Ctrl+Up), `MoveStageDown` (Ctrl+Down), `ToggleEffectBypass` (`b`).
  `NextSection`/`PrevSection` visit each processing stage individually.
- **rendering.rs**: Replaced hardcoded section layout with chain loop.
  Empty chain renders "(no processing)" placeholder.

**New keybindings:** `Ctrl+Up` move_stage_up, `Ctrl+Down`
move_stage_down, `b` toggle_effect_bypass. 5 new tests (section
navigation with reordered chain, tab cycling, cursor stability after
move, toggle filter, empty chain). 121 UI tests pass.

### ~~A7. Persistence (`imbolc-core/src/state/persistence/`)~~ ✓

**Done.** Bumped `SCHEMA_VERSION` 11→12. Added
`instrument_processing_chain` table `(instrument_id, position,
stage_type, effect_id)` as an ordering index — stage data stays in
existing tables. On save, `save_processing_chain()` writes one row per
chain stage. On load, reads ordering rows and places already-loaded
filter/EQ/effects into the chain in persisted order. Legacy fallback
(filter → EQ → effects) for databases without the table or with no
rows. `instrument_eq_bands` was already present from earlier schema
versions. 2 new round-trip tests (`round_trip_processing_chain_order`,
`round_trip_processing_chain_interleaved`). 278 core tests pass.

**Files:** `schema.rs` (v12, new table, DELETE), `save.rs`
(`save_processing_chain()` + call site), `load.rs` (conditional chain
reconstruction with `table_exists` check), `tests.rs` (2 tests).

### ~~A8. Network (`imbolc-net`)~~ ✓

**Done automatically.** `Instrument` serde changes propagate via
`#[serde(default)]` on `processing_chain`. Old fields removed from
struct; network protocol is a breaking change (acceptable pre-1.0).
`StatePatch` / `DirtyFlags` unchanged (operate at instrument level).

---

## Phase B: Effects on Mixer Buses

**Problem:** Buses are dumb mixers — no processing. Can't have a
reverb bus, compression bus, or EQ on the mix bus.

**Approach:** Add `effects: Vec<EffectSlot>` to `MixerBus` (and
`LayerGroupMixer`). Build effect chains before bus output synths in
routing. No LFO modulation on bus effects initially.

### ~~B1. Types (`imbolc-types/src/state/mixer.rs`)~~ ✓

**Done.** Added `effects: Vec<EffectSlot>` and `next_effect_id:
EffectId` to both `MixerBus` and `LayerGroupMixer`. CRUD methods
(`add_effect`, `remove_effect`, `move_effect`, `effect_by_id`,
`effect_by_id_mut`, `recalculate_next_effect_id`) on both types. 9 new
tests. 203 tests pass.

### ~~B2. Actions (`imbolc-types/src/action.rs`)~~ ✓

**Done.** Extended `BusAction` with `AddEffect`, `RemoveEffect`,
`MoveEffect`, `ToggleEffectBypass`, `AdjustEffectParam`. Added
`LayerGroupAction` enum with matching effect variants. Wired
`Action::LayerGroup(LayerGroupAction)` into main `Action` enum. Added
`bus_effect_param: Option<(u8, EffectId, usize, f32)>` and
`layer_group_effect_param: Option<(u32, EffectId, usize, f32)>` to
`AudioDirty` (Option not Vec to preserve `Copy`).

### ~~B3. Dispatch (`imbolc-core/src/dispatch/bus.rs`)~~ ✓

**Done.** Handled all `BusAction` effect variants in `bus.rs` and
`LayerGroupAction` variants in the same file (no separate
`layer_group.rs` — kept together since bus/group dispatch share the
same pattern). Effect CRUD sets `routing = true`; param adjust sets
targeted `bus_effect_param`/`layer_group_effect_param`. Undo for CRUD
with `UndoScope::Session`. 8 new dispatch tests. 266 core tests pass.

### ~~B4. Audio routing (`imbolc-core/src/audio/engine/routing.rs`)~~ ✓

**Done.** Added `GROUP_BUS_PROCESSING = 350` SC group between
`GROUP_OUTPUT(300)` and `GROUP_RECORD(400)`. Added
`bus_effect_node_map: HashMap<(u8, EffectId), i32>` and
`layer_group_effect_node_map: HashMap<(u32, EffectId), i32>` to
`AudioEngine`. Built `build_bus_effect_chain()` and
`build_layer_group_effect_chain()` helpers with SidechainComp,
ConvolutionReverb, and VST special param handling. Layer group
effects+outputs execute before bus effects+outputs in BuildOutputs (so
group outputs mix into bus_audio before bus effects read it). Added
`set_bus_effect_param()` and `set_layer_group_effect_param()` for
real-time knob tweaks via priority channel. Wired through
`AudioCmd` → `audio_thread.rs` → `handle.rs` including
`send_routing_and_params()` dirty flag handling. TearDown frees and
clears both new maps. Both synchronous and phased rebuild paths
updated. 8 new routing tests. 266 core tests pass, clean build.

**Files:** `engine/mod.rs`, `engine/server.rs`, `engine/routing.rs`,
`commands.rs`, `audio_thread.rs`, `handle.rs`,
`state/instrument/mod.rs` (re-export).

### ~~B5. Persistence (`imbolc-core/src/state/persistence/`)~~ ✓

**Done.** Bumped `SCHEMA_VERSION` 8→9. Added 6 tables: `bus_effects`,
`bus_effect_params`, `bus_effect_vst_params`, `layer_group_effects`,
`layer_group_effect_params`,
`layer_group_effect_vst_params`. Extracted generic `save_effects_to()`
/ `load_effects_from()` / `load_effect_params_from()` helpers
(instrument effects refactored to delegate). `table_exists()` check
provides backward compat with v8
schemas. `recalculate_next_effect_id()` on load. 2 new round-trip
tests (`round_trip_bus_effects`,
`round_trip_layer_group_effects`). 258 tests pass, 0 warnings.

**Files:** `imbolc-core/src/state/persistence/schema.rs`, `save.rs`,
`load.rs`, `tests.rs`.

### ~~B6. UI (`imbolc-ui/src/panes/mixer_pane/`)~~ ✓

**Done.** Three detail views (instrument, bus, layer group) with full
effect chain editing. Mixer pane split into `mod.rs` / `input.rs` /
`rendering.rs`. Each detail view has section cycling (Tab/Shift+Tab),
effect CRUD (`a` add, `d` remove, `e` bypass, `<`/`>` move),
parameter adjustment (`+/-` fine, `Shift+/-` coarse), and cursor
tracking via `decode_effect_cursor()` helpers. Added `EffectTarget`
enum to `add_effect_pane.rs` — main.rs bridges mixer's current target
to the modal. Instrument pane link mode refined (press `l` twice to
confirm). 8 new mixer tests + 4 new instrument pane tests. 114 UI
tests pass.

**Files:** `imbolc-ui/src/panes/mixer_pane/{mod,input,rendering}.rs`,
`imbolc-ui/src/panes/add_effect_pane.rs`,
`imbolc-ui/src/panes/instrument_pane.rs`, `imbolc-ui/src/main.rs`,
`imbolc-types/src/state/instrument/mod.rs` (cursor helpers).

### ~~B7. Layer Group EQ~~ ✓

**Done.** Added `eq: Option<EqConfig>` to `LayerGroupMixer` (initialized
with default 12-band EQ). Toggle on/off via `LayerGroupAction::ToggleEq`,
adjust bands via `LayerGroupAction::SetEqParam`. Audio routing builds
`imbolc_eq12` synth node before effect chain in `GROUP_BUS_PROCESSING`.
Real-time param updates via `AudioSideEffect::SetLayerGroupEqParam` →
`AudioCmd::SetLayerGroupEqParam` → priority channel. Persistence bumps
schema 10→11 with `eq_enabled` column on `layer_group_mixers` and new
`layer_group_eq_bands` table. Backward-compat via `table_exists()` check.

**Files:** `imbolc-types/src/state/instrument/mod.rs` (field + methods),
`imbolc-types/src/action.rs` (LayerGroupAction variants),
`imbolc-core/src/dispatch/bus.rs` (dispatch handlers),
`imbolc-core/src/dispatch/side_effects.rs` (AudioSideEffect variant),
`imbolc-core/src/dispatch/mod.rs` (updated call site),
`imbolc-core/src/audio/action_projection.rs` (projection arms),
`imbolc-core/src/audio/commands.rs` (AudioCmd variant),
`imbolc-core/src/audio/audio_thread.rs` (routing + handler),
`imbolc-core/src/audio/handle.rs` (set_layer_group_eq_param),
`imbolc-core/src/audio/engine/mod.rs` (layer_group_eq_node_map),
`imbolc-core/src/audio/engine/routing.rs` (build EQ node, param update, teardown),
`imbolc-core/src/audio/engine/server.rs` (teardown),
`imbolc-core/src/state/persistence/schema.rs` (v11, new table),
`imbolc-core/src/state/persistence/save.rs` (save EQ),
`imbolc-core/src/state/persistence/load.rs` (load EQ),
`imbolc-core/src/state/persistence/tests.rs` (2 round-trip tests).

**Tests:** 3 types tests (toggle, accessors, eq_mut), 2 dispatch tests
(toggle_eq, set_eq_param), 2 persistence round-trip tests (EQ enabled,
EQ disabled). 213 types + 276 core tests pass.

**Deferred:** Undo support for `LayerGroupAction` variants (ToggleEq,
SetEqParam, and all pre-existing effect variants). Currently
`LayerGroupAction` falls through to `_ => false` in `is_undoable()`.
This is tracked as future work — adding undo requires choosing scopes
(Session for toggle, no undo for real-time param) and testing
undo/redo round-trips.

---

## Migration Notes

Both phases add new fields to persisted structs. To avoid breakage on
older project files or network messages:

- **Serde:** `#[serde(default)]` on `processing_chain`,
  `MixerBus::effects`, `LayerGroupMixer::effects`. Missing fields
  deserialize as empty vecs.
- **SQLite:** New tables (`instrument_processing_chain`,
  `instrument_eq_bands`, `bus_effects`, etc.) are created on schema
  migration. Loading checks for table existence — if absent, falls
  back to legacy layout (Phase A: filter → EQ → effects; Phase B: no
  bus effects).
- **Network:** Breaking protocol change, acceptable pre-1.0. Clients
  on mismatched versions will fail to deserialize
  `Instrument`/`MixerBus` — the existing connection error path handles
  this (disconnect + log).
- **Schema version:** Single bump covers both phases if shipped
  together; otherwise one bump per phase.

---

## Implementation Order

Recommended: Phase B first — it's self-contained, doesn't touch the
Instrument struct, and delivers the highest-value feature
(reverb/compression buses). Phase A is larger and more invasive.

1. ~~Phase B (bus effects)~~ — **Complete.** All B1–B7 done (B7 = layer group EQ).
2. ~~Phase A (flexible chain)~~ — **Complete.** All A1–A8 done, A3
   `MoveEffect` removal complete. 235 types + 278 core + 121 UI +
   78 net = 712 tests pass.

They share no code dependencies, so Phase A can't break Phase B. Bus
effects stay as plain `Vec<EffectSlot>` (buses don't have filter/EQ,
so no need for the unified chain enum).

---

## Verification

### Phase A
- `cargo test -p imbolc-types` — navigation helpers, chain
  manipulation, row mapping (table-driven)
- `cargo test -p imbolc-core` — dispatch handlers, persistence
  round-trip (including EQ bands), routing
- `cargo test -p imbolc-ui --bin imbolc-ui` — pane rendering, section
  navigation, cursor stability after moves
- Manual: create instrument, add filter, move it after an effect,
  verify audio chain reflects the new order

### Phase B
- `cargo test -p imbolc-types` — bus effect CRUD methods
- `cargo test -p imbolc-core` — bus dispatch, persistence round-trip,
  routing builds effect chain for buses
- `cargo test -p imbolc-ui --bin imbolc-ui` — bus detail view
  rendering
- Manual: create bus, add reverb effect, send an instrument to the
  bus, verify reverb applies

### Both
- `cargo test` — all tests pass
- No new warnings beyond pre-existing dead_code in style.rs
