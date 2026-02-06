# Phase 5: Code Organization (Mega-Files)

**Status:** FUTURE
**Last Updated:** 2025-02-06

Pure mechanical extraction — no behavioral changes. Three independent sub-phases.
Do them in order: **5A → 5B → 5C**. Run `cargo build && cargo test` after each sub-phase.

---

## Phase 5A: Extract global action handler from main.rs

**Goal:** Move helper functions and types from `src/main.rs` (816 lines) into `src/global_actions.rs`.

### What moves to `src/global_actions.rs`

Move these items from main.rs (in order of appearance):

1. `enum InstrumentSelectMode` (lines 42-46)
2. `fn select_instrument(...)` (lines 438-445)
3. `fn sync_piano_roll_to_selection(...)` (lines 449-485)
4. `fn sync_instrument_edit(...)` (lines 488-496)
5. `fn sync_pane_layer(...)` (lines 499-514)
6. `fn handle_global_action(...)` (lines 516-767) — the largest function
7. `enum GlobalResult` (lines 431-435)
8. `fn apply_status_events(...)` (lines 770-779)
9. `fn apply_dispatch_result(...)` (lines 782-815)

### Steps

1. Read `src/main.rs` fully to understand all references
2. Create `src/global_actions.rs` containing the items listed above
3. All functions and types should be `pub(crate)` visibility
4. The new file needs these imports (adapt based on what the functions actually use):
   ```rust
   use crate::audio::AudioHandle;
   use crate::action::{AudioDirty, IoFeedback};
   use crate::dispatch;
   use crate::state::{self, AppState};
   use crate::panes::{InstrumentEditPane, PianoRollPane, ServerPane, HelpPane, FileBrowserPane, VstParamPane};
   use crate::ui::{
       Action, DispatchResult, Frame, LayerStack, NavIntent, PaneManager,
       SessionAction, StatusEvent, ToggleResult, ViewState,
   };
   ```
   Also add `use crate::ui;` if `ui::ServerAction` or `ui::InstrumentAction` are referenced.
5. In `src/main.rs`:
   - Add `mod global_actions;` near the other `mod` declarations
   - Add `use global_actions::*;` (or import specific items: `GlobalResult`, `InstrumentSelectMode`, `handle_global_action`, `apply_dispatch_result`, `select_instrument`, `sync_piano_roll_to_selection`, `sync_instrument_edit`, `sync_pane_layer`, `apply_status_events`)
   - Remove the moved items from main.rs
   - The `run()` function and `main()` stay in main.rs
   - `pane_keymap()` stays in main.rs (it's only used during pane registration in `run()`)

### Verification
- `cargo build && cargo test` — everything should work identically

---

## Phase 5B: Split persistence into domain modules

**Goal:** Split `imbolc-core/src/state/persistence/save.rs` (678 lines) and `load.rs` (1241 lines) into domain-specific sub-files.

### Target structure

```
persistence/
  mod.rs          — (unchanged) save_project(), load_project(), tests
  conversion.rs   — (unchanged)
  schema.rs       — (unchanged)
  save/
    mod.rs          — module declarations + re-exports
    instruments.rs  — instrument-related save functions
    mixer.rs        — mixer save functions
    piano_roll.rs   — piano roll + sampler save functions
    automation.rs   — automation save functions
    sequencer.rs    — drum sequencer + chopper save functions
    plugins.rs      — synthdefs, VST, MIDI recording save functions
  load/
    mod.rs          — module declarations + re-exports
    instruments.rs  — instrument-related load functions
    mixer.rs        — mixer load functions
    piano_roll.rs   — piano roll + sampler load functions
    automation.rs   — automation load functions
    sequencer.rs    — drum sequencer + chopper load functions
    plugins.rs      — synthdefs, VST, MIDI recording, arpeggiator load functions
```

### Function → file mapping for save/

Read `save.rs` to identify exact functions and their line ranges, then distribute:

| File | Functions |
|------|-----------|
| `instruments.rs` | `save_instruments`, `save_eq_bands`, `save_source_params`, `save_effects`, `save_sends`, `save_modulations` (+ any helpers like `insert_mod_source`) |
| `mixer.rs` | `save_mixer` |
| `piano_roll.rs` | `save_piano_roll`, `save_sampler_configs` |
| `automation.rs` | `save_automation` |
| `sequencer.rs` | `save_drum_sequencers`, `save_chopper_states` |
| `plugins.rs` | `save_custom_synthdefs`, `save_vst_plugins`, `save_vst_param_values`, `save_effect_vst_params`, `save_midi_recording` |

### Function → file mapping for load/

Read `load.rs` to identify exact functions and their line ranges, then distribute:

| File | Functions / Types |
|------|-----------|
| `instruments.rs` | `load_instruments`, `load_eq_bands`, `load_source_params`, `load_effects`, `load_sends`, `load_modulations`, `load_arpeggiator_settings` |
| `mixer.rs` | `load_buses`, `load_master` |
| `piano_roll.rs` | `MusicalSettingsLoaded` struct, `load_piano_roll`, `load_sampler_configs` |
| `automation.rs` | `load_automation` |
| `sequencer.rs` | `load_drum_sequencers`, `load_chopper_states` |
| `plugins.rs` | `load_custom_synthdefs`, `load_vst_plugins`, `load_vst_param_values`, `load_effect_vst_params`, `load_vst_state_paths`, `load_midi_recording` |

### Steps

1. Read `save.rs` and `load.rs` fully to understand all functions, imports, and dependencies
2. Create `save/` directory. Create `save/mod.rs` with:
   - `mod instruments; mod mixer; mod piano_roll; mod automation; mod sequencer; mod plugins;`
   - `pub(crate) use instruments::*; pub(crate) use mixer::*;` ... etc. for all sub-modules
   - This preserves the call sites in `persistence/mod.rs` (e.g. `save::save_instruments(...)`)
3. Create each `save/*.rs` file with the appropriate functions and their needed imports
4. Delete `save.rs`
5. Repeat for `load/` directory: same pattern
6. Delete `load.rs`
7. `persistence/mod.rs` should NOT need changes — it already uses `mod save; mod load;` which works for both file and directory modules, and the re-exports preserve the `save::function_name` path

### Import pattern for sub-files

Each sub-file needs its own imports. Copy the relevant imports from the original `save.rs`/`load.rs`. Common patterns:
```rust
// save sub-files:
use rusqlite::Connection as SqlConnection;
use crate::state::instrument::*;  // or specific types
// ... whatever each function needs

// load sub-files:
use rusqlite::Connection as SqlConnection;
use crate::state::instrument::*;
// ... whatever each function needs
```

**Important:** The original `save.rs` and `load.rs` use `super::super::` paths to reach state types. In the new `save/instruments.rs`, the path is one level deeper, so `super::super::` becomes `super::super::super::`. Alternatively, use `crate::state::` absolute paths which are cleaner and don't break when nesting changes.

### Verification
- `cargo build && cargo test` — the tests in `persistence/mod.rs` must still pass

---

## Phase 5C: Split instrument.rs into submodules

**Goal:** Split `imbolc-core/src/state/instrument.rs` (1129 lines) into a module directory.

### Target structure

```
state/
  instrument/
    mod.rs          — Instrument struct, OutputTarget, MixerSend, MixerBus, ModSource, ModulatedParam, InstrumentId, re-exports
    source_type.rs  — SourceType enum + impl
    filter.rs       — FilterType, FilterConfig, EqBandType, EqBand, EqConfig + impls
    effect.rs       — EffectType + impl, EffectSlot + impl
    lfo.rs          — LfoShape, LfoTarget, LfoConfig + impls
    envelope.rs     — EnvConfig + impl
```

### What goes where

Read `instrument.rs` fully, then distribute:

| File | Types (enums/structs/impls) |
|------|-----------|
| `source_type.rs` | `SourceType` enum + its impl block (~420 lines). Needs imports: `super::super::custom_synthdef::*`, `super::super::param::*`, `super::super::vst_plugin::*` (or use `crate::state::` paths) |
| `filter.rs` | `FilterType` enum + impl, `FilterConfig` struct + impl, `EqBandType` enum + impl, `EqBand` struct, `EqConfig` struct + Default impl, `EQ_BAND_COUNT` constant |
| `effect.rs` | `EffectType` enum + impl, `EffectSlot` struct + impl. Needs imports for `Param`, `ParamValue`, `VstPluginRegistry` |
| `lfo.rs` | `LfoShape` enum + impl, `LfoTarget` enum + impl, `LfoConfig` struct + Default impl |
| `envelope.rs` | `EnvConfig` struct + Default impl |
| `mod.rs` (stays) | `InstrumentId` type alias, `OutputTarget` enum + impl, `MixerSend` struct + impl, `MixerBus` struct + impl, `EnvConfig` (re-export from envelope), `ModulatedParam` struct, `ModSource` enum, `Instrument` struct + impl, `MAX_BUSES` constant. Plus re-exports from all sub-modules. |

### Steps

1. Read `instrument.rs` fully
2. Create `instrument/` directory
3. Create each sub-module file with the appropriate types and their imports
4. Create `instrument/mod.rs` with:
   - Module declarations: `mod source_type; mod filter; mod effect; mod lfo; mod envelope;`
   - Re-exports: `pub use source_type::*; pub use filter::*; pub use effect::*; pub use lfo::*; pub use envelope::*;`
   - The types that stay in mod.rs: `InstrumentId`, `OutputTarget`, `MixerSend`, `MixerBus`, `ModulatedParam`, `ModSource`, `Instrument`, `MAX_BUSES`
   - The imports that mod.rs needs (for `Instrument` struct which references types from sub-modules)
5. Delete the original `instrument.rs` file
6. `state/mod.rs` should NOT need changes — it already has `mod instrument;` or `pub mod instrument;` which works for both file and directory modules

### Import pattern for sub-files

Sub-files under `instrument/` reference types from sibling modules and parent state modules:
```rust
// In source_type.rs:
use crate::state::custom_synthdef::{CustomSynthDefId, CustomSynthDefRegistry};
use crate::state::param::{Param, ParamValue};
use crate::state::vst_plugin::{VstPluginId, VstPluginRegistry};
```

### Critical: preserve all `pub` visibility

Every type, field, method, and constant that was `pub` in the original `instrument.rs` must remain `pub` in the new sub-modules. The re-exports (`pub use submodule::*`) ensure external code still works.

### Verification
- `cargo build && cargo test` — the whole codebase imports from `crate::state::instrument::*`, which is preserved by re-exports

---

## Summary

| Sub-phase | Files created | Files modified | Files deleted |
|-----------|--------------|----------------|---------------|
| 5A | `src/global_actions.rs` | `src/main.rs` | — |
| 5B | `save/mod.rs` + 6 sub-files, `load/mod.rs` + 6 sub-files (14 total) | `persistence/mod.rs` (possibly) | `save.rs`, `load.rs` |
| 5C | `instrument/mod.rs` + 5 sub-files (6 total) | — | `instrument.rs` |

Total: ~21 new files, 3 deleted, 1-2 modified. Zero behavioral changes.

## Final verification

After all three sub-phases:
```
cargo build && cargo test
```
All tests must pass. No behavioral changes — this is purely organizational.
