# Architecture Questions - Working Notes

Working through all 16 questions from plans/questions.md.

---

## Current Architecture Summary (from code reading)

### Threading Model
- **UI thread**: ~60fps render loop, owns AppState (main thread)
- **Audio thread**: 0.5ms tick interval (2kHz), runs in separate thread
- **Communication**: crossbeam channels (priority + normal), feedback via std::mpsc
- Audio thread has its own copies: `InstrumentSnapshot`, `SessionSnapshot`, `PianoRollSnapshot`
- State sync: UI sends full `UpdateState { instruments, session }` via AudioCmd
- Triple buffer exists (`triple_buffer.rs`) but snapshots are just type aliases for full state clones

### Undo System
- Full snapshot-based: `UndoSnapshot { session: SessionState, instruments: InstrumentState }`
- Clones entire SessionState + InstrumentState on every undoable action
- Max depth configurable (stack of Vec<UndoSnapshot>)
- `is_undoable()` filters out navigation/playback actions

### Persistence
- SQLite with single blob row: `project_blob(id=1, format_version, session_data, instrument_data)`
- MessagePack serialization of full SessionState + InstrumentState
- INSERT OR REPLACE (full overwrite on every save)
- WAL mode for atomic writes

### Instrument Struct (Q11)
- Monolithic: source, source_params, filter, eq, effects, lfo, amp_envelope
- Plus: level/pan/mute/solo/active (mixer), output_target, channel_config, sends
- Plus: sampler_config, drum_sequencer, vst_param_values, vst_state_path
- Plus: arpeggiator, chord_shape, convolution_ir_path, layer_group, groove
- ~25+ fields total

### OutputTarget / Routing (Q12)
- `OutputTarget` enum: `Master` or `Bus(u8)` - persisted per instrument
- BUT in `build_instrument_chain()`, the output synth always writes to hardware
- `imbolc_output` synth has no `out` param set to a bus - it goes to hardware bus 0
- Bus output synths (`imbolc_bus_out`) also write to hardware bus 0 implicitly
- So OutputTarget is stored but routing is effectively hard-wired to master out

### Sends (Q13)
- Sends tap from `source_out` bus (line 367 in routing.rs: `get_audio_bus(instrument.id, "source_out")`)
- This is PRE-filter, PRE-effects - the raw source output
- Not post-chain (instrument_final_buses is tracked but sends don't use it)

### Voice Allocator (Q14)
- Pure Rust, no SC feedback for "is voice silent?"
- Uses `release_state: Option<(Instant, f32)>` - release time from envelope config
- `cleanup_expired()` removes voices after `release_dur + 1.5` seconds
- Steal scoring: released voices scored 0-999, active 1000+ (lower = steal first)
- Blind to actual audio state

### Bus Allocator (Q15)
- Control bus pool exists: `control_bus_pool: Vec<(i32, i32, i32)>` for freq/gate/vel triples
- `return_control_buses()` returns to pool for reuse
- `alloc_control_buses()` checks pool first, then grows
- BUT: `BusAllocator.reset()` is called on full routing rebuild, clearing everything
- Voice allocator syncs watermarks after rebuild
- So there IS a recycle path, but full rebuilds reset it

### Network (Q4, Q9)
- Protocol: length-prefixed JSON over TCP
- `NetworkState { session, instruments, ownership, privileged_client }`
- Already has `StatePatch` with dirty flags! `DirtyFlags { session, instruments, ownership, privileged_client }`
- `broadcast_state_patch()` sends only changed subsystems
- `broadcast_full_sync()` every 30s as fallback
- Legacy `broadcast_state()` still exists (full NetworkState on every action)
- No NTP timestamps in the net protocol - just TCP action relay + state push
- No drift correction - audio only runs on the server machine

### Timing (Q5, Q6)
- `SCHEDULE_LOOKAHEAD_SECS = 0.015` (15ms) - hard constant in engine/mod.rs:43
- Used by: playback.rs, drum_tick.rs, arpeggiator_tick.rs, click_tick.rs
- Audio thread tick: 0.5ms (500us) interval
- Priority channel drain budget: 200us max, 128 cmds max
- Normal channel drain budget: 100us max, 64 cmds max

### State Transfer (Q7, Q8)
- UI sends AudioCmd::UpdateState { instruments, session } via crossbeam channel
- Audio thread stores its own copies (InstrumentSnapshot = InstrumentState clone)
- Triple buffer exists but NOT used for state transfer - only for AudioMonitor metering
- State transfer is clone-based via channel, not lock-free snapshot
- Priority channel (voice spawn, param) always checked before normal channel

### Docs (Q16)
- architecture.md shows `AppState` with fields that may be stale vs actual struct
- 24 doc files total, some clearly "plan" docs vs reference docs
- CLAUDE.md files per-crate are well maintained and current

---

## Decisions (all 16 resolved)

| Q | Topic | Decision | Priority |
|---|-------|----------|----------|
| 1 | Event-log architecture | Full rewrite: audio thread = timing authority, UI = projection | Long-term |
| 2 | SC vs native DSP | SC is the answer. Reduce architectural tax. | **Closed** |
| 3 | Control vs performance plane | Track: move load/connect/compile off audio thread | Near-term |
| 4 | Network timing | Not needed, audio centralized on server | **Closed** |
| 5 | Event scheduler | Ring-buffer ahead-of-time scheduling, replaces sync ticks | Long-term |
| 6 | 15ms lookahead | Bundled with Q5, no separate action | Long-term |
| 7 | UI starvation risk | Subsumed by Q1 event-log rewrite | Long-term |
| 8 | Concurrency/locking | Subsumed by Q1 event-log rewrite | Long-term |
| 9 | Network deltas | Field-level diffing (subsystem patches already done) | Near-term |
| 10 | Undo scaling | Hybrid: command diffs for undo, snapshots for persistence | Near-term |
| 11 | Instrument monolith | Keep monolith, invest in bus/routing. Modular routing later. | Long-term |
| 12 | OutputTarget routing | **Bug fix**: Bus(n) must route to bus, not hardware 0 | **Immediate** |
| 13 | Send tap points | Selectable per-send (PreInsert/PostInsert/PostFader), default PostInsert | **Immediate** |
| 14 | Voice /n_end feedback | Listen for SC /n_end, remove voices on receipt | Near-term |
| 15 | Control bus return | Bundled with Q14, return buses on /n_end | Near-term |
| 16 | Docs pruning | Delete stale docs, CLAUDE.md = living contracts | Housekeeping |

All details recorded in `plans/questions.md`.
