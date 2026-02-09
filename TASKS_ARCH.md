# Architecture Tasks

Derived from the 16 architecture questions in `plans/questions.md`.

---

## Near-term

### 1. Voice allocator /n_end feedback [Q14+Q15]

Listen for SC `/n_end` OSC notifications (sent when a node is freed by
doneAction:2). Remove voices from allocator on receipt. Return control
buses to pool on voice death. Eliminates blind `release_dur + 1.5s`
guess.

**Files:** `imbolc-core/src/audio/engine/voice_allocator.rs`,
`imbolc-core/src/audio/osc_client.rs`,
`imbolc-core/src/audio/engine/voices.rs`.

### 2. Control-plane vs performance-plane separation [Q3]

Document current blocking behavior. Move load/connect/compile ops off
the audio thread so playback never stutters during state changes.

### 3. Field-level network delta updates [Q9]

Subsystem-level dirty-flag patches verified and tested (`StatePatch` +
`DirtyFlags`, 21 new tests). Fixed `Option<Option<T>>` JSON roundtrip
bug for `privileged_client` via `double_option` serde helper. Next
step: send individual param changes as lightweight messages instead of
full `InstrumentState` blobs.

### 4. Hybrid undo diffs [Q10]

Move undo from full-state snapshots to command-based diffs (action +
inverse action). Persistence stays as full MessagePack blob snapshots
in SQLite. Avoids O(max_depth * state_size) memory growth at 64+
instruments.

---

## Long-term (architectural rewrites)

### 5. Event-log architecture [Q1+Q7+Q8]

Actions become events in a log. Audio thread is timing authority. UI
is projection-only. Eliminates clone-based state transfer and
UI-blocking-audio starvation risk. Subsumes Q7 (starvation) and Q8
(concurrency/locking).

### 6. Event scheduler with dynamic lookahead [Q5+Q6]

Pre-compute upcoming OSC bundles into a ring buffer. Replace
synchronous 0.5ms ticking with ahead-of-time scheduling via dedicated
sender thread. Compute lookahead dynamically from
buffer_size/sample_rate. Replaces the hardcoded 15ms
`SCHEDULE_LOOKAHEAD_SECS`.

### 7. Modular routing [Q11]

Instruments, effects, and buses as nodes in a signal graph. Arbitrary
routing (instrument A output -> instrument B sidechain). Breaks the
monolithic Instrument model. Current bus system handles shared FX for
now.

---

## Housekeeping

### 8. Prune docs [Q16]

Delete stale plan/design docs from `docs/`. Keep reference docs
(audio-routing, keybindings, sqlite-persistence,
polyphonic-voice-allocation). Per-crate CLAUDE.md files are the living
contracts.

---

## Closed (no action needed)

- **Q2: SC is the answer** -- SuperCollider is the long-term
  backend. Focus on reducing architectural tax.
- **Q4: Network timing** -- Not needed; audio is centralized on
  server.
