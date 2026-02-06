# CLAUDE.md

Guide for AI agents working on imbolc-core.

## What This Is

The core library for imbolc, a terminal-based DAW (Digital Audio Workstation) in Rust. Contains action dispatch, audio engine (SuperCollider via OSC), persistence, and domain logic. The TUI binary lives in `../imbolc-ui/`. Types are in `../imbolc-types/`. See the workspace root [../CLAUDE.md](../CLAUDE.md) for overview.

## Directory Structure

```
src/
  lib.rs           — Crate root, re-exports
  action.rs        — Re-exports Action from imbolc-types, DispatchResult
  config.rs        — TOML config loading (musical defaults)
  paths.rs         — Path resolution utilities
  scd_parser.rs    — SuperCollider .scd file parser
  vst3_probe.rs    — VST3 plugin discovery

  audio/           — SuperCollider OSC client and audio engine
    mod.rs           — Module exports
    handle.rs        — AudioHandle (main-thread interface)
    audio_thread.rs  — AudioThread (runs in separate thread)
    commands.rs      — AudioCmd and AudioFeedback enums
    playback.rs      — Playback scheduling, sequencer tick
    engine/          — Audio engine internals
      mod.rs           — Engine state
      backend.rs       — SuperCollider backend
      server.rs        — Server communication
      voices.rs        — Voice management
      voice_allocator.rs — Polyphonic voice allocation
      routing.rs       — Bus routing
      samples.rs       — Sample loading
      recording.rs     — Audio recording
      automation.rs    — Automation playback
      vst.rs           — VST hosting
      node_registry.rs — SC node tracking
    osc_client.rs    — OSC message sending
    bus_allocator.rs — SC bus allocation
    snapshot.rs      — State snapshots for audio thread
    triple_buffer.rs — Lock-free state transfer
    drum_tick.rs     — Drum sequencer tick
    arpeggiator_tick.rs — Arpeggiator tick

  dispatch/        — Action handler (all state mutation happens here)
    mod.rs           — Main dispatch_action(), re-exports
    local.rs         — LocalDispatcher implementation
    helpers.rs       — Dispatch utilities
    instrument/      — Instrument-related dispatch
    piano_roll.rs    — Note editing actions
    automation.rs    — Automation actions
    sequencer.rs     — Sequencer/transport actions
    mixer.rs         — Mixer actions
    session.rs       — Session actions
    server.rs        — Server control actions
    bus.rs           — Bus routing actions
    midi.rs          — MIDI configuration actions
    vst_param.rs     — VST parameter actions
    arrangement.rs   — Arrangement/clip actions
    audio_feedback.rs — Processing audio thread feedback

  midi/            — MIDI utilities
    mod.rs           — MIDI connection handling

  state/           — State management
    mod.rs           — AppState definition, re-exports from imbolc-types
    persistence/     — SQLite save/load implementation
      mod.rs           — save_project(), load_project()
      blob.rs          — Binary serialization
      tests.rs         — Persistence tests
    undo.rs          — Undo/redo history
    grid.rs          — Grid calculations
    recent_projects.rs — Recent project list
    audio_feedback.rs  — Audio feedback state
    midi_connection.rs — MIDI device state
    vst_plugin.rs    — VST plugin state
    clipboard.rs     — Re-exports from imbolc-types
    (other state files re-export from imbolc-types)
```

**Note:** State types (Instrument, SessionState, etc.) are defined in `imbolc-types`. This crate re-exports them and provides the dispatch/audio implementation.

## Key Types

| Type | Location | What It Is |
|------|----------|------------|
| `AppState` | `src/state/mod.rs` | Top-level state, passed to panes as `&AppState` |
| `Instrument` | `imbolc-types/src/state/instrument/` | One instrument: source + filter + effects + LFO + envelope |
| `InstrumentState` | `imbolc-types/src/state/instrument_state.rs` | Collection of instruments and selection state |
| `SessionState` | `imbolc-types/src/state/session.rs` | Global session data: buses, mixer, transport |
| `SourceType` | `imbolc-types/src/state/instrument/source_type.rs` | Oscillator/Source types (Saw/Sin/etc, AudioIn, BusIn, etc.) |
| `EffectSlot` | `imbolc-types/src/state/instrument/effect.rs` | One effect in the chain |
| `Action` | `imbolc-types/src/action.rs` | Action enum dispatched by the TUI binary |
| `LocalDispatcher` | `src/dispatch/local.rs` | Owns state, dispatches actions |
| `AudioHandle` | `src/audio/handle.rs` | Main-thread interface; sends AudioCmd to audio thread |

## Critical Patterns

### Action Dispatch

The TUI binary returns `Action` values from pane handlers. `dispatch/` matches on them and mutates state. When adding a new action:
1. Add variant to `Action` enum in `imbolc-types/src/action.rs`
2. Handle it in `dispatch::dispatch_action()` in `src/dispatch/mod.rs`

### State Ownership

`LocalDispatcher` owns `AppState` and `io_tx`. `AudioHandle` is kept separate to avoid borrow conflicts. The main loop calls `dispatcher.dispatch_with_audio(&action, &mut audio)`.

## Build & Test

```bash
cargo build -p imbolc-core
cargo test -p imbolc-core
```

## Configuration

TOML-based configuration with embedded defaults and optional user overrides.

- **Musical defaults:** `config.toml` (embedded via `include_str!`) + `~/.config/imbolc/config.toml` (user override)
- Config loading: `src/config.rs` — `Config::load()` parses embedded defaults, layers user overrides

Musical defaults (`[defaults]` section): `bpm`, `key`, `scale`, `tuning_a4`, `time_signature`, `snap`

## Persistence

- Format: SQLite database (`.imbolc` / `.sqlite`)
- Save/load: `save_project()` / `load_project()` in `src/state/persistence/mod.rs`
- Default path: `~/.config/imbolc/default.sqlite`
- Persists: instruments, params, effects, filters, sends, modulations, buses, mixer, piano roll, automation, sampler configs, custom synthdefs, drum sequencer, midi settings, VST plugins, VST param values, VST state paths

## Plans

Implementation plans live at workspace root: `../plans/`
