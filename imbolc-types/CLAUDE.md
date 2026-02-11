# imbolc-types

Shared type definitions for the Imbolc DAW ecosystem.

## What This Is

The leaf crate of the Imbolc workspace. Contains data structures used across imbolc-core, imbolc-ui, and imbolc-net. No internal dependencies on other Imbolc crates.

## Directory Structure

```
src/
  lib.rs              — Re-exports, type aliases (InstrumentId, EffectId, etc.)
  action.rs           — Action enum for UI -> core dispatch
  param.rs            — Param, ParamValue, frequency helpers
  audio.rs            — AudioFeedback, ServerStatus, ExportKind
  reduce/             — Pure state-mutation reducers (single source of truth)
    mod.rs              — reduce_action(), is_reducible()
    instrument.rs       — Instrument action reducer + initialize_instrument_from_registries()
    mixer.rs            — Mixer action reducer
    piano_roll.rs       — Piano roll action reducer
    automation.rs       — Automation action reducer
    bus.rs              — Bus/layer group action reducer
    session.rs          — Session action reducer
    vst_param.rs        — VST param action reducer
    click.rs            — Click track reducer
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
| `Instrument` | `state/instrument/mod.rs` | Source + processing chain + LFO + envelope + mixer |
| `SourceType` | `state/instrument/source_type.rs` | Oscillator types: Saw, Sin, AudioIn, BusIn, Sampler, VST, etc. |
| `EffectSlot` | `state/instrument/effect.rs` | One effect in chain: type + params + enabled + VST state |
| `FilterType` | `state/instrument/filter.rs` | Filter types: LowPass, HighPass, BandPass, Notch |
| `Action` | `action.rs` | Enum of all actions dispatched from UI to core |
| `Param` / `ParamValue` | `param.rs` | Generic parameter with Float/Int/Bool values |
| `InstrumentState` | `state/instrument_state.rs` | Collection of instruments + selection state |
| `SessionState` | `state/session.rs` | Global session: arrangement, mixer, automation, transport |
| `PianoRollState` | `state/piano_roll.rs` | Tracks, notes, grid settings |
| `AutomationState` | `state/automation.rs` | Automation lanes and points |
| `DrumSequencerState` | `state/drum_sequencer.rs` | Drum patterns and steps |

## Type Composition Hierarchy

```
AppState (defined in imbolc-core, composed of types from here)
├── session: SessionState
│   ├── Musical: key, scale, bpm, tuning_a4, snap, time_signature
│   ├── piano_roll: PianoRollState
│   ├── arrangement: ArrangementState
│   ├── automation: AutomationState
│   ├── mixer: MixerState
│   │   ├── buses: Vec<MixerBus> (effects, level, pan, sends)
│   │   ├── master_level, master_mute
│   │   ├── selection: MixerSelection
│   │   └── layer_group_mixers: Vec<LayerGroupMixer>
│   └── humanize, click_track, theme
│
└── instruments: InstrumentState
    └── instruments: Vec<Instrument>
        ├── source: SourceType + source_params
        ├── processing_chain: Vec<ProcessingStage> (filters/EQ/effects)
        ├── lfo: LfoConfig, amp_envelope: EnvConfig
        ├── Mixer: level, pan, mute, solo, output_target, channel_config, sends
        ├── sampler_config, drum_sequencer (source-dependent)
        ├── arpeggiator, chord_shape
        ├── convolution_ir_path
        ├── layer_group: Option<u32>, layer_octave_offset
        ├── next_effect_id
        └── groove: GrooveConfig
```

## Key Enum Categories

### SourceType (built-ins + Custom/VST)

- **Oscillators**: Saw, Sin, Sqr, Tri, Noise, Pulse, SuperSaw, Sync
- **FM/Modulation**: Ring, FBSin, FM, PhaseMod, FMBell, FMBrass
- **Physical Models**: Pluck, Formant, Bowed, Blown, Membrane
- **Mallets**: Marimba, Vibes, Kalimba, SteelDrum, TubularBell, Glockenspiel
- **Strings**: Guitar, BassGuitar, Harp, Koto
- **Drums**: Kick, Snare, HihatClosed, HihatOpen, Clap, Cowbell, Rim, Tom, Clave, Conga
- **Classic Synths**: Choir, EPiano, Organ, BrassStab, Strings, Acid
- **Experimental**: Gendy, Chaos
- **Synthesis**: Additive, Wavetable, Granular
- **Routing**: AudioIn, BusIn
- **Samplers**: PitchedSampler, TimeStretch, Kit
- **External**: Custom(CustomSynthDefId), Vst(VstPluginId)

### EffectType (built-ins + VST)

- **Time**: Delay, Reverb, SpringReverb
- **Dynamics**: Gate, TapeComp, SidechainComp, Limiter, MultibandComp
- **Modulation**: Chorus, Flanger, Phaser, Tremolo, Autopan, Leslie
- **Distortion**: Distortion, Bitcrusher, Wavefolder, Saturator
- **EQ**: TiltEq, ParaEq
- **Stereo**: StereoWidener, MidSide, Crossfader
- **Pitch**: PitchShifter, Autotune, FreqShifter
- **Granular**: GranularDelay, GranularFreeze
- **Spectral**: SpectralFreeze, Glitch, Denoise
- **Convolution**: ConvolutionReverb
- **Character**: Vinyl, Cabinet
- **Synthesis**: RingMod, Resonator, Vocoder
- **Envelope**: EnvFollower, WahPedal
- **External**: Vst(VstPluginId)

### Other Key Enums

- **MixerSelection**: `Instrument(usize)` | `LayerGroup(u32)` | `Bus(u8)` | `Master`
- **OutputTarget**: `Master` | `Bus(u8)`
- **FilterType**: LowPass, HighPass, BandPass, Notch
- **LfoShape**: Sin, Tri, Saw, Sqr, SampleAndHold, Random
- **SendTapPoint**: PreInsert, PostInsert

## Code Navigation

When cclsp MCP tools are available, prefer them over grep for navigating Rust code. See workspace root CLAUDE.md for details.

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
