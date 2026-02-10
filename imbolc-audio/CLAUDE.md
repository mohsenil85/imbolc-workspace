# imbolc-audio

Audio engine crate for the Imbolc DAW.

## What This Is

The audio subsystem extracted from imbolc-core. Communicates with the rest of the app via well-defined channels (crossbeam, mpsc, triple buffer, event log) — no shared mutable state. Depends only on imbolc-types.

## Directory Structure

```
src/
  lib.rs               — Crate root, AudioStateProvider trait, re-exports
  arp_state.rs          — ArpPlayState (runtime arpeggiator state)
  paths.rs              — SynthDef path resolution
  handle.rs             — AudioHandle (main-thread interface)
  audio_thread.rs       — AudioThread (runs in separate thread)
  commands.rs           — AudioCmd and AudioFeedback enums
  playback.rs           — Playback scheduling, sequencer tick
  action_projection.rs  — State diff → OSC command projection
  snapshot.rs           — State snapshots for audio thread
  triple_buffer.rs      — Lock-free state transfer
  osc_client.rs         — OSC message sending
  osc_sender.rs         — Background OSC sender thread
  bus_allocator.rs      — SC bus allocation
  drum_tick.rs          — Drum sequencer tick
  arpeggiator_tick.rs   — Arpeggiator tick
  click_tick.rs         — Click track tick
  event_log.rs          — Time-ordered event log
  devices.rs            — Audio device enumeration
  engine/
    mod.rs              — AudioEngine state
    backend.rs          — SuperCollider backend
    server.rs           — Server communication
    voices.rs           — Voice management
    voice_allocator.rs  — Polyphonic voice allocation
    routing.rs          — Bus routing
    samples.rs          — Sample loading
    recording.rs        — Audio recording
    automation.rs       — Automation playback
    vst.rs              — VST hosting
    node_registry.rs    — SC node tracking
```

## Key Types

| Type | Purpose |
|------|---------|
| `AudioHandle` | Main-thread API, sends `AudioCmd` via channel |
| `AudioEngine` | SC backend, node map, voice allocator |
| `AudioStateProvider` | Trait for accessing session/instrument state (implemented by AppState in core) |
| `ArpPlayState` | Runtime arpeggiator state (held notes, step index) |

## AudioStateProvider Trait

Breaks the circular dependency between audio and core:

```rust
pub trait AudioStateProvider {
    fn session(&self) -> &SessionState;
    fn instruments(&self) -> &InstrumentState;
}
```

Implemented by `AppState` in imbolc-core.

## Build & Test

```bash
cargo build -p imbolc-audio
cargo test -p imbolc-audio    # 84 tests
```
