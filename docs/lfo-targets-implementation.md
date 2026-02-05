> **Status: Partially implemented**
>
> The `LfoTarget` enum lives in `imbolc-core/src/state/instrument.rs`. Only `FilterCutoff` is wired today (in `imbolc-core/src/audio/engine/routing.rs`); the remaining targets are defined but not yet routed.

# LFO Target Implementation Plan

This document outlines how to wire up each of the 15 LFO targets defined in `imbolc-core/src/state/instrument.rs`.

## Current Status

- **FilterCutoff**: DONE - wired up in audio engine
- **14 remaining targets**: Defined in enum, not yet wired

## Implementation Pattern

Each target follows the same pattern:

### 1. SynthDef Modification (synthdefs/compile.scd)

Add a `*_mod_in` parameter that accepts a control bus:

```supercollider
SynthDef(\imbolc_example, { |out=1024, some_param=0.5, some_param_mod_in=(-1)|
    // Read from bus if connected, otherwise 0
    var mod = Select.kr(some_param_mod_in >= 0, [0, In.kr(some_param_mod_in)]);
    // Apply modulation (additive, multiplicative, or scaled)
    var finalParam = some_param + mod;  // or * (1 + mod), etc.
    // ... rest of synth
}).writeDefFile(dir);
```

### 2. Rust Audio Engine (imbolc-core/src/audio/engine/routing.rs)

In `rebuild_instrument_routing()`, connect the LFO bus to the target:

```rust
// When spawning the target synth, check if LFO targets it
if instrument.lfo.enabled && instrument.lfo.target == LfoTarget::SomeParam {
    if let Some(bus) = lfo_control_bus {
        params.push(("some_param_mod_in".to_string(), bus as f32));
    }
}
```

### 3. Recompile SynthDefs

Run `synthdefs/compile.scd` in SuperCollider to regenerate `.scsyndef` files.

---

## Target Implementation Details

### Tier 1: Easy (Additive Modulation)

#### FilterResonance
- **SynthDef**: `imbolc_lpf`, `imbolc_hpf`, `imbolc_bpf`
- **Parameter**: Add `res_mod_in=(-1)`
- **Modulation**: `resonance + mod` (clamp 0-1)
- **Notes**: Already similar structure to cutoff_mod_in

```supercollider
// In each filter SynthDef
var resMod = Select.kr(res_mod_in >= 0, [0, In.kr(res_mod_in)]);
var finalRes = (resonance + resMod).clip(0, 1);
```

#### Pan
- **SynthDef**: `imbolc_output`
- **Parameter**: Add `pan_mod_in=(-1)`
- **Modulation**: `pan + mod` (clamp -1 to 1)

```supercollider
var panMod = Select.kr(pan_mod_in >= 0, [0, In.kr(pan_mod_in)]);
var finalPan = (pan + panMod).clip(-1, 1);
var panned = Balance2.ar(sig[0], sig[1], finalPan);
```

#### DelayFeedback
- **SynthDef**: `imbolc_delay`
- **Parameter**: Add `feedback_mod_in=(-1)`
- **Modulation**: `feedback + mod` (clamp 0-1)

```supercollider
var fbMod = Select.kr(feedback_mod_in >= 0, [0, In.kr(feedback_mod_in)]);
var finalFb = (feedback + fbMod).clip(0, 1);
var delayed = CombL.ar(sig, 2.0, time, finalFb * 4);
```

#### ReverbMix
- **SynthDef**: `imbolc_reverb`
- **Parameter**: Add `mix_mod_in=(-1)`
- **Modulation**: `mix + mod` (clamp 0-1)

```supercollider
var mixMod = Select.kr(mix_mod_in >= 0, [0, In.kr(mix_mod_in)]);
var finalMix = (mix + mixMod).clip(0, 1);
var wet = FreeVerb2.ar(sig[0], sig[1], finalMix, room, damp);
```

#### SendLevel
- **SynthDef**: `imbolc_send`
- **Parameter**: Add `level_mod_in=(-1)`
- **Modulation**: `level + mod` (clamp 0-1)

```supercollider
var levelMod = Select.kr(level_mod_in >= 0, [0, In.kr(level_mod_in)]);
var finalLevel = (level + levelMod).clip(0, 1);
Out.ar(out, sig * finalLevel);
```

---

### Tier 2: Medium (Multiplicative Modulation)

#### Amplitude
- **SynthDef**: `imbolc_saw`, `imbolc_sin`, `imbolc_sqr`, `imbolc_tri`, `imbolc_sampler`
- **Parameter**: Add `amp_mod_in=(-1)`
- **Modulation**: `amp * (1 + mod)` - LFO depth controls tremolo intensity

```supercollider
var ampMod = Select.kr(amp_mod_in >= 0, [0, In.kr(amp_mod_in)]);
var finalAmp = amp * (1 + ampMod).max(0);  // Prevent negative amp
var sig = Saw.ar(freqSig) * finalAmp * velSig;
```

#### GateRate
- **SynthDef**: `imbolc_gate`
- **Parameter**: Add `rate_mod_in=(-1)`
- **Modulation**: `rate * (1 + mod)` - meta-modulation!

```supercollider
var rateMod = Select.kr(rate_mod_in >= 0, [0, In.kr(rate_mod_in)]);
var finalRate = rate * (1 + rateMod).max(0.1);
var sine = SinOsc.kr(finalRate).range(1 - depth, 1);
// ... etc for other shapes
```

#### SampleRate (Scratching!)
- **SynthDef**: `imbolc_sampler`
- **Parameter**: Add `rate_mod_in=(-1)`
- **Modulation**: `rate * (1 + mod)` - enables vinyl scratching effect

```supercollider
var rateMod = Select.kr(rate_mod_in >= 0, [0, In.kr(rate_mod_in)]);
var finalRate = rateSig * pitchRate * (1 + rateMod);
// Note: Can go negative for reverse playback if mod < -1
```

---

### Tier 3: Pitch/Time (Exponential Scaling)

#### Pitch
- **SynthDef**: All oscillators
- **Parameter**: Add `pitch_mod_in=(-1)`
- **Modulation**: `freq * 2.pow(mod)` - mod of 1 = octave up, -1 = octave down

```supercollider
var pitchMod = Select.kr(pitch_mod_in >= 0, [0, In.kr(pitch_mod_in)]);
// Scale: depth of 0.5 gives ~6 semitones swing
var finalFreq = freqSig * (2 ** pitchMod);
```

**Important**: Pitch modulation should be subtle by default. The LFO depth maps to semitones or octaves.

#### Detune
- **SynthDef**: All oscillators
- **Parameter**: Add `detune_mod_in=(-1)`
- **Modulation**: Smaller pitch offset, typically +/- cents

```supercollider
var detuneMod = Select.kr(detune_mod_in >= 0, [0, In.kr(detune_mod_in)]);
// Detune in cents: 100 cents = 1 semitone
var detuneRatio = 2 ** (detuneMod * 0.01);  // mod of 1 = 1 cent
var finalFreq = freqSig * detuneRatio;
```

**Note**: Could combine with Pitch using a scaling factor, but separate targets give more control.

#### DelayTime
- **SynthDef**: `imbolc_delay`
- **Parameter**: Add `time_mod_in=(-1)`
- **Modulation**: `time * (1 + mod)` with clamp

```supercollider
var timeMod = Select.kr(time_mod_in >= 0, [0, In.kr(time_mod_in)]);
var finalTime = (time * (1 + timeMod)).clip(0.001, 2.0);
var delayed = CombL.ar(sig, 2.0, finalTime, feedback * 4);
```

**Warning**: Rapid delay time changes cause pitch artifacts (flanging). This can be a feature!

---

### Tier 4: Unusual (Envelope Modulation)

#### Attack / Release
- **SynthDef**: All oscillators
- **Parameter**: Add `attack_mod_in=(-1)`, `release_mod_in=(-1)`
- **Modulation**: `attack * (1 + mod)` etc.

```supercollider
var attackMod = Select.kr(attack_mod_in >= 0, [0, In.kr(attack_mod_in)]);
var releaseMod = Select.kr(release_mod_in >= 0, [0, In.kr(release_mod_in)]);
var finalAttack = (attack * (1 + attackMod)).max(0.001);
var finalRelease = (release * (1 + releaseMod)).max(0.001);
var env = EnvGen.kr(Env.adsr(finalAttack, decay, sustain, finalRelease), gateSig);
```

**Note**: These only affect NEW notes, not currently playing ones. The envelope is set at note-on.

#### PulseWidth
- **SynthDef**: `imbolc_sqr` only
- **Parameter**: Add `width_mod_in=(-1)`
- **Modulation**: `0.5 + mod` (clamp 0.01-0.99)

```supercollider
var widthMod = Select.kr(width_mod_in >= 0, [0, In.kr(width_mod_in)]);
var finalWidth = (0.5 + widthMod).clip(0.01, 0.99);
var sig = Pulse.ar(freqSig, finalWidth) * amp * velSig;
```

**Note**: Classic PWM (pulse width modulation) sound!

---

## Rust Wiring Pattern

In `imbolc-core/src/audio/engine/routing.rs`, the `rebuild_instrument_routing()` function spawns synths. For each target:

```rust
// Example: Adding amplitude modulation to oscillators
fn spawn_osc_voice(&mut self, ..., lfo_bus: Option<i32>, lfo_target: LfoTarget) {
    let mut params = vec![
        ("out".to_string(), audio_bus as f32),
        ("freq_in".to_string(), freq_bus as f32),
        // ... other params
    ];

    // Wire LFO if targeting amplitude
    if let Some(bus) = lfo_bus {
        if lfo_target == LfoTarget::Amplitude {
            params.push(("amp_mod_in".to_string(), bus as f32));
        }
    }

    // Create synth...
}
```

For targets that affect multiple synth types (like Amplitude affecting all oscillators), you need to pass the LFO bus through the voice spawning chain.

---

## Suggested Implementation Order

1. **FilterResonance** - Nearly identical to existing FilterCutoff
2. **Pan** - Simple, immediate audio feedback
3. **Amplitude** - Classic tremolo, affects all oscs
4. **DelayFeedback** + **ReverbMix** - Easy effect params
5. **GateRate** - Fun meta-modulation
6. **SampleRate** - Enables scratching!
7. **Pitch** - More complex but high value
8. **SendLevel** - Useful for ducking
9. **DelayTime** - Interesting flanging effects
10. **PulseWidth** - Only affects square wave
11. **Detune** - Subtle, similar to pitch
12. **Attack/Release** - Unusual, lower priority

---

## Testing Each Target

For each target:

1. Create an instrument with appropriate source (e.g., square wave for PulseWidth)
2. Enable LFO, set target
3. Start with slow rate (0.5 Hz), moderate depth (0.5)
4. Play notes and verify modulation is audible
5. Test extreme values (very fast, very deep)
6. Verify no audio glitches or crashes

### Test Cases

| Target | Best Source | What to Listen For |
|--------|-------------|-------------------|
| FilterCutoff | Any + LPF | Wah-wah sweep |
| FilterResonance | Any + LPF | Resonance swells |
| Amplitude | Any | Tremolo effect |
| Pitch | Any | Vibrato/siren |
| Pan | Any | Auto-pan L/R |
| PulseWidth | Square only | Classic PWM |
| SampleRate | Sampler | Scratching/warping |
| DelayTime | Any + Delay | Flanging/chorus |
| DelayFeedback | Any + Delay | Echo swells |
| ReverbMix | Any + Reverb | Wet/dry sweep |
| GateRate | Any + Gate | Variable speed chop |
| SendLevel | Instrument w/send | Ducking effect |
| Detune | Any | Subtle warble |
| Attack | Any (new notes) | Variable attack |
| Release | Any (note off) | Variable release |

---

## Potential Issues

### Node Ordering
LFO synth must run BEFORE the target synth in SuperCollider's node tree. The current implementation adds LFO early in the chain, which should be fine for most targets.

### Control Rate Timing
LFO outputs at control rate (kr). All `*_mod_in` params should use `In.kr()`, not `In.ar()`.

### Value Ranges
Some parameters have different ranges:
- Pan: -1 to 1
- Most others: 0 to 1
- Frequency: 20 to 20000 Hz

The LFO depth should be scaled appropriately. Currently, depth is 0-1 and the mod signal is -depth to +depth. Targets may need internal scaling.

### Per-Voice vs Global
Currently LFO is per-instrument, affecting all voices. For per-voice LFO (each note gets its own), we'd need to spawn LFO synths per voice. That's a larger architectural change - defer for now.

---

## Code Checklist

For each target, update:

- [ ] `synthdefs/compile.scd` - Add `*_mod_in` param to relevant SynthDef(s)
- [ ] Run compile.scd in SuperCollider to regenerate .scsyndef files
- [ ] `imbolc-core/src/audio/engine/routing.rs` - Wire LFO bus when target matches
- [ ] Test with actual audio

No changes needed to:
- `imbolc-core/src/state/instrument.rs` - Targets already defined
- `src/panes/instrument_edit_pane` - UI already shows all targets
- `imbolc-core/src/state/persistence/mod.rs` - Already saves/loads all targets
