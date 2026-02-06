# SynthDef Definitions

This directory contains all SuperCollider SynthDef source files for imbolc.

## Hard Rule: One SynthDef Per File

**Every `.scd` file must contain exactly one SynthDef.** No exceptions.

- File name must match the SynthDef name: `imbolc_kick.scd` contains `\imbolc_kick`
- This makes SynthDefs easy to find, edit, and version control
- The compile script automatically loads all files from subdirectories

## Directory Structure

```
defs/
├── oscillators/      Basic waveforms (saw, sin, sqr, tri, pulse, noise, supersaw, sync)
├── synthesis/        Synthesis techniques (fm, granular, additive, wavetable, gendy)
├── physical_models/  Physical modeling (bowed, blown, guitar, marimba, kalimba)
├── drums/            Drum sounds (kick, snare, hihat, clap, tom, cowbell)
├── classic_synths/   Classic emulations (organ, epiano, brass_stab, strings, acid)
├── filters/          Filter effects (lpf, hpf, bpf, notch, comb, allpass, vowel)
├── effects/          Audio effects (delay, reverb, chorus, distortion, phaser)
├── modulation/       Modulation sources (lfo, adsr)
├── eq/               EQ processors
├── input/            Input sources (audio_in, bus_in)
├── output/           Output routing (output, send, bus_out, safety)
├── samplers/         Sample playback (sampler, timestretch)
├── analysis/         Metering (meter, spectrum, lufs_meter, scope)
└── midi/             MIDI output
```

## File Template

```supercollider
// imbolc_example SynthDef
(
var dir = thisProcess.nowExecutingPath.dirname.dirname.dirname;

SynthDef(\imbolc_example, { |out=1024, freq_in=(-1), gate_in=(-1), vel_in=(-1),
    freq=440, amp=0.5, lag=0.02, attack=0.01, decay=0.1, sustain=0.7, release=0.3,
    amp_mod_in=(-1), pitch_mod_in=(-1), detune_mod_in=(-1),
    attack_mod_in=(-1), release_mod_in=(-1), decay_mod_in=(-1), sustain_mod_in=(-1)|

    var freqSig = Select.kr(freq_in >= 0, [freq.lag(lag), In.kr(freq_in)]);
    var gateSig = Select.kr(gate_in >= 0, [1, In.kr(gate_in)]);
    var velSig = Select.kr(vel_in >= 0, [1, In.kr(vel_in)]);
    // ... modulation inputs ...

    var sig = SinOsc.ar(freqSig);
    var env = EnvGen.kr(Env.adsr(attack, decay, sustain, release), gateSig);

    Out.ar(out, (sig * amp * velSig * env) ! 2);
}).writeDefFile(dir);
)
```

## Important: var Declaration Order

SuperCollider requires all `var` declarations at the beginning of a function, before any statements.

**Wrong:**
```supercollider
var sig = SinOsc.ar(freq);
sig = sig * 0.5;           // statement
var env = EnvGen.kr(...);  // ERROR: var after statement
```

**Correct:**
```supercollider
var sig = SinOsc.ar(freq);
var env;                   // declare first
sig = sig * 0.5;
env = EnvGen.kr(...);      // assign later
```

## Compiling

From the `synthdefs/` directory:

```bash
sclang compile.scd
```

This loads all `.scd` files from subdirectories and writes `.scsyndef` files to the synthdefs directory.

## Adding a New SynthDef

1. Create a new file in the appropriate subdirectory
2. Use the template above (note: `dirname.dirname.dirname` goes up 3 levels to write to synthdefs/)
3. Run `sclang compile.scd` to verify it compiles
4. Add a corresponding `SourceType` variant in `imbolc-types/src/state/instrument/source_type.rs` if it's a new instrument type
