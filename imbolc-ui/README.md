# imbolc


imbolc is a terminal-based digital audio workstation (DAW) written in Rust. The UI is a TUI (ratatui) and the audio engine runs on SuperCollider (scsynth) via OSC. It is optimized for keyboard-first instrument editing, sequencing, and mixing inside the terminal.

## Quick start

- Install Rust (edition 2021) and SuperCollider (scsynth on PATH; sclang needed for synthdef compilation).
- Compile synthdefs: `bin/compile-synthdefs`
- Run: `cargo run --release` (or use `bin/imbolc`)
- Use number keys `1`-`5` to switch panes, `F5` for server controls, and `?` for context help.

Developer mode (UI only):

```bash
IMBOLC_NO_AUDIO=1 cargo run
```

## Features

- **Instrument model:** source + filter + FX chain + LFO (15 modulation targets) + envelope + mixer routing.
- **Sources:** classic waves + noise, sync, FM/phase mod, pluck, formant, gendy, chaos, additive, wavetable; audio in/bus in; polyphonic sampler; drum kit; custom SynthDefs; VST instruments (experimental).
- **Filters:** low-pass, high-pass, band-pass.
- **Effects:** delay, reverb, gate, tape/sidechain comp, chorus, flanger, phaser, tremolo, distortion, bitcrusher, wavefolder, saturator, tilt EQ, stereo widener, freq shifter, limiter, pitch shifter, vinyl, cabinet, granular delay/freeze, convolution reverb.
- **Sequencing:** multi-track piano roll with per-note velocity, probability, and swing.
- **Drum Machine:** 16-step drum sequencer with per-step velocity and sample selection.
- **Sampler:** Polyphonic sampler and a "Chopper" for slice-based beat making.
- **Mixer:** channel/bus levels, pan, mute/solo, 8 buses, sends, master control.
- **Automation:** per-track automation lanes for parameters (including VST params).
- **Productivity:** Full Undo/Redo history, Clipboard (copy/paste notes and steps), and a Command Palette (`Ctrl+p`) for quick actions.
- **Analysis:** Real-time master level meter, spectrum analyzer, oscilloscope, and waveform view for audio input.
- **Low-latency playback:** Dedicated audio thread (~1ms tick) using **OSC bundles with NTP timetags** for sample-accurate scheduling, decoupled from UI jitter.
- **Smart Voice Stealing:** Advanced polyphony management with multi-criteria scoring (prioritizing released voices, then lower velocity and older notes), optimized same-pitch retriggering, 5ms anti-click fades for stolen voices, and intelligent lifecycle cleanup.

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

The canonical keybinding list lives in `keybindings.toml` and is surfaced in-app via `?`.

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
- Generate the wrapper synthdefs by running `sclang synthdefs/compile_vst.scd`, then load synthdefs from the Server pane.

## Configuration & files

- Defaults: `config.toml` and `keybindings.toml` (embedded at build time).
- Overrides: `~/.config/imbolc/config.toml`, `~/.config/imbolc/keybindings.toml`.
- Project file: `~/.config/imbolc/default.sqlite`.
- Custom synthdefs: `~/.config/imbolc/synthdefs/`.
- Audio device prefs: `~/.config/imbolc/audio_devices.json`.
- scsynth log: `~/.config/imbolc/scsynth.log`.
- Recordings: `master_<timestamp>.wav` in the current working directory.

## Repo map

- `src/` - TUI app, panes, input layers, render loop.
- `imbolc-core/` - state model, dispatch, audio engine, persistence.
- `synthdefs/` - SuperCollider synth definitions (compiled `.scsyndef`).
- `docs/` - architecture, audio routing, persistence, and roadmaps.

## Build & test

```bash
cargo build
cargo test --bin imbolc
cargo test
# tmux-based E2E tests are ignored by default
cargo test -- --ignored
```

## License

This project is licensed under the GNU GPL v3.0. See [LICENSE](LICENSE) for details.
