# Phase 1: Dispatcher Trait Abstraction

**Status: COMPLETE** (2025-02-05)

## Goal

Create a clean abstraction layer at the dispatch boundary so the UI binary
can work with either local or remote dispatch without knowing the
difference. This is the seam where `imbolc-net` will plug in.

## Implementation Summary

Phase 1 is complete with a key design change from the original plan:
**AudioHandle is kept separate from LocalDispatcher** to avoid Rust borrow
checker conflicts. When both state and audio are owned by the same struct,
operations like `audio.flush_dirty(state, ...)` fail because you can't borrow
`&mut audio` and `&state` from the same struct simultaneously.

The final architecture:
- `LocalDispatcher` owns `AppState` and `io_tx`
- `AudioHandle` remains a separate variable in `main.rs`
- Functions that need both take `&mut LocalDispatcher` and `&mut AudioHandle`
- `dispatch_with_audio()` bridges dispatch to the audio handle

This design also benefits Phase 2: remote clients don't have audio, so keeping
it separate makes the abstraction cleaner.

## Previous State

The infrastructure was partially in place:

| Component | Status | Location |
|-----------|--------|----------|
| `Dispatcher` trait | Exists (minimal) | `imbolc-types/src/dispatch.rs` |
| `LocalDispatcher` | Exists | `imbolc-core/src/dispatch/local.rs` |
| UI usage | Direct `LocalDispatcher` | `imbolc-ui/src/main.rs` |

**Current trait:**
```rust
pub trait Dispatcher {
    fn dispatch(&mut self, action: &Action) -> DispatchResult;
}
```

**Problem:** The trait only handles dispatch. The UI also needs state
access for rendering, but currently accesses `&AppState` directly. For
remote mode, state comes from the network.

## Design

### Extended Dispatcher Trait

```rust
pub trait Dispatcher {
    /// Dispatch an action and return the result.
    fn dispatch(&mut self, action: &Action) -> DispatchResult;

    /// Access current application state for rendering.
    fn state(&self) -> &AppState;

    /// Mutable state access (needed for IoFeedback handling in main loop).
    fn state_mut(&mut self) -> &mut AppState;
}
```

**Why `state_mut()`?** The main loop currently mutates state directly for:
- IoFeedback handling (save/load completion updates `project.path`, `project.dirty`)
- VST state restore queuing after load
- Undo history clearing on load

These could be refactored into actions, but that's a larger change. For
Phase 1, we keep `state_mut()` to minimize disruption.

### LocalDispatcher Changes

The current `LocalDispatcher` holds temporary references (`&'a mut`). This
works for per-dispatch creation but not for a persistent dispatcher that
also provides state access.

**Implemented design:** `LocalDispatcher` owns state and io_tx, but NOT audio:

```rust
pub struct LocalDispatcher {
    state: AppState,
    io_tx: Sender<IoFeedback>,
}

impl LocalDispatcher {
    pub fn new(state: AppState, io_tx: Sender<IoFeedback>) -> Self { ... }
    pub fn state(&self) -> &AppState { &self.state }
    pub fn state_mut(&mut self) -> &mut AppState { &mut self.state }
    pub fn io_tx(&self) -> &Sender<IoFeedback> { &self.io_tx }

    /// Dispatch with explicit audio handle to avoid borrow conflicts
    pub fn dispatch_with_audio(&mut self, action: &Action, audio: &mut AudioHandle) -> DispatchResult {
        dispatch_action(action, &mut self.state, audio, &self.io_tx)
    }
}
```

**Why keep audio separate?** The borrow checker doesn't allow:
```rust
dispatcher.audio_mut().flush_dirty(dispatcher.state(), ...);
// Error: can't borrow `dispatcher` as immutable while borrowed as mutable
```

Keeping audio as a separate variable allows independent borrowing.

### UI Changes

**Before:**
```rust
// main.rs owns state separately
let mut state = AppState::new_with_defaults(...);

// Per-dispatch, create temporary LocalDispatcher
let result = LocalDispatcher::new(&mut state, &mut audio, &io_tx)
    .dispatch(&action);

// Render with direct state access
panes.render(&state);
```

**After (implemented):**
```rust
// main.rs owns dispatcher (state) and audio separately
let mut dispatcher = LocalDispatcher::new(state, io_tx);
let mut audio = AudioHandle::new(...);

// Dispatch with explicit audio
let result = dispatcher.dispatch_with_audio(&action, &mut audio);

// State access through dispatcher
panes.render(dispatcher.state());

// Audio access is direct
audio.flush_dirty(dispatcher.state(), pending_audio_dirty);

// Mutable access for IoFeedback handling
dispatcher.state_mut().project.dirty = false;
```

### AppState and Network Sync

`AppState` contains local-only types that shouldn't sync over the network:

| Field | Syncs? | Notes |
|-------|--------|-------|
| `session` | Yes | Core musical data |
| `instruments` | Yes | Core musical data |
| `clipboard` | No | Local clipboard |
| `io` | Partial | Export progress syncs, pending ops don't |
| `keyboard_layout` | No | Local preference |
| `recording` | Yes | Recording state syncs |
| `audio` | No | Local audio thread feedback |
| `recorded_waveform_peaks` | No | Local audio data |
| `undo_history` | No | Local undo stack |
| `project` | Partial | Path is local, dirty syncs |
| `midi` | No | Local MIDI hardware |

For Phase 1, we keep `AppState` as-is. The `RemoteDispatcher` (Phase 2)
will:
1. Own a full `AppState` locally
2. Receive `session` + `instruments` updates from server
3. Maintain local-only fields (`audio`, `midi`, `undo_history`) itself

This avoids splitting `AppState` into multiple types, which would be
invasive.

## Implementation Steps

### Step 1: Extend the Dispatcher Trait

File: `imbolc-types/src/dispatch.rs`

Add `state()` and `state_mut()` methods to the trait.

### Step 2: Update LocalDispatcher

File: `imbolc-core/src/dispatch/local.rs`

Change from borrowed references to owned values. The constructor takes
ownership of `AppState`, `AudioHandle`, and the `io_tx` sender.

### Step 3: AudioHandle Handling

The main loop needs direct `AudioHandle` access for:
- `audio.flush_dirty(&state, pending_audio_dirty)`
- `audio.send_cmd(...)` for VST state restore
- `audio.status()` for server pane

Options considered:
1. Add `fn audio(&self) -> &AudioHandle` to `Dispatcher` trait
2. Add `fn audio_mut(&mut self) -> &mut AudioHandle` to `Dispatcher` trait
3. Keep `AudioHandle` separate from the dispatcher

**Implemented: Option 3** - AudioHandle stays separate. This avoids borrow
conflicts and is cleaner for network mode (remote clients don't have audio).

```rust
// In main.rs
let mut dispatcher = LocalDispatcher::new(state, io_tx);
let mut audio = AudioHandle::new(...);

// Functions take both explicitly
fn handle_action(dispatcher: &mut LocalDispatcher, audio: &mut AudioHandle) { ... }
```

### Step 4: Update main.rs

1. Change initialization to create `LocalDispatcher` with owned values
2. Replace `&state` / `&mut state` with `dispatcher.state()` / `dispatcher.state_mut()`
3. Replace `&audio` / `&mut audio` with `dispatcher.audio()` / `dispatcher.audio_mut()`
4. Keep `io_tx` accessible via `dispatcher.io_tx()`

### Step 5: Update global_actions.rs

File: `imbolc-ui/src/global_actions.rs`

This file has a direct `dispatch::dispatch_action()` call. Update to use
the dispatcher abstraction.

### Step 6: Verify Tests Pass

Run `cargo test` across the workspace.

## File Changes Summary

| File | Change |
|------|--------|
| `imbolc-types/src/dispatch.rs` | Add `state()`, `state_mut()` to trait |
| `imbolc-core/src/dispatch/local.rs` | Own values instead of borrow, add accessors |
| `imbolc-core/src/dispatch/mod.rs` | Re-export changes if needed |
| `imbolc-ui/src/main.rs` | Use dispatcher abstraction throughout |
| `imbolc-ui/src/global_actions.rs` | Use dispatcher instead of direct dispatch |

## Testing Strategy

1. **Compile check:** `cargo build --workspace`
2. **Unit tests:** `cargo test --workspace`
3. **Manual testing:** Run the app, verify all panes render and actions work
4. **Specific tests:**
   - Save/load project (IoFeedback path)
   - Play/stop (audio dirty path)
   - Add/remove instruments
   - MIDI input (if available)

## Future Considerations (Phase 2+)

### Mode Detection

The binary will need to detect local vs remote mode:

```rust
enum DispatchMode {
    Local(LocalDispatcher),
    Remote(RemoteDispatcher),
}
```

Or use trait objects:

```rust
let dispatcher: Box<dyn Dispatcher> = if args.connect.is_some() {
    Box::new(RemoteDispatcher::connect(addr)?)
} else {
    Box::new(LocalDispatcher::new(state, audio, io_tx))
};
```

### Audio in Remote Mode

Remote clients don't have `AudioHandle`. Options:
- `Dispatcher` trait returns `Option<&AudioHandle>`
- Separate `LocalDispatcher` and `RemoteDispatcher` with different APIs
- Audio-specific trait: `trait AudioDispatcher: Dispatcher { fn audio(&self) -> &AudioHandle; }`

### IoFeedback in Remote Mode

Save/load operations happen on the server. The client receives state
updates. `IoFeedback` channel may not be needed for remote clients, or it
could carry network-received confirmations.

## Risks

1. **Ownership change is invasive:** Moving from borrowed to owned
   `AppState` touches many call sites. Mitigation: mechanical refactor,
   compiler catches all issues.

2. **Audio timing sensitivity:** `AudioHandle` is time-critical.
   Mitigation: keep audio path unchanged, just reorganize ownership.

3. **IoFeedback race conditions:** With owned state, need to ensure
   IoFeedback updates don't conflict with dispatch. Mitigation: both
   happen on main thread, no concurrency issues.

## Success Criteria

- [x] `LocalDispatcher` owns `AppState` and `io_tx` (audio kept separate)
- [x] `LocalDispatcher` provides `state()`, `state_mut()`, `dispatch_with_audio()`
- [x] `main.rs` uses dispatcher abstraction throughout
- [x] `global_actions.rs` takes `&mut LocalDispatcher` + `&mut AudioHandle`
- [x] All 108 unit tests pass
- [x] No direct `dispatch_action()` calls outside `LocalDispatcher`

**Note:** The `Dispatcher` trait in `imbolc-types` was not extended with
`state()` / `state_mut()` because `AppState` lives in `imbolc-core`, and
adding it to the trait would create a circular dependency. The state access
methods are on `LocalDispatcher` directly. Phase 2 can revisit this if needed
for trait object dispatch.
