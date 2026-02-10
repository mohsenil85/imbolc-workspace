# Codebase Criticisms

Substantive issues worth addressing, filtered for a solo-dev DAW
project. Not architecture-astronaut suggestions — real bugs, real
duplication, real friction.

---

## ~~1. AudioDirty last-write-wins loses updates~~ (FIXED)

**Fixed.** Targeted param fields changed from `Option<(...)>` to
small fixed-size arrays `[Option<(...)>; 4]`, preserving `Copy` while
supporting up to 4 updates per tick per param category.

---

## ~~2. Action projection has no compile-time link to dispatch~~ (FIXED)

**Fixed.** Added 156 projection parity tests that dispatch each
projectable action and verify the projected state matches. Covers all
instrument, session, and piano roll actions.

---

## 3. `is_running()` guards silently swallow operations

**Where:** Throughout `imbolc-core/src/dispatch/instrument/` —
playback.rs, effects.rs, eq.rs, etc.

Pattern:
```rust
if audio.is_running() {
    // do the thing
}
// else: silently return DispatchResult::none()
```

User presses a key, nothing happens, no feedback. Should at minimum
return a status message.

---

## ~~4. Effect management code duplicated 3x~~ (FIXED)

**Fixed.** Extracted `EffectChain` struct that `Instrument`,
`MixerBus`, and `LayerGroupMixer` all embed. Single place for
`add_effect()`, `remove_effect()`, `move_effect()`, `effect_by_id()`,
and `next_effect_id` counter.

---

## 5. Instrument is a 28-field god struct

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

A drum pad carries `sampler_config`, `arpeggiator`, `chord_shape`,
`vst_param_values`, `vst_state_path`. An audio-in carries effects and
filter it'll never use. Every new feature adds another `Option<T>`
field. Makes persistence migrations heavier and constructors noisy.

Not urgent — works fine at current scale — but worth noting as
friction grows with each new instrument feature.

---

## ~~6. Current branch has compilation errors on main~~ (FIXED)

**Fixed.** Compilation errors from half-landed layer-group EQ work
have been resolved.

---

---

# Leaning on the Compiler

Places where runtime checks, string matching, or raw primitives could
be replaced with types the compiler enforces.

---

## ~~7. EQ params are stringly-typed~~ (FIXED)

**Fixed.** `EqParamKind` enum added, `String` replaced with exhaustive
match in all 6 files. No `_ => {}` arms remain.

---

## ~~8. Effect params accessed by raw `usize` index~~ (FIXED)

**Fixed.** `ParamIndex` newtype added in `imbolc-types/src/lib.rs`.
All `AdjustEffectParam` variants (instrument, bus, layer group),
`AudioDirty` targeted param arrays, and helper methods now use
`ParamIndex` instead of raw `usize`.

---

## 9. All ID types are bare type aliases (IN PROGRESS — nearly complete)

**Where:** `imbolc-types/src/lib.rs`

All 5 ID types are now defined as newtypes with full trait derives
(`Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
Display`):

```rust
pub struct InstrumentId(u32);
pub struct EffectId(u32);
pub struct CustomSynthDefId(u32);
pub struct VstPluginId(u32);
pub struct BusId(u8);  // with new() asserting > 0
```

`BusId` fully migrated. The main codebase compiles. Down from ~43
errors to ~4 remaining in `imbolc-net` test files
(protocol_roundtrip.rs, broadcast.rs, ownership.rs) — raw integer
literals need wrapping with `InstrumentId::new()`.

---

## ~~10. Persistence decoders silently fall back on unknown variants~~ (FIXED)

**Fixed.** All 18 decoder functions now log warnings via
`eprintln!("[imbolc] persistence: unknown X '{}', using DEFAULT", other)`
on unknown variants. Still returns defaults (not `Result`), but
unknown variants are no longer silent. 18 roundtrip tests cover all
decoders.

---

## ~~11. Bus IDs are raw `u8` with no bounds enforcement~~ (FIXED)

**Fixed.** `BusId` newtype with `new()` asserting id > 0 fully
migrated across the codebase — no remaining compilation errors. All
bus-related actions, dispatch handlers, audio engine, and persistence
use `BusId` instead of raw `u8`.

---

## ~~12. Parallel sends/buses invariant enforced by convention~~ (FIXED)

**Fixed.** `Instrument.sends` changed from `Vec<MixerSend>` to
`BTreeMap<BusId, MixerSend>`. Sends are looked up by bus ID directly.
`sync_sends_with_buses` removed — missing entries mean default send
level. Custom serde deserializer handles migration from old Vec format.

---

## 13. `Option<T>` fields on Instrument that depend on SourceType

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

```rust
pub sampler_config: Option<SamplerConfig>,     // only if SourceType is PitchedSampler
pub drum_sequencer: Option<DrumSequencerState>, // only if SourceType is Kit
pub convolution_ir_path: Option<String>,        // only if ConvolutionReverb effect present
```

Nothing stops you from accessing `instrument.sampler_config` on a Saw
oscillator. The relationship between `source` and which `Option<T>`
fields are `Some` is a runtime invariant.

Full typestate (separate struct per source type) would be
high-churn. A lighter fix: accessor methods that return `Option` only
when the source type matches, making the intent explicit and
centralizing the check.

---

## Priority (all items)

| # | Issue | Severity | Effort |
|---|-------|----------|--------|
| 6 | ~~Compilation errors~~ | ~~**High**~~ FIXED | — |
| 7 | ~~Stringly-typed EQ params~~ | ~~**High**~~ FIXED | — |
| 1 | ~~AudioDirty data loss~~ | ~~**Medium**~~ FIXED | — |
| 2 | ~~Projection parity~~ | ~~**Medium**~~ FIXED | — |
| 9 | ID type newtypes (nearly complete, ~4 net test errors) | **Medium** (wrong-ID bugs) | Small (mechanical) |
| 10 | ~~Persistence silent fallbacks~~ | ~~**Medium**~~ FIXED | — |
| 11 | ~~Raw u8 bus IDs~~ | ~~**Medium**~~ FIXED | — |
| 12 | ~~Sends/buses BTreeMap~~ | ~~**Medium**~~ FIXED | — |
| 8 | ~~Raw usize param index~~ | ~~**Low**~~ FIXED | — |
| 3 | Silent `is_running()` | **Low** (UX annoyance) | Small |
| 4 | ~~Effect chain duplication~~ | ~~**Low**~~ FIXED | — |
| 13 | Option fields vs SourceType | **Low** (wrong access) | Large |
| 5 | Instrument god struct | **Low** (scaling friction) | Large |
