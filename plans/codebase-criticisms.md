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

## ~~3. `is_running()` guards silently swallow operations~~ (FIXED)

**Fixed.** All `is_running()` guards converted from `if` to
`if/else`, returning a status message ("Audio engine not running")
when the engine is offline. No more silent swallowing.

---

## ~~4. Effect management code duplicated 3x~~ (FIXED)

**Fixed.** Extracted `EffectChain` struct that `Instrument`,
`MixerBus`, and `LayerGroupMixer` all embed. Single place for
`add_effect()`, `remove_effect()`, `move_effect()`, `effect_by_id()`,
and `next_effect_id` counter.

---

## 5. Instrument god struct (28 → 18 fields, in progress with #13)

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

Down from 28 to 18 fields after three extractions:
- **`InstrumentMixer`** — level, pan, mute, solo, routing, sends
- **`LayerConfig`** — group assignment, octave
- **`NoteInputConfig`** — arpeggiator, chord shape

**In progress:** Closing together with #13 — source-type-specific
`Option<T>` fields will move into a `SourceConfig` enum, further
reducing the field count and eliminating the runtime invariant.

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

## ~~9. All ID types are bare type aliases~~ (FIXED)

**Fixed.** All 6 ID newtypes fully migrated across the entire workspace:

```rust
pub struct InstrumentId(u32);
pub struct EffectId(u32);
pub struct CustomSynthDefId(u32);
pub struct VstPluginId(u32);
pub struct BusId(u8);       // with new() asserting > 0
pub struct ParamIndex(usize);
```

Zero compilation errors. All 925 tests pass including 95 imbolc-net
tests.

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

## 13. `Option<T>` fields on Instrument that depend on SourceType (in progress with #5)

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

```rust
pub sampler_config: Option<SamplerConfig>,     // only if SourceType is PitchedSampler
pub drum_sequencer: Option<DrumSequencerState>, // only if SourceType is Kit
pub convolution_ir_path: Option<String>,        // only if ConvolutionReverb effect present
```

**In progress:** Closing together with #5 — these `Option<T>` fields
will move into a `SourceConfig` enum that makes the source-type
relationship a compile-time invariant instead of a runtime one.

---

## Priority (all items)

| # | Issue | Severity | Effort |
|---|-------|----------|--------|
| 6 | ~~Compilation errors~~ | ~~**High**~~ FIXED | — |
| 7 | ~~Stringly-typed EQ params~~ | ~~**High**~~ FIXED | — |
| 1 | ~~AudioDirty data loss~~ | ~~**Medium**~~ FIXED | — |
| 2 | ~~Projection parity~~ | ~~**Medium**~~ FIXED | — |
| 9 | ~~ID type newtypes~~ | ~~**Medium**~~ FIXED | — |
| 10 | ~~Persistence silent fallbacks~~ | ~~**Medium**~~ FIXED | — |
| 11 | ~~Raw u8 bus IDs~~ | ~~**Medium**~~ FIXED | — |
| 12 | ~~Sends/buses BTreeMap~~ | ~~**Medium**~~ FIXED | — |
| 8 | ~~Raw usize param index~~ | ~~**Low**~~ FIXED | — |
| 3 | ~~Silent `is_running()`~~ | ~~**Low**~~ FIXED | — |
| 4 | ~~Effect chain duplication~~ | ~~**Low**~~ FIXED | — |
| 5+13 | Instrument god struct + Option vs SourceType | **Low** | In progress |
