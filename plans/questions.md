# Architecture Questions

Questions to think through for Imbolc's long-term direction.

---

## 1. AppState authority vs event-log / timing authority

Do you want AppState (main thread) to remain the source of truth, or
evolve toward an event-log where the audio thread is the timing
authority and UI is projection-only (esp. BPM/playhead/drum-step
feedback loops)?

**Answer:** Full event-log architecture. Move to an event-log where
the audio thread is the timing authority and UI is
projection-only. Actions become events in a log. Eliminates
clone-based state transfer and starvation risk. (See also Q7, Q8 which
are subsumed by this.) **Done.** Four phases: (1) factor dispatch into
pure mutation + `AudioSideEffect` enum, (2) action projection
replacing full-state cloning, (3) audio thread as timing authority for
`playing` state, (4) shared retained event log replacing `AudioCmd`
state-sync variants.

---

## 2. Why SuperCollider at all vs native DSP engine

You've built a lot to talk to an external SC process over UDP (OSC
serialization, lookahead, NTP anchoring). What does SC buy you that
justifies that architectural tax (SynthDef language, UGen library,
hot-reload graphs, etc.)?

**Answer:** SC is the answer. SuperCollider is the long-term audio
backend — its SynthDef ecosystem and real-time scheduling are the core
value. Focus effort on reducing the architectural tax of the SC
integration, not replacing it. **Closed — no action needed.**

---

## 3. Control-plane vs performance-plane separation

Server ops (loading synthdefs/resources) currently block audio. Can
you strictly separate control-plane (mgmt/loading) from
performance-plane (note scheduling) so playback never stutters during
state changes?

**Answer:** ~~Track as near-term task: move control-plane ops off the
audio thread so playback never stutters during state changes.~~
**Done.** Server startup, OSC connection, and SynthDef compilation all
run in background threads. Routing rebuild uses a phased state
machine. Two-channel dispatch with priority budgets prevents bulk ops
from starving playback.

---

## 4. Network sync strategy: NTP timestamps vs drift correction

With imbolc-net and NTP-aligned timestamps: how do you handle
micro-timing differences + network jitter between machines? Custom
drift correction (Ableton Link-ish) to keep 1ms tick loops
phase-aligned?

**Answer:** Not needed — audio is centralized on the server
machine. Clients only send/receive control data over TCP. NTP/drift
correction would only matter if distributed audio (multiple SC
instances on multiple machines) is introduced. **Closed — no action
needed.**

---

## 5. Timing stability: 1ms/0.5ms tick on a non-RT OS

Audio thread attempts ~1ms ticks dispatching OSC bundles. How do you
mitigate jitter under load? Increase lookahead beyond 15ms or move to
an event scheduler that fills a ring buffer ahead-of-time instead of
synchronous ticking?

**Answer:** ~~Design an event scheduler with ring-buffer
approach. Long-term task (bundles Q5+Q6 together).~~ **Done.** Three
phases: (1) pre-scheduling with high-water mark to avoid
double-scheduling, (2) dedicated OSC sender thread with bounded
channel removing synchronous UDP I/O from audio thread, (3) dynamic
lookahead computed from `buffer_size/sample_rate` with jitter margin
and 10ms floor.

---

## 6. 15ms SCHEDULE_LOOKAHEAD_SECS: empirically tuned or derived?

Is 15ms from measurement across machines or from a worst-case model
(audio buffer + UDP RTT + scheduling jitter)? Should it adapt
dynamically and/or be user-configurable?

**Answer:** Bundled with Q5. **Done.** Hardcoded 15ms replaced by
`compute_lookahead(buffer_size, sample_rate)` →
`max(buffer_size/sample_rate + 5ms jitter, 10ms floor)`. Adapts to
actual device configuration.

---

## 7. UI thread owns state; audio thread needs updates: starvation risk

UI renders ~60fps while audio thread ticks at ~2kHz. If UI blocks on
heavy render (mixer panes, waveform drawing), do you starve the audio
thread's priority channel and cause artifacts? How do you prevent
that?

**Answer:** Subsumed by Q1. **Done** — event-log rewrite eliminated
clone-based state transfer. Shared `EventLogWriter`/`EventLogReader`
with 100µs drain budget replaced full-state sync.

---

## 8. Concurrency & locking: avoid UI stalls impacting audio

With AppState as single source of truth, how do you prevent UI
read/write operations from stalling the audio thread? Any lock-free
snapshots (triple buffering, atomic snapshots) for audio reads?

**Answer:** Subsumed by Q1. **Done** — channels preserved (no shared
locks), event-log further decoupled the threads. Audio thread drains
log entries within budget; no lock contention path exists.

---

## 9. Network collaboration: full-state broadcast after every action

StateUpdate sends entire NetworkState to all clients after every
action. With multiple clients, a knob tweak serializes/transmits the
whole session. Will you move to delta updates (dirty-flag approach
like AudioDirty) or keep full-state sync deliberately?

**Answer:** ~~Track field-level diffing as near-term task.~~ **Done.**
Instrument-level delta updates with per-instrument dirty flags
(`DirtyFlags`). `broadcast_state_patch()` sends only changed
instruments as `InstrumentPatch` entries. Rate-limited at ~30Hz with
threshold coalescing (falls back to full snapshot when >50% dirty).

---

## 10. Undo vs persistence: full snapshots & delete/reinsert scaling

Full-state snapshots for undo and full DELETE+reinsert persistence
scale with project size, not change size. At what size does this
break? Consider hybrid undo diffs while persistence remains
snapshot-based?

**Answer:** ~~Hybrid undo diffs. Near-term task.~~ **Done.** Replaced
full-state snapshots with scope-aware `UndoEntry` variants
(`SingleInstrument`, `Session`, `Full`). Scope classifier routes each
action to the narrowest scope. Persistence unaffected (undo history
never persisted).


---

## 11. Instrument-as-monolith endgame

One Instrument struct holds
source/filter/FX/LFO/ADSR/mixer/sampler/drum sequencer/VST
state/sends. Is that intentionally the complexity ceiling, or will you
need instrument groups/shared FX / modular routing that breaks the
unified model?

**Answer:** Keep monolithic Instrument for now — it works for the
common case. The bus system already handles shared FX and
sub-mixing. Invest in the bus/routing layer (Q12, Q13) for near-term
flexibility. Track modular routing (arbitrary signal flow, shared FX
racks, instrument groups, instruments as nodes in a signal graph) as a
long-term future task that would break the unified Instrument
model. **DONE** — a targeted loosening plan
(`plans/targeted-routing-loosening.md`) covers most use cases without
full modular routing.


---

## 12. OutputTarget/routing hard-wiring

OutputTarget is persisted/edited, but routing currently hard-wires
instrument output and bus output synths to hardware bus 0. Is that
temporary, or should "instrument -> bus -> master" become first-class?

**Answer:** ~~Fix now — this is a bug. Immediate priority.~~ **Done —
see TASKS_DONE.md.** `OutputTarget::Bus(n)` routes instrument output
synth to that bus's audio bus. `OutputTarget::Master` routes to
hardware bus 0. Layer groups also support output target routing.


---

## 13. Mixer philosophy: sends sourced pre-filter/pre-FX vs post-chain

Sends are built from source_out (pre-filter/pre-FX) rather than
post-chain. Is the long-term plan explicitly "pre-insert sends", or do
you want selectable pre/post-fader and pre/post-insert send tap
points?

**Answer:** ~~Selectable per-send, with PostInsert as
default. Immediate priority.~~ **Done — see TASKS_DONE.md.** Per-send
`SendTapPoint` enum: `PreInsert` (pre-filter/FX) and `PostInsert`
(post-effects, pre-fader, default). `MixerAction::CycleSendTapPoint`
action for UI cycling.


---

## 14. Voice allocator semantics under voice stealing + release tails

Allocator is in Rust. Does it have access to signal state ("is this
voice silent?") via SC feedback, or does it assume based on note-off?
If blind, how do you handle long release tails during voice stealing?

**Answer:** ~~Listen for SC `/n_end` OSC notifications. Near-term
task. (See also Q15, bundled with this.)~~ **Done.** OSC listener
receives `/n_end` and feeds via crossbeam channel to audio thread,
which removes voices and returns control buses
immediately. Timer-based `cleanup_expired()` retained as safety net.


---

## 15. Voice allocator control-bus pool return path / lifetime

Allocator has a control-bus pool API, but voices allocate control
buses without an explicit return path. Do you want monotonic bus
growth for simplicity, or deterministic recycle (doneAction/ack-driven
reclaim) to avoid long-session growth?

**Answer:** Bundled with Q14. **Done** — `/n_end` notification returns
control buses to the pool on voice death. Deterministic recycle
without monotonic growth.


---

## 16. Docs as contract vs design notebook

Several docs/plans appear stale vs implementation. Should docs be
strict architectural contracts, or is divergence acceptable because
they're a design notebook?

**Answer:** ~~Prune to essentials.~~ **Done.** Deleted 14 stale/stub
docs, moved 4 active plans to `plans/` (custom-synthdef-plan,
network-scenarios, scaling-analysis, vst3-support-roadmap). 8
reference docs remain in `docs/`. Per-crate CLAUDE.md files are the
living architectural contracts.
