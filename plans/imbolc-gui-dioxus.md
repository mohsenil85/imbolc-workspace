# Plan: imbolc-gui with Dioxus

**Status:** IN PROGRESS
**Last Updated:** 2026-02-06

## Overview

Add a new `imbolc-gui` crate providing a cross-platform GUI (Linux/Mac) using Dioxus. The GUI will be a parallel alternative to `imbolc-ui` (the TUI), sharing all core logic via `imbolc-core` and `imbolc-types`.

### Design Constraints (per discussion)
- **No waveform rendering** except in zoomed/selected detail view
- **No MIDI note rendering** except in zoomed/selected detail view
- **Sliders only** — no rotary knobs
- Clips shown as colored rectangles with labels

---

## Implementation Status

### Phase 1: Scaffold ✅ COMPLETE
- [x] Create `imbolc-gui` crate with Cargo.toml
- [x] Add to workspace
- [x] Basic main.rs that launches empty Dioxus window
- [x] Verify it builds and runs

### Phase 2: State Bridge ✅ COMPLETE
- [x] Implement `SharedState` wrapper (`src/state.rs`)
- [x] Implement `use_dispatch` hook (`src/dispatch.rs`)
- [x] Test with actions dispatching correctly

### Phase 3: Transport + Track List ✅ COMPLETE
- [x] Transport bar (play/stop, BPM display)
- [x] Track list sidebar (instrument names, selection)
- [x] Basic layout CSS

### Phase 4: Mixer ✅ COMPLETE
- [x] Mixer channel component with volume slider
- [x] Mute/solo buttons
- [x] Master channel
- [x] Level meters (CSS-based)

### Phase 5: Arrangement ✅ COMPLETE
- [x] Grid background
- [x] Clip rectangles (no waveforms)
- [x] Playhead line
- [x] Click to select clip

### Phase 6: Detail View ✅ COMPLETE
- [x] `piano_roll_view.rs` - Piano roll with grid, notes, navigation
- [x] `waveform_view.rs` - Placeholder for audio clips
- [x] `detail_view.rs` - Switches between views based on selection

### Phase 7: Instrument Editor ✅ COMPLETE
- [x] `common/dropdown.rs` - Reusable dropdown component
- [x] `effect_slot.rs` - Effect chain editing
- [x] Source type selector
- [x] Source parameter sliders
- [x] Filter section with type/cutoff/resonance
- [x] Effects chain with add/remove/reorder/bypass
- [x] LFO section with shape/target/rate/depth
- [x] Envelope (ADSR) section

### Phase 8: Polish ✅ COMPLETE
- [x] `keybindings.rs` - Keyboard shortcuts system
- [x] `file_ops.rs` - File dialogs with rfd
- [x] Keyboard handler in `app.rs`
- [x] Dark theme CSS polish
- [x] Hover/active states, transitions, focus rings

---

## Next Steps

### Phase 9: Integration & Wiring
- [ ] Wire up LFO toggle action (needs new InstrumentAction variant or use InstrumentUpdate)
- [ ] Wire up envelope parameter changes
- [ ] Wire up source parameter changes via InstrumentUpdate
- [ ] Complete file save/load flow with UI feedback
- [ ] Add loading/saving state indicators

### Phase 10: Arrangement Editing
- [ ] Create clip from arrangement view
- [ ] Drag clips to move position
- [ ] Resize clips
- [ ] Delete clips
- [ ] Duplicate clips
- [ ] Right-click context menu

### Phase 11: Piano Roll Editing
- [ ] Drag notes to move
- [ ] Resize note duration
- [ ] Velocity editing (vertical drag or separate lane)
- [ ] Selection rectangle
- [ ] Copy/paste notes
- [ ] Snap to grid options

### Phase 12: Audio Feedback
- [ ] Real-time meter updates from AudioHandle feedback
- [ ] Playhead position sync with audio engine
- [ ] Transport state sync (playing/recording indicators)

### Phase 13: Additional Features
- [ ] Undo/redo visual feedback
- [ ] Status bar with messages
- [ ] Project name in title bar
- [ ] Preferences dialog
- [ ] Keyboard shortcut customization
- [ ] Theme customization

---

## Current File Structure

```
imbolc-gui/
├── Cargo.toml
├── CLAUDE.md
└── src/
    ├── main.rs              # Entry point
    ├── app.rs               # Root component, keyboard handler
    ├── state.rs             # SharedState wrapper
    ├── dispatch.rs          # Action dispatch hook
    ├── keybindings.rs       # Keyboard shortcuts
    ├── file_ops.rs          # File dialogs (rfd)
    ├── components/
    │   ├── mod.rs
    │   ├── transport.rs
    │   ├── track_list.rs
    │   ├── mixer.rs
    │   ├── arrangement.rs
    │   ├── detail_view.rs
    │   ├── piano_roll_view.rs
    │   ├── waveform_view.rs
    │   ├── instrument_editor.rs
    │   ├── effect_slot.rs
    │   └── common/
    │       ├── mod.rs
    │       ├── slider.rs
    │       ├── meter.rs
    │       ├── button.rs
    │       └── dropdown.rs
    └── styles/
        └── main.css
```

---

## Key Shortcuts Implemented

| Key | Action |
|-----|--------|
| Space | Toggle play |
| Escape | Stop |
| Ctrl+R | Record |
| 1-4 | Focus panes |
| Ctrl+N | New project |
| Ctrl+O | Open project |
| Ctrl+S | Save project |
| Ctrl+Shift+S | Save as |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z / Ctrl+Y | Redo |
| M | Toggle mute |
| S | Toggle solo |
| Delete/Backspace | Delete |

---

## Dependencies

```toml
[dependencies]
imbolc-core = { path = "../imbolc-core" }
imbolc-types = { path = "../imbolc-types" }
dioxus = { version = "0.6", features = ["desktop"] }
async-std = "1.12"
log = "0.4"
env_logger = "0.11"
rfd = "0.14"
```

---

## Build & Run

```bash
cargo build -p imbolc-gui
cargo run -p imbolc-gui
```

---

## Known Issues / TODOs

1. **Unused import warnings** - Some components are exported but not fully integrated into layout
2. **LFO/Envelope actions** - UI is present but dispatch handlers are TODO comments
3. **Source type change** - Needs InstrumentUpdate action, currently just logs
4. **Piano roll click dispatch** - Dispatches ToggleNote but may need track/clip context
5. **File operations** - Dialogs work but need UI feedback for success/failure

---

## Testing Checklist

- [ ] Launch GUI, verify window appears
- [ ] Add instrument via track list
- [ ] Adjust mixer level/pan
- [ ] Toggle mute/solo
- [ ] Press space to play/stop
- [ ] Click clip to select
- [ ] View piano roll for selected clip
- [ ] Add effect to instrument
- [ ] Adjust effect parameters
- [ ] Toggle filter on/off
- [ ] Adjust filter cutoff/resonance
- [ ] Save project (Ctrl+S)
- [ ] Load project (Ctrl+O)
- [ ] Undo/redo (Ctrl+Z / Ctrl+Shift+Z)
