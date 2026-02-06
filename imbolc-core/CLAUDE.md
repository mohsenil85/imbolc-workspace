# CLAUDE.md

Guide for AI agents working on imbolc-core.

## What This Is

The core library for imbolc, a terminal-based DAW (Digital Audio Workstation) in Rust. Contains all application state, action dispatch, audio engine (SuperCollider via OSC), persistence, and domain logic. The TUI binary lives in `../imbolc-ui/`. Types are in `../imbolc-types/`. See the workspace root [../CLAUDE.md](../CLAUDE.md) for overview.

## Directory Structure

```
src/
  action.rs        — Action enums + DispatchResult
  audio/           — SuperCollider OSC client and audio engine
    handle.rs        — AudioHandle (main-thread interface) and AudioThread (audio thread)
    commands.rs      — AudioCmd and AudioFeedback enums
  config.rs        — TOML config loading (musical defaults)
  dispatch/        — Action handler (all state mutation happens here)
  scd_parser.rs    — SuperCollider .scd file parser
  state/           — All application state
    mod.rs           — AppState (top-level), re-exports
    instrument.rs    — Instrument, InstrumentId, SourceType, FilterType, EffectType, LFO, envelope types
    instrument_state.rs — InstrumentState (instruments, selection, persistence helpers)
    session.rs       — SessionState (mixer, global settings, automation)
    persistence/     — SQLite save/load implementation
    piano_roll.rs    — PianoRollState, Track, Note
    automation.rs    — AutomationState, lanes, points, curve types
    sampler.rs       — SamplerConfig, SampleRegistry, slices
    custom_synthdef.rs — CustomSynthDef registry and param specs
    music.rs         — Key, Scale, musical theory types
    midi_recording.rs — MIDI recording state, CC mappings
    param.rs         — Param, ParamValue (Float/Int/Bool)
  midi/            — MIDI utilities
```

## Key Types

| Type | Location | What It Is |
|------|----------|------------|
| `AppState` | `src/state/mod.rs` | Top-level state, passed to panes as `&AppState` |
| `InstrumentState` | `src/state/instrument_state.rs` | Collection of instruments and selection state |
| `SessionState` | `src/state/session.rs` | Global session data: buses, mixer, piano roll, automation |
| `Instrument` | `src/state/instrument.rs` | One instrument: source + filter + effects + LFO + envelope + mixer |
| `InstrumentId` | `src/state/instrument.rs` | `u32` — unique identifier for instruments |
| `SourceType` | `src/state/instrument.rs` | Oscillator/Source types (Saw/Sin/etc, AudioIn, BusIn, PitchedSampler, Kit, Custom, VST) |
| `EffectSlot` | `src/state/instrument.rs` | One effect in the chain: type + params + enabled + VST param values/state path |
| `VstTarget` | `src/action.rs` | `Source` or `Effect(usize)` — identifies which VST node an action targets |
| `Action` | `src/action.rs` | Action enum dispatched by the TUI binary |
| `AudioHandle` | `src/audio/handle.rs` | Main-thread interface; sends AudioCmd via MPSC channel to audio thread |

## Critical Patterns

### Action Dispatch

The TUI binary returns `Action` values from pane handlers. `dispatch/` matches on them and mutates state. When adding a new action:
1. Add variant to `Action` enum in `src/action.rs`
2. Handle it in `dispatch::dispatch_action()` in `src/dispatch/mod.rs`

## Build & Test

```bash
cargo build
cargo test
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
- Persists: instruments, params, effects, filters, sends, modulations, buses, mixer, piano roll, automation, sampler configs, custom synthdefs, drum sequencer, midi settings, VST plugins, VST param values (source + effects), VST state paths

## Plans

Implementation plans live at workspace root: `../plans/`

## Comment Box

Log difficulties, friction points, or things that gave you trouble in `COMMENTBOX.md` at the project root.
