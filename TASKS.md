# Tasks

All remaining work, organized by priority tier. Consolidated from
NEXT_STEPS.md, REFACTOR1.md, REFACTOR2.md, FEATURES.md, SAMPLER.md,
and UNWIRED.md.

See TASKS_DONE.md for completed work.

---

## Bugs

### Broken Frame settings screen

**Sources:** R2 #4

`FrameEditPane` (`src/panes/frame_edit_pane.rs`) has several issues:

1. **Escape goes to `"instrument"`** instead of returning to the
   previous pane. Should use `PopPane` or navigate back to wherever
   the user came from.
2. **Confirm behavior is inconsistent:** Enter on BPM/Tuning enters
   text edit mode, but Enter on Key/Scale/TimeSig/Snap immediately
   fires `UpdateSession` and presumably should return to the previous
   pane.
3. **No visible "save and return" flow:** Left/Right changes may not
   propagate if the user presses Escape.

**Fix:**
- Track the originating pane and return to it on Escape/confirm (use
  the pane stack)
- Make Enter always commit and return
- Make Escape discard uncommitted changes and return

**Files:** `src/panes/frame_edit_pane.rs`, `src/ui/pane.rs`

---

### Instrument deletion may be broken

**Sources:** R2 #12 (originally "strip deletion")

Pressing `d` in `InstrumentPane` dispatches `InstrumentAction::Delete`
which calls `AppState::remove_instrument()` and
`audio_engine.rebuild_instrument_routing()`.  If deletion is still
flaky, verify selection index updates and audio graph rebuild.

**Files:** `src/panes/instrument_pane.rs`, `src/dispatch.rs`,
`src/state/instrument_state.rs`, `src/state/mod.rs`,
`src/audio/engine.rs`

---

### Piano roll: remove BPM display

**Sources:** R2 #10

BPM is shown in the piano roll header (AudioIn mode and MIDI mode). It
belongs in the session/frame settings (`FrameEditPane`), not
cluttering the piano roll. Find and remove BPM display from the piano
roll header.

**Files:** `src/panes/piano_roll_pane.rs`

---

### File picker scroll wrapping

**Sources:** R2 #27

`FileBrowserPane` stops at the bottom of the list instead of wrapping
back to the top. Use modular arithmetic for the selection index:

```rust
self.selected = (self.selected + 1) % self.entries.len();           // Down wraps to top
self.selected = (self.selected + self.entries.len() - 1) % self.entries.len(); // Up wraps to bottom
```

**Files:** `src/panes/file_browser_pane.rs`

---

### `remove_instrument` doesn't clean up automation lanes

**Sources:** UNWIRED #5

`AppState::remove_instrument()` (`state/mod.rs`) removes from
instrument list and piano roll tracks but does NOT call
`self.session.automation.remove_lanes_for_instrument(id)`. Orphaned
automation lanes will accumulate.

**Files:** `src/state/mod.rs`

---

## Quick Wins

### Remove HomePane

**Sources:** R2 #26

HomePane (`src/panes/home_pane.rs`) is a 3-item menu (Rack, Mixer,
Server) that's redundant since number keys already provide direct
navigation. Uses outdated "Rack" naming.

1. Delete `src/panes/home_pane.rs` and remove from `src/panes/mod.rs`
2. Remove registration from `main.rs`
3. Change default pane on startup from `"home"` to `"strip"`
4. Remove any `SwitchPane("home")` references
5. Remove the `home` keybindings layer in `keybindings.toml`

**Files:** `src/panes/home_pane.rs` (delete), `src/panes/mod.rs`,
`src/main.rs`

---

### Remove help text along bottom

**Sources:** R2 #11

Most panes render a hardcoded help line at the bottom (e.g.,
`"Left/Right: adjust | Enter: type/confirm | Esc: cancel"`). This
clutters the UI. The `?` key already opens context-sensitive help via
`HelpPane`.

1. Remove all inline help text from pane `render()` methods
2. Optionally add a subtle `? for help` indicator in the frame chrome

**Files:** All panes in `src/panes/`, `src/ui/frame.rs`

---

### Ctrl-L to force re-render

**Sources:** R2 #18

`Ctrl-L` is standard for terminal redraw but is currently bound to
"load". Rebind:

1. Change `Ctrl-L` from "load" to "force redraw"
2. Move "load" to a different shortcut (e.g., `Ctrl-O` for "open")
3. Implement force redraw via `terminal.clear()` on the ratatui
   backend

**Files:** `src/main.rs`, `src/ui/ratatui_impl.rs`

---

### Dead code cleanup

**Sources:** UNWIRED #2-4, #6, FEATURES #7

Comprehensive cleanup of unused code:

**Unused methods** (from UNWIRED #3 — full list):

State methods: `AppState::collect_strip_updates()`,
`StripState::selected_strip_mut()`,
`StripState::strips_with_tracks()`,
`OscType::default_params_with_registry()`, `OscType::is_custom()`,
`OscType::custom_id()`, `OscType::all_with_custom()`,
`FilterType::all()`, `EffectType::all()`, `LfoShape::all()`,
`LfoTarget::all()`, `PianoRollState::find_note()`,
`PianoRollState::notes_in_range()`, `PianoRollState::beat_to_tick()`

UI framework: `Frame::inner_rect()`, `PianoKeyboard::deactivate()`,
`MixerPane::send_target()`, `StripEditPane::strip_id()`,
`PaneManager::active_keymap()`, `PaneManager::pane_ids()`,
`Keymap::bind_alt()`, `Keymap::bind_ctrl_key()`, `Style::underline()`,
`Rect::right()`, `Rect::bottom()`, `Graphics::fill_rect()`,
`TextInput::with_placeholder()`, `TextInput::with_value()`,
`TextInput::is_focused()`, `InputEvent::key()`,
`InputEvent::is_char()`, `Modifiers::none()` (test-only),
`Modifiers::ctrl()` (test-only), `Keymap::bind_ctrl()` (test-only)

Audio engine: `AudioEngine::free_sample()`,
`AudioEngine::get_sc_bufnum()`, `AudioEngine::is_buffer_loaded()`,
`ModuleId` type alias (duplicate), `OscClient::create_synth()`,
`OscClient::alloc_buffer()`, `OscClient::query_buffer()`,
`osc_time_immediate()`, `BusAllocator::get_control_bus()` (test-only),
`BusAllocator::free_module_buses()` (test-only — but should be called
on strip removal)

**Action variants never returned by any pane** (from UNWIRED #4):
`StripAction::SetParam`, `StripAction::AddEffect`,
`StripAction::RemoveEffect`, `StripAction::MoveEffect`,
`StripAction::SetFilter`, `StripAction::ToggleTrack`,
`PianoRollAction::MoveCursor`, `PianoRollAction::SetBpm`,
`PianoRollAction::Zoom`, `PianoRollAction::ScrollOctave`,
`NavAction::PushPane`

**Unused color constants:** `CORAL`, `MIDI_COLOR`, `LFO_COLOR`,
`OUTPUT_COLOR`, `AUDIO_PORT`, `CONTROL_PORT`, `GATE_PORT`

**Stale `#[allow(dead_code)]` annotations** (from UNWIRED #6):
`FrameEditPane::set_settings()`, `AudioEngine::load_sample()`,
`AudioEngine::next_bufnum` — these are actually called/used, remove
the unnecessary annotations.

**Partially unused modules** (from UNWIRED #2):
- `state/sampler.rs`: `SampleRegistry` and `SampleBuffer` unused
  outside tests
- `state/music.rs`: `snap_freq_to_scale()` has zero callers
- `state/custom_synthdef.rs`: `by_name()`, `remove()`, `is_empty()`,
  `len()` unused

**Other:** `MAX_STEPS` constant in `state/drum_sequencer.rs` is
defined but the pattern cycling code uses a hardcoded `[8, 16, 32,
64]` array.

---

## Refactors

### Split `dispatch.rs` by domain

`src/dispatch.rs` is ~1.5k LOC and hard to navigate. Break it into
domain modules (`dispatch/instrument.rs`, `dispatch/mixer.rs`,
`dispatch/piano_roll.rs`, etc.)  and keep a small
`dispatch::dispatch_action()` router in `dispatch/mod.rs`.

**Files:** `src/dispatch.rs` (split into `src/dispatch/` modules)

---

## Infrastructure

### CLI argument parsing

**Sources:** R2 #1

No CLI argument handling. `main()` calls `run()` immediately. Add
`clap`:

```
imbolc                    # launch with default session
imbolc <file.imbolc>      # open specific session file
imbolc --new              # start fresh (no auto-load)
imbolc --help             # usage info
imbolc --version          # version string
```

**Files:** `Cargo.toml` (add `clap`), `src/main.rs` or new
`src/cli.rs`

---

### Database migrations

**Sources:** R2 #3

SQLite persistence uses `CREATE TABLE IF NOT EXISTS` and writes
`schema_version`, but does not check or migrate on load. Schema
changes break old files.

1. Add a `schema_version` table (single row, integer version)
2. On load, check version and run migrations sequentially
3. Each migration is a function: `fn migrate_v1_to_v2(conn:
   &Connection)`
4. Keep migrations in `src/state/migrations.rs` or inline in
   `persistence.rs`

Important to do before any persistence format changes.

**Files:** `src/state/persistence.rs`, new `src/state/migrations.rs`

---

### Logging interface

**Sources:** R2 #21

No structured logging. Debug output goes through
`Frame::push_message()` or `eprintln!` (lost in TUI).

1. Add `log` + `env_logger` (or `tracing`)
2. File-based logging to `~/.config/imbolc/imbolc.log`
3. Replace `eprintln!` with `log::error!`, `log::warn!`, etc.
4. Keep `Frame::push_message()` for user-visible messages
5. Default to `warn`, `debug` with `--verbose` flag
6. Log key events: OSC messages, voice allocation, file operations,
   errors

**Files:** `Cargo.toml`, `src/main.rs`, throughout codebase

---

### Refactor main.rs

**Sources:** FEATURES #6

main.rs is getting large. Candidates for extraction: pane rendering,
app setup. (Dispatch and playback tick logic already extracted to
`dispatch.rs` and `playback.rs`.)

**Files:** `src/main.rs`

---

## Polish

### Keybinding consistency

**Sources:** R2 #7 (partially: FEATURES #1 number key remap is done —
see TASKS_DONE)

Keybindings are inconsistent across panes. Common actions should use
the same keys everywhere:

1. Audit all pane keymaps and document current bindings in a matrix
2. Establish a convention: `a` = add, `d` = delete, `e`/`Enter` =
   edit, `j`/`k` or Up/Down = navigate, `/` = piano keyboard, `?` =
   help, `Escape` = back
3. Add missing shortcuts where they make sense
4. Update `docs/keybindings.md`

**Files:** All panes in `src/panes/`, `docs/keybindings.md`

---

### Insert mode touch-ups

**Sources:** R2 #24

Insert mode in piano roll and strip editor has rough edges:

1. Add a clear mode indicator in the pane header (`-- INSERT --`, `--
   PIANO --`, `-- NORMAL --`)
2. Audit mode transitions in all panes
3. Ensure consistent enter/exit behavior
4. Test edge cases: switching panes while in insert mode, resize, etc.

**Files:** `src/panes/strip_edit_pane.rs`,
`src/panes/piano_roll_pane.rs`, `src/ui/piano_keyboard.rs`

---

### Handle small terminal + resize

**Sources:** R2 #13

Fixed-size boxes (height 29) break on small terminals. Resize events
not handled.

1. Minimum size check on startup and resize — show message if too
   small
2. Handle `Event::Resize` in the main event loop
3. Clamp `box_width`/`box_height` to available terminal size
4. Graceful degradation: hide optional elements on small terminals

**Files:** `src/main.rs`, `src/ui/ratatui_impl.rs`,
`src/ui/graphics.rs`

---

### Unit tests

**Sources:** R2 #22

~41 tests exist. Priority areas for new tests:

1. `dispatch.rs` — action handler state mutations
2. `persistence.rs` — round-trip save/load
3. `AudioEngine` node calculations — StripNodes, bus allocation, voice
   stealing
4. Piano roll — note placement, deletion, quantization
5. Music theory — Key, Scale, pitch calculations
6. Keymap — binding resolution, no conflicts

**Files:** Test modules within `src/` files, `tests/` directory

---

### F1: Frame Focus Mode

**Sources:** FEATURES #2

F1 enters a mode where the user can edit frame-level values (BPM,
tuning, time signature, etc.). Modal overlay or inline editing in the
top bar.

---

### Meter Display on Frame

**Sources:** FEATURES #3

Add a level meter to the outer frame (off to the right) showing
real-time audio output level.

---

## Features

### Click track and recording countdown

**Sources:** R2 #29

**Click track:** Minimal percussive SC SynthDef (short sine burst,
fast decay). Two pitches: high for downbeat, low for other beats. Fire
during `tick_playback()` when tick crosses a beat boundary.

**Recording countdown:** When recording starts with countdown enabled,
play N countdown beats before actual recording begins. Visual
indicator: `"Recording in: 3... 2... 1..."`. Model as pre-roll
(playhead starts at `start_tick - countdown_ticks`, recording begins
at `start_tick`).

**SessionState additions:** `click_enabled: bool`, `countdown_enabled:
bool`, `countdown_beats: u8`

**Files:** `src/ui/frame.rs`, `src/panes/frame_edit_pane.rs`,
`src/playback.rs`, `src/audio/engine.rs`, `src/state/piano_roll.rs`

---



### LFO modulation targets

**Sources:** NEXT_STEPS #1, R2 #16

14 targets defined in `LfoTarget` enum but only `FilterCutoff` is
wired. For each target:

1. Add a `*_mod_in` control-rate input to the SC SynthDef
2. Wire the LFO bus to that input in
   `AudioEngine::rebuild_strip_routing()`
3. Test modulation

**Priority:** Amplitude (tremolo), Pitch (vibrato), Pan (auto-pan),
FilterResonance, PulseWidth (PWM), DelayTime/DelayFeedback, then
remaining.

**Status:** Partially done — FilterCutoff wired.

**Files:** `src/audio/engine.rs`, SuperCollider SynthDef files,
`src/state/strip.rs`

---



### Automation Recording

**Sources:** NEXT_STEPS #4, UNWIRED #2 (automation module partially
unwired)

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

### Global shortcuts + export/import

**Sources:** R2 #2, R2 #14

**Shortcuts (partially done):** `Ctrl-S` save and `Ctrl-L` load
exist. Missing: Save As (`Ctrl-Shift-S`), Open (`Ctrl-O`).

**Export/import (not started):**
- Export strip: serialize a single Strip to a portable format
- Import strip: deserialize and add with new StripId
- Export/import effect chains
- UI flow: file browser in save/load mode

**Files:** `src/main.rs`, `src/panes/file_browser_pane.rs`,
`src/state/persistence.rs`, `src/ui/pane.rs`, `src/dispatch.rs`

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

### Patch View: Tree View

**Sources:** FEATURES #5

Replace or augment the current patch view with a `tree`-style display
exploiting the chain-like nature of SC signal flow:

```
Output-3
+-- Lpf-2
    +-- SawOsc-1
        +-- Midi-0
```

---

### Better UI/input primitives

**Sources:** R2 #17

Current UI primitives are minimal (TextInput, SelectList,
Graphics). Each pane manually positions elements with absolute
coordinates.

Proposed widgets:
1. Numeric input with arrow-key increment, min/max clamping
2. Multi-line text input
3. Scrollable list widget (replace bespoke scroll logic)
4. Layout helpers (row/column auto-positioning)
5. Form widget (label + value pairs with field navigation)

**Files:** `src/ui/widgets/`, `src/ui/graphics.rs`

---



## Long-term

### Custom synths + VST support

**Sources:** NEXT_STEPS #7, R2 #9

Custom SynthDef import already exists (`state/custom_synthdef.rs`,
`scd_parser.rs`). Remaining:

See `docs/vst3-support-roadmap.md` for the current VST3 support plan
and UI goals.

**Phase 1 — Custom synthdef polish:**
- Management screen (list, rename, delete imported synthdefs)
- Show discovered parameters with ranges and defaults
- Parameter mapping in strip editor

**Phase 2 — VST support:**
- Research `vst-rs` or CLAP plugin hosting
- Requires local audio processing path alongside SC OSC path
- Large architectural change — document requirements first

**Files:** `src/state/custom_synthdef.rs`,
`src/panes/strip_edit_pane.rs`

---

### Multi-track Audio Recording

**Sources:** NEXT_STEPS #8

Record live audio input to tracks. Requires `cpal` crate for audio
capture, waveform display, overdub sync.

---

### UI themes

**Sources:** R2 #19

All colors hardcoded in `src/ui/style.rs`. Define a `Theme` struct
with semantic color slots, ship 2-3 built-in themes (Default, Light,
High Contrast), store active theme in `AppState`, add theme
switcher. Large change touching every pane.

**Files:** `src/ui/style.rs`, `src/state/mod.rs`, all panes

---

### Documentation cleanup

**Sources:** R2 #15

Partially done (CLAUDE.md and architecture docs rewritten). Remaining:
audit each file in `docs/` for accuracy, remove abandoned docs, verify
`CLAUDE.md` references.

**Files:** All files in `docs/`

---

### Stale LSP diagnostics

**Sources:** R2 #20

Dev tooling issue: cclsp sometimes reports stale diagnostics after
edits. Investigate cclsp refresh behavior, consider tree-sitter MCP,
report upstream if it's a bug.

**Files:** `.mcp.json`, `cclsp.json`
