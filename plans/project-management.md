# Project Management — Implementation Plan

## Current State

Persistence works: `save_project()` and `load_project()` read/write a SQLite file. But the workflow is locked to a single hard-coded path (`~/.config/imbolc/default.sqlite`). There is no way to manage multiple projects, no dirty-state tracking, no warning before discarding unsaved work, and no UI for project operations beyond Ctrl+S / Ctrl+L.

### What exists
- `save_project(path, session, instruments)` — serializes all state to SQLite
- `load_project(path)` — deserializes state from SQLite
- Async I/O with generation counters (prevents stale callbacks)
- `Frame::project_name` — displayed in header bar, derived from filename stem
- `SessionAction::Save` / `SessionAction::Load` — dispatch to `default_rack_path()`
- File browser pane — modal pane with extension filtering, directory traversal

### What's missing
- Project path stored in app state (currently hard-coded)
- Dirty-state flag (has the project changed since last save?)
- New/Open/Save-As/Rename actions
- Recent projects list
- Confirmation dialogs (quit with unsaved changes, load over unsaved changes)
- Project browser pane
- Auto-load last project on startup

---

## Phase 1: Project Path & Dirty State Tracking

Foundation work that everything else depends on.

### 1a. Add project path to AppState

**File: `imbolc-core/src/state/mod.rs`**

Add to `AppState`:
```rust
pub project_path: Option<PathBuf>,  // None = untitled/new project
```

Initialize to `None` in `new_with_defaults()`. Set on save/load completion.

### 1b. Add dirty flag

**File: `imbolc-core/src/state/mod.rs`**

Add to `AppState`:
```rust
pub dirty: bool,  // true if state has changed since last save/load
```

Initialize to `false`. The flag is set in two places:

1. **In `main.rs` undo-snapshot logic** — any action that passes `is_undoable()` already triggers an undo snapshot push. Set `state.dirty = true` immediately after pushing to the undo stack. This captures all meaningful state mutations without touching the dispatch layer.

2. **On save completion** — set `state.dirty = false` after a successful `IoFeedback::SaveComplete`.

3. **On load completion** — set `state.dirty = false` after a successful `IoFeedback::LoadComplete`.

4. **On new project** — set `state.dirty = false` (fresh state is clean).

### 1c. Display dirty indicator in frame header

**File: `src/ui/frame.rs`**

In `render_buf()`, append `*` to the project name when dirty:
```
IMBOLC - default* Key: C Scale: Major ...
```

When no project file exists (untitled), display `untitled*` or `untitled`.

---

## Phase 2: New Session Actions

### 2a. Extend SessionAction enum

**File: `imbolc-core/src/action.rs`**

```rust
pub enum SessionAction {
    Save,
    SaveAs(PathBuf),       // Save to a specific path
    Load,
    LoadFrom(PathBuf),     // Load a specific project file
    NewProject,            // Reset to blank state
    // ... existing variants unchanged
}
```

Mark `SaveAs`, `LoadFrom`, `NewProject` as non-undoable in `undo.rs`.

### 2b. Dispatch handlers

**File: `imbolc-core/src/dispatch/session.rs`**

**`SessionAction::SaveAs(path)`:**
- Same async pattern as `Save`, but uses the provided `path` instead of `default_rack_path()`
- On `SaveComplete`, update `state.project_path = Some(path)` and `state.dirty = false`
- Extend `IoFeedback::SaveComplete` to include the path: `SaveComplete { id, result, path }`

**`SessionAction::LoadFrom(path)`:**
- Same async pattern as `Load`, but uses the provided `path`
- On `LoadComplete`, update `state.project_path = Some(path)` and `state.dirty = false`
- Extend `IoFeedback::LoadComplete` to include the path

**`SessionAction::NewProject`:**
- Reset `state.session` to `SessionState::new()` with config defaults applied
- Reset `state.instruments` to `InstrumentState::new()`
- Set `state.project_path = None`, `state.dirty = false`
- Clear undo history (return a flag in DispatchResult, or handle in main.rs)
- Mark all audio dirty
- Switch to "add" pane (no instruments)

### 2c. Update existing Save/Load to use project_path

**`SessionAction::Save`:**
- If `state.project_path.is_some()`, save to that path
- If `state.project_path.is_none()`, either:
  - Save to `default_rack_path()` (backward-compatible), or
  - Trigger a save-as flow (open file browser / prompt for name)

Decision: Use `default_rack_path()` as fallback for now. A "save untitled" project gets saved as `default.sqlite`. This keeps Ctrl+S always working without requiring a dialog. Save-as is the explicit "choose a new location" action.

### 2d. Propagate path through IoFeedback

**File: `imbolc-core/src/action.rs`**

Update IoFeedback variants to carry the path back:
```rust
pub enum IoFeedback {
    SaveComplete { id: u64, result: Result<String, String>, path: PathBuf },
    LoadComplete { id: u64, result: Result<(SessionState, InstrumentState, String), String>, path: PathBuf },
    // ...
}
```

**File: `src/main.rs`**

On successful save/load, set `state.project_path = Some(path)`.

---

## Phase 3: Recent Projects

### 3a. Recent projects storage

**File: new `imbolc-core/src/state/recent_projects.rs`**

```rust
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: SystemTime,
}

pub struct RecentProjects {
    pub entries: Vec<RecentProject>,  // Newest first, max 20
}
```

Persistence: a simple JSON or TOML file at `~/.config/imbolc/recent.json`. SQLite is overkill here — the data is tiny and global (not per-project). Use `serde_json`.

`RecentProjects` methods:
- `load()` — read from disk, return empty list on error
- `save()` — write to disk
- `add(path, name)` — insert/promote to top, trim to max entries
- `remove(path)` — remove a specific entry (e.g., file was deleted)

### 3b. Update save/load to record recent projects

In `main.rs`, after a successful `SaveComplete` or `LoadComplete`:
1. Call `recent_projects.add(path, name)`
2. Write to disk (fire-and-forget, non-blocking)

Store `recent_projects` as a field in the run function's local scope (not in AppState — it's a UI/session concern, not core state).

---

## Phase 4: Confirmation Dialog

### 4a. Confirm pane (modal dialog)

**File: new `src/panes/confirm_pane.rs`**

A minimal modal pane that displays a message and Yes/No options:

```rust
pub struct ConfirmPane {
    message: String,
    selected: bool,           // true = Yes, false = No
    on_confirm: PendingAction, // What to do if user confirms
    keymap: Keymap,
}

pub enum PendingAction {
    Quit,
    LoadProject(PathBuf),
    NewProject,
    LoadDefault,
}
```

Behavior:
- Rendered as a centered box (using `center_rect`) with message + two options
- Left/Right or Tab to toggle selection
- Enter to confirm
- Escape to cancel (pop back)
- On confirm: return the appropriate Action and pop
- On cancel: return `Action::None` and pop

Register in `main.rs` with `panes.add_pane(Box::new(ConfirmPane::new(...)))`.

### 4b. Intercept quit when dirty

**File: `src/global_actions.rs`**

In the `"quit"` handler:
- If `state.dirty`, push the confirm pane with message "Unsaved changes. Quit anyway?" and `PendingAction::Quit`
- If `!state.dirty`, quit immediately as before

### 4c. Intercept load when dirty

When `SessionAction::Load`, `SessionAction::LoadFrom`, or `SessionAction::NewProject` is dispatched and `state.dirty` is true:
- Instead of immediately loading, push the confirm pane with "Discard unsaved changes?"
- On confirm, proceed with the original action
- On cancel, do nothing

This interception happens in `global_actions.rs` (for keybinding-triggered loads) or in the dispatch layer.

---

## Phase 5: Project Browser Pane

### 5a. Project browser UI

**File: new `src/panes/project_browser_pane.rs`**

A modal pane (pushed via pane stack) that shows:

```
╭─ Projects ──────────────────────────────╮
│                                         │
│  Recent                                 │
│  ┌─────────────────────────────────┐    │
│  │ > my-song          2 hours ago  │    │
│  │   ambient-track     yesterday   │    │
│  │   test-project      3 days ago  │    │
│  └─────────────────────────────────┘    │
│                                         │
│  [N]ew  [O]pen file  [D]elete          │
│                                         │
╰─────────────────────────────────────────╯
```

Keybindings:
- Up/Down — navigate recent projects list
- Enter — load selected project
- `n` — new project (triggers NewProject with dirty check)
- `o` — open file browser (reuse existing FileBrowserPane with a new `FileSelectAction::OpenProject`)
- `d` — remove entry from recent list (does not delete the file)
- Escape — close browser, return to previous pane

### 5b. Add FileSelectAction::OpenProject

**File: `imbolc-core/src/action.rs`**

Add variant to `FileSelectAction`:
```rust
pub enum FileSelectAction {
    OpenProject,  // new
    // ... existing variants
}
```

When a `.sqlite`/`.imbolc` file is selected in the file browser for this action, dispatch `SessionAction::LoadFrom(path)`.

### 5c. Add keybinding for project browser

**File: `keybindings.toml`**

Add to the `[layers.global]` section:
```toml
{ key = "Ctrl+o", action = "open_project_browser", description = "Project browser" }
```

Alternatively, put it under `Ctrl+p` for "project". Choose based on what doesn't conflict.

Handle `"open_project_browser"` in `global_actions.rs` — push the project browser pane.

### 5d. Wire project browser to dispatch

When the user selects a project in the browser:
1. If `state.dirty`, show confirm dialog first
2. On confirm (or if not dirty), dispatch `SessionAction::LoadFrom(path)`
3. Pop the project browser pane

When `NewProject` is selected:
1. If `state.dirty`, show confirm dialog
2. On confirm, dispatch `SessionAction::NewProject`

---

## Phase 6: Save-As Flow

### 6a. Save-as keybinding

**File: `keybindings.toml`**

```toml
{ key = "Ctrl+Shift+s", action = "save_as", description = "Save project as..." }
```

Note: Shift bindings for special keys exist but Ctrl+Shift combos may need testing in the TUI backend. Alternative: use a different key or trigger from the project browser.

### 6b. Save-as implementation

Approach: Reuse the file browser to pick a directory, then prompt for a filename. Or simpler — use a text input modal to type a project name, and save to `~/.config/imbolc/projects/{name}.sqlite`.

**Simpler approach (recommended):** Define a projects directory `~/.config/imbolc/projects/`. Save-as prompts for a name via a small text-input modal (the codebase already has text edit layer support). The file is saved as `~/.config/imbolc/projects/{name}.sqlite`.

This avoids the complexity of a full filesystem save dialog in a TUI.

**File: new text input in confirm pane or a dedicated `SaveAsPane`**

Minimal: a centered box with "Project name: [___________]" and Enter to confirm, Escape to cancel.

On confirm:
1. Construct path: `~/.config/imbolc/projects/{name}.sqlite`
2. If file exists, show confirm dialog "Overwrite existing project?"
3. Dispatch `SessionAction::SaveAs(path)`

---

## Phase 7: Auto-Load & Startup

### 7a. Auto-load last project on startup

**File: `src/main.rs`**

After creating `AppState` and before entering the event loop:
1. Load `RecentProjects` from disk
2. If the most recent entry exists on disk, dispatch `SessionAction::LoadFrom(path)` (or call load directly since the event loop hasn't started)
3. If no recent projects, load from `default_rack_path()` if it exists
4. If nothing to load, start with blank state (current behavior)

Since the async I/O pattern expects the event loop to be running, the simplest approach is to synchronously load at startup (blocking is fine before the UI appears):
```rust
let recent = RecentProjects::load();
if let Some(entry) = recent.entries.first() {
    if entry.path.exists() {
        if let Ok((session, instruments)) = load_project(&entry.path) {
            state.session = session;
            state.instruments = instruments;
            state.project_path = Some(entry.path.clone());
            app_frame.set_project_name(entry.name.clone());
        }
    }
}
```

### 7b. Config option for auto-load behavior

**File: `config.toml`**

```toml
[project]
auto_load = "last"  # "last" | "default" | "none"
```

- `"last"` — load most recent project (default)
- `"default"` — always load `default.sqlite`
- `"none"` — start with blank state

---

## Implementation Order

```
Phase 1: Project path + dirty state + header indicator
    No new UI, no new panes. Just tracking state.

Phase 2: New actions (SaveAs, LoadFrom, NewProject)
    Dispatch plumbing. Testable without UI.

Phase 3: Recent projects storage
    File I/O, no UI yet. Writes recent.json on save/load.

Phase 4: Confirmation dialog
    First new pane. Enables dirty-state warnings.
    Intercept quit + load when dirty.

Phase 5: Project browser pane
    Full project management UI. Uses recent projects list.
    Wires to LoadFrom, NewProject with dirty checks.

Phase 6: Save-as flow
    Text input for project name. Uses SaveAs action.

Phase 7: Auto-load on startup
    Reads recent projects, loads last project.
    Config option for behavior.
```

Phases 1-3 are pure infrastructure with no UI changes. Phase 4 introduces the first user-visible safety net. Phases 5-6 add the full workflow. Phase 7 is a polish item.

---

## Files Changed (Summary)

| Phase | Modified | Created |
|-------|----------|---------|
| 1 | `state/mod.rs`, `ui/frame.rs`, `main.rs` | — |
| 2 | `action.rs`, `dispatch/session.rs`, `main.rs`, `state/undo.rs` | — |
| 3 | `main.rs` | `state/recent_projects.rs` |
| 4 | `main.rs`, `global_actions.rs`, `panes/mod.rs` | `panes/confirm_pane.rs` |
| 5 | `action.rs`, `global_actions.rs`, `main.rs`, `keybindings.toml`, `panes/mod.rs` | `panes/project_browser_pane.rs` |
| 6 | `keybindings.toml`, `global_actions.rs` | `panes/save_as_pane.rs` (or extend confirm pane) |
| 7 | `main.rs`, `config.rs`, `config.toml` | — |

---

## Dependencies

- `serde_json` — for recent projects file (if not already a dependency; check `Cargo.toml`). Alternative: use the existing TOML crate.

## Open Questions

1. **Projects directory vs. arbitrary paths:** Should save-as allow saving anywhere on disk (full file browser), or only to `~/.config/imbolc/projects/`? The managed directory is simpler for TUI but less flexible.

2. **Project rename:** Renaming could be just "save-as with new name + delete old file," or a dedicated rename that moves the file. Save-as-then-delete is simpler and sufficient.

3. **Ctrl+Shift+S availability:** Need to verify the TUI backend (crossterm) correctly reports Ctrl+Shift+S as distinct from Ctrl+S. If not, use an alternative binding.

4. **Auto-save:** Out of scope for this plan, but worth noting as a future addition. Could be a timed save (every N minutes if dirty) writing to a `.autosave` file alongside the project.
