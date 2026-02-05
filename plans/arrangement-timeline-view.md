# Arrangement / Timeline View — Implementation Plan

## Overview

Transform the loop-based DAW into a full arrangement system with reusable clips on a timeline. **Key insight: the audio thread requires zero changes.** Song mode works by flattening clip placements into a temporary `PianoRollState` before sending it as the snapshot via the existing `flush_dirty()` path.

**Decisions:**
- Shared-reference clips (editing one instance edits all copies)
- Global timeline automation only (per-clip automation noted for future in `plans/remaining-features.md`)
- Dual mode: Pattern mode (current loop behavior) + Song mode (arrangement playback)

---

## 1. New State Types

### New file: `imbolc-core/src/state/arrangement.rs`

```rust
use std::collections::HashMap;
use super::instrument::InstrumentId;
use super::piano_roll::Note;  // Reuse existing Note type

pub type ClipId = u32;
pub type PlacementId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode { Pattern, Song }
impl Default for PlayMode { fn default() -> Self { Self::Pattern } }

/// Reusable pattern of notes for a single instrument.
/// Notes use tick positions relative to clip start (0-based).
#[derive(Debug, Clone)]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub instrument_id: InstrumentId,
    pub length_ticks: u32,
    pub notes: Vec<Note>,
}

/// A placement of a clip on the timeline. Multiple placements can share a clip.
#[derive(Debug, Clone)]
pub struct ClipPlacement {
    pub id: PlacementId,
    pub clip_id: ClipId,
    pub instrument_id: InstrumentId,
    pub start_tick: u32,           // Absolute position on timeline
    pub length_override: Option<u32>, // Trim shorter than clip, None = use clip.length_ticks
}

/// Saved context when editing a clip in the piano roll
#[derive(Debug, Clone)]
pub struct ClipEditContext {
    pub clip_id: ClipId,
    pub instrument_id: InstrumentId,
    pub stashed_notes: Vec<Note>,      // Original piano roll track notes
    pub stashed_loop_start: u32,
    pub stashed_loop_end: u32,
    pub stashed_looping: bool,
}

/// Top-level arrangement state. Owned by SessionState.
#[derive(Debug, Clone)]
pub struct ArrangementState {
    pub clips: Vec<Clip>,
    pub placements: Vec<ClipPlacement>,
    pub play_mode: PlayMode,
    pub editing_clip: Option<ClipEditContext>,

    // UI state (persisted)
    pub selected_placement: Option<usize>,
    pub selected_lane: usize,
    pub view_start_tick: u32,
    pub ticks_per_col: u32,        // Zoom: ticks per terminal column (default 120)
    pub cursor_tick: u32,

    next_clip_id: ClipId,
    next_placement_id: PlacementId,
}
```

**Key methods on `ArrangementState`:**
- `add_clip(name, instrument_id, length_ticks) -> ClipId`
- `clip(id) -> Option<&Clip>`, `clip_mut(id) -> Option<&mut Clip>`
- `remove_clip(id)` — also removes all placements referencing this clip
- `clips_for_instrument(instrument_id) -> Vec<&Clip>`
- `add_placement(clip_id, instrument_id, start_tick) -> PlacementId`
- `remove_placement(id)`, `move_placement(id, new_start)`, `resize_placement(id, new_length)`
- `placements_for_instrument(instrument_id) -> Vec<&ClipPlacement>` (sorted by start_tick)
- `placement_at(instrument_id, tick) -> Option<&ClipPlacement>` — hit test
- `flatten_to_notes() -> HashMap<InstrumentId, Vec<Note>>` — THE core function for song mode
- `arrangement_length() -> u32` — end tick of last placement
- `remove_instrument_data(instrument_id)` — cleanup on instrument delete
- `recalculate_next_ids()` — used after DB load

**`flatten_to_notes()` algorithm:**
For each placement, look up its clip. For each note in the clip that falls within the effective length (respecting `length_override`), emit a new Note with `tick = note.tick + placement.start_tick`. Clamp duration so it doesn't extend past clip boundary. Sort each instrument's result by tick.

**`ClipPlacement` helpers:**
- `effective_length(&self, clip: &Clip) -> u32` — `length_override.unwrap_or(clip.length_ticks)`
- `end_tick(&self, clip: &Clip) -> u32` — `start_tick + effective_length(clip)`

---

## 2. State Integration

### `imbolc-core/src/state/session.rs`
- Add `use super::arrangement::ArrangementState;`
- Add field: `pub arrangement: ArrangementState,`
- Initialize in `new_with_defaults()`: `arrangement: ArrangementState::new(),`

### `imbolc-core/src/state/mod.rs`
- Add `pub mod arrangement;`
- Add re-exports: `pub use arrangement::{ArrangementState, Clip, ClipId, ClipPlacement, PlayMode, PlacementId};`
- In instrument delete handling, call `self.session.arrangement.remove_instrument_data(id);`

Since `SessionState` already derives `Clone`, and `ArrangementState` derives `Clone`, the undo snapshot system captures arrangement state automatically — no changes to the snapshot mechanism.

---

## 3. Actions

### `imbolc-core/src/action.rs`

Add imports: `use crate::state::arrangement::{ClipId, PlacementId};`

New enum:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ArrangementAction {
    TogglePlayMode,
    CreateClip { instrument_id: InstrumentId, length_ticks: u32 },
    CaptureClipFromPianoRoll { instrument_id: InstrumentId },
    DeleteClip(ClipId),
    RenameClip(ClipId, String),
    PlaceClip { clip_id: ClipId, instrument_id: InstrumentId, start_tick: u32 },
    RemovePlacement(PlacementId),
    MovePlacement { placement_id: PlacementId, new_start_tick: u32 },
    ResizePlacement { placement_id: PlacementId, new_length: Option<u32> },
    DuplicatePlacement(PlacementId),
    SelectPlacement(Option<usize>),
    SelectLane(usize),
    MoveCursor(i32),       // delta in columns
    ScrollView(i32),       // delta in scroll units
    ZoomIn,
    ZoomOut,
    EnterClipEdit(ClipId),
    ExitClipEdit,
    PlayStop,
}
```

Add to `Action` enum: `Arrangement(ArrangementAction),`

---

## 4. Dispatch Handler

### New file: `imbolc-core/src/dispatch/arrangement.rs`

```rust
pub(super) fn dispatch_arrangement(
    action: &ArrangementAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult
```

Key handler behaviors:

| Action | State mutation | Dirty flags | Nav |
|--------|---------------|-------------|-----|
| `TogglePlayMode` | Toggle `play_mode` | `piano_roll=true` | — |
| `CaptureClipFromPianoRoll` | Copy notes from `piano_roll.tracks[id]` in loop region into new clip (make ticks relative to 0) | — | — |
| `CreateClip` | Add empty clip | — | — |
| `DeleteClip` | Remove clip + cascade placements | `piano_roll=true` | — |
| `PlaceClip` | Add placement | `piano_roll=true` | — |
| `RemovePlacement` | Remove placement | `piano_roll=true` | — |
| `MovePlacement` | Update start_tick | `piano_roll=true` | — |
| `ResizePlacement` | Update length_override | `piano_roll=true` | — |
| `DuplicatePlacement` | Clone placement, place at end of original | `piano_roll=true` | — |
| `EnterClipEdit` | Stash piano roll track notes + loop bounds, load clip notes into track, set loop to clip length | `piano_roll=true` | `PushTo("piano_roll")` |
| `ExitClipEdit` | Copy edited notes back to clip, restore stashed notes + loop bounds | `piano_roll=true` | `PopOrSwitchTo("track")` |
| `PlayStop` | Toggle `piano_roll.playing`, call `audio.set_playing()` / `audio.reset_playhead()` / `audio.release_all_voices()` | — | — |
| `ZoomIn` | `ticks_per_col = (ticks_per_col / 2).max(30)` | — | — |
| `ZoomOut` | `ticks_per_col = (ticks_per_col * 2).min(1920)` | — | — |
| `SelectPlacement/Lane/MoveCursor/ScrollView` | Update cursor/selection state | — | — |

### `imbolc-core/src/dispatch/mod.rs`
- Add `mod arrangement;`
- Add match arm: `Action::Arrangement(a) => arrangement::dispatch_arrangement(a, state, audio),`

---

## 5. Song Mode Audio Integration

### `imbolc-core/src/audio/handle.rs` — modify `flush_dirty()`

Replace the `if dirty.piano_roll` block:

```rust
if dirty.piano_roll {
    if state.session.arrangement.play_mode == PlayMode::Song
        && state.session.arrangement.editing_clip.is_none()
    {
        // Flatten arrangement into temporary PianoRollState
        let mut flat_pr = state.session.piano_roll.clone();
        let flattened = state.session.arrangement.flatten_to_notes();
        for (&instrument_id, track) in &mut flat_pr.tracks {
            track.notes = flattened.get(&instrument_id).cloned().unwrap_or_default();
        }
        let arr_len = state.session.arrangement.arrangement_length();
        if arr_len > 0 {
            flat_pr.loop_end = arr_len;
            flat_pr.looping = false;  // Linear playback
        }
        self.update_piano_roll_data(&flat_pr);
    } else {
        self.update_piano_roll_data(&state.session.piano_roll);
    }
}
```

Add import: `use crate::state::arrangement::PlayMode;`

**This is the entire audio-side change.** The audio thread's `tick_playback()` sees notes at different absolute positions and plays them. No changes to `audio_thread.rs`, `playback.rs`, or the SuperCollider engine.

---

## 6. TrackPane Rewrite

### `src/panes/track_pane.rs` — full rewrite (currently 253 lines, read-only stub)

**Struct:**
```rust
pub struct TrackPane {
    keymap: Keymap,
    scroll_offset: usize,
}
```

**Layout** (97x29 centered, same as current):
```
+--------------------------------------- Track ----------------------------------------+
| [Song] |  1       2       3       4       5       6       7       8     bars          |
|--------|------------------------------------------------------------------------- ... |
| > 1 Saw|  [Clip A      ] [Clip A      ]        [Clip B     ]                         |
|   2 Sin|                    [Clip C   ]  [Clip C   ]                                  |
|   3 Kit|  [Pattern 1]  [Pattern 1]  [Pattern 2]                                      |
|--------|-----------------------------------------------------------------------------|
| n:new  p:place  Enter:edit  d:del  m:mode  Space:play     Cursor: Bar 1 Beat 1       |
+---------------------------------------------------------------------------------------+
```

- Left column (20 chars): instrument number, name, source type (keep existing color logic)
- Separator `|`
- Timeline area: clip blocks rendered as `[ClipName    ]` with source_color fill
- Selected placement highlighted with `Color::SELECTION_BG`
- Bar lines every `ticks_per_bar / ticks_per_col` columns
- Playhead as vertical `│` line at `(playhead - view_start_tick) / ticks_per_col`
- Cursor as `▏` or highlighted column
- Header: bar numbers, mode indicator `[Song]`/`[Pattern]`
- Footer: key hints, cursor position as Bar.Beat

**Keybindings** — add `[layers.track]` to `keybindings.toml`:

| Key | Action | Description |
|-----|--------|-------------|
| Up/Down | `lane_up`/`lane_down` | Select instrument lane |
| Left/Right | `cursor_left`/`cursor_right` | Move cursor |
| Home/End | `cursor_home`/`cursor_end` | Jump to start/end |
| `n` | `new_clip` | Capture clip from piano roll loop region |
| `N` | `new_empty_clip` | Create empty 1-bar clip |
| `p` | `place_clip` | Place most recent clip at cursor |
| Enter | `edit_clip` | Edit clip under cursor in piano roll |
| `d` | `delete` | Delete selected placement |
| `D` | `delete_clip` | Delete clip and all its placements |
| `y` | `duplicate` | Duplicate placement (after original) |
| `m` | `toggle_mode` | Toggle Song/Pattern mode |
| Space | `play_stop` | Play/Stop |
| Shift+Left/Right | `move_left`/`move_right` | Move selected placement |
| `z`/`x` | `zoom_in`/`zoom_out` | Zoom timeline |
| Tab/Shift+Tab | `select_next_placement`/`select_prev_placement` | Cycle placements |
| `[`/`]` | `select_prev_clip`/`select_next_clip` | Cycle available clips |

**`handle_action()`** returns `Action::Arrangement(...)` variants. Get current `instrument_id` from `state.instruments.instruments[arr.selected_lane].id`. Use `arr.cursor_tick` for tick positions. For `edit_clip`, hit-test with `arr.placement_at(instrument_id, cursor_tick)`.

---

## 7. Piano Roll Integration

### Clip editing workflow:

1. User selects clip in TrackPane, presses Enter
2. `ArrangementAction::EnterClipEdit(clip_id)` dispatched:
   - Stash current piano roll track notes for that instrument
   - Stash loop_start, loop_end, looping
   - Load clip.notes into piano roll track
   - Set loop_start=0, loop_end=clip.length_ticks, looping=true, playhead=0
   - Store `ClipEditContext` in `arrangement.editing_clip`
   - Return `NavIntent::PushTo("piano_roll")`
3. User edits notes normally in piano roll
4. When user navigates away (F3, any nav key), auto-exit triggers:
   - Copy piano roll track notes back to clip
   - Update clip.length_ticks from loop_end (in case user changed it)
   - Restore stashed notes and loop bounds
   - Clear `editing_clip`
   - Re-flatten if in song mode

### `src/main.rs` — auto-exit clip edit on navigation

After `panes.process_nav()` (around line 206), add:
```rust
if matches!(&pane_action, Action::Nav(_)) {
    if state.session.arrangement.editing_clip.is_some()
        && panes.active().id() != "piano_roll"
    {
        let exit_result = dispatch::dispatch_action(
            &Action::Arrangement(ArrangementAction::ExitClipEdit),
            &mut state, &mut audio, &io_tx,
        );
        pending_audio_dirty.merge(exit_result.audio_dirty);
        apply_dispatch_result(exit_result, &mut state, &mut panes, &mut app_frame);
    }
}
```

### Piano roll title indicator

In `src/panes/piano_roll_pane/rendering.rs`, where the block title `" Piano Roll "` is set:
```rust
let title = if let Some(ref ctx) = state.session.arrangement.editing_clip {
    let name = state.session.arrangement.clip(ctx.clip_id)
        .map(|c| c.name.as_str()).unwrap_or("?");
    format!(" Piano Roll - Editing: {} ", name)
} else {
    " Piano Roll ".to_string()
};
```

---

## 8. Persistence

### Schema — `imbolc-core/src/state/persistence/schema.rs`

4 new tables:
```sql
CREATE TABLE IF NOT EXISTS arrangement_clips (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    instrument_id INTEGER NOT NULL,
    length_ticks INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS arrangement_clip_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    pitch INTEGER NOT NULL,
    velocity INTEGER NOT NULL,
    probability REAL NOT NULL DEFAULT 1.0,
    FOREIGN KEY (clip_id) REFERENCES arrangement_clips(id)
);

CREATE TABLE IF NOT EXISTS arrangement_placements (
    id INTEGER PRIMARY KEY,
    clip_id INTEGER NOT NULL,
    instrument_id INTEGER NOT NULL,
    start_tick INTEGER NOT NULL,
    length_override INTEGER,
    FOREIGN KEY (clip_id) REFERENCES arrangement_clips(id)
);

CREATE TABLE IF NOT EXISTS arrangement_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    play_mode TEXT NOT NULL DEFAULT 'pattern',
    view_start_tick INTEGER NOT NULL DEFAULT 0,
    ticks_per_col INTEGER NOT NULL DEFAULT 120,
    cursor_tick INTEGER NOT NULL DEFAULT 0,
    selected_lane INTEGER NOT NULL DEFAULT 0,
    selected_placement INTEGER
);
```

Add DELETE FROM statements. Bump schema version.

### Save — new file `imbolc-core/src/state/persistence/save/arrangement.rs`
- `save_arrangement(conn, session) -> SqlResult<()>`
- INSERT clips, clip_notes, placements, settings
- Wire into `save/mod.rs` and `persistence/mod.rs`

### Load — new file `imbolc-core/src/state/persistence/load/arrangement.rs`
- `load_arrangement(conn) -> SqlResult<ArrangementState>`
- Query all 4 tables, reconstruct state, call `recalculate_next_ids()`
- Wire into `load/mod.rs` and `persistence/mod.rs`
- In `load_project()`, assign `session.arrangement = arrangement;`

---

## 9. Undo Integration

### `imbolc-core/src/state/undo.rs` — add to `is_undoable()`

```rust
Action::Arrangement(a) => match a {
    ArrangementAction::TogglePlayMode
    | ArrangementAction::SelectPlacement(_)
    | ArrangementAction::SelectLane(_)
    | ArrangementAction::MoveCursor(_)
    | ArrangementAction::ScrollView(_)
    | ArrangementAction::PlayStop => false,
    _ => true,  // All clip/placement mutations are undoable
},
```

No changes to UndoSnapshot needed — arrangement state is inside SessionState which is already cloned.

---

## 10. Implementation Order

Each step must compile and pass `cargo test` before proceeding.

### Phase 1: Core State
1. Create `imbolc-core/src/state/arrangement.rs` with all types and methods
2. Add `pub mod arrangement` to `state/mod.rs`, add re-exports
3. Add `pub arrangement: ArrangementState` to `SessionState`, initialize in constructors
4. Add `remove_instrument_data()` call in instrument delete path
5. Write unit tests for: add/remove clip, add/remove placement, flatten, cascade delete, arrangement_length, placement_at
6. `cargo test` — verify all existing tests pass

### Phase 2: Actions & Dispatch
7. Add `ArrangementAction` enum and `Arrangement(ArrangementAction)` variant to `action.rs`
8. Create `imbolc-core/src/dispatch/arrangement.rs` — all handlers
9. Add `mod arrangement` and match arm in `dispatch/mod.rs`
10. Add `Arrangement` match arm to `is_undoable()` in `undo.rs`
11. `cargo test`

### Phase 3: Audio Integration
12. Modify `flush_dirty()` in `handle.rs` — song mode flattening branch
13. Manual test: toggle song mode, verify audio thread receives flattened data

### Phase 4: TrackPane
14. Rewrite `src/panes/track_pane.rs` — rendering with clip blocks, playhead, cursor
15. Add `[layers.track]` keybindings to `keybindings.toml`
16. Implement `handle_action()` returning `ArrangementAction` variants
17. Add EnterClipEdit/ExitClipEdit dispatch (with NavIntent)
18. Add auto-exit clip edit in `main.rs`
19. Add clip editing title indicator in piano roll rendering

### Phase 5: Persistence
20. Add 4 tables to `schema.rs`, bump version
21. Create `save/arrangement.rs` and `load/arrangement.rs`
22. Wire into `save/mod.rs`, `load/mod.rs`, `persistence/mod.rs`
23. Add persistence round-trip test

### Phase 6: Post-implementation
24. Add note about per-clip automation to `plans/remaining-features.md`

---

## 11. File Change Summary

### New files (4)
| File | Purpose |
|------|---------|
| `imbolc-core/src/state/arrangement.rs` | All arrangement state types, flatten logic, unit tests |
| `imbolc-core/src/dispatch/arrangement.rs` | ArrangementAction dispatch handler |
| `imbolc-core/src/state/persistence/save/arrangement.rs` | SQLite save |
| `imbolc-core/src/state/persistence/load/arrangement.rs` | SQLite load |

### Modified files (12)
| File | Change |
|------|--------|
| `imbolc-core/src/state/mod.rs` | Add `pub mod arrangement`, re-exports, instrument delete cleanup |
| `imbolc-core/src/state/session.rs` | Add `pub arrangement: ArrangementState` field + init |
| `imbolc-core/src/action.rs` | Add `ArrangementAction` enum and `Arrangement` variant |
| `imbolc-core/src/dispatch/mod.rs` | Add `mod arrangement` and match arm |
| `imbolc-core/src/audio/handle.rs` | Song mode flatten branch in `flush_dirty()` |
| `imbolc-core/src/state/undo.rs` | Add `Arrangement` match in `is_undoable()` |
| `imbolc-core/src/state/persistence/schema.rs` | Add 4 tables, bump version |
| `imbolc-core/src/state/persistence/mod.rs` | Wire save/load calls |
| `imbolc-core/src/state/persistence/save/mod.rs` | Add `mod arrangement` |
| `imbolc-core/src/state/persistence/load/mod.rs` | Add `mod arrangement` |
| `src/panes/track_pane.rs` | Full rewrite to interactive arrangement editor |
| `src/main.rs` | Auto-exit clip edit on navigation |
| `src/panes/piano_roll_pane/rendering.rs` | Clip editing title indicator |
| `keybindings.toml` | Add `[layers.track]` section |

---

## 12. Verification

- `cargo build` succeeds
- `cargo test --bin imbolc` passes (unit tests)
- `cargo test` passes (all tests including e2e)
- Launch app, F3 shows TrackPane with no clips
- Create instrument, add notes in piano roll, `n` in TrackPane captures clip
- `p` places clip at cursor, visible as colored block
- Enter opens clip in piano roll with `[Editing: ClipName]` title
- Navigate away restores original piano roll notes
- `m` toggles Song mode, Space plays through arrangement
- Ctrl+S saves, Ctrl+L reloads — arrangement persists
- Ctrl+Z undoes clip placement
- Deleting an instrument removes its clips and placements
