# CLAUDE.md

Guide for AI agents working on imbolc-core.

## What This Is

The core library for imbolc, a terminal-based DAW (Digital Audio Workstation) in Rust. Contains action dispatch, audio engine (SuperCollider via OSC), persistence, and domain logic. The TUI binary lives in `../imbolc-ui/`. Types are in `../imbolc-types/`. See the workspace root [../CLAUDE.md](../CLAUDE.md) for overview.

## Directory Structure

```
synthdefs/           — SuperCollider SynthDef source files
  compile.scd          — Master compile script (loads all defs)
  defs/                — SynthDef definitions organized by category
    oscillators/         — Basic waveform oscillators (saw, sin, sqr, tri, etc.)
    synthesis/           — Synthesis techniques (FM, granular, additive, etc.)
    physical_models/     — Physical modeling (bowed, blown, guitar, etc.)
    drums/               — Drum sounds (kick, snare, hihat, etc.)
    classic_synths/      — Classic synth emulations (organ, epiano, etc.)
    filters/             — Filter effects (lpf, hpf, bpf, etc.)
    effects/             — Audio effects (delay, reverb, chorus, etc.)
    modulation/          — Modulation sources (lfo, adsr)
    eq/                  — EQ processors
    input/               — Input sources (audio_in, bus_in)
    output/              — Output and routing (output, send, bus_out)
    samplers/            — Sample playback
    analysis/            — Metering and analysis
    midi/                — MIDI output

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

## Dispatch Sub-Handler Routing

Top-level dispatch is in `src/dispatch/mod.rs`. Each `Action` variant routes to a sub-handler:

| Action | Handler file | Purpose |
|---|---|---|
| `Instrument(a)` | `instrument/mod.rs` → sub-modules | See instrument table below |
| `Mixer(a)` | `mixer.rs` | Level/pan/mute/solo, bus/group params |
| `PianoRoll(a)` | `piano_roll.rs` | Note add/delete/move, selection |
| `Arrangement(a)` | `arrangement.rs` | Clip CRUD, timeline placement |
| `Session(a)` | `session.rs` | Save/load, BPM, key, scale |
| `Sequencer(a)` | `sequencer.rs` | Drum sequencer pad/step editing |
| `Chopper(a)` | `sequencer.rs` | Sample chopper slice config |
| `Automation(a)` | `automation.rs` | Automation lane/point CRUD |
| `Bus(a)` | `bus.rs` | Bus add/delete, bus effects |
| `LayerGroup(a)` | `bus.rs` | Layer group effects/mixer |
| `Server(a)` | `server.rs` | SC server start/stop, device config |
| `Midi(a)` | `midi.rs` | MIDI CC mapping |
| `VstParam(a)` | `vst_param.rs` | VST parameter editing |
| `Undo/Redo` | inline in `mod.rs` | Pop undo/redo stack, AudioDirty::all() |

**Instrument sub-handlers** (`src/dispatch/instrument/`):

| Module | Handles |
|---|---|
| `crud.rs` | Add, Delete, Edit, Update |
| `playback.rs` | PlayNote, PlayNotes, PlayDrumPad |
| `selection.rs` | Select, SelectNext/Prev/First/Last |
| `effects.rs` | AddEffect, RemoveEffect, MoveEffect, ToggleBypass, AdjustParam |
| `filter.rs` | SetFilter, ToggleFilter, CycleFilterType, AdjustCutoff/Resonance |
| `lfo.rs` | ToggleLfo, AdjustRate/Depth, SetShape/Target |
| `envelope.rs` | AdjustAttack/Decay/Sustain/Release |
| `eq.rs` | SetEqParam, ToggleEq |
| `arpeggiator.rs` | ToggleArp, CycleDirection/Rate, AdjustOctaves/Gate, ChordShape |
| `groove.rs` | Swing, humanize, timing offset, time signature |
| `layer.rs` | LinkLayer, UnlinkLayer |
| `sample.rs` | LoadSampleResult |

## Audio Engine Modules

| Module | Purpose |
|---|---|
| `handle.rs` | `AudioHandle` — main-thread API, sends `AudioCmd` via channel |
| `audio_thread.rs` | Audio thread loop, reads triple buffer snapshots |
| `engine/mod.rs` | `AudioEngine` — SC backend, node map, voice allocator |
| `engine/routing.rs` | Amortized 5-phase SC node graph rebuild (TearDown → AllocBuses → BuildInstrument → BuildOutputs → Finalize) |
| `engine/voice_allocator.rs` | Polyphonic voice pool (16/instrument), stealing by score, control bus recycling |
| `engine/voices.rs` | Voice spawning, note-off envelopes |
| `engine/backend.rs` | `AudioBackend` trait, OSC socket I/O |
| `engine/server.rs` | SC server boot/kill, device enumeration |
| `engine/samples.rs` | Sample buffer management |
| `engine/recording.rs` | Disk recording, export, stem bounce |
| `engine/automation.rs` | Automation playback, curve evaluation |
| `triple_buffer.rs` | Lock-free state transfer (main → audio thread, 3 slots: front/middle/back) |
| `event_log.rs` | Time-ordered event log for playback pre-scheduling |
| `osc_sender.rs` | Background OSC sender thread |
| `bus_allocator.rs` | SC audio/control bus allocation |
| `playback.rs` | Sequencer tick, lookahead scheduling |
| `drum_tick.rs` / `arpeggiator_tick.rs` / `click_tick.rs` | Per-feature playback ticks |

## State Mutation Patterns

### Undo Integration

Every undoable action pushes a scoped snapshot before mutation:
- `SingleInstrument(id)` — snapshot one instrument
- `Instruments` — all instruments
- `Session` — session state only
- `Full` — both session + instruments

Undo/Redo pops the stack, swaps state, sets `AudioDirty::all()`.

### Dirty Flags (AudioDirty)

`AudioDirty` is `Copy` (must stay Copy). Boolean flags OR on merge. Targeted param flags use `Option<(...)>`.

- **Structural**: `instruments`, `session`, `piano_roll`, `automation`, `routing`, `mixer_params`
- **Targeted optimization**: `routing_instrument: Option<InstrumentId>` (single-instrument rebuild)
- **Real-time /n_set**: `filter_param`, `effect_param`, `lfo_param`, `bus_effect_param`, `layer_group_effect_param`

See `CODE_MAP.md` at workspace root for the complete field-by-field reference.

### Persistence Triggers

- **Save**: explicit (`SessionAction::Save/SaveAs`), async background thread, SQLite relational schema v7+
- **Load**: explicit (`SessionAction::Load/LoadFrom`), async, replaces entire state
- **Checkpoints**: labeled snapshots within project DB (create/restore)
- `project.dirty` flag set on all undoable actions, cleared after save

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

## SynthDefs

SynthDef source files live in `synthdefs/defs/` organized by category.

**Hard rule: One SynthDef per file.** Each `.scd` file contains exactly one SynthDef definition. File names match the SynthDef name (e.g., `imbolc_kick.scd` contains `\imbolc_kick`).

### File template

```supercollider
// imbolc_example SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_example, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1)|
    // ... synthesis code ...
}).writeDefFile(dir);
)
```

### Adding a new SynthDef

1. Create file in appropriate subdirectory: `synthdefs/defs/<category>/<name>.scd`
2. Follow the template above (note `dirname.dirname.dirname` for correct output path)
3. Compile: `cd synthdefs && sclang compile.scd`
4. Add corresponding `SourceType` variant in `imbolc-types/src/state/instrument/source_type.rs`

### SuperCollider var declaration rule

All `var` declarations must come before any statements in a function. This is invalid:

```supercollider
var sig = SinOsc.ar(freq);
sig = sig * 0.5;           // statement
var env = EnvGen.kr(...);  // ERROR: var after statement
```

Fix by declaring all variables upfront:

```supercollider
var sig = SinOsc.ar(freq);
var env;
sig = sig * 0.5;
env = EnvGen.kr(...);
```

## Plans

Implementation plans live at workspace root: `../plans/`
