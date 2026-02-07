# imbolc-net: Networked Jam Space

**Status:** IN_PROGRESS
**Last Updated:** 2025-02-06

## Problem

Multiple musicians in the same physical space want to jam together
using imbolc. Each person has their own screen and controller. Audio
inputs (guitars, mics) are cabled to a central machine. MIDI
controllers connect via RTP-MIDI over local ethernet (discovered by
the OS, not us). There is a single master audio output. We never send
audio over the network — only control data.

## Architecture

### Crate Structure

```
imbolc-types    — All shared types: AppState, Action, Instrument, etc.
                  No logic, just data structures + serde derives

imbolc-core     — Dispatch logic, audio engine, SuperCollider communication
                  Depends on: imbolc-types

imbolc-net      — Network layer: RemoteDispatcher (client), NetServer (server)
                  Depends on: imbolc-types (NOT imbolc-core)

imbolc          — TUI binary
                  Depends on: imbolc-types, imbolc-core (local), imbolc-net (remote)
```

Extracting `imbolc-types` keeps the client lightweight — it only needs
the data structures for serialization, not the dispatch/audio code.

**Dependency graph:**

```
                    imbolc-types
                    /     |     \
                   /      |      \
                  v       v       v
          imbolc-core  imbolc-net  imbolc (binary)
                  \       /         /
                   \     /         /
                    v   v         v
               [server binary]  [client binary]
```

Note: `imbolc-net` does NOT depend on `imbolc-core`. They're siblings
that share `imbolc-types`.

### The Dispatch Seam

The network boundary lives at the dispatch layer. The TUI doesn't know
or care whether dispatch is local or remote.

```rust
trait Dispatcher {
    fn dispatch(&mut self, action: Action) -> DispatchResult;
    fn state(&self) -> &AppState;
}
```

Two implementations:

- **LocalDispatcher** — calls `imbolc-core::dispatch()` directly, owns
  the state and audio engine
- **RemoteDispatcher** — serializes the action, sends to server,
  receives state updates

The binary picks which one at startup. Pane code never changes.

### Networked Mode

```
┌─────────────────────────┐              ┌─────────────────────────────────┐
│  Client machine         │              │  Server machine                 │
│                         │     LAN      │                                 │
│  imbolc (TUI)           │              │  imbolc-net (NetServer)         │
│  imbolc-net (Remote-    │  ─Action──>  │  imbolc-core (LocalDispatcher)  │
│    Dispatcher)          │  <──State──  │  SuperCollider                  │
│  imbolc-types           │              │  imbolc-types                   │
│                         │              │  Audio I/O (all of it)          │
│  No SC, no audio        │              │                                 │
└─────────────────────────┘              └─────────────────────────────────┘
         x N clients                                1 server
```

### Local Mode

When running solo, `imbolc-net` is not used. The binary instantiates
`LocalDispatcher` directly.

```
imbolc (TUI) -> LocalDispatcher (imbolc-core) -> SuperCollider
```

The binary detects which mode at startup (flag, config, or presence of
server).

### Binaries

One binary, multiple modes:

```
imbolc                     # local mode (default, same as today)
imbolc --server            # server mode: headless, runs NetServer + LocalDispatcher + SC
imbolc --server --tui      # server mode with TUI (host is also playing)
imbolc --connect <addr>    # client mode: TUI + RemoteDispatcher, no SC
```

Alternatively, separate binaries (`imbolc` and `imbolc-server`), but
flags are simpler to start.

## What `imbolc-net` Does

A thin crate with two components. Depends only on `imbolc-types`, not
`imbolc-core`.

### RemoteDispatcher (client component)

Implements the `Dispatcher` trait for network mode:

- `dispatch()` serializes the `Action` and sends it to the server
- `state()` returns the cached `AppState` received from the server
- Maintains TCP connection to server
- Background thread receives state updates, swaps the cached state

The TUI calls the same `Dispatcher` interface whether local or remote
— it has no awareness of the network.

### NetServer (server component)

Runs on the server machine alongside `imbolc-core`:

- Listens for client connections (TCP)
- Receives `Action` messages from clients
- Validates ownership (is this client allowed to do this?)
- Forwards valid actions to the `LocalDispatcher`
- After dispatch, broadcasts the new `AppState` to all connected
  clients
- Manages connection lifecycle (join, disconnect, reconnect)

The server binary might be headless (no TUI) or have a TUI for the
host who's also playing.

### What It Does NOT Do

- Audio transport — all audio is local to the server
- MIDI transport — RTP-MIDI handles this at the OS layer
- Complex conflict resolution — server is authoritative, last write
  wins
- Depend on `imbolc-core` — only needs types for serialization

## State Model

**Full mirror.** Every client holds a complete copy of `AppState`. The
server broadcasts the full state (or diffs — optimization for
later). Clients render the same UI as local mode.

This is simpler to build, and visibility restrictions can be added in
the UI layer later without changing the protocol.

## Ownership

Each connected client owns one or more instruments. Ownership
determines which actions the server will accept from a given client.

- A client can only mutate state on instruments it owns
- Transport controls (play, stop, BPM) — TBD, may require a privileged
  node
- Piano roll edits — scoped to owned instruments' tracks
- Mixer — TBD (own channel only? or global?)

Ownership is assigned on connect. Mechanism TBD (server assigns,
client requests, configured in advance).

## Protocol

LAN only. Control data is small. Latency budget is generous for
non-audio data on a local network (sub-millisecond typical).

- **Transport:** TCP for reliability. Messages are small and
  infrequent enough that TCP's overhead doesn't matter. UDP adds
  complexity for no real gain at this scale.
- **Serialization:** serde — `Action` and `AppState` already derive or
  can derive `Serialize`/`Deserialize`. Wire format TBD (bincode for
  compactness, or MessagePack, or JSON for debuggability during
  development).
- **Message types:**
  - Client -> Server: `Action` (already the unit of intent)
  - Server -> Client: `AppState` snapshot (initially), then possibly
    diffs

## Discovery

TBD. Options:

- **mDNS/Bonjour** — zero-config, appropriate for LAN
- **Manual IP** — simple, always works
- **Both** — mDNS with manual fallback

Not a priority for v1. Manual IP is fine to start.

## Monitoring

The server machine has the audio hardware. Players need to hear
themselves and the mix. Options (not mutually exclusive):

- Dedicated hardware outputs per player (server needs a multi-output
  interface)
- Cue bus system within SuperCollider (per-player headphone mixes)
- Single shared monitor output (simplest, maybe fine for jamming)

This is a hardware/SC routing question more than an `imbolc-net`
question. Defer until we can try things.

## Deferred Decisions

These are intentionally left open. They'll be resolved by feel once
the basic system is running.

| Question | Options | Notes |
|----------|---------|-------|
| Ownership granularity | Per-instrument, per-track, per-set | Start with per-instrument |
| Privileged node | One host with extra powers vs. all equal | Leaning toward one privileged node for transport/save/load |
| Global read-only scope | See everything, see piano roll only, see nothing | Start with full visibility |
| Monitoring | Hardware outs, cue buses, shared | Hardware dependent |
| Save/load authority | Server only, privileged client, any client | Server only is safest default |
| Reconnection | Rejoin with same ownership, reassign, manual | Needs to feel right |
| Wire format | bincode, MessagePack, JSON | JSON for dev, compact format for later |
| Discovery | mDNS, manual, both | Manual first |

## Implementation Sketch

### Phase 0: Extract Types

Create `imbolc-types` crate with all shared data structures. Most
types in imbolc-core are already pure data — this is largely a
mechanical move.

#### Progress (as of 2026-02-05)

**Completed:**
- `imbolc-types` crate exists with core types
- Action enum and all sub-actions
- Param, ParamValue
- Instrument types (SourceType, EffectType, FilterType, etc.)
- PianoRollState, Note, Track
- Automation types: AutomationLaneId, CurveType, AutomationPoint,
  AutomationTarget
- AutomationLane, AutomationState (with full impl blocks)
- CustomSynthDefRegistry, CustomSynthDef, ParamSpec
- VstPluginRegistry, VstPlugin, VstParamSpec, VstPluginKind
- Clipboard, ClipboardContents
- PendingRender, PendingExport, KeyboardLayout, VisualizationState,
  IoGeneration
- MixerSelection, MusicalSettings
- ExportKind (added Serialize/Deserialize)
- **Phase 0.5 COMPLETE**: MixerState, HumanizeSettings,
  RecordingState, IoState, ProjectMeta (all in imbolc-types)
- **Phase 0.5 COMPLETE**: AudioFeedbackState, MidiConnectionState (in
  imbolc-core, local-only)
- **Phase 0.5 COMPLETE**: BPM/TimeSignature sync invariants via setter
  methods
- **Phase 0 COMPLETE (2026-02-05)**: MidiRecordingState + supporting
  types (RecordMode, MidiCcMapping, PitchBendConfig, cc module)
- **Phase 0 COMPLETE (2026-02-05)**: ArrangementState + supporting
  types (Clip, ClipPlacement, ClipEditContext, PlayMode)
- **Phase 0 COMPLETE (2026-02-05)**: SessionState (full struct with
  all impl blocks)
- **Phase 0 COMPLETE**: InstrumentState, Instrument, and all
  dependencies (SamplerConfig, ArpeggiatorConfig, etc.)

**Remaining:**
- AppState — stays in imbolc-core (contains local-only types:
  AudioFeedbackState, MidiConnectionState, UndoHistory)

**Previous Blockers (RESOLVED):** ~~SessionState and AppState are "god
objects" that aggregate many concerns.~~  Phase 0.5 refactoring
completed - both structs now compose smaller, focused sub-structs.

See **Phase 0.5** below for details.

**Definitely moves (pure data, no dependencies):**

From `action.rs` (~100% of file):
- `Action`, `DispatchResult`, `AudioDirty`
- All sub-action enums: `InstrumentAction`, `MixerAction`,
  `PianoRollAction`, `SequencerAction`, `AutomationAction`,
  `SessionAction`, `ArrangementAction`, `NavAction`, etc.
- `VstTarget`, `VstParamAction`, `FilterParamKind`, `LfoParamKind`
- `ToggleResult`, `FileSelectAction`, `NavIntent`, `StatusEvent`

From `state/param.rs`:
- `Param`, `ParamValue`

From `state/instrument/`:
- `Instrument`, `InstrumentId`, `SourceType`, `EffectType`,
  `EffectSlot`, `EffectId`
- `FilterType`, `FilterConfig`, `EqBandType`, `EqBand`, `EqConfig`
- `LfoConfig`, `EnvConfig`, `ModulatedParam`, `ModSource`,
  `InstrumentSection`
- `OutputTarget`, `MixerSend`, `MixerBus`

From `state/piano_roll.rs`:
- `Note`, `Track`, `PianoRollState`

From `state/automation/`:
- `AutomationLaneId`, `CurveType`, `AutomationPoint`

From `state/arrangement.rs`:
- `ClipId`, `PlacementId`, `PlayMode`, `Clip`, `ClipPlacement`,
  `ClipEditContext`, `ArrangementState`

From `state/session.rs`:
- `MixerSelection`, `MusicalSettings`

From `state/vst_plugin.rs`:
- `VstPluginId`, `VstPluginKind`, `VstParamSpec`, `VstPlugin`

From `state/custom_synthdef.rs`:
- `CustomSynthDefId`, `ParamSpec`, `CustomSynthDef`

From `state/mod.rs`:
- `PendingRender`, `PendingExport`, `KeyboardLayout`,
  `VisualizationState`, `IoGeneration`

From `audio/engine/`:
- `ServerStatus` (simple enum, only external dep in state types)

**Needs consideration:**

These types have methods that orchestrate other types. The struct
definitions are pure data, but they have impl blocks with business
logic:

- `AppState` — top-level state, has `ServerStatus` dependency
- `SessionState` — contains registries, has utility methods
- `InstrumentState` — instrument collection management
- `AutomationState` — lane management
- `CustomSynthDefRegistry`, `VstPluginRegistry` — lookup logic
- `Clipboard` — likely pure, just needs verification

**Strategy:** Move the struct/enum definitions to `imbolc-types`. Keep
impl blocks with complex logic in `imbolc-core` (Rust allows impl
blocks in a different crate than the type definition, as long as they
don't impl foreign traits). Simple accessor methods can move with the
types.

**Tasks:**
1. Create `imbolc-types` crate at `../imbolc-types/`
2. Move type definitions (structs, enums, type aliases)
3. Add `Serialize`/`Deserialize` derives to everything
4. `imbolc-core` depends on `imbolc-types`, re-exports for backwards
   compatibility
5. Move simple impl blocks (accessors, pure helpers) with the types
6. Keep complex impl blocks (state management, registry lookups) in
   `imbolc-core`
7. Verify existing code still compiles

This is the biggest mechanical change. Everything else is additive.

#### Phase 0 Summary (2026-02-05)

Phase 0 is now **complete**. All types needed for network
serialization are in imbolc-types:

| Type | Status |
|------|--------|
| SessionState | ✓ Migrated |
| ArrangementState | ✓ Migrated |
| MidiRecordingState | ✓ Migrated |
| InstrumentState | ✓ Migrated |
| Instrument | ✓ Migrated |
| SamplerConfig | ✓ Migrated |
| ArpeggiatorConfig | ✓ Migrated |
| PianoRollState | ✓ Already done |
| AutomationState | ✓ Already done |
| MixerState | ✓ Already done |
| CustomSynthDefRegistry | ✓ Already done |
| VstPluginRegistry | ✓ Already done |

**Local-only (stays in imbolc-core):** AppState, AudioFeedbackState,
MidiConnectionState, UndoHistory — these contain runtime/hardware state
that shouldn't sync to clients.

### Phase 0.5: Refactor Large State Types — COMPLETE (2026-02-05)

Refactored SessionState and AppState into smaller, focused
structs. This makes the migration cleaner and improves the codebase
regardless of networking.

**Completed phases:**
| Phase | Type | Location | Status |
|-------|------|----------|--------|
| 1 | MixerState | imbolc-types | Done |
| 2 | HumanizeSettings | imbolc-types | Done |
| 3 | BPM/TimeSignature setters | imbolc-core | Done |
| 4 | RecordingState | imbolc-types | Done |
| 5 | IoState | imbolc-types | Done |
| 6 | AudioFeedbackState | imbolc-core | Done |
| 7 | MidiConnectionState | imbolc-core | Done |
| 8 | ProjectMeta | imbolc-types | Done |

All 278 tests passing. See
`/Users/log/.claude/plans/cozy-mapping-waterfall.md` for detailed
implementation notes.

#### Original Problems (now resolved)

**SessionState** (was 19 fields, now composed):
- ~~Musical settings~~ → flat fields (key, scale, bpm, etc.) +
  `set_bpm()`/`set_time_signature()` setters
- ~~Humanization~~ → `pub humanize: HumanizeSettings`
- Sub-states unchanged (piano_roll, arrangement, automation,
  midi_recording)
- Registries unchanged (custom_synthdefs, vst_plugins)
- ~~Mixer state~~ → `pub mixer: MixerState`

**AppState** (was 18+ fields, now composed):
- Session data unchanged (session, instruments, clipboard)
- ~~Runtime recording state~~ → `pub recording: RecordingState`
- ~~I/O state~~ → `pub io: IoState`
- ~~Audio feedback~~ → `pub audio: AudioFeedbackState`
- ~~MIDI state~~ → `pub midi: MidiConnectionState`
- ~~Persistence state~~ → `pub project: ProjectMeta`

#### Proposed Refactoring

**1. Split SessionState by domain:**

```rust
// Already exists, could expand
pub struct MusicalSettings {
    pub key: Key,
    pub scale: Scale,
    pub bpm: u16,
    pub tuning_a4: f32,
    pub time_signature: (u8, u8),
    pub snap: bool,
}

// NEW: Extract mixer-related state
pub struct MixerState {
    pub buses: Vec<MixerBus>,
    pub next_bus_id: u8,
    pub master_level: f32,
    pub master_mute: bool,
    pub selection: MixerSelection,
}

// NEW: Extract humanization settings
pub struct HumanizeSettings {
    pub velocity: f32,  // 0.0-1.0
    pub timing: f32,    // 0.0-1.0
}

// SessionState as implemented (2026-02-05)
// Note: Musical settings kept flat with setter methods for BPM/time_signature sync
pub struct SessionState {
    // Musical settings (flat, with setters for sync invariants)
    pub key: Key,
    pub scale: Scale,
    pub bpm: u16,
    pub tuning_a4: f32,
    pub snap: bool,
    pub time_signature: (u8, u8),

    // Composed sub-states
    pub mixer: MixerState,           // imbolc-types
    pub humanize: HumanizeSettings,  // imbolc-types

    // Complex sub-states (unchanged)
    pub piano_roll: PianoRollState,
    pub arrangement: ArrangementState,
    pub automation: AutomationState,
    pub midi_recording: MidiRecordingState,
    pub custom_synthdefs: CustomSynthDefRegistry,
    pub vst_plugins: VstPluginRegistry,
}
```

**2. Split AppState by lifecycle:**

```rust // NEW: Runtime playback state (not persisted, changes
rapidly) pub struct PlaybackState { pub playing: bool, pub playhead:
u32, pub bpm: f32, // from audio thread }

// NEW: Recording state
pub struct RecordingState {
    pub recording: bool,
    pub recording_secs: u64,
    pub automation_recording: bool,
    pub pending_recording_path: Option<PathBuf>,
}

// NEW: I/O operation state
pub struct IoState {
    pub pending_render: Option<PendingRender>,
    pub pending_export: Option<PendingExport>,
    pub export_progress: f32,
    pub generation: IoGeneration,
}

// NEW: Audio feedback (from audio thread, not persisted)
pub struct AudioFeedbackState {
    pub visualization: VisualizationState,
    pub server_status: ServerStatus,
}

// NEW: MIDI connection state
pub struct MidiConnectionState {
    pub port_names: Vec<String>,
    pub connected_port: Option<String>,
}

// NEW: Persistence metadata
pub struct ProjectState {
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub default_settings: MusicalSettings,
}

// AppState as implemented (2026-02-05)
pub struct AppState {
    // Persisted project data
    pub session: SessionState,
    pub instruments: InstrumentState,
    pub clipboard: Clipboard,
    pub undo_history: UndoHistory,

    // Composed sub-states
    pub io: IoState,                     // imbolc-types
    pub recording: RecordingState,       // imbolc-types
    pub project: ProjectMeta,            // imbolc-types
    pub audio: AudioFeedbackState,       // imbolc-core (local only)
    pub midi: MidiConnectionState,       // imbolc-core (local only)

    // Remaining flat fields
    pub keyboard_layout: KeyboardLayout,
    pub recorded_waveform_peaks: Option<Vec<f32>>,
}
```

**3. Benefits:**

- Smaller structs are easier to move to imbolc-types
- Clear separation: what's persisted vs runtime vs audio feedback
- Network sync can target specific sub-structs (e.g., only sync
  `session` and `instruments`)
- Easier to reason about state ownership
- More focused tests

**4. Migration strategy:** COMPLETE

1. ~~Create the new smaller structs in imbolc-types~~ Done
2. ~~Update SessionState/AppState to compose with them~~ Done
3. ~~Update all access patterns~~ Done (~400 call sites updated)
4. ~~Verify tests pass~~ Done (278 tests passing)
5. Continue Phase 0 extraction with the now-smaller types — Ready to
   proceed

**5. What moved to imbolc-types:**

- MusicalSettings (already there)
- MixerState ✓
- HumanizeSettings ✓
- RecordingState ✓ (was PlaybackState in plan)
- IoState ✓ (moved here, not core — needed for network sync of export
  status)
- ProjectMeta ✓ (was ProjectState in plan)

**6. What stayed in imbolc-core:**

- AudioFeedbackState ✓ — local to server, audio thread data
- MidiConnectionState ✓ — local hardware
- UndoHistory — local undo, server has its own
- Complex impl blocks for orchestration

### Phase 1: Dispatcher Trait ✓ COMPLETE

See [phase1-dispatcher-trait.md](phase1-dispatcher-trait.md) for details.

**Implemented:**
- `LocalDispatcher` owns `AppState` and `io_tx` (audio kept separate)
- `LocalDispatcher` provides `state()`, `state_mut()`, `dispatch_with_audio()`
- `main.rs` and `global_actions.rs` use dispatcher abstraction
- All 108 unit tests pass

**Key design decision:** AudioHandle kept separate from LocalDispatcher to
avoid borrow checker conflicts. This also benefits Phase 2 since remote
clients don't need audio.

### Phase 2: Network Plumbing ✓ COMPLETE

- Create `imbolc-net` crate (depends on `imbolc-types` only)
- Implement `RemoteDispatcher`: connect, send actions, receive state
- Implement `NetServer`: listen, receive actions, broadcast state
- Define wire protocol: `NetMessage` enum (Action, StateUpdate,
  Connect, Disconnect, etc.)
- Binary flags: `--server` / `--connect <addr>`
- Get basic round-trip working: client sends action, server
  dispatches, client sees updated state

**Implemented:**
- `imbolc-net` crate created (depends only on `imbolc-types`)
- `RemoteDispatcher` in `client.rs`: connect, send actions, receive state updates
- `NetServer` in `server.rs`: listen, accept connections, broadcast state
- Wire protocol in `protocol.rs`: `ClientMessage`, `ServerMessage`, `NetworkAction`, `NetworkState`
- Length-prefixed JSON framing in `framing.rs`
- Binary flags in `imbolc-ui`: `--server` and `--connect <addr>` (feature-gated under `net`)
- Full round-trip working: action dispatch → state sync → metering at ~30Hz
- Server mode (`run_server()`) and client mode (`run_client()`) in main.rs

**Key files:**
- `/imbolc-net/src/protocol.rs` — message types
- `/imbolc-net/src/client.rs` — RemoteDispatcher
- `/imbolc-net/src/server.rs` — NetServer
- `/imbolc-net/src/framing.rs` — wire framing
- `/imbolc-ui/src/main.rs` — server/client mode integration

### Phase 3: Ownership ✓ COMPLETE

- Client identifies itself on connect (name, requested instruments)
- Server tracks ownership table
- Server rejects actions that violate ownership
- UI indicates which instruments are owned by whom

**Implemented:**
- `ClientMessage::Hello { client_name, requested_instruments }` for client identification
- `NetServer.ownership: HashMap<InstrumentId, ClientId>` for tracking
- `validate_action()` rejects unauthorized instrument/piano roll actions
- Client-side `owns(instrument_id)` API for UI to query ownership

**Key files:**
- `/imbolc-net/src/protocol.rs` — ownership-aware message types
- `/imbolc-net/src/server.rs` — ownership tracking and validation
- `/imbolc-net/src/client.rs` — client-side ownership awareness

**Deferred to Phase 4:**
- UI indicator showing which instruments are owned by whom

### Phase 4: Polish

- Reconnection handling
- Discovery (mDNS)
- State diffing instead of full broadcasts
- Monitoring / cue bus routing
- Privileged node semantics

---

## Future Extensions

### Per-Client Instrument Bookmarks

Currently `InstrumentState.selected` is a global `Option<usize>` index and number keys 1-10 select by list position. In multi-client, each musician needs independent selection and number-key mappings.

**Client-local state (never sent to server):**
- `selected: Option<InstrumentId>` — which instrument this client is viewing/editing
- `bookmarks: [Option<InstrumentId>; 10]` — maps number keys 1-10 to instrument IDs

**How it works:**
- Pressing `1` → looks up `bookmarks[0]` → gets `InstrumentId` → sets client-local `selected`
- Before rendering, client patches `state.instruments.selected` in its local AppState copy to match
- Panes call `state.instruments.selected_instrument()` unchanged — no pane modifications needed
- Selection is instant (no server round-trip, purely client-local)

**Bookmark management:**
- Auto-bookmark on instrument claim (fills next free slot)
- Manual override: `Ctrl+1` to assign current instrument to slot 1
- Bookmarks persist client-side even after releasing ownership (for easy re-claim)

### Per-Client Undo Stacks

- Server maintains a separate undo stack per client
- Each stack only contains actions that client made on instruments they owned
- Undo reverses your own most recent action — never touches another client's work
- If you release an instrument, your undo entries for it are discarded (you gave up ownership)
- This keeps undo predictable: you can only undo what you did, on things you own

### Registers (Shared Named Buffers)

Named storage slots (`a`-`z` or string keys) on the server. Any client can read or write.

**Content types:**
- **Note sequences** — piano roll selections (pitch, time, duration, velocity)
- **Drum patterns** — sequencer steps (hits, accents, per-step velocity)
- **Instrument presets** — full instrument config (source, filter, effects, envelope, LFO, mixer params)
- **Automation curves** — lane segments (control points, curve types)
- **Keybinding snippets** — custom keymap fragments to share workflows

**Actions:**
- `YankToRegister { name: String, content: RegisterContent }` — store data
- `PasteFromRegister { name: String, target: ... }` — retrieve and apply
- `ListRegisters` → server responds with register names + content summaries
- `ClearRegister { name: String }`

**RegisterContent enum:**
```rust
#[derive(Serialize, Deserialize)]
enum RegisterContent {
    Notes(Vec<ClipboardNote>),
    DrumPattern { steps: Vec<DrumStep>, length: u32 },
    InstrumentPreset(Instrument),
    AutomationCurve { points: Vec<AutomationPoint>, curve: CurveType },
    Keybindings(String),  // TOML fragment
}
```

Registers persist in server memory for the session. Could optionally save to disk.

### Import/Export & Preset Library

Multi-client makes import/export a first-class workflow. Each musician has their own preset library on their machine and needs to move instruments/patterns in and out of shared sessions.

**Preset format** (`.imbolc-preset`): Self-contained SQLite file containing:
- Instrument definition (MessagePack blob, same format as persistence)
- Embedded sample data (audio bytes stored as BLOBs, keyed by original filename)
- SynthDef source if custom
- Fully portable — works on any machine without external file dependencies

**Preset library:** `~/.config/imbolc/presets/` on each musician's machine. Browsable from a client-local preset browser pane.

**Import to server:**
1. Client reads `.imbolc-preset` from local disk
2. Client sends `ImportPreset { instrument: Instrument, samples: Vec<(String, Vec<u8>)>, synthdef: Option<String> }` to server
3. Server writes sample files to its local sample directory, updates paths in instrument
4. Server creates instrument, auto-owns to importer, auto-bookmarks
5. Server loads samples into SC buffers

**Export from server:**
1. Client sends `ExportInstrument(InstrumentId)` (must own it)
2. Server gathers the instrument + all referenced sample files + synthdef source
3. Server sends back the full bundle
4. Client writes `.imbolc-preset` to their local preset library

**Session snapshot (local backup):**
1. Client sends `RequestSessionSnapshot`
2. Server sends full `SessionState + InstrumentState` + all sample data
3. Client writes it locally as a `.imbolc` SQLite file using existing persistence code
4. Acts as a fork point — can be opened standalone later

**Relationship to Registers:**
- Registers = ephemeral shared buffers, session-scoped, lightweight (no file I/O)
- Presets = persistent files, disk-scoped, self-contained (embed samples)
- Both carry similar content types but serve different purposes (quick sharing vs. permanent library)
