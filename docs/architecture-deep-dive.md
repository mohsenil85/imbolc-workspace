# Imbolc Architecture Deep Dive

This document details the internal architecture of Imbolc, focusing on the low-latency audio engine, concurrency model, and integration with SuperCollider.

## High-Level Overview

Imbolc follows a strict separation of concerns between the **UI (Main Thread)** and the **Audio Engine (Audio Thread)**.

*   **Main Thread:** Runs the TUI (Ratatui), handles user input, manages the "source of truth" `AppState`, and renders the interface.
*   **Audio Thread:** A dedicated thread that runs the sequencer clock, manages the SuperCollider server (scsynth), and handles all real-time audio logic.
*   **SuperCollider (scsynth):** Runs as a subprocess. Performs the actual DSP (Digital Signal Processing). Communications happen via Open Sound Control (OSC) over UDP.

```mermaid
graph TD
    User[User Input] -->|Events| Main[Main Thread (TUI)]
    Main -->|AudioCmd (Priority + Normal Channels)| Audio[Audio Thread]
    Audio -->|AudioFeedback (MPSC)| Main
    Audio -->|OSC Bundles (UDP)| SC[SuperCollider (scsynth)]
    SC -->|OSC Reply (UDP)| Audio
    SC -->|Audio Out| Speakers

    subgraph Shared Memory
    Monitor[AudioMonitor (Atomics + Triple-Buffers)]
    end

    Audio -.->|Writes Meters/Scope| Monitor
    Main -.->|Reads Meters/Scope| Monitor
```

## Threading Model & Audio Processing

The core of Imbolc's timing stability lies in the `AudioThread` loop (`imbolc-core/src/audio/audio_thread.rs`).

### 1. The Audio Thread Loop
The audio thread does not rely on the UI framerate. It runs a tight loop that:
1.  **Priority Command Handling:** Uses `crossbeam-channel` with `select!` to prioritize time-critical commands (voice spawn/release, param changes) over normal commands (state sync, routing rebuilds).
2.  **Drains Commands:** Processes priority commands first, then normal commands.
3.  **Checks Time:** Calculates the precise elapsed `Duration` since the last tick using `std::time::Instant`.
4.  **Ticks:** If sufficient time has passed (>= 0.5ms), it calls `tick()`, converting the elapsed duration into **musical ticks** based on the current BPM.
5.  **Polls Engine:** Checks for server health, compilation results, and recording state changes.

**Priority Channel Design:**
- **Priority Channel (time-critical):** Voice spawn/release, param changes, playback control, automation
- **Normal Channel:** State sync, routing rebuilds, recording control, server lifecycle
- This ensures MIDI input and knob tweaks are processed immediately, even during large state syncs.

### 2. Decoupled Playback Logic
Playback logic is decoupled from wall-clock time. The sequencer uses an **f64 fractional tick accumulator** to convert elapsed wall-clock time into musical ticks. Each cycle, the fractional remainder is preserved rather than truncated:

```rust
*tick_accumulator += elapsed.as_secs_f64() * (bpm / 60.0) * tpb;
let tick_delta = *tick_accumulator as u32;
*tick_accumulator -= tick_delta as f64;
```

This prevents the systematic jitter that truncation would cause. The f64 accumulator provides ~15 digits of precision, avoiding drift over long sessions.

### 3. Low Latency & Jitter Compensation (The "Schedule Ahead" Pattern)
To prevent audible jitter, Imbolc uses **OSC Bundles with Timestamps**.

When a note is triggered:
1.  The sequencer determines the note starts at `tick X`.
2.  It calculates the exact offset in seconds from "now".
3.  It calls `osc_time_from_now(offset)` (`imbolc-core/src/audio/osc_client.rs`), which computes an absolute NTP timestamp.
4.  This timestamp is attached to the OSC bundle sent to SuperCollider.

**The Result:** SuperCollider receives the message *before* the sound needs to play and schedules it for the *exact* sample frame requested.

### Latency Targets

| Metric | Target |
|--------|--------|
| MIDI note → OSC send | <0.5ms p99 |
| Param change → `/n_set` | <1ms p99 |
| Tick interval stddev | <100μs |
| Monitor read latency | <10μs (lock-free) |

### 4. Monotonic Clock for Timetags
OSC timetags use NTP epoch timestamps. Timetags are derived from a **monotonic clock anchor** to prevent clock adjustments (like NTP syncs) from causing glitches during a session:

```rust
// Captured once at init
static CLOCK_ANCHOR: LazyLock<(Instant, f64)> = LazyLock::new(|| {
    let wall = SystemTime::now().duration_since(UNIX_EPOCH)...;
    (Instant::now(), wall)
});
```

### 5. Sequencer Features
The sequencer (`imbolc-core/src/audio/playback.rs`) supports advanced features handled at the tick level:
*   **Loop Boundary Scanning:** Correctly handles notes wrapping around the loop point by scanning two ranges (`[old, end)` and `[start, new]`).
*   **Swing:** Delays offbeat notes by a calculated offset.
*   **Humanization:** Adds random jitter to timing and velocity for a more natural feel.
*   **Probability:** Per-note probability checks before spawning voices.
*   **Arpeggiator:** Handled in `arpeggiator_tick.rs`, converting held notes into rhythmic patterns with sub-step precision.

## Concurrency & State Management

### Command / Feedback Pattern
*   **Main -> Audio:** `Sender<AudioCmd>`. Commands like `SpawnVoice`, `UpdateState`, `SetBpm`, `StartInstrumentRender`.
*   **Shadow State:** The audio thread maintains its own "shadow copy" of relevant state (`InstrumentSnapshot`, `SessionSnapshot`, `PianoRollSnapshot`).
*   **Audio -> Main:** `Sender<AudioFeedback>`. Events like `PlayheadPosition`, `ServerStatus`, `RecordingState`, `VstParamsDiscovered`.

### Shared Monitoring State (Lock-Free)
For high-frequency visual data, `AudioMonitor` uses a hybrid lock-free architecture:

**Atomic Fields (for scalars):**
*   `meter_data: AtomicU64` — Stereo peak meters packed as two f32s
*   `sc_cpu: AtomicU32` — SuperCollider CPU load
*   `osc_latency_ms: AtomicU32` — OSC round-trip latency

**Triple-Buffered Fields (for complex types):**
*   `audio_in_waveforms: TripleBufferHandle<HashMap<...>>` — Per-instrument waveform data
*   `spectrum_data: TripleBufferHandle<[f32; 7]>` — 7-band spectrum analyzer
*   `lufs_data: TripleBufferHandle<(f32, f32, f32, f32)>` — LUFS metering
*   `scope_buffer: TripleBufferHandle<VecDeque<f32>>` — Oscilloscope buffer
*   `vst_params: TripleBufferHandle<HashMap<...>>` — VST parameter discovery replies

**Triple-Buffer Benefits:**
- Writer (OSC thread) never blocks waiting for reader
- Reader (UI thread) gets latest complete frame without tearing
- No mutex contention between threads
- Custom inline implementation (~100 lines) in `triple_buffer.rs`

This design eliminates UI thread contention from blocking the time-critical OSC receive path.

## Audio Engine Internals

The `AudioEngine` acts as the driver for `scsynth`. It manages the node graph, resource allocation, and VST integration.

### Node Management & Routing Graph
Imbolc enforces a strict topological sort using SuperCollider Groups:

1.  **Group 100 (Sources):** Oscillators, Samplers, VST Instruments, Audio Input.
2.  **Group 200 (Processing):** Filters, EQs, Insert Effects.
3.  **Group 300 (Output):** Master bus, Hardware output, Send effects (returns).
4.  **Group 400 (Record):** Disk recording (DiskOut).
5.  **Group 999 (Safety):** Safety limiter to prevent ear-blasting feedback.

### Instrument Signal Chain
Each instrument is built with a deterministic chain of synth nodes:
`Source` -> `LFO` -> `Filter` -> `EQ` -> `Effects (Chain)` -> `Output`

### Voice Allocation
Polyphony is managed by `VoiceAllocator` (`imbolc-core/src/audio/engine/voice_allocator.rs`).
When a note plays:
1.  **Stealing:** If max voices reached, the allocator picks a victim (released voices first, then quietest/oldest).
2.  **Allocation:** A new `Group` is created inside Group 100.
3.  **Chain:** Inside this group, a `imbolc_midi` control node and a `Source` synth are created.
4.  **Pooling:** Control buses (freq, gate, velocity) are pooled and reused to reduce overhead.

### VST Integration
VSTs are hosted inside SuperCollider using `VSTPlugin`.
*   **VST Instruments:** A persistent `imbolc_vst_instrument` synth is created. MIDI events are sent via `/u_cmd` to the VSTPlugin UGen.
*   **VST Effects:** Wrapped in `imbolc_vst_effect`.
*   **State:** VST state (programs) is saved/loaded via temporary files passed to the plugin.

## Key Files Guide

*   `imbolc-core/src/audio/audio_thread.rs`: The main audio loop, command processing, priority channel handling, and tick orchestration.
*   `imbolc-core/src/audio/playback.rs`: Sequencer logic (notes, swing, humanize, probability).
*   `imbolc-core/src/audio/osc_client.rs`: UDP socket, NTP timestamping, `AudioMonitor` with atomics and triple-buffers.
*   `imbolc-core/src/audio/triple_buffer.rs`: Lock-free triple-buffer implementation for complex shared data.
*   `imbolc-core/src/audio/handle.rs`: `AudioHandle` with priority/normal channel routing.
*   `imbolc-core/src/audio/commands.rs`: `AudioCmd` enum with `is_priority()` classification.
*   `imbolc-core/src/audio/engine/mod.rs`: The central `AudioEngine` struct.
*   `imbolc-core/src/audio/engine/routing.rs`: Builds the synth node graph (`build_instrument_chain`).
*   `imbolc-core/src/audio/engine/voices.rs`: Logic for spawning voices (`spawn_voice`, `spawn_sampler_voice`).
*   `imbolc-core/src/audio/engine/voice_allocator.rs`: Smart voice stealing and resource pooling.
*   `imbolc-core/src/audio/engine/vst.rs`: VST-specific command handling.
