# Architecture Questions

Questions to think through for Imbolc's long-term direction.

---

## 1. AppState authority vs event-log / timing authority

Do you want AppState (main thread) to remain the source of truth, or evolve toward an event-log where the audio thread is the timing authority and UI is projection-only (esp. BPM/playhead/drum-step feedback loops)?

**Answer:**


---

## 2. Why SuperCollider at all vs native DSP engine

You've built a lot to talk to an external SC process over UDP (OSC serialization, lookahead, NTP anchoring). What does SC buy you that justifies that architectural tax (SynthDef language, UGen library, hot-reload graphs, etc.)?

**Answer:**


---

## 3. Control-plane vs performance-plane separation

Server ops (loading synthdefs/resources) currently block audio. Can you strictly separate control-plane (mgmt/loading) from performance-plane (note scheduling) so playback never stutters during state changes?

**Answer:**


---

## 4. Network sync strategy: NTP timestamps vs drift correction

With imbolc-net and NTP-aligned timestamps: how do you handle micro-timing differences + network jitter between machines? Custom drift correction (Ableton Link-ish) to keep 1ms tick loops phase-aligned?

**Answer:**


---

## 5. Timing stability: 1ms/0.5ms tick on a non-RT OS

Audio thread attempts ~1ms ticks dispatching OSC bundles. How do you mitigate jitter under load? Increase lookahead beyond 15ms or move to an event scheduler that fills a ring buffer ahead-of-time instead of synchronous ticking?

**Answer:**


---

## 6. 15ms SCHEDULE_LOOKAHEAD_SECS: empirically tuned or derived?

Is 15ms from measurement across machines or from a worst-case model (audio buffer + UDP RTT + scheduling jitter)? Should it adapt dynamically and/or be user-configurable?

**Answer:**


---

## 7. UI thread owns state; audio thread needs updates: starvation risk

UI renders ~60fps while audio thread ticks at ~2kHz. If UI blocks on heavy render (mixer panes, waveform drawing), do you starve the audio thread's priority channel and cause artifacts? How do you prevent that?

**Answer:**


---

## 8. Concurrency & locking: avoid UI stalls impacting audio

With AppState as single source of truth, how do you prevent UI read/write operations from stalling the audio thread? Any lock-free snapshots (triple buffering, atomic snapshots) for audio reads?

**Answer:**


---

## 9. Network collaboration: full-state broadcast after every action

StateUpdate sends entire NetworkState to all clients after every action. With multiple clients, a knob tweak serializes/transmits the whole session. Will you move to delta updates (dirty-flag approach like AudioDirty) or keep full-state sync deliberately?

**Answer:**


---

## 10. Undo vs persistence: full snapshots & delete/reinsert scaling

Full-state snapshots for undo and full DELETE+reinsert persistence scale with project size, not change size. At what size does this break? Consider hybrid undo diffs while persistence remains snapshot-based?

**Answer:**


---

## 11. Instrument-as-monolith endgame

One Instrument struct holds source/filter/FX/LFO/ADSR/mixer/sampler/drum sequencer/VST state/sends. Is that intentionally the complexity ceiling, or will you need instrument groups/shared FX / modular routing that breaks the unified model?

**Answer:**


---

## 12. OutputTarget/routing hard-wiring

OutputTarget is persisted/edited, but routing currently hard-wires instrument output and bus output synths to hardware bus 0. Is that temporary, or should "instrument -> bus -> master" become first-class?

**Answer:**


---

## 13. Mixer philosophy: sends sourced pre-filter/pre-FX vs post-chain

Sends are built from source_out (pre-filter/pre-FX) rather than post-chain. Is the long-term plan explicitly "pre-insert sends", or do you want selectable pre/post-fader and pre/post-insert send tap points?

**Answer:**


---

## 14. Voice allocator semantics under voice stealing + release tails

Allocator is in Rust. Does it have access to signal state ("is this voice silent?") via SC feedback, or does it assume based on note-off? If blind, how do you handle long release tails during voice stealing?

**Answer:**


---

## 15. Voice allocator control-bus pool return path / lifetime

Allocator has a control-bus pool API, but voices allocate control buses without an explicit return path. Do you want monotonic bus growth for simplicity, or deterministic recycle (doneAction/ack-driven reclaim) to avoid long-session growth?

**Answer:**


---

## 16. Docs as contract vs design notebook

Several docs/plans appear stale vs implementation. Should docs be strict architectural contracts, or is divergence acceptable because they're a design notebook?

**Answer:**

