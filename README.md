# imbolc


imbolc is a terminal-based digital audio workstation (DAW) written in Rust. The UI is a TUI (ratatui) and the audio engine runs on SuperCollider (scsynth) via OSC. It is optimized for keyboard-first instrument editing, sequencing, and mixing inside the terminal.

## Quick start

- Install Rust (edition 2021) and SuperCollider (scsynth on PATH; sclang needed for synthdef compilation).
- Compile synthdefs: `imbolc-core/bin/compile-synthdefs`
- Run: `cargo run -p imbolc-ui --release`
- Use number keys `1`-`5` to switch panes, `F5` for server controls, and `?` for context help.

Developer mode (UI only):

```bash
IMBOLC_NO_AUDIO=1 cargo run -p imbolc-ui
```

## Features

### Audio Engine (Backend)

- **Instrument model:** Source + filter + FX chain + LFO (15 modulation targets) + envelope + mixer routing.
- **Sources (51 types):** Classic waves, FM/phase mod, physical models (pluck, bowed, blown, membrane), mallet percussion (marimba, vibes, kalimba, steel drum), plucked strings (guitar, harp, koto), drum synthesis, classic synths (choir, organ, brass), experimental (gendy, chaos), additive/wavetable/granular synthesis, audio in/bus in, pitched sampler, time stretch, kit, custom SynthDefs, VST instruments.
- **Filters:** Low-pass, high-pass, band-pass.
- **Effects (37 types):** Delay, reverb, gate, tape/sidechain comp, chorus, flanger, phaser, tremolo, distortion, bitcrusher, wavefolder, saturator, tilt EQ, stereo widener, freq shifter, limiter, pitch shifter, vinyl, cabinet, granular delay/freeze, convolution reverb, vocoder, ring mod, autopan, resonator, multiband comp, para EQ, spectral freeze, glitch, Leslie, spring reverb, env follower, mid/side, crossfader, denoise, autotune, wah pedal.
- **Smart voice stealing:** Multi-criteria scoring (released voices first, then lower velocity, older notes), same-pitch retriggering, 5ms anti-click fades.
- **Low-latency playback:** Dedicated audio thread (~1ms tick), OSC bundles with NTP timetags for sample-accurate scheduling.
- **Persistence:** SQLite project files with full state serialization.

### Sequencing & Arrangement

- **Piano roll:** Multi-track note editor with per-note velocity, probability, and swing.
- **Drum sequencer:** 16-step sequencer with per-step velocity and sample selection.
- **Sample chopper:** Slice-based beat making with auto-slice and manual markers.
- **Arrangement view:** Clip-based timeline with placement and editing.
- **Automation:** Per-track automation lanes for any parameter (including VST params).
- **Arpeggiator:** Configurable direction, octave range, gate length.

### Mixer & Routing

- **Mixer:** Channel/bus levels, pan, mute/solo, 8 buses, sends, master control.
- **Bus routing:** Insert and send effects, flexible output targets.
- **12-band parametric EQ:** Per-instrument frequency shaping.

### Terminal UI (Frontend)

- **27 panes:** Instrument list, instrument editor, piano roll, sequencer, mixer, automation, EQ, VST params, server control, waveform/spectrum/oscilloscope, project browser, command palette, help, and more.
- **Keyboard-first navigation:** Single-key actions, context-sensitive help (`?`), command palette (`Ctrl+p`).
- **Performance mode:** Piano/pad keyboard overlay (`/`) for live playing.
- **Analysis:** Real-time master level meter, spectrum analyzer, oscilloscope, braille waveform display.
- **Productivity:** Full undo/redo history, clipboard (copy/paste notes and steps).
- **Project management:** Save/load, project browser, recent projects.

## UI tour (defaults)

- `1` **Instruments:** list/manage instruments, `Enter` to edit the signal chain.
- `2` **Piano Roll:** multi-track MIDI sequencing.
- `3` **Sequencer / Chopper:** 16-step drum sequencer and sample slicing.
- `4` **Mixer:** levels, pan, mute/solo, sends.
- `5` **Server:** scsynth status, device selection, synthdef build/load, recording.
- `?` **Help:** Context-sensitive help for the active pane.
- `/` **Performance:** Toggle performance mode (piano/pad keyboard).
- `Ctrl+p` **Command Palette:** Search and execute commands.
- `Ctrl+f` **Frame Edit:** BPM, time signature, tuning, key/scale, snap.
- `Ctrl+s` / `Ctrl+l` Save/load default project.
- `u` / `Ctrl+r` Undo / Redo.
- `` ` `` / `~` Navigate back/forward through pane history.

The canonical keybinding list lives in `imbolc-ui/keybindings.toml` and is surfaced in-app via `?`.

## VST support (experimental)

VST support is routed through SuperCollider's VSTPlugin UGen and is still evolving.

What works today:
- Manual import of `.vst` / `.vst3` bundles for instruments and effects.
- VST instruments are hosted as persistent nodes; note-on/off is sent via `/u_cmd` MIDI messages.
- VST effects can be inserted in instrument FX chains.
- A VST parameter pane exists (search, adjust, reset, add automation lane).

Current gaps:
- Parameter discovery replies from VSTPlugin are not wired yet (UI exists, but populating requires manual trigger).
- No parameter UI for VST effects (only VST instruments have a param pane today).
- No preset/program browser; VST state save/restore is not surfaced in the UI yet.

Setup notes:
- Install the [VSTPlugin](https://git.iem.at/pd/vstplugin) extension in SuperCollider.
- Generate the wrapper synthdefs by running `sclang imbolc-core/synthdefs/compile_vst.scd`, then load synthdefs from the Server pane.

## Configuration & files

- Defaults: `imbolc-core/config.toml` and `imbolc-ui/keybindings.toml` (embedded at build time).
- Overrides: `~/.config/imbolc/config.toml`, `~/.config/imbolc/keybindings.toml`.
- Project file: `~/.config/imbolc/default.sqlite`.
- Custom synthdefs: `~/.config/imbolc/synthdefs/`.
- Audio device prefs: `~/.config/imbolc/audio_devices.json`.
- scsynth log: `~/.config/imbolc/scsynth.log`.
- Recordings: `master_<timestamp>.wav` in the current working directory.

## Workspace structure

This is a Cargo workspace with multiple crates:

```
imbolc/
├── imbolc-ui/       Terminal UI binary (ratatui + crossterm)
├── imbolc-core/     Core engine (state, dispatch, audio, persistence)
│   └── synthdefs/   SuperCollider synth definitions (.scsyndef)
├── imbolc-types/    Shared type definitions
├── imbolc-net/      (future) Network/collaboration layer
└── docs/            Architecture, audio routing, persistence docs
```

## Build & test

```bash
cargo build                 # Build all crates
cargo build -p imbolc-ui    # Build UI only
cargo run -p imbolc-ui      # Run the DAW
cargo test                  # Run all tests
cargo test -- --ignored     # Include tmux-based E2E tests
```

## License

This project is licensed under the GNU GPL v3.0. See [LICENSE](LICENSE) for details.
