# Render to WAV — Implementation Plan

Render a MIDI instrument's piano roll track (loop region) to a WAV file in real-time, then convert the instrument to a PitchedSampler loaded with the result.

## Approach

Use the existing `StartRecording`/`StopRecording` infrastructure (SuperCollider `DiskOut.ar()`) to record from the instrument's post-effects bus while playing back the loop region. No new SuperCollider synthdefs or NRT rendering needed.

**Key insight**: Each instrument's audio chain ends at a `current_bus` before the output node. Recording from this bus captures source + filter + effects without mixer level/pan, which is the correct behavior for a rendered sample.

## Files to Modify

| File | Change |
|------|--------|
| `imbolc-core/src/action.rs` | Add `RenderToWav` to `PianoRollAction` |
| `imbolc-core/src/audio/commands.rs` | Add `StartInstrumentRender` to `AudioCmd`, `RenderComplete` to `AudioFeedback` |
| `imbolc-core/src/audio/handle.rs` | Add `start_instrument_render()` method, `RenderState` struct on audio thread, render-stop check in `tick_playback()` |
| `imbolc-core/src/audio/engine/routing.rs` | Store final `current_bus` per instrument in a HashMap after routing |
| `imbolc-core/src/audio/engine/mod.rs` | Add `instrument_final_buses: HashMap<InstrumentId, i32>` field |
| `imbolc-core/src/dispatch/piano_roll.rs` | Handle `RenderToWav` dispatch |
| `imbolc-core/src/state/mod.rs` | Add `pending_render` field to `AppState` |
| `src/main.rs` | Handle `RenderComplete` feedback — load WAV, convert instrument |
| `src/panes/piano_roll_pane/input.rs` | Add keybinding for render action |
| `src/panes/piano_roll_pane/rendering.rs` | Add "RENDERING" indicator |

## Implementation Steps

### 1. Track instrument final bus in routing

In `engine/routing.rs`, after the output synth is created (line ~378), store:
```rust
self.instrument_final_buses.insert(instrument.id, current_bus);
```
Add `instrument_final_buses: HashMap<InstrumentId, i32>` to `AudioEngine`. This is the bus after filter+effects but before the output/mixer node — captures the full sound design.

### 2. Add action variant

In `action.rs`, add to `PianoRollAction`:
```rust
RenderToWav,
```

### 3. Add AudioCmd and AudioFeedback variants

In `commands.rs`:
```rust
// AudioCmd
StartInstrumentRender {
    instrument_id: InstrumentId,
    path: PathBuf,
    reply: Sender<Result<(), String>>,
},

// AudioFeedback
RenderComplete {
    instrument_id: InstrumentId,
    path: PathBuf,
},
```

### 4. Add render state to AppState

In `state/mod.rs`, add:
```rust
pub pending_render: Option<PendingRender>,
```
With struct:
```rust
pub struct PendingRender {
    pub instrument_id: InstrumentId,
    pub path: PathBuf,
    pub was_looping: bool,
}
```

### 5. Add render state to AudioThread

In `handle.rs`, add to AudioThread:
```rust
render_state: Option<RenderState>,
```
With struct:
```rust
struct RenderState {
    instrument_id: InstrumentId,
    path: PathBuf,
    loop_end: u32,
    tail_ticks: u32, // extra ticks after loop_end for release tails
}
```

### 6. Implement `start_instrument_render` on AudioHandle

Synchronous method that sends `StartInstrumentRender` and waits for reply. Sets `is_recording = true` on success.

### 7. Handle `StartInstrumentRender` on AudioThread

In `handle_cmd`:
1. Look up `self.engine.instrument_final_buses[&instrument_id]`
2. Call `self.engine.start_recording(bus, &path)` (reuses existing recording infra)
3. Set `self.render_state = Some(RenderState { ... })` with `loop_end` from `self.piano_roll.loop_end`
4. Calculate `tail_ticks` ~= 1 second worth of ticks for envelope release

### 8. Monitor playhead in `tick_playback`

After advancing playhead, check:
```rust
if let Some(ref render) = self.render_state {
    if self.piano_roll.playhead >= render.loop_end + render.tail_ticks {
        let path = self.engine.stop_recording();
        self.piano_roll.playing = false;
        self.engine.release_all_voices();
        let render = self.render_state.take().unwrap();
        if let Some(wav_path) = path {
            self.feedback_tx.send(AudioFeedback::RenderComplete {
                instrument_id: render.instrument_id,
                path: wav_path,
            });
        }
    }
}
```

Also: when `render_state` is active, skip the normal loop-wrapping behavior in `advance()` so the playhead proceeds linearly past `loop_end`.

### 9. Dispatch `RenderToWav`

In `dispatch/piano_roll.rs`:
1. Guard: no existing render, audio running, instrument selected, track has notes
2. Generate path: `~/.config/imbolc/renders/render_{instrument_id}_{timestamp}.wav`
3. Save `pending_render` with `was_looping = pr.looping`
4. Set `pr.playhead = pr.loop_start`, `pr.playing = true`, `pr.looping = false`
5. Call `audio.start_instrument_render(instrument_id, &path)`
6. Mark `audio_dirty.piano_roll = true`
7. Return status "Rendering..."

### 10. Handle `RenderComplete` feedback in main.rs

In the feedback drain loop:
1. Stop playback: `pr.playing = false`, `pr.playhead = 0`
2. Restore looping: `pr.looping = pending_render.was_looping`
3. Clear `pending_render`
4. Allocate a new buffer ID from `SampleRegistry`
5. Send `AudioCmd::LoadSample` with the rendered WAV path
6. Change instrument source: `instrument.source = SourceType::PitchedSampler`
7. Initialize `SamplerConfig` with the buffer, a single full-file slice at root note 60
8. Mark audio dirty (instruments + routing) to rebuild the synth chain
9. Show status: "Render complete"

### 11. Keybinding

In piano roll input handling, bind `R` (shift+r) to `"render_to_wav"` action:
```rust
"render_to_wav" => Action::PianoRoll(PianoRollAction::RenderToWav),
```

### 12. Render indicator

In piano roll rendering, when `state.pending_render.is_some()`, draw a "RENDERING" label in the header (red/orange text).

## Edge Cases

- **Already recording master**: `start_recording` returns `Err("Already recording")` — guard on this and show error status
- **No notes in loop region**: Check before starting. Notes exist on track but may not be within loop bounds — still render (silence sections are fine)
- **Release tails**: Add ~1 second of tail ticks after `loop_end` before stopping recording
- **Playback during render**: Other instruments will play too but only the target instrument's bus is recorded — no interference
- **Cancel**: Could add a cancel mechanism later; for now, render runs to completion

## Verification

1. `cargo build` — ensure it compiles
2. Launch with SuperCollider running
3. Create an instrument (e.g., Saw), add notes in the piano roll
4. Set loop region around the notes
5. Press `R` — should see "Rendering..." status
6. Playback runs through loop region then stops
7. WAV file appears in `~/.config/imbolc/renders/`
8. Instrument source changes to PitchedSampler
9. Playing notes now triggers the rendered sample
10. Original piano roll notes still present
