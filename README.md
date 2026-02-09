# imbolc

Imbolc is a complete music studio that runs right in your terminal. Write beats, layer synthesizers, shape sounds with effects, mix tracks, and record finished songs — all without leaving your keyboard. It ships with 55 built-in instruments, 39 effects, a piano roll, a drum sequencer, a mixer, and even real-time collaboration over your local network. It's free, open-source, and yours to keep.

Under the hood it's a Rust application powered by SuperCollider for audio synthesis, with a terminal UI built on ratatui and an experimental Dioxus GUI — both sharing the same core engine.

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

## Architecture

### Threading model

Three main threads: **UI** (60 fps render + input), **dispatch** (action handling + state mutation), and **audio** (~2 kHz tick loop driving SuperCollider over OSC). A shared retained **event log** (`EventLogWriter`/`EventLogReader`) connects dispatch to audio — the main thread appends `Arc<LogEntry>` entries and the audio thread drains them within a 100 µs budget. No shared locks; all communication is channel-based.

The audio thread is the **timing authority** for transport state (`playing`, `playhead`, `bpm`). The main thread keeps a read-only mirror updated via `PlayingChanged` feedback. Dispatch dual-writes for immediate UI consistency.

### Dispatch & side effects

Dispatch functions are **pure state mutations** that return typed `AudioSideEffect` variants (~30 covering voice management, transport, samples, mixer, click track, tuner, drums, automation, EQ, server lifecycle, recording, VST). The top-level `dispatch_with_audio()` collects effects and applies them after dispatch returns — dispatchers never call audio methods directly.

Actions forwarded to the audio thread are applied as **incremental projections** (`action_projection.rs`) rather than full-state clones.

### Scheduling & lookahead

Playback pre-schedules notes by scanning the piano roll ahead of the playhead using a high-water mark to avoid double-scheduling. A **dedicated OSC sender thread** (bounded 512-entry channel) removes synchronous UDP I/O from the audio thread — bundles are pre-encoded and queued, then transmitted asynchronously.

Lookahead is **computed dynamically** from audio device parameters: `max(buffer_size / sample_rate + 5 ms jitter margin, 10 ms floor)`. This adapts to actual hardware (e.g. 64/44100 → 10 ms, 1024/44100 → 28.2 ms) instead of using a hardcoded constant.

### Control-plane separation

Heavy operations run off the audio thread: server startup, OSC connection, and SynthDef compilation each run in background threads that the audio thread polls. Routing rebuilds use a phased state machine (~0.5 ms per phase). A two-channel dispatch system gives priority operations (voice spawn, param changes) a 200 µs budget and normal operations (state updates, routing) a 100 µs budget.

### Voice allocation

Polyphonic voice stealing with SC `/n_end` OSC feedback for ground-truth voice death. Voices and control buses are reclaimed immediately on `/n_end`; timer-based `cleanup_expired()` is retained as a safety net.

### Undo

Scope-aware `UndoEntry` variants (`SingleInstrument`, `Session`, `Full`). A scope classifier routes each action to the narrowest scope — a single-instrument param tweak clones one instrument instead of all 64. Persistence is unaffected (full-state SQLite snapshots).

### Networking

LAN collaboration via `imbolc-net`: single audio server, multiple clients, control data over TCP (no audio over network, no drift correction needed). Per-instrument dirty flags with `InstrumentPatch` delta updates, rate-limited at ~30 Hz with threshold coalescing (falls back to full snapshot when >50% of instruments are dirty).

### Routing

Output targets route instruments to master (hardware bus 0) or named buses. Per-send tap points (`PreInsert` or `PostInsert`, default post-insert) control where sends read from the signal chain.

### Roadmap

- Modular routing: targeted loosening of the fixed instrument → bus → master topology toward a more flexible signal graph.
- Documentation pruning: reduce `docs/` to actively-maintained reference material.
- SuperCollider remains the long-term audio backend.

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
