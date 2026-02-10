# imbolc-gui

Cross-platform GUI for Imbolc DAW using Dioxus.

## What This Is

A Dioxus-based desktop GUI alternative to the terminal UI (imbolc-ui). It is **experimental** and not yet feature-parity with the TUI, focusing on transport, arrangement, mixer, and basic instrument editing. Shares core logic via imbolc-core and imbolc-types.

## Directory Structure

```
src/
  main.rs           — Entry point, app launch
  app.rs            — Root component, state initialization
  state.rs          — SharedState wrapper for AppState + AudioHandle
  dispatch.rs       — Action dispatch integration (DispatchExt trait)
  components/
    mod.rs            — Component exports
    transport.rs      — Play/stop/record, BPM, time signature
    track_list.rs     — Instrument/track list sidebar
    mixer.rs          — Mixer channels with sliders
    arrangement.rs    — Timeline grid with clip rectangles
    detail_view.rs    — Piano roll or waveform (selected clip)
    instrument_editor.rs  — Source, filter, effects
    common/
      slider.rs       — Reusable slider component
      meter.rs        — Level meter (CSS-based)
      button.rs       — Toggle, momentary buttons
  styles/
    main.css          — All styling
```

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `SharedState` | `src/state.rs` | Wraps AppState + AudioHandle for Dioxus |
| `DispatchExt` | `src/dispatch.rs` | Extension trait for dispatching actions |
| `App` | `src/app.rs` | Root component |

## Architecture

### State Bridge

The GUI uses Dioxus Signals to wrap the core AppState:

```rust
// state.rs
pub struct SharedState {
    pub app: AppState,
    pub audio: AudioHandle,
    io_tx: Sender<IoFeedback>,
    io_rx: Receiver<IoFeedback>,
}

// Used via Signal<SharedState> in components
```

### Dispatch Pattern

Components dispatch actions via the `DispatchExt` trait:

```rust
// In a component
let mut dispatch = use_dispatch();
dispatch.dispatch_action(Action::Mixer(MixerAction::ToggleMute));
```

### Audio Feedback Polling

Audio feedback is polled at ~30fps in the App component using `use_future`.

## Build & Run

```bash
cargo build -p imbolc-gui
cargo run -p imbolc-gui
```

## Design Constraints

- **No waveform rendering** except in zoomed/selected detail view
- **No MIDI note rendering** except in zoomed/selected detail view
- **Sliders only** — no rotary knobs
- Clips shown as colored rectangles with labels

## Styling

All styles are in `src/styles/main.css`. Uses CSS variables for theming.

## Dependencies

- `dioxus` — UI framework with desktop feature
- `imbolc-core` — State management, dispatch, audio engine
- `imbolc-types` — Shared type definitions
- `async-std` — For async sleeping in feedback polling
