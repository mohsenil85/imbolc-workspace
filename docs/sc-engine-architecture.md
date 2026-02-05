# SC-Based Audio Engine Modules (Reason-clone, TUI)

## What SuperCollider Gives You (scsynth)
- Real-time DSP graph (UGens), mixing, node graph
- Audio device I/O (platform/driver dependent)
- Buffers, buses, groups, nodes
- OSC control interface

## What You Still Need (DAW glue)
- Project model + editing semantics (tracks/clips/devices/automation)
- Deterministic mapping: project state → server state
- Timing correctness (transport/tempo map, scheduling, latency handling)
- Recording + export/bounce pipeline + asset management
- Crash/restart strategy (scsynth will die sometimes)

---

## Audio Engine Modules (on top of scsynth)

### 1) Server Runtime Manager
- Boot/quit/reboot scsynth, detect failure, auto-reconnect
- Configure device, sample rate, block size, I/O channels
- Manage server options (memory, max nodes, wire buffers, etc.)
- Health monitoring (CPU/load/node counts, status)

### 2) OSC Transport Layer
- OSC client with batching
- Time-tagged OSC bundles (for timing correctness)
- Fencing/acks via `/sync` (know when state is applied)
- Separate queues: RT-critical control vs slow operations (disk/buffer loads)

### 3) SynthDef & Node Library
- Compile/load SynthDefs (or ship precompiled)
- Versioning of SynthDefs (project references stable versions)
- Parameter schema extraction (for UI/automation)
- Registry mapping DAW “device types” → SynthDefs + wiring templates

### 4) Bus & Routing Allocator
- Allocate/track audio buses + control buses deterministically
- Maintain group hierarchy (master → tracks → device chains)
- Enforce processing order: instrument → inserts → sends → fader → bus → master

### 5) Clock / Transport Bridge (Timeline Master)
- Prefer: **app is master**, SC follows
- App transport + tempo map drives:
  - timestamped note/event scheduling
  - tempo changes
  - loop points and seeks

### 6) Event Scheduler (Clips/Automation → OSC)
- Convert MIDI clips + automation lanes into timestamped bundles
- Lookahead scheduling window (e.g., 100–500ms)
- Seeking: kill/rebuild playing nodes, re-chase automation at playhead
- Loop handling with deterministic state re-init

### 7) Recording & Rendering Pipeline
**Live recording**
- Input routing to SC buses
- Capture to disk (SC server recording or external capture)
- Latency compensation / alignment to timeline

**Export/bounce**
- Prefer: NRT render via SC score files (“project → score” exporter)
- Fallback: real-time bounce (acceptable for v1, annoying long-term)

### 8) State Reconciler (Project ↔ Server)
- Apply diffs: create/remove/move nodes, change params, reroute buses
- Use `/sync` fencing to ensure applied state
- Stable ID mapping: project device instance IDs ↔ SC node IDs/bus IDs

### 9) Metering / Analysis Feeds
- Meters via analysis synths + `SendReply` / control buses
- Decimate for UI (terminal doesn’t need high-rate updates)
- Peak hold / RMS integration if needed

---

## Hard Constraints (don’t lie to yourself)
- SC is not a DAW timeline engine: your app must own timeline semantics
- Timing needs timestamped bundles + lookahead; immediate OSC will jitter
- Seek/loop require “rebuild to known state” strategies
- Reliable bounce/export is required for a non-toy system

---

## Minimal v1 That Makes Music
- Server Runtime Manager
- OSC Transport Layer (timestamped bundles + `/sync`)
- Bus & Routing Allocator
- SynthDef/Node Library (basic synth + basic FX)
- Event Scheduler (notes + minimal automation)
- Either: basic recording OR basic NRT export (pick one first)
