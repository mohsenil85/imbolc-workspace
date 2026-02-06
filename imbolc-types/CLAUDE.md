# imbolc-types

Shared type definitions for the Imbolc DAW ecosystem.

## What This Is

The leaf crate of the Imbolc workspace. Contains data structures used across imbolc-core, imbolc-ui, and (future) imbolc-net. No internal dependencies on other Imbolc crates.

## Directory Structure

```
src/
  lib.rs          — Re-exports, type aliases (InstrumentId, EffectId, etc.)
  action.rs       — Action enum for UI -> core dispatch
  dispatch.rs     — Dispatcher trait
  param.rs        — Param, ParamValue, frequency helpers
  audio.rs        — AudioFeedback, ServerStatus, ExportKind
  state/          — All state types
    instrument.rs   — Instrument, SourceType, FilterType, EffectType, LFO, Envelope
    ...
```

## Key Types

| Type | Purpose |
|------|---------|
| `InstrumentId` | `u32` — unique identifier for instruments |
| `Instrument` | Source + filter + effects + LFO + envelope + mixer |
| `SourceType` | Oscillator types: Saw, Sin, AudioIn, BusIn, Sampler, VST, etc. |
| `Action` | Enum of all actions dispatched from UI to core |
| `Param` / `ParamValue` | Generic parameter with Float/Int/Bool values |
| `Dispatcher` | Trait for action dispatch implementations |

## Build & Test

```bash
cargo build
cargo test
```

## Migration Status

Types are being incrementally migrated from imbolc-core. When moving a type:
1. Move the type definition here
2. Re-export from imbolc-core for backwards compatibility (temporary)
3. Update imbolc-ui imports to use imbolc-types directly
4. Remove re-export from imbolc-core
