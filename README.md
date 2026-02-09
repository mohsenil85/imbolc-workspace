# imbolc

Imbolc is a terminal-first digital audio workstation (DAW) written in Rust. The audio engine runs on SuperCollider (`scsynth`) over OSC, with a ratatui TUI (`imbolc-ui`) and an experimental Dioxus GUI (`imbolc-gui`) that share the same core engine.

## Quick start

- Install Rust (edition 2021) and SuperCollider (`scsynth` on PATH; `sclang` needed for SynthDef compilation).
- Compile SynthDefs: `imbolc-core/bin/compile-synthdefs`
- Run the TUI: `cargo run -p imbolc-ui --release`
- Run the GUI: `cargo run -p imbolc-gui --release`

Developer mode (UI only):

```bash
IMBOLC_NO_AUDIO=1 cargo run -p imbolc-ui
```

Optional: override SynthDef location with `IMBOLC_SYNTHDEFS_DIR=/path/to/synthdefs`.

## Features

### Audio engine (SuperCollider backend)

- Instrument model: source + filter + EQ + FX chain + LFO + envelope + mixer; mono/stereo per instrument.
- Sources: 55 built-in types (oscillators, FM/PM, physical models, mallets, strings, drums, classic synths, experimental, additive/wavetable/granular, audio/bus input, samplers, time stretch, kit) plus custom SynthDefs and VST instruments.
- Filters: 8 types (low/high/band-pass, notch, comb, allpass, vowel, resdrive).
- Effects: 39 built-ins (delay/reverb/comp, modulation, distortion, EQ, granular, spectral, utility, etc.) plus VST effects.
- Modulation + automation share a unified `ParameterTarget` covering mixer, filter, envelope, synthesis, FX, EQ, groove, VST, and session params.
- Voice allocation: polyphonic voice stealing with `/n_end` feedback for accurate release + control-bus recycling.
- Low-latency scheduling: dedicated audio thread with lookahead OSC bundling.

### Sequencing & arrangement

- Piano roll with per-note velocity, probability, swing, and per-track groove/humanize.
- Drum sequencer with 16-step patterns, variable grid resolution, per-step velocity/pitch, and sample selection.
- Sample chopper with waveform preview, auto-slice, manual slices, and pad assignment.
- Track/arrangement view with clip capture, placement, duplication, and play modes.
- Automation lanes for instrument, bus, and global parameters (including VST params) with curve types.
- Arpeggiator with direction, octave range, and gate settings.

### Mixer & routing

- Mixer with per-instrument and bus levels, pan, mute/solo, 8 buses, sends, and master control.
- Output targets route instruments to master or buses; `BusIn` instruments can read from buses.
- Per-send tap points (pre-insert or post-insert; default post-insert).
- 12-band parametric EQ per instrument.

### UI

- TUI with 27 panes: instruments, instrument editor, piano roll, sequencer, track/arrangement, mixer, automation, EQ, VST params, server control, waveform/spectrum/oscilloscope/LUFS meter, project browser, docs, command palette, help, groove, tuner, checkpoints, and more.
- Keyboard-first navigation with contextual help and command palette.
- Performance mode: piano/pad overlay (`/`).
- Full undo/redo history and clipboard.

### Recording & export

- Real-time master recording to WAV.
- Per-instrument render, master bounce, and stem export (with progress UI).

### Networking (optional)

- LAN collaboration via `imbolc-net`: single audio server, multiple clients, control data only (no audio over network).
- Per-instrument dirty-flag patches with full-snapshot fallback.

## UI tour (TUI defaults)

- `F1` Instruments, `F2` Piano Roll / Sequencer / Waveform, `F3` Track, `F4` Mixer, `F5` Server
- `F6` Docs (selected instrument), `Shift+F6` Learn (topic browser)
- `F7` Automation, `F8` EQ, `F9` Groove, `F10` Tuner
- `:` Command palette, `;` Pane switcher, `?` Context help
- `Space` Play/Stop, `Ctrl+r` Master record
- `Ctrl+s` Save, `Ctrl+l` Load, `Ctrl+z` Undo, `Ctrl+Z` Redo
- `Ctrl+o` Project browser, `Ctrl+f` Frame edit, `Ctrl+m` MIDI settings
- `1`-`9`, `0`, `_` Instrument select
- `T` Cycle UI theme

The canonical keybinding list lives in `imbolc-ui/keybindings.toml` and is surfaced in-app via `?`.

## VST support (experimental)

VST support is routed through SuperCollider's VSTPlugin UGen.

What works today:
- Manual import of `.vst` / `.vst3` bundles for instruments and effects.
- Parameter discovery via VST3 probe or OSC query (in VST Params pane press `d`).
- Search/adjust/reset parameters and add automation lanes.
- State save to `.fxp` and automatic restore on project load.

Current gaps:
- Plugin scanning/catalog and preset/program browser.
- Latency compensation (PDC).
- Full MIDI-learn workflow in the UI.

Setup notes:
- Install the VSTPlugin extension in SuperCollider.
- Generate wrapper SynthDefs: `sclang imbolc-core/synthdefs/compile_vst.scd`, then load synthdefs from the Server pane.

## Architecture status & roadmap

Current architecture (completed from `TASKS_ARCH.md` / `plans/questions.md`):
- Control-plane operations (server start/connect, synthdef compilation, routing rebuild) are async or phased to protect the audio thread.
- Voice allocator listens for `/n_end` to reclaim voices and control buses (timer cleanup retained as a fallback).
- Network sync uses per-instrument dirty flags and `InstrumentPatch` updates with rate limiting and snapshot fallback.
- Undo uses scope-aware diffs (single-instrument/session/full) while persistence stays full-state SQLite snapshots.

Long-term direction:
- Event-log architecture with the audio thread as timing authority and UI as projection-only.
- Event scheduler with dynamic lookahead and ring-buffered OSC bundles.
- Modular routing as a signal graph for arbitrary signal flow.
- Documentation pruning: keep reference docs current; per-crate `CLAUDE.md` files are the living contracts.

Decisions:
- SuperCollider remains the long-term backend.
- Network timing drift correction is not required while audio is centralized.

## Configuration & files

- Defaults: `imbolc-core/config.toml` and `imbolc-ui/keybindings.toml` (embedded at build time).
- Overrides: `~/.config/imbolc/config.toml`, `~/.config/imbolc/keybindings.toml`.
- Project file: `~/.config/imbolc/default.sqlite`.
- Custom synthdefs: `~/.config/imbolc/synthdefs/` (or `IMBOLC_SYNTHDEFS_DIR`).
- Audio device prefs: `~/.config/imbolc/audio_devices.json`.
- scsynth log: `~/.config/imbolc/scsynth.log`.
- App log: `~/.config/imbolc/imbolc.log`.
- Recordings: `master_<timestamp>.wav` in the current working directory.
- Renders: `~/.config/imbolc/renders/render_<instrument>_<timestamp>.wav`.
- Exports: `~/.config/imbolc/exports/bounce_<timestamp>.wav` and `stem_<name>_<timestamp>.wav`.
- VST state: `~/.config/imbolc/vst_states/*.fxp`.

## Workspace structure

```
imbolc/
├── imbolc-ui/       Terminal UI (ratatui + crossterm)
├── imbolc-gui/      Experimental GUI (Dioxus)
├── imbolc-core/     Core engine (state, dispatch, audio, persistence)
│   └── synthdefs/   SuperCollider SynthDefs (.scd → .scsyndef)
├── imbolc-types/    Shared type definitions
├── imbolc-net/      Network collaboration layer
├── docs/            Architecture + reference docs
└── plans/           Architecture questions and implementation plans
```

## Build & test

```bash
cargo build
cargo run -p imbolc-ui
cargo run -p imbolc-gui
cargo test
cargo test -- --ignored     # Include tmux-based E2E tests
```

Network builds:

```bash
cargo build -p imbolc-ui --features net
cargo build -p imbolc-ui --features mdns
```

## License

This project is licensed under the GNU GPL v3.0. See [LICENSE](LICENSE) for details.

## Support the Project

Imbolc is free and open source. If you find it useful, consider supporting development:

**Sponsorship**
- [GitHub Sponsors](https://github.com/sponsors/mohsenil85)
- [Ko-fi](https://ko-fi.com/mohsenil85)

**Paid Support**
- Priority support and consulting available — [email](mailto:mohsenil85@gmail.com)
