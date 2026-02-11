# CODE_MAP.md

Comprehensive code map for agents. Read this first to avoid re-exploring.

## Action Dispatch Routing Table

### Top-Level Dispatch (`imbolc-core/src/dispatch/mod.rs`)

| Action Variant | Handler | Purpose |
|---|---|---|
| `Quit` | inline | Returns `DispatchResult::with_quit()` |
| `Nav(_)` | inline | No-op (handled by PaneManager in UI) |
| `Instrument(a)` | `instrument::dispatch_instrument` | Instrument CRUD, playback, effects, params |
| `Mixer(a)` | `mixer::dispatch_mixer` | Mixer level/pan/mute/solo, bus/group params |
| `PianoRoll(a)` | `piano_roll::dispatch_piano_roll` | Note add/delete/move, selection, grid |
| `Arrangement(a)` | `arrangement::dispatch_arrangement` | Clip CRUD, timeline placement |
| `Server(a)` | `server::dispatch_server` | SC server start/stop, device config |
| `Session(a)` | `session::dispatch_session` | Save/load, BPM, key, scale, tuning |
| `Sequencer(a)` | `sequencer::dispatch_sequencer` | Drum sequencer pad/step editing |
| `Chopper(a)` | `sequencer::dispatch_chopper` | Sample chopper slice config |
| `Automation(a)` | `automation::dispatch_automation` | Automation lane/point CRUD |
| `Midi(a)` | `midi::dispatch_midi` | MIDI CC mapping, channel config |
| `Bus(a)` | `bus::dispatch_bus` | Bus add/delete, bus effects |
| `LayerGroup(a)` | `bus::dispatch_layer_group` | Layer group effects, mixer params |
| `VstParam(a)` | `vst_param::dispatch_vst_param` | VST parameter value editing |
| `Click(a)` | `dispatch_click` (inline) | Click track toggle/volume |
| `Tuner(a)` | `dispatch_tuner` (inline) | Reference pitch play/stop |
| `AudioFeedback(f)` | `audio_feedback::dispatch_audio_feedback` | Process audio thread feedback |
| `Undo` | inline | Pop undo stack, apply, audio dirty based on undo scope |
| `Redo` | inline | Pop redo stack, apply, audio dirty based on undo scope |
| `None` | inline | No-op |
| `ExitPerformanceMode` | inline | No-op (handled in UI) |
| `PushLayer(_)` | inline | No-op (handled in UI) |
| `PopLayer(_)` | inline | No-op (handled in UI) |
| `SaveAndQuit` | inline | No-op (handled in main.rs) |

### Instrument Sub-Handlers (`imbolc-core/src/dispatch/instrument/`)

| Module | InstrumentAction Variants | Purpose |
|---|---|---|
| `crud.rs` | Add, Delete, Edit, Update | Instrument lifecycle, source params |
| `playback.rs` | PlayNote, PlayNotes, PlayDrumPad | Voice triggering via audio side-effects |
| `selection.rs` | Select, SelectNext/Prev/First/Last | Instrument selection state |
| `effects.rs` | AddEffect, RemoveEffect, ToggleEffectBypass, AdjustEffectParam, LoadIRResult, OpenVstEffectParams | Effect chain management |
| `filter.rs` | SetFilter, ToggleFilter, CycleFilterType, AdjustFilterCutoff/Resonance | Filter configuration |
| `lfo.rs` | ToggleLfo, AdjustLfoRate/Depth, SetLfoShape/Target | LFO modulation |
| `envelope.rs` | AdjustEnvelopeAttack/Decay/Sustain/Release | ADSR envelope |
| `eq.rs` | SetEqParam, ToggleEq | Parametric EQ |
| `arpeggiator.rs` | ToggleArp, CycleArpDirection/Rate, AdjustArpOctaves/Gate, CycleChordShape, ClearChordShape | Arpeggiator + chord |
| `groove.rs` | SetTrackSwing/SwingGrid, AdjustTrackSwing, SetTrackHumanize*, SetTrackTimingOffset, ResetTrackGroove, Set/CycleTrackTimeSignature | Per-track groove/timing |
| `layer.rs` | LinkLayer, UnlinkLayer | Layer group membership |
| `sample.rs` | LoadSampleResult | Sample buffer loading |

## Action Dispatch Pipeline

1. **User input** → crossterm key event
2. **Layer stack resolution** → `LayerResult::Action(id)` or `Blocked`/`Unresolved`
3. **Global handler** → `handle_global_action()` checks for global actions (quit, save, undo, nav)
4. **Pane handler** → `pane.handle_action(id)` or `pane.handle_raw_input()` returns `Action`
5. **Layer management** → process PushLayer/PopLayer/ExitPerformanceMode
6. **Navigation** → `panes.process_nav()` handles pane switching
7. **Dispatch** → `dispatcher.dispatch_with_audio(&action, &mut audio)` mutates state, returns `DispatchResult`
8. **Audio sync** → `audio.apply_dirty(state, audio_dirty, needs_full_sync)` sends changes to audio thread

## Type Composition Hierarchy

```
AppState (imbolc-core/src/state/mod.rs)
├── session: SessionState
│   ├── Musical: key, scale, bpm, tuning_a4, snap, time_signature
│   ├── piano_roll: PianoRollState (tracks, notes, grid)
│   ├── arrangement: ArrangementState (clips, placements)
│   ├── automation: AutomationState (lanes, points)
│   ├── midi_recording: MidiRecordingState
│   ├── custom_synthdefs: CustomSynthDefRegistry
│   ├── vst_plugins: VstPluginRegistry
│   ├── mixer: MixerState
│   │   ├── buses: Vec<MixerBus> (id, name, level, pan, mute, solo, effects)
│   │   ├── master_level, master_mute
│   │   ├── selection: MixerSelection (Instrument | LayerGroup | Bus | Master)
│   │   └── layer_group_mixers: Vec<LayerGroupMixer>
│   ├── humanize: HumanizeSettings
│   ├── click_track: ClickTrackState
│   └── theme: Theme
│
├── instruments: InstrumentState
│   └── instruments: Vec<Instrument>
│       └── Instrument
│           ├── id, name, source: SourceType, source_params
│           ├── processing_chain: Vec<ProcessingStage> (filters/EQ/effects)
│           ├── lfo: LfoConfig (enabled, rate, depth, shape, targets)
│           ├── amp_envelope: EnvConfig (ADSR)
│           ├── Mixer: level, pan, mute, solo, active, output_target, channel_config, sends
│           ├── sampler_config: Option<SamplerConfig>
│           ├── drum_sequencer: Option<DrumSequencerState>
│           ├── vst_param_values, vst_state_path
│           ├── arpeggiator, chord_shape
│           ├── convolution_ir_path
│           ├── layer_group: Option<u32>, layer_octave_offset
│           ├── next_effect_id
│           └── groove: GrooveConfig (swing, humanize, timing)
│
├── clipboard: Clipboard
├── io: IoState
├── keyboard_layout: KeyboardLayout
├── recording: RecordingState
├── audio: AudioFeedbackState (visualization, playhead)
├── undo_history: UndoHistory
├── project: ProjectMeta (path, dirty flag)
├── midi: MidiConnectionState
└── network: Option<NetworkDisplayContext>
```

## AudioEffect Reference

`AudioEffect` is the typed event stream replacing the old `AudioDirty` boolean flags. Dispatch handlers push `AudioEffect` variants into `DispatchResult.audio_effects: Vec<AudioEffect>`.

### AudioEffect Variants

| Variant | Purpose |
|---|---|
| `RebuildInstruments` | Instrument add/delete, source/processing chain change |
| `UpdateSession` | BPM/key/scale/time signature/humanize changes |
| `UpdatePianoRoll` | Note edits, loop change |
| `UpdateAutomation` | Automation lane/point change |
| `RebuildRouting` | Full SC node graph rebuild |
| `RebuildRoutingForInstrument(InstrumentId)` | Targeted per-instrument rebuild |
| `AddInstrumentRouting(InstrumentId)` | Add instrument without teardown |
| `DeleteInstrumentRouting(InstrumentId)` | Delete instrument without teardown |
| `RebuildBusProcessing` | Bus/layer-group effect changes |
| `UpdateMixerParams` | Level/pan/mute/solo on instruments/buses/groups |
| `UpdateFilterParam(InstrumentId, FilterParamKind, f32)` | Direct filter node update |
| `UpdateEffectParam(InstrumentId, EffectId, usize, f32)` | Direct effect node update |
| `UpdateLfoParam(InstrumentId, LfoParamKind, f32)` | Direct LFO node update |
| `UpdateBusEffectParam(u8, EffectId, usize, f32)` | Direct bus effect update |
| `UpdateLayerGroupEffectParam(u32, EffectId, usize, f32)` | Direct layer group effect update |
| `UpdateEqParam(InstrumentId)` | EQ parameter update |

Effects are collected in `DispatchResult.audio_effects: Vec<AudioEffect>` and applied by the runtime after dispatch returns.

## Module Maps

### imbolc-core: dispatch/

| File | Purpose |
|---|---|
| `mod.rs` | Top-level `dispatch_action()`, undo/redo handling |
| `local.rs` | `LocalDispatcher` — owns state, provides `dispatch_with_audio()` |
| `helpers.rs` | Dispatch utilities |
| `instrument/mod.rs` | Routes `InstrumentAction` to sub-handlers |
| `instrument/crud.rs` | Add/delete/edit/update instruments |
| `instrument/playback.rs` | Note/pad triggering → AudioSideEffect |
| `instrument/selection.rs` | Instrument selection state |
| `instrument/effects.rs` | Effect chain CRUD + param adjustment |
| `instrument/filter.rs` | Filter type/cutoff/resonance |
| `instrument/lfo.rs` | LFO toggle/rate/depth/shape/target |
| `instrument/envelope.rs` | ADSR adjustment |
| `instrument/eq.rs` | Parametric EQ params |
| `instrument/arpeggiator.rs` | Arpeggiator + chord shapes |
| `instrument/groove.rs` | Per-track swing/humanize/timing |
| `instrument/layer.rs` | Layer group link/unlink |
| `instrument/sample.rs` | Sample load results |
| `piano_roll.rs` | Note editing actions |
| `automation.rs` | Automation lane/point actions |
| `sequencer.rs` | Drum sequencer + chopper actions |
| `mixer.rs` | Mixer level/pan/mute/solo/send actions |
| `session.rs` | Save/load/BPM/key/scale/tuning |
| `server.rs` | SC server control |
| `bus.rs` | Bus + layer group CRUD/effects |
| `midi.rs` | MIDI CC mapping |
| `vst_param.rs` | VST parameter editing |
| `arrangement.rs` | Clip/arrangement actions |
| `audio_feedback.rs` | Audio feedback processing |

### imbolc-audio: src/ (separate crate, re-exported by core as `pub use imbolc_audio as audio`)

| File | Purpose |
|---|---|
| `lib.rs` | Crate root, `AudioStateProvider` trait, re-exports |
| `handle.rs` | `AudioHandle` — main-thread API, sends `AudioCmd` to audio thread |
| `audio_thread.rs` | Audio thread main loop, processes commands |
| `commands.rs` | `AudioCmd` and `AudioFeedback` enums |
| `playback.rs` | Playback scheduling, sequencer tick, lookahead |
| `arp_state.rs` | `ArpPlayState` — runtime arpeggiator state |
| `arpeggiator_tick.rs` | Arpeggiator tick scheduling |
| `drum_tick.rs` | Drum sequencer tick scheduling |
| `click_tick.rs` | Click track tick scheduling |
| `snapshot.rs` | State snapshot type aliases |
| `event_log.rs` | Event log for projectable actions + snapshots |
| `osc_sender.rs` | Background OSC bundle sender |
| `engine/mod.rs` | `AudioEngine` — SC backend, node map, bus tracking |
| `engine/backend.rs` | `AudioBackend` trait, OSC socket I/O |
| `engine/server.rs` | SC server boot/kill, device enumeration |
| `engine/voices.rs` | Voice spawning, note-off envelope |
| `engine/voice_allocator.rs` | Polyphonic voice pool, stealing, control bus recycling |
| `engine/routing.rs` | Amortized 5-phase SC node graph rebuild |
| `engine/node_registry.rs` | Best-effort SC node liveness tracking |
| `engine/samples.rs` | Sample buffer management |
| `engine/recording.rs` | Disk recording, export, stem bounce |
| `engine/automation.rs` | Automation point playback, curve eval |
| `engine/vst.rs` | VST hosting, param discovery |
| `bus_allocator.rs` | SC audio/control bus allocation |
| `triple_buffer.rs` | Lock-free state transfer (main → audio thread) |
| `paths.rs` | SynthDef path resolution |
| `devices.rs` | Audio device enumeration |
| `osc_client.rs` | OSC message construction |
| `osc_sender.rs` | Background OSC sender thread |
| `event_log.rs` | Time-ordered event log for pre-scheduling |
| `drum_tick.rs` | Drum sequencer playback |
| `arpeggiator_tick.rs` | Arpeggiator playback |
| `click_tick.rs` | Click track timing |
| `devices.rs` | Audio device enumeration |
| `telemetry.rs` | OSC queue depth, latency metrics |

### imbolc-core: state/

| File | Purpose |
|---|---|
| `mod.rs` | `AppState` struct, helper methods |
| `undo.rs` | `UndoHistory` — scoped snapshots, undo/redo |
| `persistence/mod.rs` | `save_project()`, `load_project()` entry points |
| `persistence/schema.rs` | SQLite table definitions (v7+ relational) |
| `persistence/save.rs` | Relational save logic |
| `persistence/load.rs` | Relational load + legacy blob fallback |
| `persistence/blob.rs` | Legacy binary format v1-v2 |
| `persistence/checkpoint.rs` | Named checkpoint create/restore |
| `persistence/tests.rs` | Persistence round-trip tests |
| `grid.rs` | Grid calculations, snap resolution |
| `recent_projects.rs` | MRU project list |
| `audio_feedback.rs` | Audio feedback state |
| `midi_connection.rs` | MIDI device state |
| `clipboard.rs` | Re-exports from imbolc-types |

### imbolc-ui: panes/ (30 panes)

| File | Pane ID | Purpose |
|---|---|---|
| `instrument_edit_pane/` | instrument_edit | Main instrument parameter editor (source, filter, effects, ADSR, LFO) |
| `instrument_pane.rs` | instrument | Instrument list with CRUD |
| `piano_roll_pane/` | piano_roll | Note editor with grid, selection, zoom |
| `mixer_pane/` | mixer | Console view: channels, buses, groups, faders |
| `track_pane.rs` | track | Timeline clip arrangement |
| `sequencer_pane.rs` | sequencer | 16-pad drum sequencer |
| `automation_pane/` | automation | Parameter automation lanes |
| `server_pane/` | server | SC server control, diagnostics |
| `waveform_pane.rs` | waveform | Waveform/spectrum/oscilloscope/LUFS |
| `eq_pane.rs` | eq | 12-band parametric EQ |
| `home_pane.rs` | home | Welcome screen |
| `add_pane.rs` | add | Add instrument type selector |
| `add_effect_pane.rs` | add_effect | Effect type selector |
| `file_browser_pane.rs` | file_browser | File/directory navigator |
| `project_browser_pane.rs` | project_browser | Recent projects |
| `sample_chopper_pane.rs` | sample_chopper | Sample slicing into pads |
| `save_as_pane.rs` | save_as | Save dialog with text input |
| `confirm_pane.rs` | confirm | Yes/No confirmation |
| `quit_prompt_pane.rs` | quit_prompt | Save/Don't Save/Cancel |
| `command_palette_pane.rs` | command_palette | Fuzzy command search |
| `pane_switcher_pane.rs` | pane_switcher | Quick pane navigation |
| `help_pane.rs` | help | Context-sensitive keybinding help |
| `frame_edit_pane.rs` | frame_edit | BPM/key/scale/tuning editor |
| `midi_settings_pane.rs` | midi_settings | MIDI port + CC mapping |
| `vst_param_pane/` | vst_params | VST parameter editor |
| `docs_pane/` | docs | Built-in documentation viewer |
| `checkpoint_list_pane.rs` | checkpoint_list | Undo checkpoint browser |
| `groove_pane.rs` | groove | Swing/humanize/timing settings |
| `tuner_pane.rs` | tuner | Reference pitch player |
| `instrument_picker_pane.rs` | instrument_picker | Instrument selector for drum pads |

### imbolc-ui: ui/ (framework modules)

| File | Purpose |
|---|---|
| `pane.rs` | `Pane` trait, `PaneManager`, navigation |
| `action_id.rs` | Typed action ID enums for keybinding system |
| `keymap.rs` | `Keymap` builder + lookup |
| `layer.rs` | `LayerStack` — context-sensitive input layers |
| `input.rs` | `KeyCode`, `InputEvent`, `MouseEvent`, `Modifiers` |
| `keybindings.rs` | TOML keybinding loading (embedded + user override) |
| `frame.rs` | `Frame` — header bar, master meter, metrics |
| `render.rs` | `RenderBuf` — rendering abstraction over ratatui |
| `style.rs` | `Color`, `Style`, semantic color constants |
| `piano_keyboard.rs` | Virtual piano keyboard (C/A layouts) |
| `pad_keyboard.rs` | Virtual 4x4 pad grid |
| `list_selector.rs` | Reusable list selection widget |
| `layout_helpers.rs` | `center_rect()` and layout utilities |

## Key Enums Quick Reference

### SourceType (56 built-in + 2 parametric)

- **Oscillators**: Saw, Sin, Sqr, Tri, Noise, Pulse, SuperSaw, Sync
- **FM/Modulation**: Ring, FBSin, FM, PhaseMod, FMBell, FMBrass
- **Physical Models**: Pluck, Formant, Bowed, Blown, Membrane
- **Mallets**: Marimba, Vibes, Kalimba, SteelDrum, TubularBell, Glockenspiel
- **Strings**: Guitar, BassGuitar, Harp, Koto
- **Drums**: Kick, Snare, HihatClosed, HihatOpen, Clap, Cowbell, Rim, Tom, Clave, Conga
- **Classic Synths**: Choir, EPiano, Organ, BrassStab, Strings, Acid
- **Experimental**: Gendy, Chaos
- **Synthesis**: Additive, Wavetable, Granular
- **Routing**: AudioIn, BusIn
- **Samplers**: PitchedSampler, TimeStretch, Kit
- **External**: Custom(CustomSynthDefId), Vst(VstPluginId)

### EffectType (40 built-in + 1 parametric)

- **Time**: Delay, Reverb, SpringReverb
- **Dynamics**: Gate, TapeComp, SidechainComp, Limiter, MultibandComp
- **Modulation**: Chorus, Flanger, Phaser, Tremolo, Autopan, Leslie
- **Distortion**: Distortion, Bitcrusher, Wavefolder, Saturator
- **EQ**: TiltEq, ParaEq
- **Stereo**: StereoWidener, MidSide, Crossfader
- **Pitch**: PitchShifter, Autotune, FreqShifter
- **Granular**: GranularDelay, GranularFreeze
- **Spectral**: SpectralFreeze, Glitch, Denoise
- **Convolution**: ConvolutionReverb
- **Character**: Vinyl, Cabinet
- **Synthesis**: RingMod, Resonator, Vocoder
- **Envelope**: EnvFollower, WahPedal
- **External**: Vst(VstPluginId)

### MixerSelection

```
Instrument(usize) — index into instruments vec
LayerGroup(u32)   — layer group ID
Bus(u8)           — bus 1-8
Master            — master fader
```

### OutputTarget

```
Master    — routes to master output
Bus(u8)   — routes to specific bus (1-8)
```

## State Mutation Patterns

### Undo Integration

Every undoable dispatch pushes a scoped snapshot before mutation:
- `SingleInstrument(id)` — one instrument only
- `Instruments` — all instruments
- `Session` — session state only
- `Full` — both session + instruments

Undo/Redo replaces state from stack and sets `AudioDirty::all()`.

### Persistence

- **Format**: SQLite, relational schema v7+, WAL mode
- **Save**: explicit via `Session(Save/SaveAs)`, async on background thread
- **Load**: explicit via `Session(Load/LoadFrom)`, async, replaces entire state
- **Checkpoints**: labeled snapshots within project DB

### Audio Thread Communication

- **Main → Audio**: Triple buffer (lock-free), state snapshots
- **Audio → Main**: `AudioFeedback` channel (playhead position, server status, meters)
- **Routing rebuild**: Amortized 5-phase state machine (TearDown → AllocBuses → BuildInstrument(i) → BuildOutputs → Finalize), each phase bounded ~0.5ms
