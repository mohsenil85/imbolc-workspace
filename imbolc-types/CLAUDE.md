# imbolc-types

Shared type definitions for the Imbolc DAW ecosystem.

## What This Is

The leaf crate of the Imbolc workspace. Contains data structures used across imbolc-core, imbolc-ui, and imbolc-net. No internal dependencies on other Imbolc crates.

## Directory Structure

```
src/
  lib.rs              — Re-exports, type aliases (InstrumentId, EffectId, etc.)
  action.rs           — Action enum for UI -> core dispatch
  dispatch.rs         — Dispatcher trait
  param.rs            — Param, ParamValue, frequency helpers
  audio.rs            — AudioFeedback, ServerStatus, ExportKind
  state/
    mod.rs            — AppState components, re-exports
    instrument/       — Instrument types
      mod.rs            — Instrument struct, Layer
      source_type.rs    — SourceType enum (oscillators, samplers, VST)
      filter.rs         — FilterType, FilterState
      effect.rs         — EffectType, EffectSlot
      envelope.rs       — Envelope, EnvelopePoint
      lfo.rs            — LFO, LfoTarget, LfoShape
    instrument_state.rs — InstrumentState (collection + selection)
    session.rs          — SessionState (global settings, buses, transport)
    piano_roll.rs       — PianoRollState, Track, Note
    automation.rs       — AutomationState, lanes, points
    arrangement.rs      — ArrangementState, clips
    sampler.rs          — SamplerConfig, SampleRegistry, slices
    drum_sequencer.rs   — DrumSequencerState, patterns
    mixer.rs            — MixerState, channel routing
    groove.rs           — GrooveState, swing patterns
    arpeggiator.rs      — ArpeggiatorState, patterns
    music.rs            — Key, Scale, musical theory types
    midi_recording.rs   — MIDI recording state, CC mappings
    custom_synthdef.rs  — CustomSynthDef registry, param specs
    clipboard.rs        — Clipboard for copy/paste
    humanize.rs         — Humanize settings for velocity/timing
    recording.rs        — RecordingState for audio capture
    project.rs          — ProjectState (path, dirty flag)
    io.rs               — IoState (pending save/load/export)
    vst.rs              — VST plugin types
```

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `InstrumentId` | `lib.rs` | `u32` — unique identifier for instruments |
| `Instrument` | `state/instrument/mod.rs` | Source + filter + effects + LFO + envelope + mixer |
| `SourceType` | `state/instrument/source_type.rs` | Oscillator types: Saw, Sin, AudioIn, BusIn, Sampler, VST, etc. |
| `EffectSlot` | `state/instrument/effect.rs` | One effect in chain: type + params + enabled + VST state |
| `FilterType` | `state/instrument/filter.rs` | Filter types: LowPass, HighPass, BandPass, Notch |
| `Action` | `action.rs` | Enum of all actions dispatched from UI to core |
| `Param` / `ParamValue` | `param.rs` | Generic parameter with Float/Int/Bool values |
| `Dispatcher` | `dispatch.rs` | Trait for action dispatch implementations |
| `InstrumentState` | `state/instrument_state.rs` | Collection of instruments + selection state |
| `SessionState` | `state/session.rs` | Global session: buses, mixer, transport, BPM |
| `PianoRollState` | `state/piano_roll.rs` | Tracks, notes, grid settings |
| `AutomationState` | `state/automation.rs` | Automation lanes and points |
| `DrumSequencerState` | `state/drum_sequencer.rs` | Drum patterns and steps |

## Build & Test

```bash
cargo build -p imbolc-types
cargo test -p imbolc-types
```

## Usage from Other Crates

```rust
// In imbolc-core or imbolc-ui
use imbolc_types::{Action, InstrumentId, Instrument, SourceType};
use imbolc_types::state::{SessionState, InstrumentState};
```
