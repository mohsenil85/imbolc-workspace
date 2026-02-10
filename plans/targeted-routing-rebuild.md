# Targeted Routing Rebuild

> Part of the [scaling analysis](scaling-analysis.md) — addresses the #1 local
> scaling issue: "Full routing rebuild tears down ALL voices."

## Context

A full routing rebuild (`routing=true`) tears down ALL SuperCollider nodes and
voices across ALL instruments, then recreates everything from scratch. With 30
instruments x 3 effects that's 150+ node creates — audible dropout. The
single-instrument path (`rebuild_single_instrument_routing`) already proves
targeted rebuilds work. This plan extends the approach to the remaining
full-rebuild triggers.

### Current full-rebuild triggers
- Add instrument (`dispatch/instrument/crud.rs:16`)
- Delete instrument (`dispatch/instrument/crud.rs:59`)
- Bus effect add/remove/move/toggle (`dispatch/bus.rs` — 10 sites)
- Layer group effect add/remove/move/toggle + EQ toggle (`dispatch/bus.rs`)
- Undo/redo (`dispatch/mod.rs` — `AudioDirty::all()`)
- Merge escalation: two different `routing_instrument` IDs in one frame

### Key insight: bus allocator is incremental
`BusAllocator` uses `get_or_alloc_audio_bus()` which returns cached values for
existing instruments and extends the watermark for new ones. No reset needed for
add/delete. `rebuild_single_instrument_routing` already relies on this.

---

## Phase 1: Add instrument without full rebuild

**Status**: Done

**Impact**: High — common workflow, currently kills all voices.

Change `handle_add()` to set a new `routing_add_instrument` flag instead of
`routing = true`. On the audio thread, call `build_instrument_chain()` +
`build_instrument_sends()` for just the new instrument. The bus allocator
naturally extends. No teardown of existing instruments.

**Files:**
- `imbolc-types/src/action.rs` — add `routing_add_instrument: Option<InstrumentId>` to `AudioDirty`, update `merge()` / `any()` / `all()`
- `imbolc-audio/src/event_log.rs` — add `add_instrument_routing: Option<InstrumentId>` to `LogEntryKind::Action`
- `imbolc-audio/src/handle.rs` — thread the flag through `forward_action()`
- `imbolc-audio/src/audio_thread.rs` — handle in `apply_log_entry()`: call new `add_instrument_routing()` method
- `imbolc-audio/src/engine/routing.rs` — add `pub fn add_instrument_routing(id, state, session)` that builds the new instrument's chain + sends + syncs bus watermarks
- `imbolc-core/src/dispatch/instrument/crud.rs` — `handle_add()`: set `routing_add_instrument = Some(new_id)` instead of `routing = true`

**Merge rule**: if both sides have `routing_add_instrument` with different IDs, or if it conflicts with `routing_delete_instrument`, escalate to `routing = true`.

## Phase 2: Delete instrument without full rebuild

**Status**: Done

**Impact**: High — mirrors Phase 1. Currently kills all voices.

Change `handle_delete()` to set `routing_delete_instrument` flag. On the audio
thread, free only that instrument's nodes/voices/sends using the same pattern as
`rebuild_single_instrument_routing()` teardown (lines 888–922), then call
`bus_allocator.free_module_buses(id)` (already exists, currently dead code).

**Files:**
- `imbolc-types/src/action.rs` — add `routing_delete_instrument: Option<InstrumentId>`
- `imbolc-audio/src/event_log.rs` — add `delete_instrument_routing: Option<InstrumentId>`
- `imbolc-audio/src/handle.rs` — thread the flag
- `imbolc-audio/src/audio_thread.rs` — handle: call new `delete_instrument_routing()`
- `imbolc-audio/src/engine/routing.rs` — add `pub fn delete_instrument_routing(id)` that frees nodes + voices + sends + buses for one instrument
- `imbolc-core/src/dispatch/instrument/crud.rs` — `handle_delete()`: set targeted flag

**Note**: bus index "leaks" (watermark only moves forward) are fine — SC has 1024+ audio buses, enough for ~100 add/delete cycles per session.

## Phase 3: Bus/group effect changes without instrument teardown

**Status**: Done

**Impact**: Medium-high — 10+ dispatch sites in `bus.rs` all set `routing = true`.

Bus/group effects live in `GROUP_BUS_PROCESSING` (350), completely separate from
instrument chains in groups 100-300. Changing a bus effect should only rebuild
the bus processing section.

Add `routing_bus_processing: bool` flag. On the audio thread, free all nodes in
`bus_node_map`, `bus_effect_node_map`, `layer_group_node_map`,
`layer_group_send_node_map`, `layer_group_effect_node_map`,
`layer_group_eq_node_map`, then re-run the BuildOutputs phase logic. Do NOT
touch instrument nodes or voices.

**Files:**
- `imbolc-types/src/action.rs` — add `routing_bus_processing: bool`
- `imbolc-audio/src/event_log.rs` — add `rebuild_bus_processing: bool`
- `imbolc-audio/src/audio_thread.rs` — handle the flag
- `imbolc-audio/src/engine/routing.rs` — add `pub fn rebuild_bus_processing(state, session)` that tears down + rebuilds only GROUP_BUS_PROCESSING content
- `imbolc-core/src/dispatch/bus.rs` — all 10+ sites: `routing_bus_processing = true` instead of `routing = true`

## Phase 4: Multi-instrument rebuild (avoid escalation)

**Status**: Done

**Impact**: Medium — prevents the merge escalation where two different
`routing_instrument` IDs in one frame trigger a full rebuild.

Replace `routing_instrument: Option<InstrumentId>` with
`routing_instruments: [Option<InstrumentId>; 4]`. Merge collects unique IDs;
overflows to `routing = true` (very unlikely — user actions target one
instrument per frame).

**Files:**
- `imbolc-types/src/action.rs` — change field type, update merge/any/all, add helper `AudioDirty::for_instrument(id)`
- All dispatch sites setting `routing_instrument = Some(id)` — use the helper
- `imbolc-audio/src/event_log.rs` — update field type
- `imbolc-audio/src/audio_thread.rs` — loop over non-None entries, call `rebuild_single_instrument_routing()` for each
- `imbolc-audio/src/handle.rs` — update forwarding

## Phase 5: Scoped undo

**Status**: Done

**Impact**: Medium — undo/redo currently always sets `AudioDirty::all()`.

For `UndoScope::SingleInstrument(id)` undos, set `routing_instruments` for that
instrument instead of full rebuild. All other scopes (`Session`, `Instruments`,
`Full`) keep `AudioDirty::all()`.

**Files:**
- `imbolc-core/src/dispatch/mod.rs` — in Undo/Redo arms, inspect the applied entry's scope. For `SingleInstrument(id)`, set targeted flags.
- `imbolc-core/src/state/undo.rs` — ensure `undo()`/`redo()` return the scope of the applied entry

---

## Priority ordering

| Phase | Impact | Risk | Do when |
|-------|--------|------|---------|
| 1. Add instrument | High | Low | First |
| 2. Delete instrument | High | Low | Second (mirrors Phase 1) |
| 3. Bus/group effects | Medium-high | Low | Third |
| 4. Multi-instrument | Medium | Medium (API change) | Fourth |
| 5. Scoped undo | Medium | Low | Fifth |

Phases 1-3 are independent. Phase 4 changes the `routing_instrument` field type
(touches many call sites). Phase 5 depends on Phase 4 being in place.

## Audio thread priority logic

```
if routing { full_rebuild() }
else {
    if routing_delete_instrument { delete_instrument_routing(id) }
    if routing_add_instrument { add_instrument_routing(id) }
    for id in routing_instruments.flatten() { rebuild_single_instrument_routing(id) }
    if routing_bus_processing { rebuild_bus_processing() }
}
```

Delete before add ensures correctness if both happen in one frame.

## Verification

After each phase:
1. `cargo test -p imbolc-types` — AudioDirty merge/any tests
2. `cargo test -p imbolc-audio` — routing tests
3. `cargo test -p imbolc-core` — dispatch tests
4. Manual: add/delete instruments, toggle bus effects, undo/redo — verify no audio dropout and correct routing
