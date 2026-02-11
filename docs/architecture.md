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
    pub clipboard: Clipboard,
    pub io: IoState,
    pub keyboard_layout: KeyboardLayout,
    pub recording: RecordingState,
    pub audio: AudioFeedbackState,
    pub recorded_waveform_peaks: Option<Vec<f32>>,
    pub undo_history: UndoHistory,
    pub project: ProjectMeta,
    pub midi: MidiConnectionState,
    pub network: Option<NetworkDisplayContext>,
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
    pub next_layer_group_id: u32,
    /// Internal: id → index lookup (rebuilt on load/undo)
    pub id_index: HashMap<InstrumentId, usize>,
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
    pub arrangement: ArrangementState,
    pub automation: AutomationState,
    pub midi_recording: MidiRecordingState,
    pub custom_synthdefs: CustomSynthDefRegistry,
    pub vst_plugins: VstPluginRegistry,
    pub mixer: MixerState,
    pub humanize: HumanizeSettings,
    pub click_track: ClickTrackState,
    pub theme: Theme,
}
```

## The Instrument Model

An `Instrument` is the fundamental unit — it combines what were previously separate rack modules (oscillator, filter, effects, output) into a single entity:

```rust
// imbolc-types/src/state/instrument/mod.rs
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    pub source: SourceType,        // Oscillators, samplers, AudioIn/BusIn, Custom, VST
    pub source_params: Vec<Param>,
    pub processing_chain: Vec<ProcessingStage>, // Filters, EQ, effects (ordered)
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
    pub channel_config: ChannelConfig, // Mono/Stereo
    pub sends: Vec<MixerSend>,
    pub sampler_config: Option<SamplerConfig>,
    pub drum_sequencer: Option<DrumSequencerState>,
    pub vst_param_values: Vec<(u32, f32)>,  // VST source param overrides
    pub vst_state_path: Option<PathBuf>,     // VST source state file
    pub arpeggiator: ArpeggiatorConfig,
    pub chord_shape: Option<ChordShape>,
    pub convolution_ir_path: Option<String>,
    pub layer_group: Option<u32>,
    pub layer_octave_offset: i8,
    pub next_effect_id: EffectId,
    pub groove: GrooveConfig,
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
    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action;
    fn handle_raw_input(&mut self, event: &InputEvent, state: &AppState) -> Action;
    fn handle_mouse(&mut self, event: &MouseEvent, area: Rect, state: &AppState) -> Action;
    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState);
    fn keymap(&self) -> &Keymap;
    fn on_enter(&mut self, _state: &AppState) {}
    fn on_exit(&mut self, _state: &AppState) {}
    fn tick(&mut self, _state: &AppState) -> Vec<Action> { vec![] }
    fn toggle_performance_mode(&mut self, _state: &AppState) -> ToggleResult { ToggleResult::NotSupported }
    fn activate_piano(&mut self) {}
    fn activate_pad(&mut self) {}
    fn deactivate_performance(&mut self) {}
    fn supports_performance_mode(&self) -> bool { false }
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

Every pane receives `&AppState` for input handling and rendering. `handle_action()`
is called only when the layer stack resolves a key to an `ActionId`; otherwise
`handle_raw_input()` receives the raw event (e.g., for text input).

### Registered Panes

| ID | Pane | Key | Purpose |
|----|------|-----|---------|
| `instrument` | `InstrumentPane` | `F1` | Main view — list of instruments with params |
| `piano_roll` | `PianoRollPane` | `F2` (context) | Note grid editor with playback |
| `sequencer` | `SequencerPane` | `F2` (context) | Drum sequencer / song structure |
| `waveform` | `WaveformPane` | `F2` (context) | Waveform, spectrum, oscilloscope, level meter |
| `track` | `TrackPane` | `F3` | Timeline overview (Arrangement view) |
| `mixer` | `MixerPane` | `F4` | Mixer channels, buses, master |
| `server` | `ServerPane` | `F5` | SuperCollider server status/control |
| `docs` | `DocsPane` | `F6` / `Shift+F6` | Documentation viewer (context / browser) |
| `automation` | `AutomationPane` | `F7` | Automation lanes and point editing |
| `eq` | `EqPane` | `F8` | 12-band parametric EQ editor |
| `groove` | `GroovePane` | `F9` | Groove & humanize settings |
| `tuner` | `TunerPane` | `F10` | Reference tuner |
| `help` | `HelpPane` | `?` | Context-sensitive keybinding help |
| `command_palette` | `CommandPalettePane` | `:` | Search and execute commands |
| `pane_switcher` | `PaneSwitcherPane` | `;` | Global pane navigation |
| `frame_edit` | `FrameEditPane` | `Ctrl+f` | Session settings (BPM, key, etc.) |
| `midi_settings` | `MidiSettingsPane` | `Ctrl+m` | Configure MIDI input devices |
| `project_browser` | `ProjectBrowserPane` | `Ctrl+o` | Open/manage projects |
| `save_as` | `SaveAsPane` | `Ctrl+S` | Save project as new file |
| `checkpoint_list` | `CheckpointListPane` | `Ctrl+k` | Checkpoints list |
| `home` | `HomePane` | — | Welcome screen |
| `add` | `AddPane` | — | Instrument creation menu |
| `add_effect` | `AddEffectPane` | — | Add an effect (including VST effects) to the current target |
| `instrument_edit` | `InstrumentEditPane` | — | Edit instrument params/effects |
| `instrument_picker` | `InstrumentPickerPane` | — | Select instrument for actions |
| `vst_params` | `VstParamPane` | — | VST parameter browser (search/adjust/automation) |
| `file_browser` | `FileBrowserPane` | — | File selection for imports |
| `sample_chopper` | `SampleChopperPane` | — | Slice audio and assign pads |
| `confirm` | `ConfirmPane` | — | Confirmation dialog |
| `quit_prompt` | `QuitPromptPane` | — | Quit confirmation with unsaved changes warning |

### Pane Communication

Panes communicate exclusively through `Action` values. A pane's `handle_action()` or
`handle_raw_input()` returns an `Action`, which is dispatched by
`dispatch::dispatch_action()` in `imbolc-core/src/dispatch/mod.rs`. Dispatch mutates
`AppState`, returns a `DispatchResult` containing `AudioEffect` events; the runtime
layer flushes them via `AudioHandle::apply_effects()` after dispatch returns.

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
  → layer_stack.resolve(event) → ActionId or Blocked/Unresolved
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

Located in `imbolc-audio/src/`. Communicates with SuperCollider (scsynth) via OSC over UDP. `AudioEngine` runs on a dedicated audio thread; the main thread communicates with it via `AudioHandle` (priority/normal channels) and the `EventLogWriter` (projectable actions + state snapshots).

### Key Components

- `AudioHandle` (`imbolc-audio/src/handle.rs`) — main-thread interface; sends `AudioCmd`, receives `AudioFeedback`
- `AudioThread` (`imbolc-audio/src/audio_thread.rs`) — dedicated thread; owns `AudioEngine`, runs tick loop at ~0.5ms resolution
- `AudioCmd` / `AudioFeedback` (`imbolc-audio/src/commands.rs`) — command and feedback enums for cross-thread communication
- `EventLogWriter` / `EventLogReader` (`imbolc-audio/src/event_log.rs`) — projectable action + snapshot sync
- `AudioEngine` (`imbolc-audio/src/engine/mod.rs`) — manages synth nodes, bus allocation, routing, voice allocation
- `OscClient` (`imbolc-audio/src/osc_client.rs`) — OSC message/bundle sending + audio monitor
- `OscSender` (`imbolc-audio/src/osc_sender.rs`) — background OSC bundle sender thread
- `BusAllocator` (`imbolc-audio/src/bus_allocator.rs`) — audio/control bus allocation

### SuperCollider Groups

```
GROUP_SOURCES    = 100  — all source synths (oscillators, samplers)
GROUP_PROCESSING = 200  — filters, effects, mixer processing
GROUP_OUTPUT     = 300  — output synths
GROUP_BUS_PROCESSING = 350 — bus/layer-group processing
GROUP_RECORD     = 400  — recording nodes
GROUP_SAFETY     = 999  — safety limiter / analysis
```

### Instrument → Synth Mapping

Instruments map to SuperCollider nodes in two ways:
1. **Persistent Nodes:** `AudioIn` and `BusIn` instruments have static source nodes.
2. **Polyphonic Voices:** Oscillator and sampler instruments spawn a per-note voice chain (group containing `imbolc_midi` + source synth) on note-on.
3. **VST Instruments:** Persistent node hosts the VSTPlugin UGen; note-on/off is sent via `/u_cmd` MIDI messages.

The per-instrument processing chain is built from `processing_chain` stages (filters, EQ, effects) plus a dedicated output node. These nodes are shared by all voices for that instrument (voice chains feed into the chain’s source bus).

**LFO Modulation:** Each instrument’s LFO writes to a control bus (`lfo_out`). Routing-level targets (filter/EQ/effect params, sends, pan) are wired during routing rebuilds by passing `*_mod_in` bus IDs to synths. Voice-level targets (amplitude, pitch, detune, sample rate, envelope params) are applied at spawn time by reading control buses in the voice chain. All SynthDefs accept `*_mod_in` params defaulting to -1 (no modulation).

### OSC Communication

- `OscClient::send_message()` — fire-and-forget single message
- `OscClient::set_params_bundled()` — multiple params in one timestamped bundle
- `OscClient::send_bundle()` — multiple messages in one timestamped bundle
- `osc_time_from_now(offset_secs)` — NTP timetag for sample-accurate scheduling

Use bundles for timing-sensitive operations (note events). Individual messages are fine for UI parameter changes.

## Playback Engine

Lives on the dedicated audio thread (`imbolc-audio/src/playback.rs`), ticking at ~0.5ms resolution independently of the UI frame rate:

1. Compute elapsed real time since last tick
2. Convert to ticks: `seconds * (bpm / 60) * ticks_per_beat`
3. Advance playhead, handle loop wrapping
4. Scan all tracks for notes starting in the elapsed tick range (high-water mark + lookahead window)
5. Call `AudioEngine::spawn_voice()` for note-ons (timestamped OSC bundles)
6. Track active notes and call `AudioEngine::release_voice()` when expired
7. Evaluate automation lanes at the new playhead and send a single automation bundle

Tick resolution is 480 ticks per beat. Notes and automation are sent as OSC bundles with NTP timetags for sample-accurate scheduling. Because the audio thread is decoupled from UI rendering, playback timing is unaffected by UI load.

## Persistence

SQLite database via `rusqlite`. Implementation in `imbolc-core/src/state/persistence/`.
The authoritative schema lives in `imbolc-core/src/state/persistence/schema.rs` (current `SCHEMA_VERSION = 12`).

### What's Persisted (high level)

- Session settings: key/scale/BPM, time signature, humanize, click track, theme
- Instruments + processing chains + mixer params + sends + layer groups
- Piano roll tracks/notes, arrangement clips/placements
- Automation lanes/points (session + clip automation)
- Sampler + drum sequencer + chopper state
- Mixer buses + bus effects + layer-group mixer state
- MIDI recording mappings
- Custom synthdef registry + VST plugin registry + VST param/state values
- Checkpoints and recent project metadata

### What's NOT Persisted

- Playback position and tick accumulators
- Audio engine state (rebuilt on connect)
- UI-only overlays (layer stack, transient selections)
- Visualization buffers (meters, scope, waveforms)

## Undo/Redo & Clipboard

### Undo System
The undo system (`imbolc-core/src/state/undo.rs`) stores scoped snapshots in an `UndoHistory` with coalescing for parameter sweeps. Scopes include `SingleInstrument`, `Session`, and `Full`, letting undo/redo trigger minimal audio rebuilds where possible.

### Clipboard
The clipboard (`imbolc-core/src/state/clipboard.rs`) supports typed contents:
- Piano roll notes (`ClipboardContents::PianoRollNotes`)
- Drum sequencer steps (`ClipboardContents::DrumSteps`)
- Automation points (`ClipboardContents::AutomationPoints`)
