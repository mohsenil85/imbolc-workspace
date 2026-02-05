# RenderBuf Migration — Remaining 6 Panes

## Context

We are in Phase 3 of the Rendering Abstraction Layer plan (see `~/.claude/plans/cozy-exploring-micali.md`). The `RenderBuf` struct wraps ratatui's `Buffer` and provides four convenience methods that accept our own `Style`/`Color` types natively, eliminating `ratatui::style::Style::from(...)` conversions and `Paragraph::new(Line::from(Span::styled(...)))` chains.

**Already migrated (17 panes):** confirm_pane, help_pane, logo_pane, home_pane, midi_settings_pane, save_as_pane, project_browser_pane, add_effect_pane, instrument_pane, vst_param_pane (mod.rs + rendering.rs), command_palette_pane, frame_edit_pane, sample_chopper_pane, sequencer_pane, add_pane, server_pane, file_browser_pane, track_pane.

**Remaining (6 panes, 9 files):**
1. `eq_pane.rs`
2. `waveform_pane.rs`
3. `mixer_pane.rs`
4. `automation_pane/` (mod.rs + rendering.rs)
5. `instrument_edit_pane/` (mod.rs + rendering.rs)
6. `piano_roll_pane/` (mod.rs + rendering.rs)

---

## RenderBuf API Reference

File: `src/ui/render.rs`

```rust
pub struct RenderBuf<'a> { buf: &'a mut Buffer }

impl<'a> RenderBuf<'a> {
    pub fn new(buf: &'a mut Buffer) -> Self;

    /// Set single char at (x, y). Clips out-of-bounds silently.
    pub fn set_cell(&mut self, x: u16, y: u16, ch: char, style: Style);

    /// Draw string at (x, y), no wrapping. Clips silently.
    pub fn draw_str(&mut self, x: u16, y: u16, text: &str, style: Style);

    /// Bordered block with title. Returns inner Rect.
    pub fn draw_block(&mut self, area: Rect, title: &str, border_style: Style, title_style: Style) -> Rect;

    /// Styled spans on one line. Replaces Paragraph::new(Line::from(Span::styled(...))).
    pub fn draw_line(&mut self, area: Rect, spans: &[(&str, Style)]);

    /// Escape hatch for raw ratatui Buffer access.
    pub fn raw_buf(&mut self) -> &mut Buffer;
}
```

`Style` and `Color` are the project's own types from `src/ui/style.rs`. They implement `From<Style> for ratatui::style::Style`, which means you can pass our `Style` to ratatui's `Cell::set_style()` without an explicit import — the `Into` conversion is implicit.

---

## Migration Recipe (applies to every pane)

For each pane file, follow these mechanical steps:

### Step 1: Remove ratatui imports

Delete these lines (whichever are present):
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

### Step 2: Remove the escape hatch

In the `Pane::render()` method, delete:
```rust
let buf = buf.raw_buf();
```
Now `buf` is `&mut RenderBuf` instead of `&mut Buffer`.

### Step 3: Change helper function signatures

Any helper functions that take `&mut Buffer` (or `&mut ratatui::buffer::Buffer`) must change to `&mut RenderBuf`. Update both the signature and all call sites. Add the import:
```rust
use crate::ui::RenderBuf;
```
to any `rendering.rs` file that needs it (the `mod.rs` files already have it via `use crate::ui::*`).

For rendering.rs files, also add:
```rust
use crate::ui::{Rect, RenderBuf, Color, Style};
```
(replacing whatever `use ratatui::...` imports were there)

### Step 4: Replace Block patterns

**Before:**
```rust
let block = Block::default()
    .borders(Borders::ALL)
    .title(title)
    .border_style(ratatui::style::Style::from(Style::new().fg(Color::CYAN)))
    .title_style(ratatui::style::Style::from(Style::new().fg(Color::CYAN)));
let inner = block.inner(rect);
block.render(rect, buf);
```

**After:**
```rust
let border_style = Style::new().fg(Color::CYAN);
let inner = buf.draw_block(rect, title, border_style, border_style);
```

### Step 5: Replace Paragraph patterns

**Before:**
```rust
Paragraph::new(Line::from(Span::styled(text, ratatui::style::Style::from(style))))
    .render(area, buf);
```

**After:**
```rust
buf.draw_line(area, &[(text, style)]);
```

For multi-span lines:
```rust
// Before:
Paragraph::new(Line::from(vec![
    Span::styled(a, style_a),
    Span::styled(b, style_b),
])).render(area, buf);

// After:
buf.draw_line(area, &[(a, style_a), (b, style_b)]);
```

### Step 6: Replace cell_mut patterns

**Before:**
```rust
if let Some(cell) = buf.cell_mut((x, y)) {
    cell.set_char(ch).set_style(ratatui::style::Style::from(style));
}
```

**After:**
```rust
buf.set_cell(x, y, ch, style);
```

### Step 7: Replace buf[(x, y)] index patterns (eq_pane only)

**Before:**
```rust
buf[(x, y)].set_char(ch).set_style(ratatui::style::Style::from(style));
```

**After:**
```rust
buf.set_cell(x, y, ch, style);
```

Note: `buf[(x,y)]` panics on out-of-bounds. `set_cell` silently clips.

### Step 8: Remove all `ratatui::style::Style::from(...)` wrappers

Anywhere you see:
```rust
ratatui::style::Style::from(Style::new().fg(...).bg(...))
```
Just use:
```rust
Style::new().fg(...).bg(...)
```
The `RenderBuf` methods accept our `Style` directly.

### Step 9: Handle read-back patterns (cell.symbol() checks)

If code reads a cell's content before writing (e.g., `if cell.symbol() == " "`), use `buf.raw_buf()` inline:
```rust
if let Some(cell) = buf.raw_buf().cell_mut((x, y)) {
    if cell.symbol() == " " {
        cell.set_char('|').set_style(cursor_style);  // Into conversion is implicit
    }
}
```
No ratatui import needed — `Cell::set_style()` accepts `impl Into<ratatui::style::Style>` and our `Style` implements `Into`.

### Step 10: Handle TextInput.render_buf() calls

`TextInput::render_buf()` takes a raw `&mut Buffer`. Use inline `raw_buf()`:
```rust
// Before:
edit_input.render_buf(buf, x, y, width);

// After:
edit_input.render_buf(buf.raw_buf(), x, y, width);
```

### Step 11: Build and test

```bash
cargo build
```
Then manually launch the app and navigate to the pane to verify rendering is identical.

---

## Per-Pane Details

### 1. eq_pane.rs (~420 lines)

**Difficulty: Medium** — uses `buf[(x,y)]` index notation (unique to this file).

**Ratatui imports to remove (lines 3-5):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};
```

**Escape hatch to remove:** Line ~89: `let buf = buf.raw_buf();`

**Helper functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_centered_text(area: Rect, buf: &mut Buffer, text: &str, color: Color)` (~line 179)
- `render_frequency_curve(x, y, width, height, eq, selected_band, buf: &mut Buffer)` (~line 278)
- `render_band_info(x, y, width, eq, selected_band, selected_param, buf: &mut Buffer)` (~line 347)
- `render_text_at(x, y, text, style, max_width, buf: &mut Buffer)` (~line 405)

**Patterns to replace:**
- 1 Block pattern → `draw_block()`
- 12 `ratatui::style::Style::from(...)` → remove wrapper
- 6 `buf[(x,y)].set_char(ch).set_style(...)` → `buf.set_cell(x, y, ch, style)` — **IMPORTANT: this file uses direct index `buf[(x,y)]` instead of `cell_mut`. Convert all to `set_cell`.**
- 0 cell.symbol() read-back patterns
- 0 TextInput usage

**Note:** The helper functions are standalone `fn` (not methods on Self). Their signatures need `buf: &mut RenderBuf` and they need `use crate::ui::RenderBuf;` at the top of the file. Since this file is a single file (not mod+rendering), the import likely comes from the existing `use crate::ui::{...}` line — just add `RenderBuf` there if not already present.

---

### 2. waveform_pane.rs (~460 lines)

**Difficulty: Medium** — many helper functions with `&mut Buffer` signatures.

**Ratatui imports to remove (lines ~3-5):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

**Escape hatch to remove:** Line ~450: `let buf = buf.raw_buf();`

**Helper functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_waveform(&self, area: Rect, buf: &mut Buffer, state: &AppState)` (~line 90)
- `render_spectrum(&self, area: Rect, buf: &mut Buffer, state: &AppState)` (~line 172)
- `render_oscilloscope(&self, area: Rect, buf: &mut Buffer, state: &AppState)` (~line 238)
- `render_lufs_meter(&self, area: Rect, buf: &mut Buffer, state: &AppState)` (~line 310)
- `render_single_meter(x, y, width, height, peak, rms, label: &str, buf: &mut Buffer)` (~line 347)
- `render_border(&self, rect: Rect, buf: &mut Buffer, title: &str, color: Color)` (~line 410)
- `render_header(&self, rect: Rect, buf: &mut Buffer, state: &AppState, mode_name: &str)` (~line 419)

**Patterns to replace:**
- 1 Block pattern (in `render_border`) → `draw_block()`
- 20 `ratatui::style::Style::from(...)` → remove wrapper
- 11 Paragraph chains → `draw_line()`
- 9 `cell_mut` calls → `set_cell()`
- 0 cell.symbol() read-back patterns
- 0 TextInput usage

---

### 3. mixer_pane.rs (~1240 lines)

**Difficulty: Hard** — largest file, 6 helper functions, some are free functions (not methods).

**Ratatui imports to remove (lines ~3-5):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

**Escape hatch to remove:** Line ~408: `let buf = buf.raw_buf();`

**Helper functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_mixer_buf(&self, buf: &mut Buffer, area: Rect, state: &AppState)` (~line 627) — method on Self
- `render_detail_buf(&self, buf: &mut Buffer, area: Rect, state: &AppState)` (~line 764) — method on Self
- `write_str(buf: &mut Buffer, x, y, text, style: ratatui::style::Style)` (~line 1059) — **free function, takes `ratatui::style::Style` directly! Change to our `Style`.**
- `render_channel_buf(buf: &mut Buffer, x, ...)` (~line 1068) — free function
- `render_empty_channel_buf(buf: &mut Buffer, x, ...)` (~line 1171) — free function
- `render_meter_buf(buf: &mut Buffer, x, ...)` (~line 1214) — free function

**Patterns to replace:**
- 2 Block patterns → `draw_block()`
- 34 `ratatui::style::Style::from(...)` → remove wrapper
- 3 Paragraph chains → `draw_line()`
- 17 `cell_mut` calls → `set_cell()`
- 0 cell.symbol() read-back patterns
- 0 TextInput usage

**Special attention:**
- The `write_str` helper takes `ratatui::style::Style` as a parameter. Change it to our `Style`. Then convert all call sites from `ratatui::style::Style::from(Style::new()...)` to just `Style::new()...`.
- The free functions (`write_str`, `render_channel_buf`, `render_empty_channel_buf`, `render_meter_buf`) need `use crate::ui::RenderBuf;` since they're outside the impl block. Add it to the file's imports.

---

### 4. automation_pane/ (mod.rs ~165 lines + rendering.rs ~363 lines)

**Difficulty: Medium** — two files, one read-back pattern.

#### automation_pane/mod.rs

**Ratatui imports to remove (~line 6):**
```rust
use ratatui::widgets::{Block, Borders, Widget};
```

**Escape hatch to remove:** Line ~109: `let buf = buf.raw_buf();`

**Patterns in mod.rs to replace:**
- 1 Block pattern → `draw_block()`
- 4 `ratatui::style::Style::from(...)` → remove wrapper
- 2 `cell_mut` calls → `set_cell()`

**After removing escape hatch, pass `buf: &mut RenderBuf` to the rendering.rs helpers:**
```rust
// Before:
self.render_lane_list(buf, lane_area, state);
self.render_timeline(buf, timeline_area, state);

// After (same — buf is now &mut RenderBuf automatically):
self.render_lane_list(buf, lane_area, state);
self.render_timeline(buf, timeline_area, state);
```

#### automation_pane/rendering.rs

**Ratatui imports to remove (lines 1-3):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

**Add imports:**
```rust
use crate::ui::{Rect, RenderBuf, Color, Style};
```

**Helper functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_lane_list(&self, buf: &mut Buffer, area: Rect, state: &AppState)` (~line 20)
- `render_timeline(&self, buf: &mut Buffer, area: Rect, state: &AppState)` (~line 130)
- `render_target_picker(&self, buf: &mut Buffer, area: Rect)` (~line 304)

**Patterns to replace:**
- 1 Block pattern (in render_target_picker) → `draw_block()`
- 23 `ratatui::style::Style::from(...)` → remove wrapper
- 2 Paragraph chains → `draw_line()`
- 12 `cell_mut` calls → `set_cell()`

**Read-back pattern (~line 233):**
```rust
} else if cell.symbol() == " " {
    cell.set_char('│').set_style(...);
}
```
Use inline `buf.raw_buf().cell_mut((x, y))` for this block. The surrounding code already has a `cell_mut` call with a conditional — convert the whole `if let Some(cell) = buf.cell_mut(...)` block to use `buf.raw_buf().cell_mut(...)`. Our `Style` implements `Into<ratatui::style::Style>` so `cell.set_style(style)` works without explicit conversion.

---

### 5. instrument_edit_pane/ (mod.rs ~260 lines + rendering.rs ~493 lines)

**Difficulty: Hard** — highest Style::from count (42), TextInput usage, 3 free helper functions.

#### instrument_edit_pane/mod.rs

**Ratatui imports to remove:** None (this file has no direct ratatui imports).

**Escape hatch to remove:** Line ~239: `let buf = buf.raw_buf();`

After removing the escape hatch, the call to `self.render_impl(area, buf, state)` passes `&mut RenderBuf` directly.

#### instrument_edit_pane/rendering.rs

**Ratatui imports to remove (lines ~1-3):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

**Add imports:**
```rust
use crate::ui::{Rect, RenderBuf, Color, Style};
```

**Functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_impl(&mut self, area: Rect, buf: &mut Buffer, state: &AppState)` (~line 12) — method on Self
- `render_label_value_row_buf(buf: &mut Buffer, x, y, label, value, color, is_sel)` — free function
- `render_param_row_buf(buf: &mut Buffer, x, y, param, is_sel, editing, edit_input: &mut TextInput)` — free function
- `render_value_row_buf(buf: &mut Buffer, x, y, label, value, min, max, is_sel, editing, edit_input: &mut TextInput)` — free function

**Patterns to replace:**
- 1 Block pattern → `draw_block()`
- 42 `ratatui::style::Style::from(...)` → remove wrapper (highest count of all files!)
- 15 Paragraph chains → `draw_line()`
- 12 `cell_mut` calls → `set_cell()`

**TextInput.render_buf() calls (lines ~380, ~447):**
```rust
// Before:
edit_input.render_buf(buf, x + 34, y, 10);

// After:
edit_input.render_buf(buf.raw_buf(), x + 34, y, 10);
```

---

### 6. piano_roll_pane/ (mod.rs ~235 lines + rendering.rs ~436 lines)

**Difficulty: Medium-Hard** — heavy cell_mut usage for the note grid.

#### piano_roll_pane/mod.rs

**Ratatui imports to remove:** None (no direct ratatui imports).

**Escape hatch to remove:** Line ~192: `let buf = buf.raw_buf();`

After removing escape hatch, the calls to `self.render_notes_buf(buf, ...)` and `self.render_automation_overlay(buf, ...)` pass `&mut RenderBuf` directly.

#### piano_roll_pane/rendering.rs

**Ratatui imports to remove (lines ~1-3):**
```rust
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
```

**Add imports:**
```rust
use crate::ui::{Rect, RenderBuf, Color, Style};
```

**Functions to change from `&mut Buffer` to `&mut RenderBuf`:**
- `render_automation_overlay(&self, buf: &mut Buffer, overlay_area: Rect, grid_x: u16, grid_width: u16, state: &AppState)` (~line 32)
- `render_notes_buf(&self, buf: &mut Buffer, area: Rect, state: &AppState)` (~line 147)

**Patterns to replace:**
- 1 Block pattern → `draw_block()`
- 31 `ratatui::style::Style::from(...)` → remove wrapper
- 6 Paragraph chains → `draw_line()`
- 12 `cell_mut` calls → `set_cell()`
- 0 cell.symbol() read-back patterns
- 0 TextInput usage

---

## Suggested Order

1. **eq_pane.rs** — single file, medium complexity, unique `buf[(x,y)]` pattern
2. **waveform_pane.rs** — single file, many helpers but straightforward
3. **automation_pane/** — two files, has one read-back pattern to handle carefully
4. **piano_roll_pane/** — two files, heavy cell_mut but no special patterns
5. **instrument_edit_pane/** — two files, highest Style::from count + TextInput
6. **mixer_pane.rs** — single file but 1240 lines, 6 helpers including free functions

Build after each pane: `cargo build`

---

## Totals

Across all 6 remaining panes:
- **166** `ratatui::style::Style::from(...)` wrappers to remove
- **37** Paragraph chains to replace with `draw_line()`
- **7** Block patterns to replace with `draw_block()`
- **64** `cell_mut` / `buf[(x,y)]` calls to replace with `set_cell()`
- **1** cell.symbol() read-back pattern (automation_pane/rendering.rs)
- **2** TextInput.render_buf() calls needing `raw_buf()` (instrument_edit_pane/rendering.rs)

## After All Migrations

Once all 6 panes are done, verify the entire build is clean:
```bash
cargo build
cargo test --bin imbolc
```

Then confirm no ratatui imports remain in any pane file:
```bash
grep -r "use ratatui" src/panes/
```
The only ratatui usage should be in `src/ui/render.rs`, `src/ui/ratatui_impl.rs`, `src/ui/style.rs`, and `src/ui/widgets/`.
