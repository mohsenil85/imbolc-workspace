> **Status: Implemented**
>
> Implemented in `imbolc-core/src/audio/engine/voices.rs` + `imbolc-core/src/audio/engine/routing.rs` (Option 2). This document predates the current instrument naming; read legacy "module" references as "instrument".

# Polyphonic Voice Allocation

## Problem

Each MIDI module currently maps to one static SuperCollider synth node
created during `rebuild_routing()`. The piano roll sets `freq`, `vel`,
and `gate` on that single node. This is monophonic — overlapping notes
retrigger the same synth, cutting off whatever was sounding.

## Design: Option 2 — Pre-allocate buses, spawn voices on demand

The routing topology (buses, groups, connections) remains static and is
set up during `rebuild_routing()`. But MIDI modules no longer get a
persistent synth node. Instead, voices are spawned with `/s_new` on
note-on and released on note-off, exactly like a MIDI keyboard plugged
into SuperCollider.

### Why this approach

- **Matches real MIDI behavior.** A MIDI keyboard doesn't pre-allocate
  voices. A note-on spawns a voice, note-off releases it. The latency
  of `/s_new` is one audio buffer (~1.5-3ms at 64-128 samples /
  44.1kHz), well below the human perception threshold of ~10ms.
- **No wasted resources.** Static nodes sit on the DSP graph burning
  CPU even when silent. Spawned voices only exist while sounding.
- **Natural polyphony.** Each overlapping note is a separate synth
  instance. Chords just work.
- **Timestamped bundles still apply.** We already send OSC bundles with
  NTP timetags for sub-frame accuracy. `/s_new` inside a bundle is
  scheduled at the same sample-accurate precision as `/n_set`.

### What stays the same

- `resolve_routing()` still allocates buses for all modules (including
  MIDI modules' control output buses: freq, gate, vel)
- `topological_sort()` still determines execution order
- Groups (sources → processing → output) still exist
- The downstream chain (oscillators, filters, effects, output) is still
  pre-created with static nodes that read from buses
- `rebuild_routing()` still tears down and recreates everything when
  the rack topology changes (add/remove module, connect/disconnect)

### What changes

#### 1. `rebuild_routing()` skips MIDI module node creation

Currently line 503-512 in `engine.rs` creates a synth for every module
in topological order. Change this to skip `ModuleType::Midi`:

```rust
for module_id in sorted_modules {
    if let Some(module) = rack.modules.get(&module_id) {
        if module.module_type == ModuleType::Midi {
            // Don't create a static node — voices spawned on demand
            // But DO store the bus assignment for later use
            continue;
        }
        let assignment = assignments.get(&module_id).cloned().unwrap_or_default();
        self.create_synth_with_routing(module_id, module.module_type, &module.params, &assignment)?;
    }
}
```

#### 2. Store bus assignments for MIDI modules

Add a field to `AudioEngine`:

```rust
/// Bus assignments for MIDI modules (needed to spawn voices with correct routing)
midi_bus_assignments: HashMap<ModuleId, BusAssignment>,
```

Populated during `rebuild_routing()`:

```rust
// After resolve_routing(), before the synth creation loop:
self.midi_bus_assignments.clear();
for (&module_id, assignment) in &assignments {
    if let Some(module) = rack.modules.get(&module_id) {
        if module.module_type == ModuleType::Midi {
            self.midi_bus_assignments.insert(module_id, assignment.clone());
        }
    }
}
```

#### 3. Voice map: track active voices

```rust
/// Active voices: (module_id, pitch) -> node_id
voice_map: HashMap<(ModuleId, u8), i32>,
```

This maps each sounding note to its SC node ID, keyed by (module,
pitch) so we can target the exact node on note-off.

#### 4. New engine methods: `spawn_voice` / `release_voice`

```rust
/// Spawn a new synth voice for a MIDI module note-on.
/// Returns the node_id of the created synth.
pub fn spawn_voice(
    &mut self,
    module_id: ModuleId,
    pitch: u8,
    velocity: f32,
    offset_secs: f64,
) -> Result<i32, String> {
    let client = self.client.as_ref().ok_or("Not connected")?;
    let bus_assignment = self.midi_bus_assignments.get(&module_id)
        .ok_or_else(|| format!("No bus assignment for MIDI module {}", module_id))?
        .clone();

    let node_id = self.next_node_id;
    self.next_node_id += 1;

    let freq = 440.0 * (2.0_f64).powf((pitch as f64 - 69.0) / 12.0);

    // Build params: freq, vel, gate + bus routing
    let mut params: Vec<(String, f32)> = vec![
        ("note".into(), pitch as f32),
        ("freq".into(), freq as f32),
        ("vel".into(), velocity),
        ("gate".into(), 1.0),
    ];

    // Add control output bus assignments (freq_out, gate_out, vel_out)
    for (port_name, bus) in &bus_assignment.control_outs {
        params.push((format!("{}_out", port_name), *bus as f32));
    }

    // Create synth in the sources group via a timestamped bundle
    let time = super::osc_client::osc_time_from_now(offset_secs);
    let mut args: Vec<rosc::OscType> = vec![
        rosc::OscType::String("imbolc_midi".into()),
        rosc::OscType::Int(node_id),
        rosc::OscType::Int(1),              // addToTail
        rosc::OscType::Int(GROUP_SOURCES),
    ];
    for (name, value) in &params {
        args.push(rosc::OscType::String(name.clone()));
        args.push(rosc::OscType::Float(*value));
    }
    let msg = rosc::OscMessage { addr: "/s_new".into(), args };
    client.send_bundle(vec![msg], time).map_err(|e| e.to_string())?;

    // Track the voice
    self.voice_map.insert((module_id, pitch), node_id);

    Ok(node_id)
}

/// Release a voice (gate off → envelope release → free).
pub fn release_voice(
    &mut self,
    module_id: ModuleId,
    pitch: u8,
    offset_secs: f64,
) -> Result<(), String> {
    let client = self.client.as_ref().ok_or("Not connected")?;

    if let Some(node_id) = self.voice_map.remove(&(module_id, pitch)) {
        let time = super::osc_client::osc_time_from_now(offset_secs);

        // gate=0 triggers the envelope release
        client.set_params_bundled(node_id, &[("gate", 0.0)], time)
            .map_err(|e| e.to_string())?;

        // Schedule node free after release time.
        // Could use /n_set with doneAction=2 in the synthdef instead
        // (see "Automatic Cleanup" below).
    }

    Ok(())
}
```

#### 5. Update `imbolc_midi` synthdef for voice-per-note

The current `imbolc_midi` is a persistent node that writes to control
buses whenever its params change. For voice-per-note, we need it to:

1. Write to control buses on creation (the downstream oscillators read
   from these buses)
2. Release cleanly when gate goes to 0
3. Free itself after release (using `doneAction: 2`)

```supercollider
SynthDef(\imbolc_midi, {
    |freq_out=0, gate_out=0, vel_out=0, note=60, vel=0.8, gate=1|
    var freq = note.midicps;
    var env = EnvGen.kr(
        Env.asr(0.001, 1, 0.01),  // near-instant attack/release
        gate,
        doneAction: 2             // free this node when env finishes
    );
    Out.kr(freq_out, freq);
    Out.kr(gate_out, env);  // envelope-shaped gate, not raw gate
    Out.kr(vel_out, vel);
}).writeDefFile(dir);
```

Key changes from current:
- `gate` defaults to 1 (voice starts sounding immediately on `/s_new`)
- `doneAction: 2` frees the node when the envelope completes after
  gate→0, so we don't need to manually `/n_free`
- Gate output is envelope-shaped rather than raw 0/1, giving a tiny
  anti-click ramp to the downstream oscillators

With `doneAction: 2`, `release_voice()` just sends `gate=0` and the
node cleans itself up. No need to track timing or schedule `/n_free`.

#### 6. Update main.rs playback engine

Replace the current `send_note_on_bundled` / `send_note_off_bundled`
calls with `spawn_voice` / `release_voice`:

```rust
// Note-on
for (module_id, pitch, velocity, duration, note_tick) in &note_ons {
    let ticks_from_now = ...;
    let offset = ticks_from_now * secs_per_tick;
    let vel_f = *velocity as f32 / 127.0;
    let _ = audio_engine.spawn_voice(*module_id, *pitch, vel_f, offset);
    active_notes.push((*module_id, *pitch, *duration));
}

// Note-off
for (module_id, remaining) in &note_offs {
    let offset = *remaining as f64 * secs_per_tick;
    let _ = audio_engine.release_voice(*module_id, *pitch, offset);
}
```

The `active_notes` tracking in main.rs stays the same — it counts down
remaining ticks and triggers release_voice when expired.

#### 7. Cleanup on `rebuild_routing()`

When the rack topology changes, `rebuild_routing()` frees all existing
nodes. It must also free any active voices:

```rust
// In rebuild_routing(), alongside existing node cleanup:
for &node_id in self.voice_map.values() {
    let _ = client.free_node(node_id);
}
self.voice_map.clear();
```

Similarly, on playback stop:

```rust
// In PianoRollPlayStop handler:
for ((module_id, pitch), _) in audio_engine.voice_map.drain() {
    // gate=0 with doneAction:2 handles cleanup
    let _ = audio_engine.release_voice(module_id, pitch, 0.0);
}
active_notes.clear();
```

### Voice Stealing

Without a voice limit, a held sustain pedal or a dense piano roll could
spawn hundreds of nodes. Add a per-module voice cap:

```rust
const MAX_VOICES_PER_MODULE: usize = 16;
```

In `spawn_voice()`, before creating a new node, count active voices for
this module. If at the limit, steal the oldest:

```rust
let active_for_module: Vec<_> = self.voice_map.keys()
    .filter(|(mid, _)| *mid == module_id)
    .cloned()
    .collect();

if active_for_module.len() >= MAX_VOICES_PER_MODULE {
    // Steal oldest (first inserted — HashMap doesn't guarantee order,
    // so use a Vec<(ModuleId, u8, i32, Instant)> instead for LRU)
    let oldest = active_for_module[0];
    self.release_voice(oldest.0, oldest.1, 0.0)?;
}
```

For proper LRU stealing, replace `voice_map` with:

```rust
/// Active voices in insertion order for LRU stealing
voice_list: Vec<VoiceEntry>,
```

```rust
struct VoiceEntry {
    module_id: ModuleId,
    pitch: u8,
    node_id: i32,
}
```

Use a `Vec` so oldest voices are at the front. `spawn_voice` pushes to
back, `release_voice` removes by (module_id, pitch). Stealing pops from
front (filtered to same module).

### Control Bus Contention

This is the one subtlety. Currently, the MIDI module writes freq/gate/vel
to control buses, and downstream oscillators read from those buses.
With polyphony, multiple voices write to the **same** control buses:

```
Voice 1 (C4): Out.kr(freq_bus, 261.6)
Voice 2 (E4): Out.kr(freq_bus, 329.6)
                                        → oscillator reads freq_bus
```

SuperCollider sums control bus writes within a cycle (`Out.kr` adds,
doesn't replace). So the oscillator would see 261.6 + 329.6 = 591.2 Hz
— wrong.

**Solutions:**

**A. One bus set per voice (correct but expensive).**
Allocate separate freq/gate/vel buses for each voice, and spawn a
separate downstream oscillator chain per voice. This is true
voice-per-note polyphony but means the entire signal chain is
duplicated per voice.

**B. Use `ReplaceOut.kr` and accept last-write-wins.**
Change `Out.kr` to `ReplaceOut.kr` in the MIDI synthdef. The last voice
created wins — newer notes override older ones on the bus. This is
still monophonic in effect (oscillator follows latest note) but voices
can overlap their release tails. Good enough for many use cases.

**C. Skip the MIDI node entirely for polyphonic playback.**
Instead of spawning a `imbolc_midi` synth that writes to control buses,
have the playback engine directly set `freq`/`gate`/`vel` on the
downstream oscillator nodes. This sidesteps the bus contention entirely
but only works for piano roll playback (not for real-time MIDI input
that goes through the MIDI module).

**D. Full voice allocation: spawn the entire chain per voice.**
This is what real polyphonic synths do. Each note-on spawns a complete
chain: MIDI → Oscillator → Filter → ... → writes to a shared output
bus. Effects and output remain shared (they sum audio, which is
correct). This requires:
- A "voice template" derived from the module graph
- Per-voice bus allocation for internal chain connections
- Shared buses only at the effect/output stage

**Recommended: Start with B, evolve to D.**

Option B (`ReplaceOut.kr`) is a one-line synthdef change and gives us
polyphonic release tails immediately. The oscillator always follows the
newest note, but older notes fade out naturally through their envelope
release. This matches how many monosynths with "legato" mode behave.

Option D is the correct long-term solution but is a much larger
refactor — it requires the engine to understand the module graph as a
"voice template" and instantiate entire subgraphs per note.

### Implementation Order

1. **Store `midi_bus_assignments` in engine** — small, no behavior
   change
2. **Skip MIDI node creation in `rebuild_routing()`** — breaks current
   playback (do with step 3)
3. **Add `spawn_voice` / `release_voice`** — restores playback with
   monophonic `ReplaceOut.kr`
4. **Update `imbolc_midi` synthdef** — `gate=1` default, `doneAction:
   2`, `ReplaceOut.kr`
5. **Update main.rs playback** — swap `send_note_on_bundled` →
   `spawn_voice`
6. **Add voice stealing** — safety cap at 16 voices
7. **Cleanup in `rebuild_routing()`** — free active voices
8. **Test** — overlapping notes in piano roll should produce release
   tails

### What This Doesn't Solve

- **True polyphonic oscillators.** The downstream oscillator is still
  one static node reading from one set of control buses. Two
  simultaneous notes with different pitches won't produce two
  frequencies from one oscillator. For that, you need Option D (voice
  chains).
- **Per-note filter/effect state.** If a note passes through a filter,
  all voices share that filter instance. Separate filter states per
  voice again requires Option D.
- **MIDI input from external controllers.** This design only covers
  piano roll playback. Real-time MIDI input would use the same
  `spawn_voice` / `release_voice` path but triggered from an external
  MIDI event listener instead of the tick engine.

These are all solvable with Option D but are separate, larger efforts.
