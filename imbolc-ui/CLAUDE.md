# CLAUDE.md

Guide for AI agents working on this codebase.

## What This Is

A terminal-based DAW (Digital Audio Workstation) in Rust. Uses ratatui for TUI rendering and SuperCollider via OSC for audio synthesis. Instruments combine an oscillator source, filter, effects chain, LFO, envelope, and mixer controls into a single unit. Instruments are sequenced via piano roll.

## Directory Structure

```
src/
  main.rs          — Binary event loop, global keybindings, render loop
  panes/           — UI views (see Panes section below)
  ui/              — TUI framework (pane trait, keymap, input, style, widgets)
  setup.rs         — Auto-startup for SuperCollider
```

Core library lives in `../imbolc-core/`. Types are in `../imbolc-types/`. See the workspace root [../CLAUDE.md](../CLAUDE.md) for overview.

## Key Types

| Type | Location | What It Is |
|------|----------|------------|
| `AppState` | `imbolc-core/src/state/mod.rs` | Top-level state, owned by `main.rs`, passed to panes as `&AppState` |
| `Instrument` | `imbolc-types/src/state/instrument/` | One instrument: source + filter + effects + LFO + envelope + mixer |
| `InstrumentState` | `imbolc-types/src/state/instrument_state.rs` | Collection of instruments and selection state |
| `SessionState` | `imbolc-types/src/state/session.rs` | Global session data: buses, mixer, piano roll, automation |
| `InstrumentId` | `imbolc-types/src/lib.rs` | `u32` — unique identifier for instruments |
| `SourceType` | `imbolc-types/src/state/instrument/source_type.rs` | Oscillator/Source types (Saw/Sin/etc, AudioIn, BusIn, PitchedSampler, Kit, Custom, VST) |
| `EffectSlot` | `imbolc-types/src/state/instrument/effect.rs` | One effect in the chain: type + params + enabled + VST param values/state path |
| `Action` | `imbolc-types/src/action.rs` | Action enum (re-exported in `src/ui/pane.rs`) |
| `Pane` | `src/ui/pane.rs` | Trait: `id()`, `handle_action()`, `handle_raw_input()`, `handle_mouse()`, `render()`, `keymap()` |
| `PaneManager` | `src/ui/pane.rs` | Owns all panes, manages active pane, coordinates input |
| `LocalDispatcher` | `imbolc-core/src/dispatch/local.rs` | Owns state, provides dispatch |
| `AudioHandle` | `imbolc-core/src/audio/handle.rs` | Main-thread interface; sends AudioCmd via MPSC channel to audio thread |

## Panes

Single-file panes (20):
- `add_effect_pane.rs` — Effect selector modal
- `add_pane.rs` — Add instrument/bus modal
- `command_palette_pane.rs` — Command palette
- `confirm_pane.rs` — Confirmation dialog
- `eq_pane.rs` — EQ editor
- `file_browser_pane.rs` — File browser for samples
- `frame_edit_pane.rs` — Frame/layout settings
- `help_pane.rs` — Help overlay
- `home_pane.rs` — Home/welcome screen
- `instrument_pane.rs` — Instrument list
- `instrument_picker_pane.rs` — Instrument selection for actions
- `midi_settings_pane.rs` — MIDI configuration
- `pane_switcher_pane.rs` — Global pane navigation
- `project_browser_pane.rs` — Project browser
- `quit_prompt_pane.rs` — Quit confirmation
- `sample_chopper_pane.rs` — Sample slicing
- `save_as_pane.rs` — Save as dialog
- `sequencer_pane.rs` — Drum sequencer
- `track_pane.rs` — Track view
- `waveform_pane.rs` — Waveform display

Module panes (input/rendering split, 7):
- `automation_pane/` — Automation lane editor
- `docs_pane/` — Built-in documentation viewer
- `instrument_edit_pane/` — Instrument parameter editor
- `mixer_pane/` — Mixer view
- `piano_roll_pane/` — Note editor
- `server_pane/` — SuperCollider server status
- `vst_param_pane/` — VST parameter editor

## Critical Patterns

See [../docs/architecture.md](../docs/architecture.md) for detailed architecture, state ownership, borrow patterns, and persistence.

### Action Dispatch

Panes return `Action` values from `handle_action()` / `handle_raw_input()`. `imbolc-core/src/dispatch/` matches on them and mutates state. Panes never mutate state directly.

When adding a new action:
1. Add variant to `Action` enum in `imbolc-types/src/action.rs`
2. Return it from the pane's `handle_action()` (or `handle_raw_input()` if it bypasses layers)
3. Handle it in `dispatch::dispatch_action()` in `imbolc-core/src/dispatch/mod.rs`

### Navigation

Pane switching uses function keys: `F1`=instrument, `F2`=piano roll / sequencer / waveform (context-driven), `F3`=track, `F4`=mixer, `F5`=server, `F7`=automation. `` ` ``/`~` for back/forward. `?` for context-sensitive help. `Ctrl+f` opens the frame settings.

Number keys select instruments: `1`-`9` select instruments 1-9, `0` selects 10, `_` enters two-digit instrument selection.

### Pane Registration

New panes must be:
1. Created in `src/panes/` and added to `src/panes/mod.rs`
2. Registered in `main.rs`: `panes.add_pane(Box::new(MyPane::new()));`
3. Given a number-key binding in the global key match block (if navigable)

## UI Framework API

### Keymap

```rust
Keymap::new()
    .bind('q', "action_name", "Description")
    .bind_key(KeyCode::Up, "action_name", "Description")
    .bind_ctrl('s', "action_name", "Description")
    .bind_alt('x', "action_name", "Description")
    .bind_ctrl_key(KeyCode::Left, "action_name", "Desc")
    .bind_shift_key(KeyCode::Right, "action_name", "Desc")
```

Shift bindings only exist for special keys (e.g. `Shift+Right`). For shifted
characters, bind the literal char (`?`, `A`, `+`) rather than a Shift+ variant.

### Colors

`Color::new(r, g, b)` for custom RGB. Named constants: `Color::WHITE`, `Color::PINK`, `Color::SELECTION_BG`, `Color::MIDI_COLOR`, `Color::METER_LOW`. **No `Color::rgb()`** — use `Color::new()`.

### Pane Sizing

Use `ui::layout_helpers::center_rect(area, width, height)` to center a sub-rect. Most panes derive an inner rect from the frame and then place content relative to that.

## Build & Test

```bash
cargo build -p imbolc-ui
cargo test -p imbolc-ui
cargo run -p imbolc-ui      # Run the DAW
```

All workspace tests: `cargo test` from workspace root

## Configuration

TOML-based configuration system with embedded defaults and optional user overrides.

- **Musical defaults:** `config.toml` (embedded) + `~/.config/imbolc/config.toml` (user override)
- **Keybindings:** `keybindings.toml` (embedded) + `~/.config/imbolc/keybindings.toml` (user override)
- Config loading: `imbolc-core/src/config.rs` — `Config::load()` parses embedded defaults, layers user overrides
- Keybinding loading: `src/ui/keybindings.rs` — same embedded + user override pattern
- User override files are optional; missing fields fall back to embedded defaults

Musical defaults (`[defaults]` section): `bpm`, `key`, `scale`, `tuning_a4`, `time_signature`, `snap`

## Persistence

- Format: SQLite database (`.imbolc` / `.sqlite`)
- Save/load: `save_project()` / `load_project()` in `imbolc-core/src/state/persistence/mod.rs`
- Default path: `~/.config/imbolc/default.sqlite`

## LSP Integration (CCLSP)

Configured as MCP server (`cclsp.json` + `.mcp.json`). Provides rust-analyzer access. Prefer LSP tools over grep for navigating Rust code — they understand types, scopes, and cross-file references.

## Detailed Documentation

See `../docs/` for all documentation:
- [../docs/architecture.md](../docs/architecture.md) — state ownership, instrument model, pane rendering, action dispatch
- [../docs/audio-routing.md](../docs/audio-routing.md) — bus model, insert vs send, node ordering
- [../docs/keybindings.md](../docs/keybindings.md) — keybinding philosophy and conventions

## Plans

Implementation plans live at workspace root: `../plans/`

## SuperCollider Notes

In SuperCollider, all `var` declarations must appear at the top of a function block (or `( )` expression block), before any non-var statements. This is a language-level requirement, not a style convention.
