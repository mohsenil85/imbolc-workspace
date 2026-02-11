# Imbolc: Road to Public Alpha/Beta

_Last updated: 2026-02-11_

## Context

Imbolc is now a 6-crate workspace (`imbolc-ui`, `imbolc-core`,
`imbolc-types`, `imbolc-audio`, `imbolc-net`, `imbolc-gui`). Core
music-making flow is implemented: sequencing, playback, mixing,
persistence, MIDI, recording/export, and automation lanes.

Current product posture for alpha:

- TUI is the primary product experience.
- GUI (`imbolc-gui`) is experimental and non-blocking for alpha.
- Networking (`imbolc-net`) is optional and feature-flagged, not a gate
  for local alpha quality.
- Priority remains: **stability > usability > features**.

---

## Status Snapshot (2026-02-11)

| Phase | Status | Notes |
|------|--------|-------|
| Phase 0: Hygiene | DONE | Compile error + warning cleanup and quick UI hygiene were completed. |
| Phase 1: Crash resilience | DONE | Status bar, resize handling, and panic hook are in place. |
| Phase 2: CI/CD | NOT STARTED | `.github/workflows/` is currently missing. |
| Phase 3: Onboarding/docs | PARTIAL | README quick-start exists; dedicated install/tutorial/docs audit still pending. |
| Phase 4: Core alpha features | PARTIAL | Export + automation recording/playback are done; autosave/recovery is still missing. |
| Phase 5: Polish/quality | PARTIAL | Sequencer grid selection is done; test expansion and MIDI learn workflow remain. |
| Phase 6: Pre-release | NOT STARTED | Packaging/versioning/release assets not yet prepared. |

---

## Phase 0: Hygiene (1-2 days) -- DONE

- [x] Fix compile blockers and warning noise
- [x] Clean up quick UI clutter from `TASKS.md`

## Phase 1: Crash Resilience & Error Feedback (1-2 weeks) -- DONE

- [x] Notification/status bar with levels and expiry
- [x] Minimum terminal size + resize-safe rendering
- [x] Panic hook restores terminal state
- [x] Production `unwrap()` audit/cleanup pass

---

## Phase 2: CI/CD (3-5 days) -- NOT STARTED

Required before inviting broader testers.

### 2.1 GitHub Actions CI

- [ ] Create `.github/workflows/ci.yml`
- [ ] Matrix: `ubuntu-latest`, `macos-latest`
- [ ] Steps: `cargo check`, `cargo build --release`, `cargo test` (skip
  e2e), `cargo clippy -- -D warnings`
- [ ] Trigger on pushes to `main` and PRs

### 2.2 Release workflow

- [ ] Create `.github/workflows/release.yml` for `v*` tags
- [ ] Build release binaries: macOS (aarch64 + x86_64), Linux (x86_64)
- [ ] Bundle `synthdefs/`
- [ ] Attach artifacts to GitHub Releases

### 2.3 Clippy policy

- [ ] Fix remaining clippy warnings
- [ ] Consider `clippy::unwrap_used` as warning-level signal

---

## Phase 3: Onboarding & Docs (1 week) -- PARTIAL

### 3.1 Installation guide -- PARTIAL

- [x] Basic install/run info in `README.md` (Rust, SuperCollider,
  SynthDef compile, Linux deps)
- [ ] Add dedicated `docs/installation.md` with platform-specific
  troubleshooting and known issues

### 3.2 Getting Started tutorial -- NOT STARTED

- [ ] Create step-by-step first session flow: launch -> add instrument ->
  play notes -> sequence -> add effects -> save -> export

### 3.3 Docs audit -- NOT STARTED

- [ ] Add "Last verified" stamp to maintained docs
- [ ] Move stale/obsolete docs to `docs/archive/`
- [ ] Ensure architecture and keybinding references are current

---

## Phase 4: Core Alpha Features (2-3 weeks) -- PARTIAL

### 4.1 Audio export (WAV render/bounce/stems) -- DONE

- [x] Trigger path from UI actions (`render_to_wav`, `bounce_to_wav`,
  `export_stems`)
- [x] Dispatch wiring to audio command layer
- [x] Audio thread export lifecycle + progress feedback + completion
- [x] UI progress indicators and completion status messages

**Key files:** `imbolc-ui/src/panes/piano_roll_pane/input.rs`,
`imbolc-core/src/dispatch/piano_roll.rs`,
`imbolc-audio/src/audio_thread.rs`,
`imbolc-core/src/dispatch/audio_feedback.rs`,
`imbolc-ui/src/panes/piano_roll_pane/rendering.rs`

### 4.2 Autosave / crash recovery -- NOT STARTED

- [ ] Periodic autosave (every 2-5 min) to `.imbolc.autosave`
- [ ] Startup autosave detection + recovery prompt
- [ ] Non-blocking save path through existing IO channel

### 4.3 Automation recording + playback -- DONE

- [x] Recording mode toggle and lane arming
- [x] Capture parameter changes during playback (`RecordValue` flow)
- [x] Playback interpolation via lane `value_at()` in tick loop
- [x] Automation editing pane with lane/point operations

**Key files:** `imbolc-core/src/dispatch/automation.rs`,
`imbolc-audio/src/playback.rs`, `imbolc-types/src/state/automation.rs`,
`imbolc-ui/src/panes/automation_pane/`

---

## Phase 5: Polish & Quality (1-2 weeks) -- PARTIAL

### 5.1 Test coverage expansion -- IN PROGRESS

- [ ] Raise integration/regression coverage in critical dispatch and
  persistence paths
- [ ] Add render smoke tests for critical panes (`TestBackend`)
- [ ] Keep/save/load round-trip coverage for session integrity

### 5.2 Sequencer grid selection -- DONE

- [x] Keybind-based step resolution cycle implemented in sequencer

**Key files:** `imbolc-ui/src/panes/sequencer_pane.rs`,
`imbolc-ui/keybindings.toml`, `imbolc-ui/src/ui/action_id.rs`

### 5.3 MIDI Learn workflow -- PARTIAL

- [x] CC mapping data model + persistence + mapping management actions
- [x] MIDI settings pane shows current CC mappings
- [ ] Learn mode: "next incoming CC binds selected target"
- [ ] Clear in-flow UX for selecting target then arming learn

---

## Phase 6: Pre-Release (1 week) -- NOT STARTED

### 6.1 Workspace/release posture

- [ ] Decide alpha stance for `imbolc-gui` and `imbolc-net` in release
  messaging (primary vs experimental)
- [ ] Update README release framing accordingly

### 6.2 Release prep

- [ ] Create `CHANGELOG.md`
- [ ] Verify LICENSE and release metadata
- [ ] Add CI badge + refreshed terminal captures in README
- [ ] Version bump all crates to `0.1.0-alpha.1`
- [ ] Tag and publish alpha release artifacts

---

## Explicitly Deferred (Post-Alpha)

| Item | Reason |
|------|--------|
| UI themes overhaul | Touches every pane, cosmetic for alpha |
| Multi-track audio recording | Larger cpal + timeline design effort |
| Plugin scanning/cataloging | Manual import is usable for alpha |
| Latency compensation (PDC) | Deep DSP/timing work |
| MIDI clock sync | External sync is post-alpha |
| Sidechain visualization | Polish |
| Group/bus metering polish | Polish |

---

## Architecture Decisions (Pragmatic Defaults)

| Question | Default for Alpha |
|----------|-------------------|
| AppState authority | Keep AppState as source of truth; no event-log rewrite. |
| Audio backend | Keep SuperCollider for alpha. |
| Scheduling | Keep current tick/lookahead model; tune via config only if needed. |
| Undo scaling | Keep capped history and monitor in real use. |
| Networking | Keep feature-flagged and optional; local workflow is the alpha gate. |
| GUI scope | Keep experimental; no alpha blocker dependency. |
| Docs policy | Treat docs as maintained notes with explicit verification dates. |

---

## Next Execution Order

1. Phase 2 (CI/CD) — block regressions before widening testers.
2. Phase 4.2 (autosave/recovery) — close the largest data-loss risk.
3. Phase 3 (install/tutorial/docs) — reduce onboarding friction.
4. Phase 5.1 + 5.3 (coverage and MIDI learn UX).
5. Phase 6 (packaging/release prep).

## Verification Gates

After each phase:

- `cargo build` with no new warnings
- `cargo test` passing
- `cargo clippy` clean (from Phase 2 onward)
- Manual smoke test: launch -> add instrument -> play -> sequence ->
  mix -> save -> load -> export
