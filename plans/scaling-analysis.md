# Scaling Analysis

Analysis of scaling bottlenecks in Imbolc's local and network architectures,
ranked by impact.

---

## Local Scaling Issues

### High Impact

1. **Full routing rebuild tears down ALL voices** (`audio/engine/routing.rs`).
   Adding/removing an instrument or toggling an effect frees every synth node
   across all instruments, then recreates them. With 30 instruments each having
   3 effects, that's ~150+ node creates in one burst — audible dropout. The
   single-instrument rebuild path (`rebuild_single_instrument_routing`) only
   fires in specific cases.

2. **Full state clone on every undoable action** (`dispatch/mod.rs:65-69`).
   Every undoable action clones the entire `SessionState` + `InstrumentState`
   for the undo stack (up to 500 deep). As note count and instrument count
   grow, this becomes a significant CPU spike on every edit.

3. **Full state clone sent to audio thread** (`audio/handle.rs:358`). When
   dirty flags are set, the entire `InstrumentState`/`SessionState` is cloned
   and sent via channel. Targeted param updates (`filter_param`,
   `mixer_params`, etc.) bypass this for common real-time operations, but
   structural changes always trigger the full clone.

### Medium Impact

4. **Main thread serialization ceiling**. Event polling, dispatch, undo
   cloning, audio feedback, MIDI, and rendering all happen on one thread in one
   loop iteration. Heavy dispatch + complex pane render could approach the 16ms
   frame budget.

5. **16-voice-per-instrument hard cap** (`voice_allocator.rs:8`). Dense
   polyphonic passages with sustain will hit this.

6. **Arrangement flattening** creates full note copies for song mode. Cache
   helps when arrangement is unchanged, but any clip edit triggers full flatten
   + clone + send.

7. **Linear instrument lookups** (`instrument_state.rs:79`). `Vec<Instrument>`
   with `iter().find()` — O(n) per lookup, called repeatedly in playback tick
   loop.

### What's Well-Designed Locally

- Lock-free audio thread
- Priority/normal dual channels
- OSC timetags with schedule-ahead
- Targeted param updates for common operations
- Async persistence
- Arrangement cache
- Binary search on sorted notes

---

## Network (imbolc-net) Scaling Issues

### High Impact

1. **Slow-client poisons the server**. `broadcast()` writes sequentially to
   each client on the main thread. If one client's TCP buffer is full,
   `write_all` blocks, stalling updates to all other clients. No outbound
   queue, no dropping stale updates.

2. **Subsystem-level granularity, not field-level**. `StatePatch` tracks dirty
   at the `session`/`instruments` level. Adding one note sends the entire
   `PianoRollState` (all tracks, all notes). Changing one instrument param
   sends all instruments. As projects grow, these payloads go from ~50KB to
   potentially 500KB+.

3. **Per-client JSON re-serialization**. `write_message` calls
   `serde_json::to_vec` for each client separately. Broadcast to 5 clients = 5
   serializations of the same message. Easy win: serialize once, write the
   bytes to each client.

4. **No action batching or throttling**. Every keystroke/param change is
   immediately serialized and flushed. Dragging a knob sends dozens of
   individual messages, each triggering a full subsystem broadcast to all
   clients. With 3 clients each producing 10 actions/sec and 100KB state
   payloads, that's ~9 MB/s of JSON serialization.

### Medium Impact

5. **Double full-state clone per server loop iteration** (`network.rs`
   run_server). `NetworkState` is cloned twice every 2ms even when no actions
   arrived.

6. **JSON wire format**. Already flagged as a known tradeoff. ~3-5x larger
   than bincode, ~10-50x slower to serialize. Metering at 30Hz x N clients
   adds constant overhead.

7. **Single-threaded server loop**. Everything — accept, poll, dispatch,
   serialize, write — on one thread with 2ms sleep. Hard ceiling for client
   count and action throughput.

8. **No state catchup on reconnect**. After `ReconnectSuccessful`, the server
   doesn't push current state immediately — the client waits for the next
   broadcast cycle, leaving a stale-state gap.

### What's Well-Designed in Net

- No locks (ownership by construction)
- Clean reader-thread → mpsc → main-thread pipeline
- Suspension preserving ownership for reconnect
- Dirty flags avoiding no-op broadcasts

---

## Summary

**Locally**, the biggest risks are the full routing rebuild (audible glitches)
and full-state cloning on every undo/audio sync (CPU scaling with project
size).

**On the network side**, the slow-client blocking and coarse state-patch
granularity are the first things that will break as you add collaborators or
grow project complexity.
