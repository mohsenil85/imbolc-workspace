# Plan: imbolc-gui with Dioxus

**Status:** FUTURE
**Last Updated:** 2025-02-06

## Overview

Add a new `imbolc-gui` crate providing a cross-platform GUI (Linux/Mac) using Dioxus. The GUI will be a parallel alternative to `imbolc-ui` (the TUI), sharing all core logic via `imbolc-core` and `imbolc-types`.

### Design Constraints (per discussion)
- **No waveform rendering** except in zoomed/selected detail view
- **No MIDI note rendering** except in zoomed/selected detail view
- **Sliders only** — no rotary knobs
- Clips shown as colored rectangles with labels

---

## Crate Structure

```
imbolc/
├── imbolc-gui/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # Entry point, app launch
│   │   ├── app.rs            # Root component, state bridge
│   │   ├── state.rs          # Dioxus signals wrapping AppState
│   │   ├── dispatch.rs       # Action dispatch integration
│   │   ├── components/
│   │   │   ├── mod.rs
│   │   │   ├── transport.rs      # Play/stop/record, BPM, time sig
│   │   │   ├── track_list.rs     # Instrument/track list sidebar
│   │   │   ├── mixer.rs          # Mixer channels with sliders
│   │   │   ├── arrangement.rs    # Timeline grid with clip rectangles
│   │   │   ├── detail_view.rs    # Piano roll or waveform (selected clip)
│   │   │   ├── instrument_editor.rs  # Source, filter, effects
│   │   │   └── common/
│   │   │       ├── slider.rs     # Reusable slider component
│   │   │       ├── meter.rs      # Level meter (CSS-based)
│   │   │       └── button.rs     # Toggle, momentary buttons
│   │   ├── styles/
│   │   │   └── main.css      # All styling
│   │   └── assets/           # Icons, fonts if needed
│   └── CLAUDE.md
```

**Dependency graph:**
```
imbolc-types
     │
     ▼
imbolc-core
     │
     ├──────────────┐
     ▼              ▼
imbolc-ui      imbolc-gui (new)
```

---

## Cargo.toml

```toml
[package]
name = "imbolc-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
imbolc-core = { path = "../imbolc-core" }
imbolc-types = { path = "../imbolc-types" }
dioxus = { version = "0.6", features = ["desktop"] }
log = { workspace = true }
env_logger = "0.11"
```

Add to workspace root `Cargo.toml`:
```toml
members = ["imbolc-types", "imbolc-core", "imbolc-ui", "imbolc-gui"]
```

---

## State Bridge Architecture

### The Challenge
- `AppState` is a large struct owned by the main loop
- Dioxus uses reactive signals (`use_signal`)
- Need to bridge without cloning entire state every frame

### Solution: Shared State + Selective Signals

```rust
// src/state.rs
use std::sync::{Arc, RwLock};
use imbolc_core::state::AppState;

/// Shared state accessible from components
pub type SharedState = Arc<RwLock<AppState>>;

/// UI-specific reactive state (things that change frequently)
#[derive(Clone)]
pub struct UiState {
    pub selected_track: Signal<Option<InstrumentId>>,
    pub selected_clip: Signal<Option<ClipId>>,
    pub view_mode: Signal<ViewMode>,
    pub playhead: Signal<u32>,
    pub is_playing: Signal<bool>,
}

pub enum ViewMode {
    Arrangement,
    Mixer,
    InstrumentEdit,
    ClipDetail,
}
```

Components read from `SharedState` via `state.read()` and dispatch actions to mutate it.

---

## Main Loop Integration

```rust
// src/main.rs
fn main() {
    env_logger::init();
    dioxus::launch(App);
}

// src/app.rs
#[component]
fn App() -> Element {
    // Initialize core state
    let app_state = use_signal(|| {
        Arc::new(RwLock::new(AppState::default()))
    });

    // Initialize audio handle
    let audio = use_signal(|| {
        AudioHandle::new().expect("Failed to create audio handle")
    });

    // Audio feedback polling (runs every frame)
    use_effect(move || {
        // Poll audio feedback, update playhead/meters
        // This runs on an interval via spawn
    });

    // Provide state to all children
    use_context_provider(|| app_state);
    use_context_provider(|| audio);

    rsx! {
        div { class: "app",
            Transport {}
            div { class: "main-content",
                TrackList {}
                ArrangementView {}
            }
            Mixer {}
        }
    }
}
```

---

## Dispatch Integration

```rust
// src/dispatch.rs
use imbolc_types::Action;
use imbolc_core::dispatch::dispatch_action;

/// Hook for dispatching actions from any component
pub fn use_dispatch() -> impl Fn(Action) {
    let state = use_context::<Signal<SharedState>>();
    let audio = use_context::<Signal<AudioHandle>>();

    move |action: Action| {
        let mut state_guard = state.read().write().unwrap();
        let mut audio_guard = audio.write();
        let (io_tx, _) = std::sync::mpsc::channel(); // Simplified for now

        let result = dispatch_action(&action, &mut state_guard, &mut audio_guard, &io_tx);

        // Handle result (navigation, status messages, etc.)
        if result.quit {
            std::process::exit(0);
        }
    }
}
```

Usage in components:
```rust
#[component]
fn PlayButton() -> Element {
    let dispatch = use_dispatch();

    rsx! {
        button {
            onclick: move |_| dispatch(Action::PianoRoll(PianoRollAction::PlayStop)),
            "▶"
        }
    }
}
```

---

## Component Examples

### Transport Bar
```rust
#[component]
fn Transport() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let dispatch = use_dispatch();

    let is_playing = {
        let s = state.read().read().unwrap();
        s.session.piano_roll.playing
    };
    let bpm = {
        let s = state.read().read().unwrap();
        s.session.bpm
    };

    rsx! {
        div { class: "transport",
            button {
                onclick: move |_| dispatch(Action::PianoRoll(PianoRollAction::PlayStop)),
                if is_playing { "⏸" } else { "▶" }
            }
            button {
                onclick: move |_| dispatch(Action::Server(ServerAction::Stop)),
                "⏹"
            }
            span { class: "bpm", "{bpm} BPM" }
        }
    }
}
```

### Mixer Channel (with slider)
```rust
#[component]
fn MixerChannel(instrument_id: InstrumentId) -> Element {
    let state = use_context::<Signal<SharedState>>();
    let dispatch = use_dispatch();

    let (name, level, mute, solo) = {
        let s = state.read().read().unwrap();
        let inst = s.instruments.get(instrument_id);
        (inst.name.clone(), inst.level, inst.mute, inst.solo)
    };

    rsx! {
        div { class: "mixer-channel",
            div { class: "channel-name", "{name}" }

            input {
                r#type: "range",
                min: "0",
                max: "1",
                step: "0.01",
                value: "{level}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f32>() {
                        dispatch(Action::Mixer(MixerAction::AdjustLevel(v - level)));
                    }
                }
            }

            div { class: "channel-buttons",
                button {
                    class: if mute { "active" } else { "" },
                    onclick: move |_| dispatch(Action::Mixer(MixerAction::ToggleMute)),
                    "M"
                }
                button {
                    class: if solo { "active" } else { "" },
                    onclick: move |_| dispatch(Action::Mixer(MixerAction::ToggleSolo)),
                    "S"
                }
            }
        }
    }
}
```

### Arrangement (colored rectangles)
```rust
#[component]
fn ArrangementView() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let dispatch = use_dispatch();

    let placements = {
        let s = state.read().read().unwrap();
        s.session.arrangement.placements.clone()
    };

    rsx! {
        div { class: "arrangement",
            for placement in placements {
                div {
                    class: "clip",
                    style: "left: {placement.start_tick}px; width: {placement.length}px;",
                    onclick: move |_| {
                        // Select clip for detail view
                    },
                    "{placement.clip_id}"
                }
            }
        }
    }
}
```

---

## Level Meters (CSS-only, no Dioxus diffing)

```css
/* styles/main.css */
.meter {
    width: 8px;
    height: 100px;
    background: #333;
    position: relative;
}

.meter-fill {
    position: absolute;
    bottom: 0;
    width: 100%;
    background: linear-gradient(to top, green, yellow, red);
    transition: height 50ms linear;
}
```

```rust
#[component]
fn Meter(level: f32) -> Element {
    let height = (level * 100.0).clamp(0.0, 100.0);

    rsx! {
        div { class: "meter",
            div {
                class: "meter-fill",
                style: "height: {height}%;"
            }
        }
    }
}
```

---

## Implementation Phases

### Phase 1: Scaffold
- [ ] Create `imbolc-gui` crate with Cargo.toml
- [ ] Add to workspace
- [ ] Basic main.rs that launches empty Dioxus window
- [ ] Verify it builds and runs

### Phase 2: State Bridge
- [ ] Implement `SharedState` wrapper
- [ ] Implement `use_dispatch` hook
- [ ] Test with a single button that dispatches an action
- [ ] Verify state mutations work

### Phase 3: Transport + Track List
- [ ] Transport bar (play/stop, BPM display)
- [ ] Track list sidebar (instrument names, selection)
- [ ] Basic layout CSS

### Phase 4: Mixer
- [ ] Mixer channel component with volume slider
- [ ] Mute/solo buttons
- [ ] Master channel
- [ ] Level meters (CSS-based)

### Phase 5: Arrangement
- [ ] Grid background
- [ ] Clip rectangles (no waveforms)
- [ ] Playhead line
- [ ] Click to select clip

### Phase 6: Detail View
- [ ] Piano roll view (for selected MIDI clip)
- [ ] Waveform view (for selected audio clip)
- [ ] Only renders when a clip is selected

### Phase 7: Instrument Editor
- [ ] Source type selector
- [ ] Parameter sliders
- [ ] Effect chain management

### Phase 8: Polish
- [ ] Keyboard shortcuts
- [ ] File open/save dialogs
- [ ] Preferences
- [ ] Dark theme styling

---

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` (root) | Add `imbolc-gui` to workspace members |
| `imbolc-gui/Cargo.toml` | New file |
| `imbolc-gui/src/*.rs` | New files |
| `imbolc-gui/src/styles/main.css` | New file |
| `imbolc-gui/CLAUDE.md` | New file with GUI-specific guidance |

---

## Verification

After each phase:
1. `cargo build -p imbolc-gui` — compiles without errors
2. `cargo run -p imbolc-gui` — window appears, no crashes
3. Manual test of new functionality
4. Verify actions dispatch correctly by checking state changes

Final verification:
1. Create a new project
2. Add instruments via GUI
3. Adjust mixer levels
4. Play/stop transport
5. Place clips in arrangement
6. Select clip and view detail
7. Save and reload project

---

## Decisions

- **Detail view rendering**: HTML Canvas (better for many notes, requires JS interop via `web-sys` or `eval`)
- **Audio feedback polling**: 30fps for meters
- **Hot reload**: Use `dx serve` during development
