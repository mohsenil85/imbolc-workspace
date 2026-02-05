# Copy / Cut / Paste — Implementation Plan

## Overview

Add clipboard operations (Ctrl+C copy, Ctrl+X cut, Ctrl+V paste) with Shift+Arrow
range selection. Covers piano roll notes, drum sequencer steps, and automation
points. Ctrl+A selects all notes/steps in the current context.

## Keybinding Conflict

`Ctrl+C` is currently bound to `clear_pattern` in the sequencer layer.
`Ctrl+A` is currently bound to `add_instrument` in the global layer.

**Resolution:** Remap `clear_pattern` from `Ctrl+C` to `X` (uppercase) in
`keybindings.toml` sequencer layer. Remap `add_instrument` from `Ctrl+A` to
just the existing `a` key on the instrument list pane (it already exists there).
Remove `Ctrl+A` from global layer — instrument adding via the `a` key on the
instrument pane is sufficient. This frees up Ctrl+A/C/X/V globally.

---

## Part 1: Clipboard & Selection Data Structures

### File: `imbolc-core/src/state/clipboard.rs` (new)

```rust
use super::piano_roll::Note;
use super::drum_sequencer::DrumStep;

/// A note stored with position relative to the selection anchor.
/// anchor = (min_tick of selected notes, min_pitch of selected notes)
#[derive(Debug, Clone)]
pub struct ClipboardNote {
    pub tick_offset: u32,    // tick - anchor_tick
    pub pitch_offset: i16,   // pitch as i16 - anchor_pitch as i16
    pub duration: u32,
    pub velocity: u8,
    pub probability: f32,
}

/// Clipboard contents — one variant per context
#[derive(Debug, Clone)]
pub enum ClipboardContents {
    /// Piano roll notes with relative positions
    PianoRollNotes(Vec<ClipboardNote>),
    /// Drum sequencer steps: Vec<(pad_index, step_offset, DrumStep)>
    DrumSteps {
        steps: Vec<(usize, usize, DrumStep)>, // (pad_idx, step_offset, step_data)
    },
    /// Automation points: Vec<(tick_offset, value)>
    AutomationPoints {
        points: Vec<(u32, f32)>, // (tick_offset, value)
    },
}

/// App-wide clipboard (lives in AppState)
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    pub contents: Option<ClipboardContents>,
}
```

### Selection state on PianoRollPane

Add to `PianoRollPane` struct in `src/panes/piano_roll_pane/mod.rs`:

```rust
/// Selection anchor — set when Shift+Arrow begins. None = no active selection.
pub(super) selection_anchor: Option<(u32, u8)>,  // (tick, pitch)
```

The selection region is the rectangle from `selection_anchor` to
`(cursor_tick, cursor_pitch)`. When anchor is `Some`, render all notes within
the rectangle with a highlight style.

### Selection state on SequencerPane

Add to `SequencerPane` struct in `src/panes/sequencer_pane.rs`:

```rust
/// Selection anchor (pad, step). None = no selection.
selection_anchor: Option<(usize, usize)>,
```

Region is from anchor to `(cursor_pad, cursor_step)`.

### Selection state on AutomationPane

Add to `AutomationPane`:

```rust
selection_anchor_tick: Option<u32>,
```

Region is `anchor_tick..cursor_tick` on the selected lane.

---

## Part 2: Actions

### File: `imbolc-core/src/action.rs`

Add new action variants to the `PianoRollAction` enum:

```rust
/// Delete all notes in the given region (used by Cut)
DeleteNotesInRegion {
    track: usize,
    start_tick: u32,
    end_tick: u32,
    start_pitch: u8,
    end_pitch: u8,
},
/// Paste notes at a position from clipboard
PasteNotes {
    track: usize,
    anchor_tick: u32,
    anchor_pitch: u8,
    notes: Vec<ClipboardNote>,
},
```

Add new action variants to the `SequencerAction` enum:

```rust
/// Delete steps in region (used by Cut)
DeleteStepsInRegion {
    start_pad: usize,
    end_pad: usize,
    start_step: usize,
    end_step: usize,
},
/// Paste drum steps at cursor
PasteSteps {
    anchor_pad: usize,
    anchor_step: usize,
    steps: Vec<(usize, usize, DrumStep)>,
},
```

Add new action variants to the `AutomationAction` enum:

```rust
/// Delete automation points in tick range on a lane
DeletePointsInRange(AutomationLaneId, u32, u32),
/// Paste automation points at offset
PastePoints(AutomationLaneId, u32, Vec<(u32, f32)>),
```

### Undo integration

In `imbolc-core/src/state/undo.rs`, add all new action variants to the
`is_undoable()` function — `DeleteNotesInRegion`, `PasteNotes`,
`DeleteStepsInRegion`, `PasteSteps`, `DeletePointsInRange`, `PastePoints`
should all return `true`. The undo system takes full-state snapshots so no
other undo changes are needed.

---

## Part 3: Clipboard in AppState

### File: `imbolc-core/src/state/mod.rs`

Add `pub clipboard: Clipboard` field to `AppState`. Initialize as
`Clipboard::default()` in `AppState::new()`.

Also add `pub mod clipboard;` to the state module.

---

## Part 4: Dispatch Handlers

### File: `imbolc-core/src/dispatch/piano_roll.rs`

Handle `DeleteNotesInRegion`:
```rust
PianoRollAction::DeleteNotesInRegion { track, start_tick, end_tick, start_pitch, end_pitch } => {
    if let Some(t) = state.session.piano_roll.track_at_mut(*track) {
        t.notes.retain(|n| {
            !(n.pitch >= *start_pitch && n.pitch <= *end_pitch
              && n.tick >= *start_tick && n.tick < *end_tick)
        });
    }
    result.audio_dirty.piano_roll = true;
}
```

Handle `PasteNotes`:
```rust
PianoRollAction::PasteNotes { track, anchor_tick, anchor_pitch, notes } => {
    if let Some(t) = state.session.piano_roll.track_at_mut(*track) {
        for cn in notes {
            let tick = *anchor_tick + cn.tick_offset;
            let pitch_i16 = *anchor_pitch as i16 + cn.pitch_offset;
            if pitch_i16 < 0 || pitch_i16 > 127 { continue; }
            let pitch = pitch_i16 as u8;
            // Avoid duplicates at same (pitch, tick)
            if !t.notes.iter().any(|n| n.pitch == pitch && n.tick == tick) {
                let pos = t.notes.partition_point(|n| n.tick < tick);
                t.notes.insert(pos, Note {
                    tick,
                    duration: cn.duration,
                    pitch,
                    velocity: cn.velocity,
                    probability: cn.probability,
                });
            }
        }
    }
    result.audio_dirty.piano_roll = true;
}
```

### File: `imbolc-core/src/dispatch/sequencer.rs`

Handle `DeleteStepsInRegion`:
```rust
SequencerAction::DeleteStepsInRegion { start_pad, end_pad, start_step, end_step } => {
    if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
        let pattern = &mut seq.patterns[seq.current_pattern];
        for pad in *start_pad..=*end_pad {
            for step in *start_step..=*end_step {
                if pad < pattern.steps.len() && step < pattern.steps[pad].len() {
                    pattern.steps[pad][step] = DrumStep::default();
                }
            }
        }
    }
}
```

Handle `PasteSteps`:
```rust
SequencerAction::PasteSteps { anchor_pad, anchor_step, steps } => {
    if let Some(seq) = state.instruments.selected_drum_sequencer_mut() {
        let pattern = &mut seq.patterns[seq.current_pattern];
        for (pad_offset, step_offset, step_data) in steps {
            let pad = anchor_pad + pad_offset;
            let step = anchor_step + step_offset;
            if pad < pattern.steps.len() && step < pattern.steps[pad].len() {
                pattern.steps[pad][step] = step_data.clone();
            }
        }
    }
}
```

### File: `imbolc-core/src/dispatch/automation.rs`

Handle `DeletePointsInRange` and `PastePoints` similarly — iterate the lane's
points, remove those in range, or insert new ones at offset positions.

---

## Part 5: Keybinding Changes

### File: `keybindings.toml`

**Global layer** — remove `Ctrl+A` (add_instrument). Add:

```toml
{ key = "Ctrl+c", action = "copy", description = "Copy" },
{ key = "Ctrl+x", action = "cut", description = "Cut" },
{ key = "Ctrl+v", action = "paste", description = "Paste" },
{ key = "Ctrl+a", action = "select_all", description = "Select all" },
```

**Sequencer layer** — change `Ctrl+c` from `clear_pattern` to use `X`:

```toml
{ key = "X", action = "clear_pattern", description = "Clear entire pattern" },
```

Remove the `{ key = "Ctrl+c", ... }` line from sequencer layer.

**Piano roll layer** — add Shift+Arrow bindings for selection:

```toml
{ key = "Shift+Up", action = "select_up", description = "Extend selection up" },
{ key = "Shift+Down", action = "select_down", description = "Extend selection down" },
{ key = "Shift+Left", action = "select_left", description = "Extend selection left" },
{ key = "Shift+Right", action = "select_right", description = "Extend selection right" },
```

Note: `Shift+Right` and `Shift+Left` are currently `grow_duration` and
`shrink_duration`. These need to move to `Alt+Right` / `Alt+Left`:

```toml
{ key = "Alt+Right", action = "grow_duration", description = "Grow note duration" },
{ key = "Alt+Left", action = "shrink_duration", description = "Shrink note duration" },
```

**Sequencer layer** — add Shift+Arrow for selection (currently Shift+Up/Down
are vel_up/vel_down and Shift+Left/Right are pad_level). Remap velocity to
`+`/`-` and pad level to `Ctrl+Left`/`Ctrl+Right`:

```toml
{ key = "Shift+Up", action = "select_up", description = "Extend selection up" },
{ key = "Shift+Down", action = "select_down", description = "Extend selection down" },
{ key = "Shift+Left", action = "select_left", description = "Extend selection left" },
{ key = "Shift+Right", action = "select_right", description = "Extend selection right" },
```

Existing displaced bindings — move to:
- `vel_up`/`vel_down`: already have `+`/`-` in sequencer, keep those as primary
- `pad_level_up`/`pad_level_down`: move to `Ctrl+Right`/`Ctrl+Left`

---

## Part 6: Input Handling — Piano Roll

### File: `src/panes/piano_roll_pane/input.rs`

Add selection action handlers in `handle_action_impl`:

```rust
"select_up" | "select_down" | "select_left" | "select_right" => {
    // Set anchor if not already set
    if self.selection_anchor.is_none() {
        self.selection_anchor = Some((self.cursor_tick, self.cursor_pitch));
    }
    // Move cursor (same as normal navigation)
    match action {
        "select_up" => { if self.cursor_pitch < 127 { self.cursor_pitch += 1; } }
        "select_down" => { if self.cursor_pitch > 0 { self.cursor_pitch -= 1; } }
        "select_right" => { self.cursor_tick += self.ticks_per_cell(); }
        "select_left" => {
            let step = self.ticks_per_cell();
            self.cursor_tick = self.cursor_tick.saturating_sub(step);
        }
        _ => {}
    }
    self.scroll_to_cursor();
    Action::None
}
```

Clear selection on any non-selection navigation (in `"up"`, `"down"`, `"left"`,
`"right"`, `"home"`, `"end"`, etc.): add `self.selection_anchor = None;` at the
start of each.

### File: `src/global_actions.rs`

Handle "copy", "cut", "paste", "select_all" in `handle_global_action`:

```rust
"copy" => {
    // Delegate to the active pane's copy behavior
    // The pane must be accessed to read selection + build clipboard contents
    copy_from_active_pane(state, panes);
}
"cut" => {
    undo_history.push(&state.session, &state.instruments);
    let action = cut_from_active_pane(state, panes);
    if let Some(action) = action {
        let r = dispatch::dispatch_action(&action, state, audio, io_tx);
        pending_audio_dirty.merge(r.audio_dirty);
        apply_dispatch_result(r, state, panes, app_frame);
    }
}
"paste" => {
    undo_history.push(&state.session, &state.instruments);
    let action = paste_to_active_pane(state, panes);
    if let Some(action) = action {
        let r = dispatch::dispatch_action(&action, state, audio, io_tx);
        pending_audio_dirty.merge(r.audio_dirty);
        apply_dispatch_result(r, state, panes, app_frame);
    }
}
"select_all" => {
    select_all_in_active_pane(state, panes);
}
```

These four helper functions inspect `panes.active().id()` to determine context
and perform the appropriate operation.

#### `copy_from_active_pane`

For `"piano_roll"`:
1. Get the PianoRollPane via `panes.get_pane_mut::<PianoRollPane>("piano_roll")`
2. Read `selection_anchor` and cursor to compute the selection rectangle
3. If no selection, copy the single note at cursor (if one exists)
4. Iterate `state.session.piano_roll.track_at(current_track).notes`
5. Collect all notes within rectangle
6. Convert to `Vec<ClipboardNote>` with offsets relative to (min_tick, min_pitch)
7. Store in `state.clipboard.contents = Some(ClipboardContents::PianoRollNotes(...))`

For `"sequencer"`:
1. Get SequencerPane, read anchor + cursor
2. Collect DrumSteps in the rectangular region (pad range × step range)
3. Store as `ClipboardContents::DrumSteps { steps }`

For `"automation"`:
1. Get AutomationPane, read anchor_tick + cursor_tick + selected_lane
2. Collect points in the tick range
3. Store as `ClipboardContents::AutomationPoints { points }`

#### `cut_from_active_pane`

Same as copy, but also returns the appropriate `DeleteNotesInRegion` /
`DeleteStepsInRegion` / `DeletePointsInRange` action to dispatch.

#### `paste_to_active_pane`

1. Read `state.clipboard.contents`
2. Match on variant
3. For `PianoRollNotes`: return `PianoRollAction::PasteNotes` with cursor
   position as anchor
4. For `DrumSteps`: return `SequencerAction::PasteSteps` with cursor position
5. For `AutomationPoints`: return `AutomationAction::PastePoints` with cursor tick

After pasting, clear the selection anchor (but keep clipboard for repeat paste).

#### `select_all_in_active_pane`

For `"piano_roll"`:
1. Find the min tick and max tick+duration of all notes on current track
2. Find the min and max pitch of all notes
3. Set `selection_anchor = Some((min_tick, min_pitch))` and
   `cursor_tick = max_tick`, `cursor_pitch = max_pitch`

For `"sequencer"`:
1. Set `selection_anchor = Some((0, 0))`, cursor to `(NUM_PADS-1, pattern.length-1)`

---

## Part 7: Rendering — Selection Highlighting

### File: `src/panes/piano_roll_pane/rendering.rs`

In `render_notes_buf`, after computing `is_cursor`, add selection check:

```rust
let in_selection = self.selection_anchor.map_or(false, |(anchor_tick, anchor_pitch)| {
    let (t0, t1) = if anchor_tick <= self.cursor_tick {
        (anchor_tick, self.cursor_tick + self.ticks_per_cell())
    } else {
        (self.cursor_tick, anchor_tick + self.ticks_per_cell())
    };
    let (p0, p1) = if anchor_pitch <= self.cursor_pitch {
        (anchor_pitch, self.cursor_pitch)
    } else {
        (self.cursor_pitch, anchor_pitch)
    };
    tick >= t0 && tick < t1 && pitch >= p0 && pitch <= p1
});
```

Then in the style selection block, add a branch for `in_selection`:

```rust
} else if in_selection && has_note {
    // Selected note
    ('█', Style::new().fg(Color::WHITE).bg(Color::new(60, 30, 80)))
} else if in_selection {
    // Selection region background
    ('░', Style::new().fg(Color::new(60, 30, 80)))
}
```

Insert this between the `is_cursor` check and the `has_note` check.

### File: `src/panes/sequencer_pane.rs`

Similar: compute `in_selection` from `selection_anchor` and cursor, apply a
highlight bg color to selected step cells.

### Status line update

When a selection is active, show the selection dimensions in the status line:
```
Sel: 4 notes (2 beats × 5 pitches)
```

---

## Part 8: Selection clearing

Selection should clear on:
- Any non-Shift arrow navigation (up/down/left/right/home/end/octave)
- Escape key
- Pane switch (in `on_enter`)
- After cut (clear automatically)
- Mouse click (clear in handle_mouse)

Selection should NOT clear on:
- Copy (preserve for re-copy or follow-up cut)
- Paste (preserve for repeat paste)
- Playback toggle

---

## File Change Summary

| File | Change |
|------|--------|
| `imbolc-core/src/state/clipboard.rs` | **NEW** — ClipboardNote, ClipboardContents, Clipboard |
| `imbolc-core/src/state/mod.rs` | Add `pub mod clipboard;` and `clipboard` field to AppState |
| `imbolc-core/src/action.rs` | Add DeleteNotesInRegion, PasteNotes, DeleteStepsInRegion, PasteSteps, DeletePointsInRange, PastePoints |
| `imbolc-core/src/state/undo.rs` | Mark new actions as undoable in `is_undoable()` |
| `imbolc-core/src/dispatch/piano_roll.rs` | Handle DeleteNotesInRegion, PasteNotes |
| `imbolc-core/src/dispatch/sequencer.rs` | Handle DeleteStepsInRegion, PasteSteps |
| `imbolc-core/src/dispatch/automation.rs` | Handle DeletePointsInRange, PastePoints |
| `keybindings.toml` | Add Ctrl+C/X/V/A globally; remap Shift+Arrow in piano_roll; remap clear_pattern in sequencer; remap grow/shrink_duration to Alt; remap Shift+Up/Down in sequencer |
| `src/panes/piano_roll_pane/mod.rs` | Add `selection_anchor` field |
| `src/panes/piano_roll_pane/input.rs` | Handle select_up/down/left/right; clear selection on non-shift nav |
| `src/panes/piano_roll_pane/rendering.rs` | Render selection highlight; update status line |
| `src/panes/sequencer_pane.rs` | Add `selection_anchor`, handle select actions, render selection |
| `src/global_actions.rs` | Handle copy/cut/paste/select_all with per-pane delegation |

---

## Implementation Order

1. **Clipboard data structures** — `clipboard.rs`, wire into AppState
2. **Keybinding changes** — `keybindings.toml` remaps
3. **Piano roll selection** — anchor field, select_* input handlers, selection clearing, render highlighting
4. **Piano roll copy/cut/paste** — global action handlers, dispatch for DeleteNotesInRegion + PasteNotes
5. **Sequencer selection + copy/cut/paste** — same pattern
6. **Automation copy/cut/paste** — same pattern (lower priority, can defer)
7. **Select all** — per-pane implementation

---

## Verification

1. `cargo build` — must compile clean
2. `cargo test` — all existing tests pass
3. Manual test sequence:
   - Open piano roll, place 4 notes in a pattern
   - Shift+Arrow to select them (verify highlight renders)
   - Ctrl+C to copy, move cursor, Ctrl+V to paste (verify new notes appear)
   - Ctrl+Z to undo paste (verify notes disappear)
   - Shift+Arrow to select, Ctrl+X to cut (verify notes removed + in clipboard)
   - Ctrl+V to paste at new position
   - Ctrl+A to select all, verify entire track selected
   - Switch to sequencer, toggle some steps, Shift+Arrow select, Ctrl+C, Ctrl+V
   - Verify Ctrl+C no longer clears sequencer pattern (X does instead)
   - Verify Shift+Right/Left in piano roll now extends selection (not grow/shrink duration)
   - Verify Alt+Right/Left grows/shrinks duration
