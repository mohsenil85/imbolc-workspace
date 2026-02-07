# Tasks

All remaining work, organized by priority tier.

See TASKS_DONE.md for completed work.

---

## Bugs

### Piano roll: remove time signature display

**Sources:** R2 #10

Time signature is shown in the piano roll header. It belongs in the
session/frame settings (`FrameEditPane`), not cluttering the piano roll.
Find and remove time signature display from the piano roll header.

**Files:** `imbolc-ui/src/panes/piano_roll_pane.rs`

---

## Quick Wins

### Remove help text along bottom

**Sources:** R2 #11

Most panes render a hardcoded help line at the bottom (e.g.,
`"Left/Right: adjust | Enter: type/confirm | Esc: cancel"`). This
clutters the UI. The `?` key already opens context-sensitive help via
`HelpPane`.

1. Remove all inline help text from pane `render()` methods
2. Optionally add a subtle `? for help` indicator in the frame chrome

**Files:** All panes in `imbolc-ui/src/panes/`, `imbolc-ui/src/ui/frame.rs`

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

## Long-term

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
