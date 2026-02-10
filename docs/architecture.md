# Architecture

Detailed architecture reference for the imbolc codebase. See [CLAUDE.md](../CLAUDE.md) for quick reference.
For VST3 plans and UI targets, see `plans/vst3-support-roadmap.md`.

## State Ownership

All state lives in `AppState`, owned by `main.rs` and passed to panes by reference:

```rust
// imbolc-core/src/state/mod.rs (types defined in imbolc-types)
pub struct AppState {
    pub session: SessionState,
    pub instruments: InstrumentState,
    pub pending_recording_path: Option<PathBuf>,
    pub keyboard_layout: KeyboardLayout,
    pub recording: bool,
    pub recording_secs: u64,
    pub automation_recording: bool,
}
```

`InstrumentState` contains the instruments:

```rust
// imbolc-types/src/state/instrument_state.rs
pub struct InstrumentState {
    pub instruments: Vec<Instrument>,
    pub selected: Option<usize>,
    pub next_id: InstrumentId,
    pub next_sampler_buffer_id: u32,
    pub editing_instrument_id: Option<InstrumentId>,
}
```

`SessionState` contains global settings and other state:

```rust
// imbolc-types/src/state/session.rs
pub struct SessionState {
    pub key: Key,
    pub scale: Scale,
    pub bpm: u16,
    pub tuning_a4: f32,
    pub snap: bool,
    pub time_signature: (u8, u8),
    pub piano_roll: PianoRollState,
    pub automation: AutomationState,
    pub midi_recording: MidiRecordingState,
    pub custom_synthdefs: CustomSynthDefRegistry,
    pub vst_plugins: VstPluginRegistry,
    pub buses: Vec<MixerBus>,
    pub master_level: f32,
    pub master_mute: bool,
    pub mixer_selection: MixerSelection,
}
```

## The Instrument Model

An `Instrument` is the fundamental unit — it combines what were previously separate rack modules (oscillator, filter, effects, output) into a single entity:

```rust
// imbolc-types/src/state/instrument/mod.rs
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    pub source: SourceType,        // Saw/Sin/Sqr/Tri, Noise/Pulse/SuperSaw/etc, AudioIn, BusIn, PitchedSampler, Kit, Custom, VST
    pub source_params: Vec<Param>,
    pub filter: Option<FilterConfig>,
    pub effects: Vec<EffectSlot>,   // Each slot has its own vst_param_values and vst_state_path
    pub lfo: LfoConfig,
    pub amp_envelope: EnvConfig,
    pub polyphonic: bool,
    // Integrated mixer controls
    pub level: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub active: bool,
    pub output_target: OutputTarget,  // Master or Bus(1-8)
    pub sends: Vec<MixerSend>,
    pub sampler_config: Option<SamplerConfig>,
    pub drum_sequencer: Option<DrumSequencerState>,
    pub vst_param_values: Vec<(u32, f32)>,  // VST source param overrides
    pub vst_state_path: Option<PathBuf>,     // VST source state file
}
```

When an instrument is added:
- A piano roll track is auto-created
- Sampler instruments get a default `SamplerConfig`
- Kit instruments get a `DrumSequencerState`
- Custom synthdef instruments get params from the registry

## Pane Trait & Rendering

All panes implement the `Pane` trait (`src/ui/pane.rs`):

```rust
pub trait Pane {
    fn id(&self) -> &'static str;
    fn handle_action(&mut self, action: &str, event: &InputEvent, state: &AppState) -> Action;
    fn handle_raw_input(&mut self, event: &InputEvent, state: &AppState) -> Action;
    fn handle_mouse(&mut self, event: &MouseEvent, area: RatatuiRect, state: &AppState) -> Action;
    fn render(&self, area: RatatuiRect, buf: &mut Buffer, state: &AppState);
    fn keymap(&self) -> &Keymap;
    fn on_enter(&mut self, _state: &AppState) {}
    fn on_exit(&mut self, _state: &AppState) {}
    fn toggle_performance_mode(&mut self, _state: &AppState) -> ToggleResult { ToggleResult::NotSupported }
    fn activate_piano(&mut self) {}
    fn activate_pad(&mut self) {}
    fn deactivate_performance(&mut self) {}
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

Every pane receives `&AppState` for input handling and rendering. `handle_action()`
is called only when the layer stack resolves a key to an action string; otherwise
`handle_raw_input()` receives the raw event (e.g., for text input).

### Registered Panes

| ID | Pane | Key | Purpose |
|----|------|-----|---------|
| `instrument` | `InstrumentPane` | `F1` | Main view — list of instruments with params |
| `piano_roll` | `PianoRollPane` | `F2` | Note grid editor with playback |
| `sequencer` | `SequencerPane` | `F2` (context) | Drum sequencer / song structure |
| `mixer` | `MixerPane` | `F4` | Mixer channels, buses, master |
| `server` | `ServerPane` | `F5` | SuperCollider server status/control |
| `track` | `TrackPane` | `F3` | Timeline overview (Arrangement view) |
| `automation` | `AutomationPane` | `F7` | Automation lanes and point editing |
| `eq` | `EqPane` | `F8` | 12-band parametric EQ editor |
| `waveform` | `WaveformPane` | `F2` (context) | Waveform, spectrum, oscilloscope, level meter |
| `home` | `HomePane` | — | Welcome screen |
| `add` | `AddPane` | — | Instrument creation menu |
| `add_effect` | `AddEffectPane` | — | Add an effect (including VST effects) to the current instrument |
| `instrument_edit` | `InstrumentEditPane` | — | Edit instrument params/effects/filter |
| `instrument_picker` | `InstrumentPickerPane` | — | Select instrument for actions |
| `vst_params` | `VstParamPane` | — | VST parameter browser (search/adjust/automation) |
| `frame_edit` | `FrameEditPane` | `Ctrl+f` | Session settings (BPM, key, etc.) |
| `file_browser` | `FileBrowserPane` | — | File selection for imports |
| `sample_chopper` | `SampleChopperPane` | — | Slice audio and assign pads |
| `command_palette` | `CommandPalettePane` | `Ctrl+p` | Search and execute commands |
| `pane_switcher` | `PaneSwitcherPane` | `;` | Global pane navigation |
| `confirm` | `ConfirmPane` | — | Confirmation dialog |
| `quit_prompt` | `QuitPromptPane` | — | Quit confirmation with unsaved changes warning |
| `midi_settings` | `MidiSettingsPane` | `Ctrl+m` | Configure MIDI input devices |
| `project_browser` | `ProjectBrowserPane` | `Ctrl+o` | Open/manage projects |
| `save_as` | `SaveAsPane` | `Ctrl+S` | Save project as new file |
| `docs` | `DocsPane` | — | Built-in documentation viewer |
| `help` | `HelpPane` | `?` | Context-sensitive keybinding help |

### Pane Communication

Panes communicate exclusively through `Action` values. A pane's `handle_action()` or
`handle_raw_input()` returns an `Action`, which is dispatched by
`dispatch::dispatch_action()` in `imbolc-core/src/dispatch/mod.rs`. This function receives
`&mut AppState` and `&mut AudioHandle`, mutates state, sends audio commands, and returns a
`DispatchResult` (nav/status/audio-dirty) that `main.rs` applies.

For cross-pane data passing (e.g., opening the editor with a specific instrument's data), the dispatch function uses `PaneManager::get_pane_mut::<T>()` to downcast and configure the target pane before switching to it.

## Borrow Patterns

When dispatch needs data from one pane to configure another:

```rust
// Extract data first, then use — the two borrows don't overlap
let inst_id = state.instruments.selected.map(|idx| state.instruments.instruments[idx].id);
if let Some(id) = inst_id {
    if let Some(edit) = panes.get_pane_mut::<InstrumentEditPane>("instrument_edit") {
        edit.set_instrument(id);
    }
    panes.switch_to("instrument_edit", state);
}
```

The key constraint: extracted data must be owned (cloned), not a reference. Each `get_pane_mut` borrows `&mut self` on `PaneManager`, so you can't hold two simultaneously.

## Action Dispatch Flow

```
User Input
  → main.rs: poll_event()
  → layer_stack.resolve(event) → Action string or Blocked/Unresolved
  → handle_global_action(...) for global layer actions
  → Pane::handle_action(action, event, state) OR Pane::handle_raw_input(event, state)
  → returns Action
  → dispatch::dispatch_action() matches on Action
  → mutates AppState / sends commands via AudioHandle
  → returns DispatchResult (quit/nav/status/audio_dirty)
  → main.rs applies nav/status + audio flush
```

The `dispatch_action()` function (`imbolc-core/src/dispatch/mod.rs`) handles all action variants and returns a `DispatchResult` (including quit + nav intents).

## Audio Engine

Located in `imbolc-core/src/audio/`. Communicates with SuperCollider (scsynth) via OSC over UDP. `AudioEngine` runs on a dedicated audio thread; the main thread communicates with it via MPSC command/feedback channels through `AudioHandle`.

### Key Components

- `AudioHandle` (`audio/handle.rs`) — main-thread interface; sends `AudioCmd` to audio thread, receives `AudioFeedback`
- `AudioThread` (`audio/handle.rs`) — dedicated thread; owns `AudioEngine`, runs tick loop at ~1ms resolution
- `AudioCmd` / `AudioFeedback` (`audio/commands.rs`) — command and feedback enums for cross-thread communication
- `AudioEngine` (`audio/engine/mod.rs`) — manages synth nodes, bus allocation, routing, voice allocation (lives on audio thread)
- `OscClient` (`audio/osc_client.rs`) — OSC message/bundle sending
- `BusAllocator` (`audio/bus_allocator.rs`) — audio/control bus allocation

### SuperCollider Groups

```
GROUP_SOURCES    = 100  — all source synths (oscillators, samplers)
GROUP_PROCESSING = 200  — filters, effects, mixer processing
GROUP_OUTPUT     = 300  — output synths
GROUP_RECORD     = 400  — recording nodes
```

### Instrument → Synth Mapping

Instruments map to SuperCollider nodes in two ways:
1. **Persistent Nodes:** `AudioIn` and `BusIn` instruments have static source nodes.
2. **Polyphonic Voices:** Oscillator and Sampler instruments spawn a new "voice chain" (group containing source + midi control node) for every note-on event.
3. **VST Instruments:** Persistent node hosts the VSTPlugin UGen; note-on/off is sent via `/u_cmd` MIDI messages.

Filters and effects are currently static per-instrument nodes (shared by all polyphonic voices), though the architecture allows for per-voice effects in the future.

**LFO Modulation:** Each instrument's LFO writes to a control bus (`lfo_out`). Routing-level targets (filter resonance, pan, delay time/feedback, reverb mix, gate rate, send level) are wired in `rebuild_instrument_routing()` by passing the LFO bus as a `*_mod_in` param to the relevant synth. Voice-level targets (amplitude, pitch, detune, pulse width, sample rate, attack, release) are wired in `spawn_voice()` / `spawn_sampler_voice()` by looking up the bus via `bus_allocator.get_control_bus()`. All SynthDefs accept `*_mod_in` params defaulting to -1 (no modulation).

### OSC Communication

- `OscClient::send_message()` — fire-and-forget single message
- `OscClient::set_params_bundled()` — multiple params in one timestamped bundle
- `OscClient::send_bundle()` — multiple messages in one timestamped bundle
- `osc_time_from_now(offset_secs)` — NTP timetag for sample-accurate scheduling

Use bundles for timing-sensitive operations (note events). Individual messages are fine for UI parameter changes.

## Playback Engine

Lives on the dedicated audio thread (`AudioThread::tick_playback` in `imbolc-core/src/audio/handle.rs`), ticking at ~1ms resolution independently of the UI frame rate:

1. Compute elapsed real time since last tick
2. Convert to ticks: `seconds * (bpm / 60) * ticks_per_beat`
3. Advance playhead, handle loop wrapping
4. Scan all tracks for notes starting in the elapsed tick range
5. Call `AudioEngine::spawn_voice()` for note-ons (sends OSC bundles)
6. Track active notes and call `AudioEngine::release_voice()` when expired

Tick resolution: 480 ticks per beat. Notes are sent as OSC bundles with NTP timetags for sample-accurate scheduling. Because the audio thread is decoupled from UI rendering, playback timing is unaffected by UI load.

## Persistence

SQLite database via `rusqlite`. Implementation in `imbolc-core/src/state/persistence/`.

### What's Persisted

Comprehensive — the full state survives save/load:
- Instruments (source type, name, filter, LFO, envelope, polyphonic, mixer controls)
- Source parameters (with type: float/int/bool)
- Effects chain (type, params, enabled, ordering, VST state paths)
- Sends (per-instrument bus sends with level and enabled)
- Filter modulation sources (LFO, envelope, or instrument-param cross-modulation)
- Mixer buses (name, level, pan, mute, solo)
- Master level and mute
- Piano roll tracks and notes
- Musical settings (BPM, time signature, key, scale, tuning, loop points)
- Automation lanes and points (with curve types)
- Sampler configs (buffer, loop mode, slices)
- Drum Sequencer state (pads, patterns, steps)
- Chopper state
- MIDI recording settings and mappings
- Custom synthdef registry (name, params, source path)
- VST plugin registry (name, path, params)
- VST param values (per-instrument source and per-effect slot)
- VST state paths (per-instrument source and per-effect slot, auto-loaded on project open)

### What's NOT Persisted

- UI selection state (instrument selection, mixer selection) is partially saved in `session` table
- Playback position
- Audio engine state (rebuilt on connect)
- Audio monitor waveforms (regenerated on load/record)

## Undo/Redo & Clipboard

### Undo System
The undo system (`imbolc-core/src/state/undo.rs`) uses a command pattern. Actions that modify state return an `UndoableAction` enum variant, which captures enough information to reverse the change.
- **Stacks:** `undo_stack` and `redo_stack` in `AppState`.
- **Capture:** State snapshots or delta-based inversion.
- **Scope:** Covers note edits, instrument parameters, mixer changes, and sequencer edits.

### Clipboard
The clipboard (`imbolc-core/src/state/clipboard.rs`) supports typed data:
- `Notes`: List of `MidiNote` (piano roll).
- `Steps`: List of sequencer steps.
- `Pattern`: Full sequencer pattern.
- `Automation`: Points and curve data.
