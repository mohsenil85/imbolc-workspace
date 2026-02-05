# Rust Rewrite Design Document

High-level design for the TUI DAW rewritten in Rust. Incorporates lessons from v1 (Clojure) and v2 (Java).

## Vision

A terminal-based modular synthesizer and DAW that feels immediate and hackable:
- **Instant startup** - No JVM warmup, ready in milliseconds
- **Single binary** - One file to distribute, no runtime dependencies
- **Shareable sessions** - `.imbolc` SQLite files you can send to a friend
- **MIDI-first** - Plug in a keyboard and play

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           TUI Layer                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │   ratatui   │  │  Renderer   │  │   Input     │                 │
│  │  (terminal) │  │  (views)    │  │  (keys)     │                 │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                 │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          ▼                                          │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                      UI Engine                                 │ │
│  │   Graphics trait, InputEvent trait, semantic colors/themes    │ │
│  └───────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Core Layer                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │    State    │  │ Dispatcher  │  │   Action    │                 │
│  │  (immut.)   │◄─┤  (routing)  │◄─┤   (enum)    │                 │
│  └──────┬──────┘  └─────────────┘  └─────────────┘                 │
│         │                                                           │
│         ▼                                                           │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │              State Transitions (pure functions)                │ │
│  │   add_module, remove_module, connect, set_param, etc.         │ │
│  └───────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Audio Layer   │ │ Persistence     │ │   MIDI Layer    │
│  ┌───────────┐  │ │  ┌───────────┐  │ │  ┌───────────┐  │
│  │    OSC    │  │ │  │  SQLite   │  │ │  │  midir    │  │
│  │   (rosc)  │  │ │  │ (rusqlite)│  │ │  │           │  │
│  └─────┬─────┘  │ │  └───────────┘  │ │  └─────┬─────┘  │
│        ▼        │ │                 │ │        ▼        │
│  ┌───────────┐  │ │                 │ │  ┌───────────┐  │
│  │ scsynth   │  │ │                 │ │  │ MPK25 etc │  │
│  │ (process) │  │ │                 │ │  │           │  │
│  └───────────┘  │ │                 │ │  └───────────┘  │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

## Key Design Decisions

### 1. UI Engine First

Abstract the terminal library immediately. Don't let `ratatui` types leak everywhere.

```rust
// Abstract traits
pub trait Graphics {
    fn put_str(&mut self, x: u16, y: u16, text: &str);
    fn set_style(&mut self, style: Style);
    fn draw_box(&mut self, rect: Rect, title: &str);
}

pub trait InputSource {
    fn poll_event(&mut self, timeout: Duration) -> Option<InputEvent>;
}

pub enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    // No terminal-library-specific types!
}
```

**Rationale:** Learned from Java version - Lanterna types everywhere made extraction painful.

### 2. Immutable State + Pure Transitions

```rust
#[derive(Clone)]
pub struct RackState {
    pub modules: HashMap<ModuleId, Module>,
    pub order: Vec<ModuleId>,
    pub patches: HashSet<Patch>,
    pub mixer: MixerState,
    pub selected: Option<ModuleId>,
    pub view: View,
    // ...
}

// Pure functions - take state, return new state
pub fn add_module(state: RackState, module: Module) -> RackState { ... }
pub fn connect(state: RackState, src: Port, dst: Port) -> RackState { ... }
```

**Rationale:** Trivially testable, easy undo/redo (just a stack of states), no spooky action at distance.

### 3. Action Enum (Exhaustive)

```rust
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    // Module operations
    AddModule,
    DeleteModule,
    EditModule,

    // Mixer
    ToggleMute,
    ToggleSolo,
    CycleOutput,
    CycleOutputPrev,

    // ...
}
```

**Rationale:** Rust's exhaustive matching ensures every action is handled. No silent drops.

### 4. View Dispatchers

```rust
pub trait ViewDispatcher {
    fn dispatch(&self, action: Action, state: RackState) -> (RackState, Vec<Effect>);
}

// Separate dispatcher per view
pub struct RackViewDispatcher;
pub struct MixerViewDispatcher;
pub struct PatchViewDispatcher;
pub struct SequencerViewDispatcher;
pub struct ServerViewDispatcher;  // scsynth management
```

**Rationale:** Keeps action handling focused, prevents god-class dispatcher.

### 5. Effects (Side Effects as Data)

```rust
pub enum Effect {
    // Audio
    CreateSynth { module_id: ModuleId, synth_def: String, params: Vec<(String, f32)> },
    FreeSynth { module_id: ModuleId },
    SetParam { module_id: ModuleId, param: String, value: f32 },

    // Persistence
    Save,
    Load { path: PathBuf },

    // System
    Quit,
}
```

Dispatch returns `(new_state, effects)`. Effects are executed by the main loop, keeping dispatchers pure.

### 6. SQLite from Day 1

Single `.imbolc` file for everything:
- Modules, patches, mixer state
- Sequencer tracks and steps
- Presets (portable, content-addressable)
- Rack templates (shareable subgraphs)
- Undo history (optional)

See `docs/sqlite-persistence.md` for schema details.

### 7. OUTPUT Modules → Mixer Channels

Signal flow: `Sources → patch to OUTPUT → mixer channels → buses/master`

- Users explicitly create OUTPUT modules where they want audio to go
- Each OUTPUT module gets a mixer channel
- Mixer view shows only OUTPUT modules (not every oscillator)
- Gives explicit control over routing

### 8. One Keybinding Scheme

No modes. No chords (except Ctrl+key for system ops). Direct key→action mapping.

- Arrows for navigation (universal, intuitive)
- Single letters for operations: `a`=add, `x`=delete, `m`=mixer
- Uppercase for destructive/toggle: `M`=mute, `S`=solo
- Ctrl+key for system: `Ctrl+S`=save, `Ctrl+Z`=undo

**Rationale:** Modal interfaces (vim-style) add cognitive overhead that isn't worth it for a DAW.

### 9. Explicit Characters for State (Not Just Style)

Don't rely solely on bold/color to indicate state. Use visible characters:

```
▸ saw-1        # ▸ = selected (not just highlighted)
  lpf-1 [M]    # [M] = muted (not just dimmed)
  out-1 [S]    # [S] = soloed (not just colored)

  CH1   CH2   CH3
  ▮▮▮▯  ▮▮▯▯  ▮▯▯▯   # level meters as chars
  -3dB  -6dB  -12dB
   M     S            # M/S indicators as text
```

**Rationale:** `tmux capture-pane` only gets text, not styling. E2E tests can't verify state that's shown only via color/bold. Explicit characters make state visible in plain text captures, logs, and screenshots.

## Audio Engine Architecture

The audio layer wraps SuperCollider (scsynth) with DAW-level semantics. SC provides real-time DSP; we provide everything else.

### What SC Gives Us
- Real-time DSP graph (UGens), mixing, node graph
- Audio device I/O
- Buffers, buses, groups, nodes
- OSC control interface

### What We Build On Top

**Server Runtime Manager** (`engine.rs`)
- Boot/quit/reboot scsynth, detect failure, auto-reconnect
- Configure device, sample rate, block size, I/O channels
- Health monitoring (CPU/load/node counts)
- Exposed via Server View for user control

**OSC Transport Layer** (`osc_client.rs`)
- Time-tagged OSC bundles (not immediate messages - those jitter)
- `/sync` fencing to know when state is applied
- Batching for efficiency

**Lookahead Scheduler** (`scheduler.rs`)
- 100-500ms scheduling window
- Converts sequencer events → timestamped OSC bundles
- Handles seek: kill active nodes, rebuild to known state
- Loop handling with deterministic re-init

**Bus & Routing Allocator** (`bus_allocator.rs`)
- Deterministic bus allocation (audio + control)
- Group hierarchy: master → tracks → device chains
- Processing order: instrument → inserts → sends → fader → bus → master

**State Reconciler** (`reconciler.rs`)
- Single source of truth for "what should SC look like"
- Diffs project state against server state
- Applies changes: create/remove/move nodes, change params, reroute
- Stable ID mapping: project IDs ↔ SC node/bus IDs

**Metering** (`metering.rs`)
- Analysis synths + `SendReply` for level data
- Decimation for TUI (terminal doesn't need 60fps meters)
- Peak hold / RMS as needed

### Hard Constraints
- SC is not a timeline engine - imbolc owns all timeline semantics
- Timing correctness requires timestamped bundles + lookahead
- Seek/loop require "rebuild to known state" strategies
- scsynth will crash sometimes - must handle reconnect + state rebuild

## Crate Structure

```
imbolc/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   │
│   ├── ui/                    # UI engine abstraction
│   │   ├── mod.rs
│   │   ├── graphics.rs        # Graphics trait
│   │   ├── input.rs           # InputEvent, InputSource trait
│   │   ├── style.rs           # SemanticColor, Theme
│   │   └── ratatui_impl.rs    # ratatui backend
│   │
│   ├── state/                 # Immutable state types
│   │   ├── mod.rs
│   │   ├── rack.rs            # RackState
│   │   ├── module.rs          # Module, ModuleType
│   │   ├── mixer.rs           # MixerState, MixerChannel
│   │   ├── sequencer.rs       # Track, Step
│   │   ├── patcher.rs         # Patch, Port
│   │   └── transitions.rs     # Pure state functions
│   │
│   ├── core/                  # Action handling
│   │   ├── mod.rs
│   │   ├── action.rs          # Action enum
│   │   ├── effect.rs          # Effect enum
│   │   ├── dispatcher.rs      # ViewDispatcher trait, routing
│   │   └── dispatchers/       # Per-view dispatchers
│   │       ├── rack.rs
│   │       ├── mixer.rs
│   │       ├── patch.rs
│   │       ├── sequencer.rs
│   │       └── server.rs      # scsynth management view
│   │
│   ├── render/                # View rendering
│   │   ├── mod.rs
│   │   ├── rack_view.rs
│   │   ├── mixer_view.rs
│   │   ├── patch_view.rs
│   │   ├── sequencer_view.rs
│   │   ├── server_view.rs     # scsynth status, start/stop/reconnect
│   │   └── widgets/           # Reusable UI components
│   │
│   ├── audio/                 # SuperCollider interface
│   │   ├── mod.rs
│   │   ├── osc_client.rs      # OSC messaging (rosc), timestamped bundles
│   │   ├── scheduler.rs       # Lookahead scheduling (100-500ms window)
│   │   ├── engine.rs          # scsynth lifecycle, health monitoring
│   │   ├── bus_allocator.rs
│   │   ├── reconciler.rs      # Project state ↔ server state diffing
│   │   └── metering.rs        # SendReply handling, level decimation
│   │
│   ├── midi/                  # MIDI input
│   │   ├── mod.rs
│   │   ├── handler.rs         # midir wrapper
│   │   └── utils.rs           # note→freq, velocity→amp
│   │
│   └── persistence/           # SQLite
│       ├── mod.rs
│       ├── database.rs        # Connection, migrations
│       ├── session.rs         # Save/load entire session
│       └── presets.rs         # Preset/template import/export
│
├── synthdefs/                 # SuperCollider SynthDef files
│   ├── compile.scd
│   ├── saw_osc.scsyndef
│   └── ...
│
└── tests/
    ├── e2e/                   # tmux-based E2E tests
    │   ├── harness.rs
    │   └── ...
    └── integration/
```

## Development Order

Based on lessons learned:

### Phase 1: Foundation (UI Engine + E2E)
1. Set up project, CI, linting (`cargo clippy`, `cargo fmt`)
2. Implement UI engine abstraction (`Graphics`, `InputSource` traits)
3. Implement ratatui backend
4. Create tmux E2E test harness
5. Basic main loop (60fps render, input handling)
6. Minimum viable UI: draw a box, handle quit

**Exit criteria:** Can run app, see a box, press `q` to quit, E2E test passes.

### Phase 2: State & Views
1. Define `RackState`, `Action`, `Effect` types
2. Implement dispatcher routing
3. Rack view: list modules, navigation, selection
4. Add view: module type selection
5. Edit view: parameter adjustment
6. Wire up state transitions

**Exit criteria:** Can add modules, edit params, navigate. All state changes testable.

### Phase 3: Persistence
1. SQLite schema, migrations
2. Save/load session
3. Undo/redo (state stack)

**Exit criteria:** Create session, save, quit, reload, everything restored.

### Phase 4: Audio
1. OSC client with timestamped bundles (rosc)
2. `/sync` fencing for state confirmation
3. Lookahead scheduler (100-500ms window)
4. scsynth process management + health monitoring
5. Server view: start/stop/reconnect UI
6. SynthDef loading
7. State reconciler: project state → server state diffing
8. Module → synth node mapping
9. Parameter changes → scheduled OSC messages
10. Metering infrastructure (SendReply → decimated levels for TUI)

**Exit criteria:** Add saw-osc, hear sound, adjust freq/amp in real-time. Server view shows status. Meters update.

### Phase 5: Patching & Mixer
1. Patch view: connect modules
2. OUTPUT module → mixer channel
3. Mixer view: levels, mute, solo, routing
4. Bus summing, master output
5. Live recording: input routing → timeline capture
6. Latency compensation / alignment to timeline

**Exit criteria:** Full signal flow from oscillator → output → mixer → speakers. Can record audio input to a track.

### Phase 6: Sequencer
1. Track/step state
2. Sequencer view rendering
3. Step editing
4. Transport: imbolc owns the clock (app is master, SC follows)
5. Playback with lookahead scheduling
6. Seeking: rebuild-to-known-state on position change
7. Loop handling with deterministic state re-init
8. Musical settings (key, scale, snap)

**Exit criteria:** Create a pattern, play it back, seek to arbitrary position, loop a section, hear notes.

### Phase 7: MIDI
1. MIDI input (midir)
2. Device selection
3. Note → freq/gate mapping
4. Velocity → amplitude
5. CC → parameter mapping (later)

**Exit criteria:** Plug in keyboard, play notes, hear sound.

### Phase 8: Polish
1. Presets (save/load module settings)
2. Rack templates (export/import subgraphs)
3. NRT export/bounce (project → score → offline render)
4. Help view
5. Error handling, recovery
6. Performance optimization

## Dependencies

```toml
[dependencies]
# TUI
ratatui = "0.26"
crossterm = "0.27"

# Audio
rosc = "0.10"              # OSC protocol

# MIDI
midir = "0.9"

# Persistence
rusqlite = { version = "0.31", features = ["bundled"] }

# Utilities
thiserror = "1.0"          # Error handling
anyhow = "1.0"             # Result type
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dev-dependencies]
# Testing
assert_cmd = "2.0"         # CLI testing
predicates = "3.0"
tempfile = "3.0"
```

## Open Questions

1. **Async or sync?** Leaning sync with polling. Async adds complexity (tokio runtime, Send+Sync bounds) without clear benefit for a TUI app.

2. **Voice allocation for polyphony?** Defer to Phase 7+. Start monophonic.

3. **Plugin system?** Out of scope for initial rewrite. Get core working first.

4. **Cross-platform audio?** Stick with SuperCollider/scsynth. It handles the hard part.

## Success Metrics

- **Startup time:** < 100ms to first render
- **Binary size:** < 10MB (ideally < 5MB)
- **Memory usage:** < 50MB for typical session
- **Latency:** MIDI note → sound < 10ms (excluding SC latency)
- **Test coverage:** 80%+ on state transitions
