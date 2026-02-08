# Imbolc: Road to Public Alpha/Beta

## Context

Imbolc is a terminal-based DAW with 269K LOC across 5 crates, 524 passing tests, 27 panes, 51 sound sources, 37 effects, and 151 SynthDefs. Core features (sequencing, playback, mixing, persistence, MIDI) work. The goal is to harden it for public alpha/beta — early adopters who'll tolerate rough edges but expect stability.

**Decisions locked in:**
- TUI is the product (imbolc-gui can be dropped/fenced)
- Network (imbolc-net) deferred entirely — solo experience only
- Architecture questions get pragmatic defaults, not deep redesigns
- Priority: **stability > usability > features**

---

## Phase 0: Hygiene (1-2 days)

Clean build = professional signal. Zero warnings makes regressions visible.

### 0.1 Fix compile error
- `imbolc-ui/src/panes/global_actions.rs:186` — non-exhaustive match on `PaneId::Tuner`. Add the missing arm.
- `imbolc-ui/src/ui/action_id.rs` — verify `PaneId::Tuner` variant exists and is handled

### 0.2 Fix all compiler warnings (~33)
- `imbolc-ui/src/ui/mod.rs:30` — remove unused imports `selected_style_bold`, `selected_style`
- `imbolc-ui/src/main.rs:64` — prefix `discover_mode` with `_` (or cfg-gate on `net` feature)
- `imbolc-ui/src/ui/input.rs:35` — `#[allow(dead_code)]` on `AppEvent` variant fields
- `imbolc-ui/src/ui/style.rs` — `#[allow(dead_code)]` on `theme_*` functions (intentional API for future themes)
- `imbolc-ui/src/ui/layout_helpers.rs:20` — annotate or use `render_dialog_frame`
- `imbolc-ui/src/ui/render.rs:73` — annotate or use `fill_line_bg`
- `imbolc-ui/src/ui/list_selector.rs` — annotate `reset`, `next_and_scroll`, `prev_and_scroll`
- Test helpers (`make_test_state`, `drive_and_collect_actions`, `send_reconnect`) — add `#[cfg(test)]` or `#[allow(dead_code)]`

**Done when:** `cargo build` produces zero warnings.

### 0.3 Quick wins from TASKS.md
- Remove time signature from piano roll header (`piano_roll_pane.rs`)
- Remove inline help text from pane `render()` methods (all panes)

---

## Phase 1: Crash Resilience & Error Feedback (1-2 weeks)

An alpha that crashes or silently fails destroys trust.

### 1.1 Notification/feedback system
**Why:** Every subsequent phase needs a way to tell users what happened. Currently errors go to server pane status or nowhere.

- Add `StatusMessage { text, level, timestamp }` ring buffer to `AppState`
- Render one-line status bar at bottom of frame (`frame.rs`)
- Auto-dismiss after 3-5s. Levels: Info (white), Warning (yellow), Error (red)
- Wire into: save/load, audio errors, MIDI connection, export progress

**Files:** `imbolc-types/src/` (new type), `imbolc-core/src/state/mod.rs`, `imbolc-ui/src/ui/frame.rs`, `imbolc-ui/src/main.rs`

### 1.2 Terminal size handling
- On startup + `Event::Resize`: check against minimum (80x24)
- If too small: render centered message, skip pane rendering
- Clamp layout dimensions to available space

**Files:** `imbolc-ui/src/main.rs`, `imbolc-ui/src/ui/frame.rs`

### 1.3 Panic recovery hook
- Set a panic hook that restores terminal state (disable raw mode, show cursor)
- Attempt autosave on panic before exiting
- Print useful error message to stderr

**Files:** `imbolc-ui/src/main.rs`

### 1.4 Audit production unwraps
- Review all `.unwrap()` outside test modules in imbolc-core and imbolc-ui
- Replace with proper error handling or document with comments
- Mutex `.lock().unwrap()` → poisoned-mutex recovery where needed

---

## Phase 2: CI/CD (3-5 days)

Without CI, regressions creep in silently. Required before inviting testers.

### 2.1 GitHub Actions CI
- `.github/workflows/ci.yml`
- Matrix: ubuntu-latest, macos-latest
- Steps: `cargo check`, `cargo build --release`, `cargo test` (skip e2e), `cargo clippy -- -D warnings`
- Run on push to main + PRs

### 2.2 Release workflow
- `.github/workflows/release.yml` triggered by `v*` tags
- Build release binaries: macOS (aarch64 + x86_64), Linux (x86_64)
- Bundle `synthdefs/` directory
- Attach to GitHub Release

### 2.3 Clippy cleanup
- Fix any new clippy warnings
- Consider `clippy::unwrap_used` as a warning (not deny)

---

## Phase 3: Onboarding & Docs (1 week)

Alpha testers need to install and run without hand-holding.

### 3.1 Installation guide
- macOS: Homebrew SuperCollider setup, scsynth PATH, VSTPlugin (optional)
- Linux: Package manager commands
- Windows: "Not supported for alpha"
- Common errors + troubleshooting section

### 3.2 Getting Started tutorial
Step-by-step: launch → add instrument → play notes → program pattern → add effects → mix → save → export

### 3.3 Docs audit
- Add "Last verified" header to each file in `docs/`
- Archive completed design docs to `docs/archive/`
- Ensure `architecture.md` and `keybindings.md` are current

---

## Phase 4: Core Alpha Features (2-3 weeks)

### 4.1 Audio export (WAV render)
**Critical** — a DAW that can't export audio isn't a DAW.

Plumbing already exists:
- `AudioCmd::Render/Export` variants in `imbolc-core/src/audio/commands.rs`
- `RenderState`, `ExportState`, `ExportKind` structs exist
- `PianoRollActionId::RenderToWav/BounceToWav/ExportStems` keybindings defined

**Need:** Wire trigger → audio thread → progress notifications → completion.

**Files:** `imbolc-core/src/audio/audio_thread.rs`, `imbolc-core/src/audio/commands.rs`, `imbolc-ui/src/panes/piano_roll_pane/`

### 4.2 Autosave / crash recovery
- Periodic autosave (every 2-5 min) to `.imbolc.autosave`
- On startup, detect autosave + offer recovery
- Use existing IO channel for non-blocking save

### 4.3 Automation recording + playback
Per TASKS.md: data structures exist and are persisted, but recording mode, playback interpolation, and editing are missing.

- Recording: Capture parameter changes as automation points during playback
- Playback: Interpolate values in tick loop (`apply_automation()` exists)
- Editing: `AutomationPane` already exists, wire point editing

**Files:** `imbolc-core/src/audio/engine/automation.rs`, `imbolc-core/src/dispatch/automation.rs`, `imbolc-ui/src/panes/automation_pane/`, `imbolc-types/src/state/automation.rs`

---

## Phase 5: Polish & Quality (1-2 weeks)

### 5.1 Test coverage expansion
Target: 700+ tests (up from 524). Focus on:
- Dispatch round-trip tests (action → state change)
- Render smoke tests for critical panes (ratatui `TestBackend`)
- Persistence round-trip (save → load → verify)

### 5.2 Sequencer: note duration grid selection
Small feature from TASKS.md — keybind to cycle grid resolution.

### 5.3 MIDI Learn
CC mapping state exists, needs "learn mode" UI toggle + auto-bind next incoming CC.

---

## Phase 6: Pre-Release (1 week)

### 6.1 Workspace cleanup
- Remove `imbolc-gui` from workspace members (or feature-gate)
- Mark `imbolc-net` as experimental/deferred in README

### 6.2 Release prep
- CHANGELOG.md
- Verify LICENSE (GPL v3)
- README: CI badge, terminal recording/screenshots, clean up sponsor links
- Version bump: all `Cargo.toml` → `0.1.0-alpha.1`
- Tag and release

---

## Explicitly Deferred (Post-Alpha)

| Item | Reason |
|------|--------|
| imbolc-net (all networking) | Solo experience first |
| imbolc-gui (Dioxus) | TUI is the product |
| UI themes | Touches every pane, cosmetic |
| Multi-track audio recording | Complex cpal integration |
| VST parameter discovery | Manual import works |
| Plugin scanning/cataloging | Manual import works |
| Latency compensation (PDC) | Complex DSP problem |
| MIDI clock sync | External sync is post-alpha |
| Sidechain visualization | Polish |
| Group/bus metering | Polish |

---

## Architecture Decisions (Pragmatic Defaults)

| Question | Default for Alpha |
|----------|-------------------|
| AppState authority | Stays as source of truth. No event-log rewrite. |
| Why SuperCollider | Keep SC. UGen library + SynthDef language justify the OSC tax. |
| Control vs performance plane | Accept brief stutter during synthdef loads. Document as known limitation. |
| 0.5ms tick / 15ms lookahead | Working. Ship as-is. Make lookahead configurable via config.toml. |
| Undo scaling | 500-entry cap is fine. Monitor in testing. |
| Instrument-as-monolith | Keep unified model. Groups/shared FX post-beta. |
| Send tap points | Pre-insert sends are the default. Post-fader optional later. |
| Voice stealing | Blind-stealing with release+1s margin. SC feedback not worth the complexity. |
| Control-bus pool | Monotonic growth for alpha. |
| Docs | Notebook, not contract. Mark stale docs with dates. |

---

## Execution Order

```
Phase 0 (days 1-2)     ████  Hygiene
Phase 1 (days 3-12)    ████████████  Crash resilience + errors
Phase 2 (days 3-8)     ██████  CI/CD (parallel with Phase 1)
Phase 3 (days 8-15)    ████████  Onboarding docs
Phase 4 (days 13-25)   ██████████████  Core features (export, autosave, automation)
Phase 5 (days 20-30)   ████████████  Polish + tests
Phase 6 (days 30-35)   ██████  Pre-release
```

## Verification

After each phase:
- `cargo build` — zero warnings
- `cargo test` — all tests pass
- `cargo clippy` — clean (after Phase 2)
- Manual smoke test: launch → add instrument → play → sequence → mix → save → load → export
