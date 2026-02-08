# Architecture Questions

Questions to think through for Imbolc's long-term direction.

---

## 1. AppState authority vs event-log / timing authority

Do you want AppState (main thread) to remain the source of truth, or
evolve toward an event-log where the audio thread is the timing
authority and UI is projection-only (esp. BPM/playhead/drum-step
feedback loops)?

**Answer:**
Full event-log architecture. Move to an event-log where the audio thread is the timing authority and UI is projection-only. Actions become events in a log. Eliminates clone-based state transfer and starvation risk. This is a significant long-term architectural rewrite. (See also Q7, Q8 which are subsumed by this.)

---

## 2. Why SuperCollider at all vs native DSP engine

You've built a lot to talk to an external SC process over UDP (OSC
serialization, lookahead, NTP anchoring). What does SC buy you that
justifies that architectural tax (SynthDef language, UGen library,
hot-reload graphs, etc.)?

**Answer:**
SC is the answer. SuperCollider is the long-term audio backend — its SynthDef ecosystem and real-time scheduling are the core value. Focus effort on reducing the architectural tax of the SC integration, not replacing it. **Closed — no action needed.**

---

## 3. Control-plane vs performance-plane separation

Server ops (loading synthdefs/resources) currently block audio. Can
you strictly separate control-plane (mgmt/loading) from
performance-plane (note scheduling) so playback never stutters during
state changes?

**Answer:**
Current blocking behavior is acceptable for now. Track as near-term task: move control-plane ops (loading SynthDefs, connecting to SC, compiling resources) off the audio thread so playback never stutters during state changes.

---

## 4. Network sync strategy: NTP timestamps vs drift correction

With imbolc-net and NTP-aligned timestamps: how do you handle
micro-timing differences + network jitter between machines? Custom
drift correction (Ableton Link-ish) to keep 1ms tick loops
phase-aligned?

**Answer:**
Not needed — audio is centralized on the server machine. Clients only send/receive control data over TCP. NTP/drift correction would only matter if distributed audio (multiple SC instances on multiple machines) is introduced. **Closed — no action needed.**

---

## 5. Timing stability: 1ms/0.5ms tick on a non-RT OS

Audio thread attempts ~1ms ticks dispatching OSC bundles. How do you
mitigate jitter under load? Increase lookahead beyond 15ms or move to
an event scheduler that fills a ring buffer ahead-of-time instead of
synchronous ticking?

**Answer:**
Design an event scheduler with ring-buffer approach. Pre-compute upcoming events (e.g., next 50ms) and use a dedicated sender thread to drain timestamped OSC bundles, replacing the synchronous tick model. Compute lookahead dynamically from buffer_size/sample_rate. Long-term task (bundles Q5+Q6 together).

---

## 6. 15ms SCHEDULE_LOOKAHEAD_SECS: empirically tuned or derived?

Is 15ms from measurement across machines or from a worst-case model
(audio buffer + UDP RTT + scheduling jitter)? Should it adapt
dynamically and/or be user-configurable?

**Answer:**
Bundle with Q5. The event scheduler redesign will fundamentally change how lookahead works, so tuning the 15ms constant separately is pointless. Address as part of the event scheduler work.

---

## 7. UI thread owns state; audio thread needs updates: starvation risk

UI renders ~60fps while audio thread ticks at ~2kHz. If UI blocks on
heavy render (mixer panes, waveform drawing), do you starve the audio
thread's priority channel and cause artifacts? How do you prevent
that?

**Answer:**
Subsumed by Q1. The event-log rewrite eliminates clone-based state transfer, which is the core starvation risk. No separate task needed.

---

## 8. Concurrency & locking: avoid UI stalls impacting audio

With AppState as single source of truth, how do you prevent UI
read/write operations from stalling the audio thread? Any lock-free
snapshots (triple buffering, atomic snapshots) for audio reads?

**Answer:**
Subsumed by Q1. Current architecture already uses channels (no shared locks), which is good. The event-log rewrite will further decouple the threads. No separate task needed.

---

## 9. Network collaboration: full-state broadcast after every action

StateUpdate sends entire NetworkState to all clients after every
action. With multiple clients, a knob tweak serializes/transmits the
whole session. Will you move to delta updates (dirty-flag approach
like AudioDirty) or keep full-state sync deliberately?

**Answer:**
Track field-level diffing as near-term task. Subsystem-level dirty-flag delta updates are already implemented (`StatePatch` + `DirtyFlags`). Next step: send individual param changes as lightweight messages instead of full `InstrumentState` blobs.

---

## 10. Undo vs persistence: full snapshots & delete/reinsert scaling

Full-state snapshots for undo and full DELETE+reinsert persistence
scale with project size, not change size. At what size does this
break? Consider hybrid undo diffs while persistence remains
snapshot-based?

**Answer:**
Hybrid undo diffs. Persistence stays as full MessagePack blob snapshots in SQLite (simple, atomic, reliable — blobs stay small enough). Undo moves to command-based diffs: store the action + inverse action instead of full state clones. Avoids O(max_depth * state_size) memory growth as projects scale to 64+ instruments. Near-term task.


---

## 11. Instrument-as-monolith endgame

One Instrument struct holds
source/filter/FX/LFO/ADSR/mixer/sampler/drum sequencer/VST
state/sends. Is that intentionally the complexity ceiling, or will you
need instrument groups/shared FX / modular routing that breaks the
unified model?

**Answer:**
Keep monolithic Instrument for now — it works for the common case. The bus system already handles shared FX and sub-mixing. Invest in the bus/routing layer (Q12, Q13) for near-term flexibility. Track modular routing (arbitrary signal flow, shared FX racks, instrument groups, instruments as nodes in a signal graph) as a long-term future task that would break the unified Instrument model.


---

## 12. OutputTarget/routing hard-wiring

OutputTarget is persisted/edited, but routing currently hard-wires
instrument output and bus output synths to hardware bus 0. Is that
temporary, or should "instrument -> bus -> master" become first-class?

**Answer:**
Fix now — this is a bug. `OutputTarget::Bus(n)` is stored and editable but routing always writes to hardware bus 0. Make it first-class: `OutputTarget::Bus(n)` routes the instrument's output synth to that bus's audio bus, `OutputTarget::Master` writes to hardware bus 0, bus output synths write to hardware bus 0. Creates proper `instrument -> bus -> master` routing. **Immediate priority.**


---

## 13. Mixer philosophy: sends sourced pre-filter/pre-FX vs post-chain

Sends are built from source_out (pre-filter/pre-FX) rather than
post-chain. Is the long-term plan explicitly "pre-insert sends", or do
you want selectable pre/post-fader and pre/post-insert send tap
points?

**Answer:**
Selectable per-send, with PostInsert as default. Current behavior (tapping from `source_out`, pre-filter/pre-FX) is wrong for typical use cases (reverb sends should hear the processed signal). Add `SendTapPoint` enum (`PreInsert | PostInsert | PostFader`) per send. Default PostInsert (post-chain, pre-fader) matches industry standard "pre-fader send." **Immediate priority.**


---

## 14. Voice allocator semantics under voice stealing + release tails

Allocator is in Rust. Does it have access to signal state ("is this
voice silent?") via SC feedback, or does it assume based on note-off?
If blind, how do you handle long release tails during voice stealing?

**Answer:**
Listen for SC `/n_end` OSC notifications (sent when a node is freed by doneAction:2) for ground truth about when voices actually die. Remove voices from the allocator on receipt. Eliminates the blind `release_dur + 1.5s` guess and prevents clicks/cutoffs from early freeing of voices with long release tails. **Near-term task.** (See also Q15, bundled with this.)


---

## 15. Voice allocator control-bus pool return path / lifetime

Allocator has a control-bus pool API, but voices allocate control
buses without an explicit return path. Do you want monotonic bus
growth for simplicity, or deterministic recycle (doneAction/ack-driven
reclaim) to avoid long-session growth?

**Answer:**
Bundled with Q14. The `/n_end` notification provides a natural lifecycle hook: return control buses to the pool when `/n_end` arrives for a voice's node. Gives deterministic recycle without monotonic growth. No separate task needed.


---

## 16. Docs as contract vs design notebook

Several docs/plans appear stale vs implementation. Should docs be
strict architectural contracts, or is divergence acceptable because
they're a design notebook?

**Answer:**
Prune to essentials. Delete stale plan/design docs from `docs/`. Keep reference docs (audio-routing, keybindings, sqlite-persistence, polyphonic-voice-allocation) current. Per-crate CLAUDE.md files are the living architectural contracts. Reduce `docs/` to only actively-maintained reference material. **Housekeeping task.**
