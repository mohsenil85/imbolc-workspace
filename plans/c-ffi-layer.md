# Plan: C FFI Layer for imbolc-core

Expose `imbolc-core` as a language-agnostic shared library (`.dylib`/`.so`/`.dll`) with a thin C FFI surface. Actions and state cross the boundary as JSON strings.

## API Surface

```c
ImbolcEngine* imbolc_create(void);
const char*   imbolc_dispatch(ImbolcEngine* engine, const char* action_json);
const char*   imbolc_get_state(ImbolcEngine* engine);
const char*   imbolc_drain_feedback(ImbolcEngine* engine);
const char*   imbolc_get_meters(ImbolcEngine* engine);
void          imbolc_destroy(ImbolcEngine* engine);
```

Any language that can call C and parse JSON can drive the engine.

## New Crate: `imbolc-ffi/`

```
imbolc-ffi/
  Cargo.toml          # cdylib + staticlib, depends on imbolc-core
  src/
    lib.rs            # extern "C" functions
    engine.rs         # ImbolcEngine wrapper (owns AppState + AudioHandle + channels)
    state_view.rs     # Serializable projection of AppState (excludes undo history, viz internals)
    feedback.rs       # Unified FfiFeedback enum (Audio + Io feedback)
```

Add `"imbolc-ffi"` to workspace members in root `Cargo.toml`.

## Key Design Decisions

### 1. String ownership: per-function pools

Each FFI function owns its last-returned `CString`. Calling `imbolc_dispatch()` doesn't invalidate the string from `imbolc_get_state()`. No `imbolc_free_string()` needed — strings are valid until the same function is called again on the same engine.

### 2. Thread safety: Mutex wrapper

The `void*` pointer is a `Box<FfiHandle>` containing `Mutex<ImbolcEngine>`. All FFI functions lock before operating. DAW actions are inherently sequential, so contention is negligible.

### 3. `&'static str` in NavAction/PushLayer/PopLayer

These are TUI-specific. The FFI action enum (`FfiAction`) simply omits `Nav`, `PushLayer`, `PopLayer`, and `ExitPerformanceMode` variants. FFI consumers don't need pane navigation. The engine wrapper handles `DispatchResult.nav` internally or returns it as strings in the JSON response.

### 4. MPSC channels in AudioCmd

Not a problem. The FFI layer calls `dispatch_action()` which uses `AudioHandle` methods internally — channels never cross the FFI boundary. The engine wrapper owns its own `(io_tx, io_rx)` channel pair.

### 5. State serialization: projection view

`imbolc_get_state()` serializes a `StateView` struct that references `&SessionState` and `&InstrumentState` plus scalar transport/status fields. Excludes `UndoHistory` (internal, large) and `VisualizationState` (high-frequency, separate endpoint). `imbolc_get_meters()` returns just peak/spectrum/latency for 60fps polling.

### 6. Audio dirty flags: handled internally

`DispatchResult.audio_dirty` is consumed inside the engine wrapper (calls `audio.flush_dirty()` after every dispatch). FFI consumers never see it. The returned JSON from `imbolc_dispatch()` includes `quit`, `status`, `project_name`, `stop_playback`, `reset_playhead` — omitting `audio_dirty` and converting `NavIntent` `&'static str` to `String`.

## Serde Derive Changes in imbolc-core

### Already have Serialize + Deserialize
- All state types in `state/instrument/`, `state/piano_roll.rs`, `state/session.rs`, `state/automation/`, `state/param.rs`, `state/sampler.rs`, `state/drum_sequencer.rs`, etc.

### Need Serialize + Deserialize added

**`action.rs`** — all action enums and supporting types:
- `InstrumentAction`, `MixerAction`, `PianoRollAction`, `ArrangementAction`, `SequencerAction`, `ChopperAction`, `ServerAction`, `SessionAction`, `MidiAction`, `VstParamAction`, `AutomationAction`
- `InstrumentUpdate`, `FileSelectAction`, `VstTarget`, `ToggleResult`
- `AudioDirty`, `FilterParamKind`, `LfoParamKind`
- `StatusEvent`, `DispatchResult` (for serializing the response)
- `IoFeedback` (for feedback drain)
- `NavAction` — skip or use `String` variant for serde
- `Action` — the top-level enum, with `#[serde(skip)]` on `Nav`, `PushLayer`, `PopLayer`, `ExitPerformanceMode` variants

**`audio/commands.rs`**:
- `AudioFeedback`, `ExportKind` — for feedback drain serialization

**`audio/engine/mod.rs`** (or wherever `ServerStatus` lives):
- `ServerStatus`

**`state/mod.rs`**:
- `KeyboardLayout`, `IoGeneration`, `PendingRender`, `PendingExport`
- `AppState` — add `Serialize` only (not `Deserialize`), with `#[serde(skip)]` on `undo_history`

**`state/clipboard.rs`**:
- `ClipboardNote`, `ClipboardContents`, `Clipboard` — if not already derived

## Implementation Order

### Phase 1: Serde derives (imbolc-core only)
1. Add derives to `ServerStatus`, `AudioFeedback`, `ExportKind`
2. Add derives to all Action sub-enums in `action.rs`
3. Add derives to remaining state types
4. Add `Serialize` to `AppState` with appropriate `#[serde(skip)]`
5. `cargo test` to verify no regressions

### Phase 2: FFI crate scaffold
6. Create `imbolc-ffi/` with `Cargo.toml` (crate-type = `["cdylib", "staticlib"]`)
7. Add to workspace
8. Implement `ImbolcEngine` wrapper (owns state + audio + channels)
9. Implement `StateView` projection
10. Implement per-function `StringPool`

### Phase 3: FFI functions
11. Implement all 6 `extern "C" fn` in `lib.rs`
12. Implement `FfiAction` (mirrors `Action` minus TUI variants) or use `#[serde(skip)]` approach
13. Implement `FfiDispatchResult` serialization
14. Wrap all FFI entry points in `catch_unwind` (panics across FFI = UB)
15. Write `imbolc.h` header (hand-written, 6 functions)

### Phase 4: Testing
16. Rust unit tests: create/destroy, dispatch add instrument, get state, invalid JSON
17. Serde round-trip tests for all Action variants
18. `cargo build --release -p imbolc-ffi` — verify `.dylib` output
19. C integration test (compile + link against dylib)
20. Python integration test (ctypes + json)

## Verification

```bash
# Build the shared library
cargo build --release -p imbolc-ffi

# Run FFI tests
cargo test -p imbolc-ffi

# Run core tests (verify serde additions didn't break anything)
cargo test -p imbolc-core

# Verify dylib exports
nm -gU target/release/libimbolc_ffi.dylib | grep imbolc_

# Quick smoke test from Python
python3 -c "
import ctypes, json
lib = ctypes.CDLL('target/release/libimbolc_ffi.dylib')
lib.imbolc_create.restype = ctypes.c_void_p
lib.imbolc_dispatch.restype = ctypes.c_char_p
lib.imbolc_dispatch.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.imbolc_get_state.restype = ctypes.c_char_p
lib.imbolc_get_state.argtypes = [ctypes.c_void_p]
lib.imbolc_destroy.argtypes = [ctypes.c_void_p]
e = lib.imbolc_create()
r = lib.imbolc_dispatch(e, b'{\"Instrument\":{\"Add\":\"Saw\"}}')
print('dispatch:', json.loads(r))
s = lib.imbolc_get_state(e)
state = json.loads(s)
print('instruments:', len(state['instruments']['instruments']))
lib.imbolc_destroy(e)
print('OK')
"
```

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Add `"imbolc-ffi"` to workspace members |
| `imbolc-core/src/action.rs` | Add Serialize/Deserialize to ~15 enums/structs |
| `imbolc-core/src/audio/commands.rs` | Add Serialize/Deserialize to AudioFeedback, ExportKind |
| `imbolc-core/src/audio/engine/mod.rs` | Add Serialize/Deserialize to ServerStatus |
| `imbolc-core/src/state/mod.rs` | Add Serialize to AppState, derives to supporting types |
| `imbolc-core/src/state/clipboard.rs` | Add Serialize/Deserialize if missing |

## Files Created

| File | Purpose |
|------|---------|
| `imbolc-ffi/Cargo.toml` | Crate config: cdylib + staticlib |
| `imbolc-ffi/src/lib.rs` | 6 extern "C" functions |
| `imbolc-ffi/src/engine.rs` | ImbolcEngine wrapper struct |
| `imbolc-ffi/src/state_view.rs` | Serializable state projection |
| `imbolc-ffi/src/feedback.rs` | Unified feedback type |
| `imbolc.h` | C header (hand-written) |
