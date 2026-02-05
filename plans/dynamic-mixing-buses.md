# Plan: Dynamic Mixing Buses

## Summary

Change the number of mixing buses from a fixed 8 to a dynamic count where users can add/remove buses at runtime.

## Current State

- `MAX_BUSES = 8` constant defined in two places:
  - `src/state/instrument/mod.rs:21`
  - `src/state/session.rs:11`
- Buses created at startup via `(1..=MAX_BUSES).map(MixerBus::new).collect()`
- Each instrument has 8 pre-allocated send slots
- `bus()` and `bus_mut()` use `(id - 1)` indexing assuming contiguous IDs 1-8
- Mixer navigation hardcodes `MAX_BUSES` bounds
- Comments reference fixed "1-8" range (session.rs:16, instrument/mod.rs:26)

## Design Decisions

| Question | Decision |
|----------|----------|
| Bus ID reuse on delete | Never reuse - use incrementing `next_bus_id` counter |
| Instrument sends on bus delete | Disable send (`enabled = false`), keep the entry |
| Instrument output on bus delete | Reset to `OutputTarget::Master` |
| Initial bus count | Configurable via config (default: 8) |
| Bus limits | Min: 0, Max: 32 (practical limit, enforced in `add_bus()`) |
| Bus storage | `Vec<MixerBus>` with ID-based lookup (simpler than HashMap for iteration/serialization) |
| `next_bus_id` persistence | Always recompute on load as `buses.iter().map(|b| b.id).max().unwrap_or(0) + 1` |

## Implementation Steps

### 1. State Changes

**`src/state/session.rs`**
- Remove `MAX_BUSES` constant
- Add `next_bus_id: u8` field to `SessionState` (skip in serde, recompute on load)
- Change `bus()` and `bus_mut()` to find by ID instead of index math:
  ```rust
  pub fn bus(&self, id: u8) -> Option<&MixerBus> {
      self.buses.iter().find(|b| b.id == id)
  }
  ```
- Add methods:
  - `add_bus(&mut self) -> Option<u8>` - creates bus with next_bus_id, returns ID (None if at max 32)
  - `remove_bus(&mut self, id: u8) -> bool` - removes bus by ID
  - `bus_ids(&self) -> impl Iterator<Item = u8>` - returns current bus IDs in order
- Update `mixer_cycle_section()` to use first bus ID via `bus_ids().next()` (not hardcoded `1`)
- Update `new_with_defaults()` to accept configurable bus count
- Remove "1-8" comment from `MixerSelection::Bus` (line 16)

**`src/state/instrument/mod.rs`**
- Remove `MAX_BUSES` constant
- Remove "1-8" comment from `OutputTarget::Bus` (line 26)

**`src/state/mod.rs`**
- Remove `MAX_BUSES` from re-export (line 30)
- Update `mixer_move()` - for Bus, clamp to actual bus list bounds using `bus_ids()`
- Update `mixer_jump()` - jump to first/last actual bus ID
- Update `mixer_cycle_output()` and `mixer_cycle_output_reverse()` - cycle through actual bus IDs

### 2. Actions

**`src/action.rs`**
- Add `BusAction` enum:
  ```rust
  pub enum BusAction {
      Add,
      Remove(u8),
      Rename(u8, String),
  }
  ```
- Add `Bus(BusAction)` variant to `Action` enum

### 3. Dispatch

**New file: `src/dispatch/bus.rs`**
- Handle `BusAction::Add`:
  - Check bus count < 32, return early if at limit
  - Create bus via `session.add_bus()`
  - Sync instrument sends for all instruments
  - Send `AudioCmd::RebuildRouting` (or mark routing dirty)
- Handle `BusAction::Remove(id)`:
  - Reset instruments with `OutputTarget::Bus(id)` to Master
  - Disable sends to this bus (keep entries for undo support)
  - Remove automation lanes for this bus
  - Remove the bus from session
  - Send `AudioCmd::RebuildRouting`
- Handle `BusAction::Rename`: update bus name

**`src/dispatch/mod.rs`**
- Add `mod bus;`
- Add match arm for `Action::Bus`

### 4. Instrument Send Sync

**`src/state/instrument_state.rs`**
- Update `add_instrument()` to initialize sends for all existing bus IDs:
  ```rust
  pub fn add_instrument_with_buses(&mut self, source: SourceType, bus_ids: &[u8]) -> InstrumentId
  ```
- Or add helper called after instrument creation

**`src/state/session.rs` or `src/state/mod.rs`**
- Add helper to ensure all instruments have sends for all current buses:
  ```rust
  pub fn sync_instrument_sends(instruments: &mut InstrumentState, bus_ids: impl Iterator<Item = u8>)
  ```
- Called after adding a bus
- Called in `AppState::add_instrument()` to sync new instrument with existing buses

### 5. Audio Layer

**`src/audio/commands.rs`**
- Verify `AudioCmd::RebuildRouting` exists and handles dynamic bus counts
- If not, the audio thread needs to query current bus state when rebuilding

**Note:** The audio thread reads bus configuration during routing rebuild. No new commands needed if `RebuildRouting` already re-reads session state.

### 6. Automation

**`src/state/automation/mod.rs`**
- Add `remove_lanes_for_bus(bus_id: u8)` method to `AutomationState`:
  ```rust
  pub fn remove_lanes_for_bus(&mut self, bus_id: u8) {
      self.lanes.retain(|lane| !matches!(lane.target, AutomationTarget::BusLevel(id) if id == bus_id));
  }
  ```

### 7. Persistence

**`src/state/persistence/mod.rs`**
- Increment `BLOB_FORMAT_VERSION`
- On load, recompute `next_bus_id`:
  ```rust
  session.next_bus_id = session.buses.iter().map(|b| b.id).max().unwrap_or(0) + 1;
  ```
- Old projects load fine (buses vec is already persisted, just recompute next_id)

### 8. Config

**`src/config.rs`**
- Add `default_bus_count: Option<u8>` to defaults (default: 8)

### 9. Tests

Update tests in:
- `src/state/session.rs`:
  - `bus_1based_indexing` - update for ID-based lookup
  - `bus_0_panics` - remove or change (no longer panics, returns None)
  - `bus_out_of_bounds` - update expected behavior
  - `mixer_cycle_section_full_cycle` - may need bus existence check
- `src/state/mod.rs`:
  - `mixer_move_clamps_bus` - update to use actual bus count
  - `mixer_jump` - update bus section test
  - `mixer_cycle_output` / `mixer_cycle_output_reverse` - update for dynamic buses

Add new tests:
- `add_bus_increments_id`
- `add_bus_respects_max_limit`
- `remove_bus_resets_instrument_outputs`
- `remove_bus_disables_sends`
- `remove_bus_clears_automation_lanes`
- `new_instrument_gets_sends_for_existing_buses`

## Files to Modify

| File | Changes |
|------|---------|
| `src/state/session.rs` | Add `next_bus_id`, new methods, fix bus access, remove comment |
| `src/state/instrument/mod.rs` | Remove `MAX_BUSES`, remove comment |
| `src/state/instrument_state.rs` | Update instrument creation for bus sends |
| `src/state/mod.rs` | Fix mixer navigation, remove re-export, sync sends on add_instrument |
| `src/action.rs` | Add `BusAction` and `Action::Bus` |
| `src/dispatch/mod.rs` | Add bus dispatch |
| `src/dispatch/bus.rs` | New file - bus action handling |
| `src/state/automation/mod.rs` | Add `remove_lanes_for_bus` |
| `src/state/persistence/mod.rs` | Version bump, recompute next_bus_id on load |
| `src/config.rs` | Add `default_bus_count` |

## Out of Scope (TUI repo)

The sibling `../imbolc` TUI repo will need updates:
- Handle `Action::Bus` in keybindings
- UI for add/remove bus (likely in mixer pane)
- Update any hardcoded bus count references

## Verification

1. **Unit tests**: Run `cargo test` - all existing tests should pass after updates
2. **Manual testing**:
   - Create new project - should start with default 8 buses
   - Add bus - new bus appears, instruments get new send slot
   - Add bus at limit (32) - should fail gracefully
   - Remove bus - instruments routing to it reset to Master, sends disabled
   - Remove bus with automation - lanes removed
   - Create new instrument - should have sends for all existing buses
   - Save/load project - bus count and IDs persist correctly
   - Mixer navigation works with varying bus counts (0, 1, 8, 32 buses)
3. **Audio**: Verify routing rebuilds correctly when buses change (play audio, add/remove bus, confirm no glitches)
