# Imbolc Architecture Deep Dive

This document details the internal architecture of Imbolc, focusing on the low-latency audio engine, concurrency model, and integration with SuperCollider.

## High-Level Overview

Imbolc follows a strict separation of concerns between the **UI/dispatch loop (main thread)** and the **audio engine (audio thread)**.

*   **Main Thread:** Runs the TUI (ratatui), handles user input, owns `AppState`, dispatches actions, and renders.
*   **Audio Thread:** A dedicated thread that runs the sequencer clock, manages the SuperCollider server (scsynth), and handles real-time audio logic.
*   **SuperCollider (scsynth):** Runs as a subprocess. Performs DSP. Communications happen via OSC over UDP.

```mermaid
graph TD
    User[User Input] -->|Events| Main["Main Thread (UI/dispatch)"]
    Main -->|AudioCmd (priority + normal)| Audio[Audio Thread]
    Main -->|EventLog entries (actions + snapshots)| Audio
    Audio -->|AudioFeedback| Main
    Audio -->|OSC Bundles (UDP)| SC[SuperCollider (scsynth)]
    SC -->|OSC Reply (UDP)| Audio
    SC -->|Audio Out| Speakers

    subgraph Shared Memory
    Monitor[AudioMonitor (atomics + triple buffers)]
    end

    Audio -.->|Writes Meters/Scope| Monitor
    Main -.->|Reads Meters/Scope| Monitor
```

## Threading Model & Audio Processing

The core of Imbolc's timing stability lies in the `AudioThread` loop (`imbolc-audio/src/audio_thread.rs`).

### 1. The Audio Thread Loop
The audio thread does not rely on the UI framerate. It runs a tight loop that:
1.  **Priority Command Handling:** Uses `crossbeam-channel` with `select!` to prioritize time-critical commands (voice spawn/release, param changes) over normal commands (state sync, routing rebuilds).
2.  **Drains Commands:** Processes priority commands first, then normal commands.
3.  **Drains Event Log:** Applies projectable actions + snapshots from the event log within a small time budget.
4.  **Checks Time:** Calculates the precise elapsed `Duration` since the last tick using `std::time::Instant`.
5.  **Ticks:** If sufficient time has passed (>= 0.5ms), it calls `tick()`, converting the elapsed duration into **musical ticks** based on the current BPM.
6.  **Polls Engine:** Checks for server health, compilation results, and recording state changes.

**Priority Channel Design:**
- **Priority Channel (time-critical):** Voice spawn/release, param changes, playback control, automation
- **Normal Channel:** Routing rebuilds, recording control, server lifecycle, device changes
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
3.  It calls `osc_time_from_now(offset)` (`imbolc-audio/src/osc_client.rs`), which computes an absolute NTP timestamp.
4.  This timestamp is attached to the OSC bundle sent to SuperCollider.

**The Result:** SuperCollider receives the message *before* the sound needs to play and schedules it for the *exact* sample frame requested.

### Telemetry

The audio thread periodically reports tick timing statistics (avg/max/p95), scheduling lookahead, and OSC send-queue depth via `AudioFeedback::TelemetrySummary`. Use these to validate performance rather than hardcoded targets.

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
The sequencer (`imbolc-audio/src/playback.rs`) supports advanced features handled at the tick level:
*   **Loop Boundary Scanning:** Correctly handles notes wrapping around the loop point by scanning two ranges (`[old, end)` and `[start, new]`).
*   **Swing:** Delays offbeat notes by a calculated offset.
*   **Humanization:** Adds random jitter to timing and velocity for a more natural feel.
*   **Probability:** Per-note probability checks before spawning voices.
*   **Arpeggiator:** Handled in `arpeggiator_tick.rs`, converting held notes into rhythmic patterns with sub-step precision.

## Concurrency & State Management

### Command / Feedback Pattern
*   **Main -> Audio (imperative):** `Sender<AudioCmd>` for server lifecycle, playback control, recording/export, targeted param updates.
*   **Main -> Audio (projectable state):** `EventLogWriter` appends actions + snapshots; the audio thread drains these and updates its shadow state.
*   **Shadow State:** The audio thread maintains its own snapshot copies (`InstrumentSnapshot`, `SessionSnapshot`, `PianoRollSnapshot`, `AutomationSnapshot`).
*   **Audio -> Main:** `Sender<AudioFeedback>` for playhead, server status, metering, VST discovery, telemetry.

### Shared Monitoring State (Lock-Free)
For high-frequency visual data, `AudioMonitor` uses a hybrid lock-free architecture:

**Atomic Fields (for scalars):**
*   `meter_data: AtomicU64` — Stereo peak meters packed as two f32s
*   `sc_cpu: AtomicU32` — SuperCollider CPU load
*   `osc_latency_ms: AtomicU32` — OSC round-trip latency

**Triple-Buffered Fields (for complex types):**
*   `audio_in_waveforms: TripleBufferHandle<HashMap<...>>` — Per-instrument waveform data
*   `spectrum_data: TripleBufferHandle<[f32; 7]>` — 7-band spectrum analyzer
*   `lufs_data: TripleBufferHandle<(f32, f32, f32, f32)>` — Peak/RMS level data (from `/lufs` reply)
*   `scope_buffer: TripleBufferHandle<VecDeque<f32>>` — Oscilloscope buffer

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
4.  **Group 350 (Bus Processing):** Bus + layer-group processing chains and sends.
5.  **Group 400 (Record):** Disk recording (DiskOut).
6.  **Group 999 (Safety):** Safety limiter / analysis synths.

### Instrument Signal Chain
Each instrument is built with a deterministic chain of synth nodes:
`Source` -> `ProcessingChain` (Filter/EQ/Effects) -> `Output`

The LFO runs as a side-channel node that writes to control buses; processing and voice nodes read from those buses for modulation.

### Voice Allocation
Polyphony is managed by `VoiceAllocator` (`imbolc-audio/src/engine/voice_allocator.rs`).
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

*   `imbolc-audio/src/audio_thread.rs`: The main audio loop, command processing, priority channel handling, and tick orchestration.
*   `imbolc-audio/src/playback.rs`: Sequencer logic (notes, swing, humanize, probability).
*   `imbolc-audio/src/osc_client.rs`: UDP socket, NTP timestamping, `AudioMonitor` with atomics and triple-buffers.
*   `imbolc-audio/src/triple_buffer.rs`: Lock-free triple-buffer implementation for complex shared data.
*   `imbolc-audio/src/handle.rs`: `AudioHandle` with priority/normal channel routing.
*   `imbolc-audio/src/commands.rs`: `AudioCmd` enum with `is_priority()` classification.
*   `imbolc-audio/src/event_log.rs`: Event log for projectable actions + snapshots.
*   `imbolc-audio/src/osc_sender.rs`: Background OSC bundle sender thread.
*   `imbolc-audio/src/engine/mod.rs`: The central `AudioEngine` struct.
*   `imbolc-audio/src/engine/routing.rs`: Builds the synth node graph (`build_instrument_chain`).
*   `imbolc-audio/src/engine/voices.rs`: Logic for spawning voices (`spawn_voice`, `spawn_sampler_voice`).
*   `imbolc-audio/src/engine/voice_allocator.rs`: Smart voice stealing and resource pooling.
*   `imbolc-audio/src/engine/vst.rs`: VST-specific command handling.
