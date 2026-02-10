# Tasks

All remaining work, organized by priority tier.

See TASKS_DONE.md for completed work.

---

## Polish

### Handle small terminal + resize

**Sources:** R2 #13

Fixed-size boxes (height 29) break on small terminals. Resize events
not handled.

1. Minimum size check on startup and resize — show message if too small
2. Handle `Event::Resize` in the main event loop
3. Clamp `box_width`/`box_height` to available terminal size
4. Graceful degradation: hide optional elements on small terminals

**Files:** `imbolc-ui/src/main.rs`, `imbolc-ui/src/ui/ratatui_impl.rs`,
`imbolc-ui/src/ui/graphics.rs`

---

## Features

### Automation Recording

**Sources:** NEXT_STEPS #4, UNWIRED #2

Record parameter changes over time. `AutomationState`,
`AutomationLane`, etc. exist and are persisted, but missing:
automation editing pane and playback tick loop integration.

| Need | Status |
|------|--------|
| Automation lanes | Data structures exist, persisted |
| Recording mode | Not implemented |
| Playback interpolation | Not implemented |
| Editing pane | Not implemented |

---

### Audio Export

**Sources:** NEXT_STEPS #5

Render to WAV via SC NRT mode or real-time capture. Progress UI for
render status.

---

### Sequencer: Note Duration Grid Selection

**Sources:** FEATURES #4

In the sequencer view, allow switching between note durations for
placement (quarter, eighth, sixteenth notes, etc.). Keybind to cycle
or select grid resolution.

---

### LayerGroup Undo Support

**Sources:** Phase B7 (layer group EQ)

`LayerGroupAction` variants (ToggleEq, SetEqParam, and all effect
CRUD variants) are not undoable. They fall through to `_ => false` in
`is_undoable()` and `_ => UndoScope::Full` in `undo_scope()`. Adding
undo requires choosing appropriate scopes (Session for structural
toggles, skip for real-time param tweaks) and testing undo/redo
round-trips.

**Files:** `imbolc-core/src/state/undo.rs`, `imbolc-core/src/dispatch/bus.rs`

---

## Long-term

### Scaling Bottlenecks

See [plans/scaling-analysis.md](plans/scaling-analysis.md) for ranked analysis
of local and network scaling issues.

---

### Multi-track Audio Recording

**Sources:** NEXT_STEPS #8

Record live audio input to tracks. Requires `cpal` crate for audio
capture, waveform display, overdub sync.

---

### UI themes

**Sources:** R2 #19

All colors hardcoded in `imbolc-ui/src/ui/style.rs`. Define a `Theme` struct
with semantic color slots, ship 2-3 built-in themes (Default, Light,
High Contrast), store active theme in `AppState`, add theme
switcher. Large change touching every pane.

**Files:** `imbolc-ui/src/ui/style.rs`, `imbolc-types/src/state/mod.rs`, all panes

---

### Input/Automation Capture

Live recording of parameter changes and MIDI input as automation
data. When a user tweaks a knob or moves a fader during playback,
those movements should be captured as automation points on the
corresponding lane. MIDI CC input should record directly to automation
lanes. Arm/disarm per-lane recording.

---

### VST Parameter Discovery

Replace synthetic 128-parameter placeholders with real parameter
names, units, and ranges from the plugin via SuperCollider OSC
replies. Currently usable but clunky — users see "Param 0", "Param 1"
instead of meaningful names.

---

### MIDI Learn

"Wiggle a knob to assign it" workflow. CC mapping state exists but
there's no interactive UI for binding a physical controller to a
parameter. Should support learn mode where the next incoming CC
automatically maps to the selected target.

---

### Notification/Feedback System

A one-line status bar across the bottom of the screen that we can
print to programatically.

---

### Test Coverage

~31 unit tests and a handful of e2e tests — low for a project this
size. The e2e harness (tmux-based) is a good foundation but covers
very little. Needs render snapshot tests, UI interaction tests,
multi-step workflow tests, and regression coverage for input handling.

---

### Sidechain Visualization

Compressor gain reduction meters, sidechain input indicators in the
mixer.

---

### Group/Bus Metering

Level meters for the 8 buses and master in the mixer view.

---

### Plugin Scanning/Cataloging

Automatic VST3 directory scanning instead of manual file
import. Plugin database with search, favorites, and categories.

---

### VST Preset/Program Browser

UI for browsing and loading VST presets and programs. Currently state
save/restore works but there's no preset management interface.

---

### Latency Compensation

Plugin delay compensation (PDC) for VST instruments and
effects. Report and compensate for processing latency to keep tracks
aligned.

---

### MIDI Clock Sync

Send and receive MIDI clock for synchronization with external hardware
and software. Tempo leader/follower modes.

---

### CPU/DSP Load Meter

Real-time display of SuperCollider CPU usage and DSP load. Warning
indicators when approaching capacity.
