# Plan: Effect Selection Menu + VST Effect Import

**Status:** FUTURE
**Last Updated:** 2025-02-06

## Overview

Replace the current "press `a` to cycle through effect types" behavior with a proper selection menu pane (modeled on `AddPane`). Add VST3 effect import support to the menu.

## Files to Modify/Create

| File | Action |
|------|--------|
| `src/panes/add_effect_pane.rs` | **Create** — new pane for effect selection menu |
| `src/panes/mod.rs` | Add `mod add_effect_pane` + `pub use` |
| `src/main.rs` | Register `AddEffectPane`, add import |
| `src/panes/instrument_edit_pane/input.rs` | Change `add_effect` to navigate to new pane |
| `src/panes/instrument_edit_pane/mod.rs` | Add `on_enter()` to re-sync from state |
| `src/panes/instrument_edit_pane/rendering.rs` | Update effects header hint text |
| `src/dispatch/instrument.rs` | Handle `InstrumentAction::AddEffect` (currently stubbed) |
| `src/dispatch/session.rs` | Fix `ImportVstPlugin` to use correct `VstPluginKind` based on context |
| `src/ui/pane.rs` | Add `ImportVstEffect(InstrumentId)` to `FileSelectAction` or use existing, add `AddEffectForInstrument` to `SessionAction` |
| `keybindings.toml` | Add `[panes.add_effect]` keybindings section |

## Step-by-Step Implementation

### 1. Add `on_enter()` to InstrumentEditPane

In `src/panes/instrument_edit_pane/mod.rs`, implement `on_enter()` on the `Pane` trait impl:

```rust
fn on_enter(&mut self, state: &AppState) {
    if let Some(id) = state.instruments.editing_instrument_id {
        if let Some(instrument) = state.instruments.instrument(id) {
            self.set_instrument(instrument);
        }
    }
}
```

This closes a gap in the current code where `editing_instrument_id` is set by dispatch but never read. It also ensures the pane re-syncs when returning from the add_effect pane.

**Important**: `set_instrument` currently resets `selected_row` to 0. Modify it to preserve `selected_row` when re-entering (add a flag or separate method). Or add a `refresh_from_instrument()` method that doesn't reset selection.

### 2. Create AddEffectPane (`src/panes/add_effect_pane.rs`)

Model directly on `AddPane`. Structure:

```rust
enum AddEffectOption {
    Effect(EffectType),
    Separator(&'static str),
    ImportVst,
}

struct AddEffectPane {
    keymap: Keymap,
    selected: usize,
    cached_options: Vec<AddEffectOption>,
}
```

**Menu contents:**
```
── Built-in ──
Delay
Reverb
Gate
Tape Comp
SC Comp
── VST ──
[registered VST effects from registry]
+ Import VST Effect...
```

Key behaviors:
- `on_enter()`: rebuild options from `state.session.vst_plugins.effects()` (VST effects registry)
- Up/Down: navigate (skip separators, wrap around)
- Enter: confirm selection
- Escape: cancel, go back to instrument_edit

**On confirm:**
- Built-in effect → return `InstrumentAction::AddEffect(instrument_id, effect_type)`
  - Read `instrument_id` from `state.instruments.editing_instrument_id`
- "Import VST Effect..." → return `SessionAction::OpenFileBrowser(FileSelectAction::ImportVstEffect)`

**Rendering:** Centered modal (same `center_rect(area, 97, 29)` pattern), with:
- FX_COLOR for built-in effects
- VST_COLOR for VST effects and import option
- SELECTION_BG for selected row
- Help text at bottom

### 3. Modify `add_effect` action in InstrumentEditPane

In `src/panes/instrument_edit_pane/input.rs`, change the `"add_effect"` handler:

```rust
"add_effect" => {
    // Save current state before navigating
    let update = self.emit_update();
    // Navigate to effect selection menu
    // Return a Nav action to switch to add_effect pane
    // But we need to emit the update first...
}
```

The issue: we need to both emit an update AND navigate. Since `handle_action` returns a single `Action`, we have two options:

**Option A**: Return `Nav(SwitchPane("add_effect"))` and don't emit update (effects are already in sync since every change emits an update).

**Option B**: Add a compound action or handle it differently.

Going with **Option A** — the instrument state is already up-to-date because every prior edit already called `emit_update()`. So just navigate:

```rust
"add_effect" => {
    Action::Nav(NavAction::SwitchPane("add_effect"))
}
```

### 4. Handle `InstrumentAction::AddEffect` in dispatch

In `src/dispatch/instrument.rs`, replace the stub:

```rust
InstrumentAction::AddEffect(id, effect_type) => {
    if let Some(instrument) = state.instruments.instrument_mut(*id) {
        instrument.effects.push(EffectSlot::new(effect_type.clone()));
    }
    if audio_engine.is_running() {
        let _ = audio_engine.rebuild_instrument_routing(&state.instruments, &state.session);
    }
    DispatchResult::with_nav(NavIntent::PopOrSwitchTo("instrument_edit"))
}
```

Remove the `#[allow(dead_code)]` from the `AddEffect` variant.

### 5. Fix VST Effect import — differentiate kind

In `src/dispatch/session.rs`, the `ImportVstPlugin` handler currently hardcodes `VstPluginKind::Instrument`. Two approaches:

**Approach**: Add a new `SessionAction` variant `ImportVstEffect(PathBuf)`, or change `ImportVstPlugin` to carry the kind. Simplest: add the kind to the action.

Change `SessionAction::ImportVstPlugin(PathBuf)` to `SessionAction::ImportVstPlugin(PathBuf, VstPluginKind)`.

Update all callers:
- `file_browser_pane.rs`: When `on_select_action` is `ImportVstInstrument`, return `ImportVstPlugin(path, VstPluginKind::Instrument)`. When `ImportVstEffect`, return `ImportVstPlugin(path, VstPluginKind::Effect)`.
- Dispatch: use the provided kind instead of hardcoding.

### 6. Register the pane

In `src/panes/mod.rs`:
```rust
mod add_effect_pane;
pub use add_effect_pane::AddEffectPane;
```

In `src/main.rs`:
```rust
panes.add_pane(Box::new(AddEffectPane::new(pane_keymap(&mut keymaps, "add_effect"))));
```

### 7. Add keybindings

In `keybindings.toml`, add section (same pattern as `[panes.add]`):

```toml
[panes.add_effect]
bindings = [
  { key = "Enter", action = "confirm", description = "Add selected effect" },
  { key = "Escape", action = "cancel", description = "Cancel" },
  { key = "Up", action = "prev", description = "Previous" },
  { key = "Down", action = "next", description = "Next" },
  { key = "k", action = "prev", description = "Previous" },
  { key = "j", action = "next", description = "Next" },
]
```

### 8. Update rendering hint text

In `src/panes/instrument_edit_pane/rendering.rs`, change:
```
"EFFECTS  (a: add, d: remove)"
```
to:
```
"EFFECTS  (a: add effect, d: remove)"
```

### 9. After VST import, navigate back to add_effect pane

After `ImportVstPlugin` dispatch succeeds, navigate back. Currently it does `NavIntent::Pop` which returns to the previous pane (file_browser was pushed on top of add_effect, so popping returns to add_effect). The `on_enter()` on `AddEffectPane` will rebuild the options list to include the newly imported VST.

## Navigation Flow

```
instrument_edit → [press 'a'] → add_effect → [select effect] → dispatch AddEffect → instrument_edit
                                            → [Import VST...] → file_browser → dispatch ImportVstPlugin → add_effect
                                            → [Escape] → instrument_edit
```

## Verification

1. `cargo build` — should compile without errors
2. `cargo test --bin imbolc` — existing tests pass
3. Manual testing:
   - Open instrument edit (press Enter on an instrument)
   - Press `a` — should show effect selection menu
   - Navigate with Up/Down/j/k — selection highlights, skips separators
   - Press Enter on "Delay" — effect added, back to instrument edit
   - Press `a` again, select different effect — adds another
   - Press `d` to remove effects — still works
   - Press `a`, select "Import VST Effect..." — file browser opens to /Library/Audio/Plug-Ins/VST3
   - Select a .vst3 file — imports as effect kind, returns to add_effect menu
   - Newly imported VST appears in the list
   - Press Escape from add_effect — returns to instrument edit without adding
