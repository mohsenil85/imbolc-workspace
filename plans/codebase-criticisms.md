# Codebase Criticisms

Substantive issues worth addressing, filtered for a solo-dev DAW project. Not architecture-astronaut suggestions — real bugs, real duplication, real friction.

---

## 1. AudioDirty last-write-wins loses updates

**Where:** `AudioDirty` struct in `imbolc-types/src/action.rs`

Targeted param fields are `Option<(...)>` to preserve `Copy`:
```rust
pub filter_param: Option<(InstrumentId, FilterParamKind, f32)>,
pub effect_param: Option<(InstrumentId, EffectId, usize, f32)>,
pub lfo_param: Option<(InstrumentId, LfoParamKind, f32)>,
```

If two filter params change in the same tick (cutoff + resonance), only the second survives. Merge uses last-write-wins. In practice mostly safe because UI events are one-per-frame, but a real data loss bug waiting to happen with MIDI CC input or automation playback sending multiple params simultaneously.

**Fix options:**
- Remove `Copy`, use `Vec<Update>` (requires refactoring ~30 call sites)
- Use a small fixed-size array: `[Option<(...)>; 4]` (still Copy, handles reasonable burst)
- Accept the limitation but add a debug assertion that catches overwrites

---

## 2. Action projection has no compile-time link to dispatch

**Where:** `imbolc-core/src/audio/action_projection.rs` vs `imbolc-core/src/dispatch/`

`project_action()` manually reimplements state mutations from dispatch handlers. If you update a dispatch handler and forget the projection, the audio thread silently diverges. No test matrix verifies parity.

**Fix:** Add a test for every projectable action: dispatch it, project it, assert final states match. Could be a macro-generated test matrix.

---

## 3. `is_running()` guards silently swallow operations

**Where:** Throughout `imbolc-core/src/dispatch/instrument/` — playback.rs, effects.rs, eq.rs, etc.

Pattern:
```rust
if audio.is_running() {
    // do the thing
}
// else: silently return DispatchResult::none()
```

User presses a key, nothing happens, no feedback. Should at minimum return a status message.

---

## 4. Effect management code duplicated 3x

**Where:**
- `Instrument` in `imbolc-types/src/state/instrument/mod.rs`
- `MixerBus` in `imbolc-types/src/state/` (bus module)
- `LayerGroupMixer` in `imbolc-types/src/state/` (layer group module)

Each has its own `add_effect()`, `remove_effect()`, `move_effect()`, `effect_by_id()`, `next_effect_id` counter. ~100 lines copy-pasted across three types.

**Fix:** Extract an `EffectChain` struct that all three embed. Consistent by construction, single place to add new effect chain logic.

---

## 5. Instrument is a 28-field god struct

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

A drum pad carries `sampler_config`, `arpeggiator`, `chord_shape`, `vst_param_values`, `vst_state_path`. An audio-in carries effects and filter it'll never use. Every new feature adds another `Option<T>` field. Makes persistence migrations heavier and constructors noisy.

Not urgent — works fine at current scale — but worth noting as friction grows with each new instrument feature.

---

## 6. Current branch has compilation errors on main

**Where:**
- `imbolc-core/src/dispatch/side_effects.rs:266` — calls `set_layer_group_eq_param` which doesn't exist (should be `set_eq_param`)
- `imbolc-core/src/state/persistence/load.rs:268` — missing `eq` field in `LayerGroupMixer` initializer

From in-progress layer-group EQ work that's half-landed.

---

---

# Leaning on the Compiler

Places where runtime checks, string matching, or raw primitives could be replaced with types the compiler enforces.

---

## 7. EQ params are stringly-typed

**Where:** `action.rs` (`SetEqParam`), `dispatch/instrument/eq.rs`, `dispatch/bus.rs`

```rust
// Action variant
SetEqParam(u32, usize, String, f32)

// Dispatch matches on string literals
match param_name {
    "freq" => band.freq = value.clamp(20.0, 20000.0),
    "gain" => band.gain = value.clamp(-24.0, 24.0),
    "q"    => band.q = value.clamp(0.1, 10.0),
    "on"   => band.enabled = value > 0.5,
    _ => {}  // silent no-op
}
```

Typo in a param name (`"frequ"`) silently does nothing. Adding a new EQ param requires updating string matches in 3+ places with no compiler help. The `_ => {}` arm means the compiler can't warn about unhandled cases.

**Fix:** `enum EqParamKind { Freq, Gain, Q, Enabled }` — exhaustive match, zero-cost, catches typos at compile time.

---

## 8. Effect params accessed by raw `usize` index

**Where:** `AdjustEffectParam(InstrumentId, EffectId, usize, f32)` in `action.rs`, dispatch in `effects.rs`, `bus.rs`

The `usize` is a positional index into `effect.params`. No type distinguishes "delay time" from "delay feedback" — it's just `params[0]` vs `params[1]`. If param order changes in a SynthDef, old automation/saves silently target the wrong param.

Not proposing per-effect typed enums (too much churn), but a `ParamIndex` newtype would at least prevent confusing it with other `usize` values in the same function signature.

---

## 9. All ID types are bare type aliases

**Where:** `imbolc-types/src/lib.rs`

```rust
pub type InstrumentId = u32;
pub type EffectId = u32;
pub type CustomSynthDefId = u32;
pub type VstPluginId = u32;
```

These are all `u32`. The compiler can't stop you from passing an `EffectId` where an `InstrumentId` is expected. Same issue with `u8` bus IDs — a bus ID and a MIDI channel are both `u8`.

**Fix:** Newtypes. `struct InstrumentId(u32)` etc. Derive `Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`. The refactor is mechanical but touches many files.

---

## 10. Persistence decoders silently fall back on unknown variants

**Where:** `imbolc-core/src/state/persistence/load.rs` — 14 decoder functions

```rust
fn decode_effect_type(s: &str) -> EffectType {
    match s {
        "Delay" => EffectType::Delay,
        "Reverb" => EffectType::Reverb,
        // ...
        _ => EffectType::Delay,  // silent fallback
    }
}
```

Same pattern for `decode_source_type`, `decode_filter_type`, `decode_lfo_shape`, `decode_arp_direction`, `decode_chord_shape`, etc. — 14 functions total, all with `_ => SomeDefault` arms.

If you add a new `EffectType` variant, save a project, then load it on an older binary, it silently becomes a Delay. No warning, no error. The compiler can't help because these are string→enum conversions.

**Fix:** Return `Result<T, String>` from decoders. Log a warning on unknown variants. Or use `serde` for the round-trip and get exhaustiveness for free.

---

## 11. Bus IDs are raw `u8` with no bounds enforcement

**Where:** `MixerBus.id: u8`, `MixerSend.bus_id: u8`, `BusAction::Remove(u8)`, `OutputTarget::Bus(u8)`

Bus IDs are 1–8 but the type allows 0–255. `OutputTarget::Bus(0)` or `Bus(100)` compile fine. Validation happens at runtime in dispatch handlers, not at construction.

**Fix:** `struct BusId(u8)` with a constructor that enforces the range. Then `OutputTarget::Bus(BusId)` is invalid-by-construction for out-of-range values.

---

## 12. Parallel sends/buses invariant enforced by convention

**Where:** `Instrument.sends: Vec<MixerSend>` must stay in sync with `SessionState` buses

```rust
// After adding a bus, must manually sync all instruments
if let Some(_new_id) = state.session.add_bus() {
    let bus_ids: Vec<u8> = state.session.bus_ids().collect();
    for inst in &mut state.instruments.instruments {
        inst.sync_sends_with_buses(&bus_ids);  // easy to forget
    }
}
```

Forgetting to call `sync_sends_with_buses` means an instrument has no send slot for the new bus. The compiler can't catch this — it's a runtime invariant maintained by discipline.

**Fix:** Use `BTreeMap<BusId, MixerSend>` instead of `Vec<MixerSend>`. Sends are looked up by bus ID, not position. No sync step needed — missing entries just mean "default send level."

---

## 13. `Option<T>` fields on Instrument that depend on SourceType

**Where:** `Instrument` in `imbolc-types/src/state/instrument/mod.rs`

```rust
pub sampler_config: Option<SamplerConfig>,     // only if SourceType is PitchedSampler
pub drum_sequencer: Option<DrumSequencerState>, // only if SourceType is Kit
pub convolution_ir_path: Option<String>,        // only if ConvolutionReverb effect present
```

Nothing stops you from accessing `instrument.sampler_config` on a Saw oscillator. The relationship between `source` and which `Option<T>` fields are `Some` is a runtime invariant.

Full typestate (separate struct per source type) would be high-churn. A lighter fix: accessor methods that return `Option` only when the source type matches, making the intent explicit and centralizing the check.

---

## Priority (all items)

| # | Issue | Severity | Effort |
|---|-------|----------|--------|
| 6 | Compilation errors | **High** (blocks build) | Minutes |
| 7 | Stringly-typed EQ params | **High** (silent bugs) | Small |
| 1 | AudioDirty data loss | **Medium** (latent bug) | Hours |
| 2 | Projection parity | **Medium** (silent correctness) | Hours |
| 9 | Bare ID type aliases | **Medium** (wrong-ID bugs) | Large (mechanical) |
| 10 | Persistence silent fallbacks | **Medium** (data loss) | Medium |
| 11 | Raw u8 bus IDs | **Medium** (invalid states) | Medium |
| 12 | Sends/buses sync invariant | **Medium** (forgotten sync) | Medium |
| 8 | Raw usize param index | **Low** (index confusion) | Small |
| 3 | Silent `is_running()` | **Low** (UX annoyance) | Small |
| 4 | Effect chain duplication | **Low** (maintenance tax) | Medium |
| 13 | Option fields vs SourceType | **Low** (wrong access) | Large |
| 5 | Instrument god struct | **Low** (scaling friction) | Large |
