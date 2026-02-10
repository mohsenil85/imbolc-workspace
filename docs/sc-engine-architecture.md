# SC-Based Audio Engine Modules (Imbolc)

This is a pragmatic checklist of what the SuperCollider-backed engine needs and how Imbolc implements it today. Items marked “Future” are aspirational.

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
- Boot/quit/restart scsynth, detect failure, auto-reconnect
- Configure device, sample rate, buffer size, I/O channels
- Health monitoring (CPU/load, status replies)

### 2) OSC Transport Layer
- OSC client with batching + time-tagged bundles
- Dedicated OSC sender thread for timed bundles
- Priority/normal command channels for RT vs non-RT control

### 3) SynthDef & Node Library
- Compile/load SynthDefs (shipped precompiled, rebuild via scripts)
- Parameter schemas for UI + automation targets
- Registry mapping device types → SynthDefs + wiring templates

### 4) Bus & Routing Allocator
- Deterministic audio/control bus allocation
- Fixed group hierarchy: sources → processing → output → bus processing → record/safety
- Incremental routing rebuilds for targeted changes (add/delete instrument, bus FX)

### 5) Clock / Transport Bridge (Timeline Master)
- App is master, SC follows
- App transport drives timestamped scheduling, tempo changes, loop points

### 6) Event Scheduler (Clips/Automation → OSC)
- Convert notes + automation lanes into timestamped bundles
- Lookahead window computed from buffer size + jitter margin
- Loop handling with high-water mark scanning

### 7) Recording & Rendering Pipeline
**Live recording**
- Input routing to SC buses
- Disk recording via SC `DiskOut` (master recording)

**Export/bounce**
- Real-time master bounce + stem export (per-instrument)
- **Future:** NRT render via SC score files

### 8) State Reconciler (Project ↔ Server)
- Apply diffs: create/remove/move nodes, change params, reroute buses
- Incremental rebuilds where possible; full rebuilds only for structural changes
- Stable ID mapping: project device instance IDs ↔ SC node IDs/bus IDs

### 9) Metering / Analysis Feeds
- Analysis synths for peak/RMS, spectrum, scope
- Lock-free shared monitor data for UI rendering

---

## Hard Constraints (don’t lie to yourself)
- SC is not a DAW timeline engine: the app owns timeline semantics
- Timing needs timestamped bundles + lookahead; immediate OSC will jitter
- Seek/loop require “rebuild to known state” strategies
- Reliable bounce/export is required for a non-toy system

---

## Minimal v1 That Makes Music
- Server runtime manager
- OSC transport (timestamped bundles)
- Bus & routing allocator
- SynthDef/node library (basic synth + basic FX)
- Event scheduler (notes + automation)
- Basic recording + real-time export
