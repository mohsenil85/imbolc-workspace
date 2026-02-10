# Audio Routing Architecture

## How Signal Flow Works in a DAW

### The Bus Model

All audio in a DAW flows through **buses** - shared channels that modules read from and write to. No module talks directly to another. Instead:

1. A module **writes** its output to a bus
2. Another module **reads** from that bus
3. The bus is just a buffer of audio samples that gets summed and cleared each cycle

SuperCollider (our audio engine) handles this natively. `Out.ar(bus, signal)` adds to a bus, `In.ar(bus, numChannels)` reads from it. Multiple writers to the same bus are automatically summed together.

```
┌──────────┐     bus 16      ┌──────────┐     bus 18      ┌──────────┐
│  SawOsc  │ ──Out.ar(16)──> │   LPF    │ ──Out.ar(18)──> │  Output  │ ──Out.ar(0)──> speakers
└──────────┘                 └──────────┘                 └──────────┘
                              In.ar(16)                    In.ar(18)
```

### Bus Types

**Audio buses** carry audio-rate signals (44100 samples/sec). Used for:
- Source output
- Filter chains
- Effects
- Final output to hardware

**Control buses** carry control-rate signals (lower rate, ~689 samples/sec). Used for:
- Frequency (from MIDI or LFO)
- Gate (note on/off)
- Velocity
- Modulation (LFO → filter cutoff/resonance, amplitude, pitch, pan, effects, etc.)

In our layout:
- Audio buses 0-15: Reserved for hardware I/O (0-1 = stereo out)
- Audio buses 16+: Private buses allocated for module routing
- Control buses 0+: Private buses for control signals

### Node Ordering

SuperCollider processes synths in order on the server. A synth reading from a bus must come **after** the synth writing to it, otherwise it reads stale data from the previous cycle (one-sample delay).

We handle this with **groups + deterministic creation order**: sources, then processing, then output, then bus processing. This guarantees bus writers run before readers within the same audio block.

```
Execution order:
  1. imbolc_midi      (writes freq/gate/vel to control buses)
  2. Source synth     (reads control buses, writes audio to bus 16)
  3. Filter/EQ/FX     (reads bus 16, writes to bus 18)
  4. Output synth     (reads bus 18, writes to hardware bus 0)
```

## Insert vs Send Routing

### Insert (Implemented)

An insert is a **serial** connection. The signal passes through each module in sequence:

```
Source ──> Filter ──> Delay ──> Output
```

Each module's output is the next module's input. The entire signal goes through the chain. This is what our connection system does.

### Send (Implemented)

A send is a **parallel** copy. The signal goes to its main destination AND a copy goes to an effects bus:

```
                    ┌──> main out
Source ──> Filter ──┤
                    └──> Reverb bus (at -6dB)
                              │
                              v
                         Reverb ──> main out
```

Key differences from inserts:
- The original signal continues unaffected
- The send has its own level control (how much signal to send)
- Multiple channels can send to the same effect (one reverb shared by all)
- The effect's output gets mixed back into the main output

This is how real mixers work - you don't put a separate reverb on every channel. You have one reverb on a bus, and each channel sends a portion of its signal to it.

### How Sends Work in SuperCollider

A send is just an additional `Out.ar` in the source synth:

```supercollider
// Source with a send
SynthDef(\source_with_send, { |out=16, send_bus=20, send_level=0|
    var sig = Saw.ar(440) * 0.5;
    Out.ar(out, sig);                           // main output (insert chain)
    Out.ar(send_bus, sig * send_level);         // send (parallel copy)
});
```

The reverb on bus 20 processes whatever lands there, and its output goes to the main bus. Because `Out.ar` sums, multiple channels sending to bus 20 all mix together into one reverb.

In imbolc, each instrument has per-bus sends (`instrument.sends`) with a tap point (`PreInsert` or `PostInsert`). The audio engine creates `imbolc_send` nodes that read from the selected tap bus (pre- or post-insert) and write into the target bus audio bus. Layer groups use the same pattern in the bus-processing stage.

## Mixer Architecture

### Channel Strip

Each mixer channel corresponds to one Output module and controls:

| Parameter | What It Does |
|-----------|-------------|
| Level     | Volume fader (0.0 - 1.0) |
| Pan       | Stereo position (-1.0 to 1.0) |
| Mute      | Silences the channel |
| Solo      | Silences all non-soloed channels |
| Output    | Where this channel routes to (master or a bus) |

Layer groups aggregate multiple instruments into a shared mixer strip. A layer group has its own effects/EQ, sends, and output target (master or bus). Instruments assigned to a group route into the group bus before bus/master processing.

### Signal Flow Through the Mixer

```

If an instrument is part of a layer group, its output first goes to the layer group's processing bus. The group output then routes to master or a bus, just like an instrument output.
Channel fader → Pan → Mute/Solo logic → Output target
                                              │
                                    ┌─────────┼──────────┐
                                    v         v          v
                                 Master    Bus 1      Bus 2
                                    │         │          │
                                    v         v          v
                                 Master    Bus fader  Bus fader
                                 fader     + mute     + mute
                                    │         │          │
                                    └────┬────┘──────────┘
                                         v
                                    Hardware out
```

### What Gets Sent to SuperCollider

The Output synth receives the **effective** level, which combines channel and master:

```
effective_level = channel.level * master.level
effective_mute  = channel.mute OR master.mute
```

These are set via OSC `/n_set` messages to the Output synth's `level` and `mute` controls.

### Solo Logic (Implemented)

Solo is more complex. When any channel is soloed:
- Soloed channels play normally
- All non-soloed channels are muted
- This requires checking `any_solo()` across all channels and conditionally muting

```
if any_channel_is_soloed:
    effective_mute = NOT channel.solo
else:
    effective_mute = channel.mute OR master.mute
```

## Summing

When multiple modules connect to the same input, their signals sum. This happens naturally in SuperCollider because `Out.ar` adds to a bus rather than replacing it.

```
SawOsc ──┐
          ├──> Filter input (bus 16)    ← both signals are summed
SinOsc ──┘
```

The bus gets cleared to zero at the start of each cycle, then each `Out.ar` adds to it. By the time the filter reads from bus 16, it sees the combined signal.

This is important for:
- Layering oscillators
- Mixing multiple sources into one effect
- Send buses receiving from multiple channels

## Groups (Implemented)

SuperCollider has **groups** - containers for synths that control execution order. imbolc uses fixed groups to guarantee ordering:

- Group 100: sources (oscillators, samplers, audio in)
- Group 200: processing (filters, insert FX, mixer processing)
- Group 300: output (instrument outputs, instrument sends)
- Group 350: bus + layer-group processing and bus outputs
- Group 400: recording nodes
- Group 999: safety limiter / analysis

```
Group 1 (sources):
  MIDI synth
  SawOsc synth
  SinOsc synth

Group 2 (processing):
  LPF synth
  Delay synth

Group 3 (output):
  Reverb synth (on send bus)
  Output synth

Group order guarantees: sources → processing → output → bus processing
```

## References

- [SuperCollider Bus documentation](https://doc.sccode.org/Classes/Bus.html)
- [SuperCollider Order of Execution](https://doc.sccode.org/Guides/Order-of-execution.html)
- [SuperCollider Groups](https://doc.sccode.org/Classes/Group.html)
- [Sound on Sound: Understanding Aux Sends](https://www.soundonsound.com/techniques/using-auxiliary-sends)
