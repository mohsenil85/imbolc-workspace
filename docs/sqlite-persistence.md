# SQLite Persistence

## File Format

- Extension: `.imbolc` or `.sqlite`
- Default path: `~/.config/imbolc/default.sqlite`
- Can be inspected with any SQLite tool (`sqlite3`, DB Browser, etc.)

## Dev Strategy

During development iteration, don't worry about migrations. The save path does a full DELETE + re-INSERT of all data, so old databases just get overwritten with the current schema. If the schema changes in a way that breaks loading, delete the database file and start fresh. Real migration infrastructure can come later once the schema stabilizes.

## Save/Load Model

Save is a full snapshot: delete everything, then insert current state. No partial updates.

```
save_project(path, session, instruments)
  1. Open connection
  2. Run tolerant renames (if needed)
  3. CREATE TABLE IF NOT EXISTS for all tables
  4. DELETE FROM all data tables
  5. INSERT schema_version (6)
  6. INSERT session (metadata + selected_instrument + selected_automation_lane)
  7. save_instruments          — instruments table
  8. save_source_params        — instrument_source_params
  9. save_effects              — instrument_effects + instrument_effect_params
  10. save_sends               — instrument_sends
  11. save_modulations         — instrument_modulations
  12. save_mixer               — mixer_buses + mixer_master
  13. save_piano_roll          — piano_roll_tracks + piano_roll_notes + musical_settings
  14. save_sampler_configs     — sampler_configs + sampler_slices
  15. save_automation          — automation_lanes + automation_points
  16. save_custom_synthdefs    — custom_synthdefs + custom_synthdef_params
  17. save_vst_plugins         — vst_plugins + vst_plugin_params
  18. save_drum_sequencers     — drum_pads + drum_patterns + drum_steps
  19. save_chopper_states      — chopper_states + chopper_slices
  20. save_midi_recording      — midi_recording_settings + midi_cc_mappings + midi_pitch_bend_configs
  21. save_vst_param_values    — instrument_vst_params
  22. save_effect_vst_params   — effect_vst_params
```

Load is the reverse. Playback state (`playing`, `playhead`, `current_step`, `step_accumulator`) is intentionally transient and resets on load.

## Schema (v6)

### Session & Metadata

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE session (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    next_instrument_id INTEGER NOT NULL,
    selected_instrument INTEGER,
    selected_automation_lane INTEGER
);
```

### Instruments

```sql
CREATE TABLE instruments (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    source_type TEXT NOT NULL,      -- saw, sin, sqr, tri, audio_in, bus_in, sample, kit, custom:N
    filter_type TEXT,               -- lpf, hpf, bpf (nullable = no filter)
    filter_cutoff REAL,
    filter_resonance REAL,
    lfo_enabled INTEGER NOT NULL DEFAULT 0,
    lfo_rate REAL NOT NULL DEFAULT 2.0,
    lfo_depth REAL NOT NULL DEFAULT 0.5,
    lfo_shape TEXT NOT NULL DEFAULT 'sine',    -- sine, square, saw, triangle
    lfo_target TEXT NOT NULL DEFAULT 'filter', -- filter_cutoff, filter_res, amp, pitch, pan, ...
    amp_attack REAL NOT NULL,
    amp_decay REAL NOT NULL,
    amp_sustain REAL NOT NULL,
    amp_release REAL NOT NULL,
    polyphonic INTEGER NOT NULL,
    level REAL NOT NULL,
    pan REAL NOT NULL,
    mute INTEGER NOT NULL,
    solo INTEGER NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    output_target TEXT NOT NULL     -- "master" or "bus:N"
);

CREATE TABLE instrument_source_params (
    instrument_id INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value REAL NOT NULL,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    param_type TEXT NOT NULL,      -- float, int, bool
    PRIMARY KEY (instrument_id, param_name)
);

CREATE TABLE instrument_effects (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    effect_type TEXT NOT NULL,     -- delay, reverb, gate, tape_comp, sidechain_comp, vst:N
    enabled INTEGER NOT NULL,
    vst_state_path TEXT,           -- path to saved VST state file (for VST effects)
    PRIMARY KEY (instrument_id, position)
);

CREATE TABLE instrument_effect_params (
    instrument_id INTEGER NOT NULL,
    effect_position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value REAL NOT NULL,
    PRIMARY KEY (instrument_id, effect_position, param_name)
);

CREATE TABLE instrument_sends (
    instrument_id INTEGER NOT NULL,
    bus_id INTEGER NOT NULL,
    level REAL NOT NULL,
    enabled INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, bus_id)
);

CREATE TABLE instrument_modulations (
    instrument_id INTEGER NOT NULL,
    target_param TEXT NOT NULL,    -- "cutoff", "resonance"
    mod_type TEXT NOT NULL,        -- "lfo", "envelope", "instrument_param"
    lfo_rate REAL,
    lfo_depth REAL,
    env_attack REAL,
    env_decay REAL,
    env_sustain REAL,
    env_release REAL,
    source_instrument_id INTEGER,
    source_param_name TEXT,
    PRIMARY KEY (instrument_id, target_param)
);
```

### Mixer

```sql
CREATE TABLE mixer_buses (
    id INTEGER PRIMARY KEY,       -- 1-8
    name TEXT NOT NULL,
    level REAL NOT NULL,
    pan REAL NOT NULL,
    mute INTEGER NOT NULL,
    solo INTEGER NOT NULL
);

CREATE TABLE mixer_master (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    level REAL NOT NULL,
    mute INTEGER NOT NULL
);
```

### Piano Roll & Musical Settings

```sql
CREATE TABLE piano_roll_tracks (
    instrument_id INTEGER PRIMARY KEY,
    position INTEGER NOT NULL,
    polyphonic INTEGER NOT NULL
);

CREATE TABLE piano_roll_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_instrument_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    pitch INTEGER NOT NULL,
    velocity INTEGER NOT NULL
);

CREATE TABLE musical_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bpm REAL NOT NULL,
    time_sig_num INTEGER NOT NULL,
    time_sig_denom INTEGER NOT NULL,
    ticks_per_beat INTEGER NOT NULL,
    loop_start INTEGER NOT NULL,
    loop_end INTEGER NOT NULL,
    looping INTEGER NOT NULL,
    key TEXT NOT NULL DEFAULT 'C',
    scale TEXT NOT NULL DEFAULT 'Major',
    tuning_a4 REAL NOT NULL DEFAULT 440.0,
    snap INTEGER NOT NULL DEFAULT 0
);
```

### Sampler

```sql
CREATE TABLE sampler_configs (
    instrument_id INTEGER PRIMARY KEY,
    buffer_id INTEGER,
    sample_name TEXT,
    loop_mode INTEGER NOT NULL,
    pitch_tracking INTEGER NOT NULL,
    next_slice_id INTEGER NOT NULL,
    selected_slice INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE sampler_slices (
    instrument_id INTEGER NOT NULL,
    slice_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    start_pos REAL NOT NULL,
    end_pos REAL NOT NULL,
    name TEXT NOT NULL,
    root_note INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, slice_id)
);
```

### VST Plugins

```sql
CREATE TABLE vst_plugins (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    plugin_path TEXT NOT NULL,
    kind TEXT NOT NULL
);

CREATE TABLE vst_plugin_params (
    plugin_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    name TEXT NOT NULL,
    default_val REAL NOT NULL,
    PRIMARY KEY (plugin_id, position),
    FOREIGN KEY (plugin_id) REFERENCES vst_plugins(id)
);
```

### VST Instance Parameters

```sql
CREATE TABLE instrument_vst_params (
    instrument_id INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (instrument_id, param_index)
);

CREATE TABLE effect_vst_params (
    instrument_id INTEGER NOT NULL,
    effect_position INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (instrument_id, effect_position, param_index)
);
```

### Automation

```sql
CREATE TABLE automation_lanes (
    id INTEGER PRIMARY KEY,
    target_type TEXT NOT NULL,            -- instrument_level, instrument_pan, filter_cutoff,
                                         -- filter_resonance, effect_param, sample_rate, sample_amp
    target_instrument_id INTEGER NOT NULL,
    target_effect_idx INTEGER,
    target_param_idx INTEGER,
    enabled INTEGER NOT NULL,
    min_value REAL NOT NULL,
    max_value REAL NOT NULL
);

CREATE TABLE automation_points (
    lane_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    value REAL NOT NULL,
    curve_type TEXT NOT NULL,             -- linear, exponential, step, scurve
    PRIMARY KEY (lane_id, tick)
);
```

### Custom SynthDefs

```sql
CREATE TABLE custom_synthdefs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    synthdef_name TEXT NOT NULL,
    source_path TEXT NOT NULL
);

CREATE TABLE custom_synthdef_params (
    synthdef_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    default_val REAL NOT NULL,
    min_val REAL NOT NULL,
    max_val REAL NOT NULL,
    PRIMARY KEY (synthdef_id, position),
    FOREIGN KEY (synthdef_id) REFERENCES custom_synthdefs(id)
);
```

### Drum Sequencer

```sql
CREATE TABLE drum_pads (
    instrument_id INTEGER NOT NULL,
    pad_index INTEGER NOT NULL,
    buffer_id INTEGER,
    path TEXT,
    name TEXT NOT NULL DEFAULT '',
    level REAL NOT NULL DEFAULT 0.8,
    PRIMARY KEY (instrument_id, pad_index)
);

CREATE TABLE drum_patterns (
    instrument_id INTEGER NOT NULL,
    pattern_index INTEGER NOT NULL,
    length INTEGER NOT NULL DEFAULT 16,
    PRIMARY KEY (instrument_id, pattern_index)
);

CREATE TABLE drum_steps (
    instrument_id INTEGER NOT NULL,
    pattern_index INTEGER NOT NULL,
    pad_index INTEGER NOT NULL,
    step_index INTEGER NOT NULL,
    velocity INTEGER NOT NULL DEFAULT 100,
    PRIMARY KEY (instrument_id, pattern_index, pad_index, step_index)
);
```

### Sample Chopper

```sql
CREATE TABLE chopper_states (
    instrument_id INTEGER PRIMARY KEY,
    buffer_id INTEGER,
    path TEXT,
    name TEXT NOT NULL,
    selected_slice INTEGER NOT NULL,
    next_slice_id INTEGER NOT NULL,
    duration_secs REAL NOT NULL
);

CREATE TABLE chopper_slices (
    instrument_id INTEGER NOT NULL,
    slice_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    start_pos REAL NOT NULL,
    end_pos REAL NOT NULL,
    name TEXT NOT NULL,
    root_note INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, slice_id)
);
```

### MIDI Recording

```sql
CREATE TABLE midi_recording_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    live_input_instrument INTEGER,
    note_passthrough INTEGER NOT NULL,
    channel_filter INTEGER
);

CREATE TABLE midi_cc_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cc_number INTEGER NOT NULL,
    channel INTEGER,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER NOT NULL,
    target_effect_idx INTEGER,
    target_param_idx INTEGER,
    min_value REAL NOT NULL,
    max_value REAL NOT NULL
);

CREATE TABLE midi_pitch_bend_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER NOT NULL,
    target_effect_idx INTEGER,
    target_param_idx INTEGER,
    center_value REAL NOT NULL,
    range REAL NOT NULL,
    sensitivity REAL NOT NULL
);
```

## Intentionally Not Persisted

| Field | Why |
|-------|-----|
| `playing`, `playhead`, `current_step`, `step_accumulator` | Playback resets on load |
| `audio_in_waveform` | Runtime visualization data |
| `waveform_peaks` (chopper) | Regenerated from audio buffer at runtime |
| `record_mode` | Always starts as `Off` |
| `scsynth_process` | Audio engine state rebuilt on connect |

## Known Issues

- **No transaction wrapper on save** — if the process crashes mid-save, the database can be left with partial data (DELETEs completed but not all INSERTs). Should wrap save in `BEGIN`/`COMMIT`.
- **No WAL mode** — could add `PRAGMA journal_mode=WAL` for better write performance.
