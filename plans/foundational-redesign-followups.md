# Foundational Redesign Follow-Up Plan

**Status:** PROPOSED
**Last Updated:** 2026-02-11

This plan addresses three post-refactor issues:

1. `Action::Quit` can be dropped in standalone runtime.
2. Reducer and core dispatch are not fully aligned for all mutations (notably instrument add semantics).
3. `plans/foundational-redesign.md` is partially stale.

---

## Priority Order

1. **Fix quit regression first** (user-visible behavior).
2. **Align reducer and dispatch semantics** (state consistency).
3. **Update docs and add drift guardrails** (prevent recurrence).

---

## Phase 1: Quit Semantics Hotfix

### Problem

`Action::Quit` is UI-only, `Action::to_domain()` returns `None`, and runtime only special-cases `SaveAndQuit`.
As a result, pane-emitted `Action::Quit` can no-op.

### Goal

All quit intents should be handled deterministically and consistently.

### Changes

1. Add explicit runtime handling for `Action::Quit` before `dispatch_with_audio()`:
   - If source is quit confirmation flow (quit prompt "Don't Save" or confirm pane `PendingAction::Quit`), quit immediately.
   - Otherwise, match existing global-quit semantics:
     - dirty project => push `quit_prompt`
     - clean project => quit immediately
2. Centralize quit handling into one helper in runtime/global actions to avoid split logic.
3. Keep `Quit` out of `DomainAction` (it is UI orchestration, not domain mutation).

### Tests

1. Add runtime/input tests covering:
   - `Action::Quit` from home/instrument pane with clean project => exits.
   - `Action::Quit` with dirty project => opens `quit_prompt`.
   - `Action::Quit` from quit prompt "Don't Save" => exits (no loop back into prompt).
2. Keep existing integration/e2e tests green.

### Exit Criteria

- No quit path is dropped.
- Behavior is identical whether quit comes from global keybinding or pane action.

---

## Phase 2: Reducer and Dispatch Alignment

### Problem

The reducer is used for audio-thread projection, but core dispatch still has separate mutation logic.
This creates divergence risk; current example: instrument add path applies richer initialization in core than reducer path.

### Goal

Ensure projected audio-thread state matches core state for all reducible actions.

### Changes

1. Short-term correctness fix:
   - Make reducer instrument add/delete behavior match core semantics exactly (custom synthdef and VST initialization included).
2. Medium-term structure:
   - Extract shared pure mutation helpers for instrument/session/mixer edits.
   - Have both core dispatch and reducer call the same helpers for reducible state mutations.
3. Keep non-pure concerns in core dispatch only:
   - undo bookkeeping
   - automation recording side effects
   - nav/status
   - direct audio command dispatch

### Tests

1. Add focused parity tests for reducer vs core mutations on high-risk actions:
   - `Instrument::Add` for `Custom` and `Vst`
   - `Instrument::Delete`
   - `Session::UpdateSession*`
   - routing-affecting mixer actions
2. Add one projection smoke test in audio-thread path that verifies state after forwarded reducible actions.

### Exit Criteria

- Reducible actions produce equivalent state in both paths.
- No known action-class divergence remains.

---

## Phase 3: Documentation Sync and Drift Prevention

### Problem

`plans/foundational-redesign.md` has stale references (for example deleted file paths) and overstates "single source of truth."

### Goal

Docs accurately describe shipped architecture and remaining gaps.

### Changes

1. Update `plans/foundational-redesign.md`:
   - Replace stale file references.
   - Clarify actual architecture:
     - `Action` + `DomainAction` bridge
     - reducer currently authoritative for projection path; core still owns primary mutation path
   - Add "Known Gaps" section with explicit status.
2. Add a lightweight docs sanity check script:
   - verify referenced in-repo paths exist
   - fail CI (or pre-merge check) on broken path references in `plans/` docs

### Tests / Verification

1. Run docs path checker in CI/local checks.
2. Manual spot review of architecture docs after updates.

### Exit Criteria

- No broken path references in planning docs.
- Claims match implementation reality.

---

## Validation Checklist

After each phase:

1. `cargo test -p imbolc-types`
2. `cargo test -p imbolc-core`
3. `cargo test -p imbolc-audio`
4. `cargo test -p imbolc-ui`

Final:

1. Run full `cargo test` in workspace.
2. Manual quit-flow smoke test in TUI:
   - clean quit
   - dirty quit prompt save/don't save/cancel
3. Manual add-instrument smoke test for `Custom` and `Vst` source types.

---

## Rollout Strategy

1. Ship **Phase 1** as a small hotfix PR.
2. Ship **Phase 2** as one or two PRs:
   - parity fix first
   - structural cleanup second
3. Ship **Phase 3** as docs/tooling PR.

This sequencing minimizes user-facing risk while moving architecture toward the original design intent.
