use rusqlite::{Connection, Result as SqlResult};

/// Schema version for the relational format.
pub const SCHEMA_VERSION: i32 = 13;

/// Create all tables for the relational schema.
pub fn create_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(SCHEMA_SQL)
}

/// Delete all data from all tables (preserving schema).
pub fn delete_all_data(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(DELETE_ALL_SQL)
}

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

-- ============================================================
-- Session & Metadata
-- ============================================================

CREATE TABLE IF NOT EXISTS session (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bpm INTEGER NOT NULL,
    time_sig_num INTEGER NOT NULL,
    time_sig_denom INTEGER NOT NULL,
    key TEXT NOT NULL,
    scale TEXT NOT NULL,
    tuning_a4 REAL NOT NULL,
    snap INTEGER NOT NULL,
    next_instrument_id INTEGER NOT NULL,
    next_sampler_buffer_id INTEGER NOT NULL,
    selected_instrument INTEGER,
    next_layer_group_id INTEGER NOT NULL DEFAULT 0,
    -- humanize
    humanize_velocity REAL NOT NULL DEFAULT 0.0,
    humanize_timing REAL NOT NULL DEFAULT 0.0,
    -- click track
    click_enabled INTEGER NOT NULL DEFAULT 0,
    click_volume REAL NOT NULL DEFAULT 0.5,
    click_muted INTEGER NOT NULL DEFAULT 0,
    -- tuning system
    tuning TEXT NOT NULL DEFAULT 'EqualTemperament',
    ji_flavor TEXT NOT NULL DEFAULT 'FiveLimit'
);

CREATE TABLE IF NOT EXISTS theme (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    background_r INTEGER NOT NULL, background_g INTEGER NOT NULL, background_b INTEGER NOT NULL,
    foreground_r INTEGER NOT NULL, foreground_g INTEGER NOT NULL, foreground_b INTEGER NOT NULL,
    border_r INTEGER NOT NULL, border_g INTEGER NOT NULL, border_b INTEGER NOT NULL,
    selection_bg_r INTEGER NOT NULL, selection_bg_g INTEGER NOT NULL, selection_bg_b INTEGER NOT NULL,
    selection_fg_r INTEGER NOT NULL, selection_fg_g INTEGER NOT NULL, selection_fg_b INTEGER NOT NULL,
    muted_r INTEGER NOT NULL, muted_g INTEGER NOT NULL, muted_b INTEGER NOT NULL,
    error_r INTEGER NOT NULL, error_g INTEGER NOT NULL, error_b INTEGER NOT NULL,
    warning_r INTEGER NOT NULL, warning_g INTEGER NOT NULL, warning_b INTEGER NOT NULL,
    success_r INTEGER NOT NULL, success_g INTEGER NOT NULL, success_b INTEGER NOT NULL,
    osc_color_r INTEGER NOT NULL, osc_color_g INTEGER NOT NULL, osc_color_b INTEGER NOT NULL,
    filter_color_r INTEGER NOT NULL, filter_color_g INTEGER NOT NULL, filter_color_b INTEGER NOT NULL,
    env_color_r INTEGER NOT NULL, env_color_g INTEGER NOT NULL, env_color_b INTEGER NOT NULL,
    lfo_color_r INTEGER NOT NULL, lfo_color_g INTEGER NOT NULL, lfo_color_b INTEGER NOT NULL,
    fx_color_r INTEGER NOT NULL, fx_color_g INTEGER NOT NULL, fx_color_b INTEGER NOT NULL,
    sample_color_r INTEGER NOT NULL, sample_color_g INTEGER NOT NULL, sample_color_b INTEGER NOT NULL,
    midi_color_r INTEGER NOT NULL, midi_color_g INTEGER NOT NULL, midi_color_b INTEGER NOT NULL,
    audio_in_color_r INTEGER NOT NULL, audio_in_color_g INTEGER NOT NULL, audio_in_color_b INTEGER NOT NULL,
    meter_low_r INTEGER NOT NULL, meter_low_g INTEGER NOT NULL, meter_low_b INTEGER NOT NULL,
    meter_mid_r INTEGER NOT NULL, meter_mid_g INTEGER NOT NULL, meter_mid_b INTEGER NOT NULL,
    meter_high_r INTEGER NOT NULL, meter_high_g INTEGER NOT NULL, meter_high_b INTEGER NOT NULL,
    waveform_grad_0_r INTEGER NOT NULL, waveform_grad_0_g INTEGER NOT NULL, waveform_grad_0_b INTEGER NOT NULL,
    waveform_grad_1_r INTEGER NOT NULL, waveform_grad_1_g INTEGER NOT NULL, waveform_grad_1_b INTEGER NOT NULL,
    waveform_grad_2_r INTEGER NOT NULL, waveform_grad_2_g INTEGER NOT NULL, waveform_grad_2_b INTEGER NOT NULL,
    waveform_grad_3_r INTEGER NOT NULL, waveform_grad_3_g INTEGER NOT NULL, waveform_grad_3_b INTEGER NOT NULL,
    playing_r INTEGER NOT NULL, playing_g INTEGER NOT NULL, playing_b INTEGER NOT NULL,
    recording_r INTEGER NOT NULL, recording_g INTEGER NOT NULL, recording_b INTEGER NOT NULL,
    armed_r INTEGER NOT NULL, armed_g INTEGER NOT NULL, armed_b INTEGER NOT NULL
);

-- ============================================================
-- Instruments
-- ============================================================

CREATE TABLE IF NOT EXISTS instruments (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    source_type TEXT NOT NULL,
    filter_type TEXT,
    filter_cutoff REAL,
    filter_cutoff_min REAL,
    filter_cutoff_max REAL,
    filter_resonance REAL,
    filter_resonance_min REAL,
    filter_resonance_max REAL,
    filter_enabled INTEGER NOT NULL DEFAULT 1,
    lfo_enabled INTEGER NOT NULL DEFAULT 0,
    lfo_rate REAL NOT NULL DEFAULT 2.0,
    lfo_depth REAL NOT NULL DEFAULT 0.5,
    lfo_shape TEXT NOT NULL DEFAULT 'Sine',
    lfo_target TEXT NOT NULL DEFAULT 'FilterCutoff',
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
    output_target TEXT NOT NULL,
    channel_config TEXT NOT NULL DEFAULT 'Stereo',
    convolution_ir_path TEXT,
    layer_group INTEGER,
    next_effect_id INTEGER NOT NULL DEFAULT 0,
    -- EQ
    eq_enabled INTEGER,
    -- Arpeggiator
    arp_enabled INTEGER NOT NULL DEFAULT 0,
    arp_direction TEXT NOT NULL DEFAULT 'Up',
    arp_rate TEXT NOT NULL DEFAULT 'Eighth',
    arp_octaves INTEGER NOT NULL DEFAULT 1,
    arp_gate REAL NOT NULL DEFAULT 0.5,
    -- Chord
    chord_shape TEXT,
    -- VST
    vst_state_path TEXT,
    -- Groove
    groove_swing_amount REAL,
    groove_swing_grid TEXT,
    groove_humanize_velocity REAL,
    groove_humanize_timing REAL,
    groove_timing_offset_ms REAL NOT NULL DEFAULT 0.0,
    groove_time_sig_num INTEGER,
    groove_time_sig_denom INTEGER,
    layer_octave_offset INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS instrument_source_params (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value_type TEXT NOT NULL,
    param_value_float REAL,
    param_value_int INTEGER,
    param_value_bool INTEGER,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    PRIMARY KEY (instrument_id, position)
);

CREATE TABLE IF NOT EXISTS instrument_effects (
    instrument_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    effect_type TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    vst_state_path TEXT,
    PRIMARY KEY (instrument_id, effect_id)
);

CREATE TABLE IF NOT EXISTS instrument_effect_params (
    instrument_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value_type TEXT NOT NULL,
    param_value_float REAL,
    param_value_int INTEGER,
    param_value_bool INTEGER,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    PRIMARY KEY (instrument_id, effect_id, position)
);

CREATE TABLE IF NOT EXISTS instrument_sends (
    instrument_id INTEGER NOT NULL,
    bus_id INTEGER NOT NULL,
    level REAL NOT NULL,
    enabled INTEGER NOT NULL,
    tap_point TEXT NOT NULL DEFAULT 'PostInsert',
    PRIMARY KEY (instrument_id, bus_id)
);

CREATE TABLE IF NOT EXISTS instrument_modulations (
    instrument_id INTEGER NOT NULL,
    target_param TEXT NOT NULL,
    mod_type TEXT NOT NULL,
    lfo_enabled INTEGER,
    lfo_rate REAL,
    lfo_depth REAL,
    lfo_shape TEXT,
    lfo_target TEXT,
    env_attack REAL,
    env_decay REAL,
    env_sustain REAL,
    env_release REAL,
    source_instrument_id INTEGER,
    source_param_name TEXT,
    PRIMARY KEY (instrument_id, target_param)
);

CREATE TABLE IF NOT EXISTS instrument_filter_extra_params (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value_type TEXT NOT NULL,
    param_value_float REAL,
    param_value_int INTEGER,
    param_value_bool INTEGER,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    PRIMARY KEY (instrument_id, position)
);

CREATE TABLE IF NOT EXISTS instrument_eq_bands (
    instrument_id INTEGER NOT NULL,
    band_index INTEGER NOT NULL,
    band_type TEXT NOT NULL,
    freq REAL NOT NULL,
    gain REAL NOT NULL,
    q REAL NOT NULL,
    enabled INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, band_index)
);

CREATE TABLE IF NOT EXISTS instrument_processing_chain (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    stage_type TEXT NOT NULL,
    effect_id INTEGER,
    PRIMARY KEY (instrument_id, position)
);

CREATE TABLE IF NOT EXISTS instrument_vst_params (
    instrument_id INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (instrument_id, param_index)
);

CREATE TABLE IF NOT EXISTS effect_vst_params (
    instrument_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (instrument_id, effect_id, param_index)
);

-- ============================================================
-- Mixer
-- ============================================================

CREATE TABLE IF NOT EXISTS mixer_buses (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    level REAL NOT NULL,
    pan REAL NOT NULL,
    mute INTEGER NOT NULL,
    solo INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mixer_master (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    level REAL NOT NULL,
    mute INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS layer_group_mixers (
    group_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    level REAL NOT NULL,
    pan REAL NOT NULL,
    mute INTEGER NOT NULL,
    solo INTEGER NOT NULL,
    output_target TEXT NOT NULL,
    eq_enabled INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS layer_group_eq_bands (
    group_id INTEGER NOT NULL,
    band_index INTEGER NOT NULL,
    freq REAL NOT NULL,
    gain REAL NOT NULL,
    q REAL NOT NULL,
    enabled INTEGER NOT NULL,
    PRIMARY KEY (group_id, band_index)
);

CREATE TABLE IF NOT EXISTS layer_group_sends (
    group_id INTEGER NOT NULL,
    bus_id INTEGER NOT NULL,
    level REAL NOT NULL,
    enabled INTEGER NOT NULL,
    tap_point TEXT NOT NULL DEFAULT 'PostInsert',
    PRIMARY KEY (group_id, bus_id)
);

CREATE TABLE IF NOT EXISTS bus_effects (
    bus_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    effect_type TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    vst_state_path TEXT,
    PRIMARY KEY (bus_id, effect_id)
);

CREATE TABLE IF NOT EXISTS bus_effect_params (
    bus_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value_type TEXT NOT NULL,
    param_value_float REAL,
    param_value_int INTEGER,
    param_value_bool INTEGER,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    PRIMARY KEY (bus_id, effect_id, position)
);

CREATE TABLE IF NOT EXISTS bus_effect_vst_params (
    bus_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (bus_id, effect_id, param_index)
);

CREATE TABLE IF NOT EXISTS layer_group_effects (
    group_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    effect_type TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    vst_state_path TEXT,
    PRIMARY KEY (group_id, effect_id)
);

CREATE TABLE IF NOT EXISTS layer_group_effect_params (
    group_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_name TEXT NOT NULL,
    param_value_type TEXT NOT NULL,
    param_value_float REAL,
    param_value_int INTEGER,
    param_value_bool INTEGER,
    param_min REAL NOT NULL,
    param_max REAL NOT NULL,
    PRIMARY KEY (group_id, effect_id, position)
);

CREATE TABLE IF NOT EXISTS layer_group_effect_vst_params (
    group_id INTEGER NOT NULL,
    effect_id INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    value REAL NOT NULL,
    PRIMARY KEY (group_id, effect_id, param_index)
);

-- ============================================================
-- Piano Roll & Musical Settings
-- ============================================================

CREATE TABLE IF NOT EXISTS musical_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bpm REAL NOT NULL,
    time_sig_num INTEGER NOT NULL,
    time_sig_denom INTEGER NOT NULL,
    ticks_per_beat INTEGER NOT NULL,
    loop_start INTEGER NOT NULL,
    loop_end INTEGER NOT NULL,
    looping INTEGER NOT NULL,
    swing_amount REAL NOT NULL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS piano_roll_tracks (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    polyphonic INTEGER NOT NULL,
    PRIMARY KEY (instrument_id)
);

CREATE TABLE IF NOT EXISTS piano_roll_notes (
    id INTEGER PRIMARY KEY,
    track_instrument_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    pitch INTEGER NOT NULL,
    velocity INTEGER NOT NULL,
    probability REAL NOT NULL DEFAULT 1.0
);

-- ============================================================
-- Sampler
-- ============================================================

CREATE TABLE IF NOT EXISTS sampler_configs (
    instrument_id INTEGER PRIMARY KEY,
    buffer_id INTEGER,
    sample_name TEXT,
    loop_mode INTEGER NOT NULL,
    pitch_tracking INTEGER NOT NULL,
    next_slice_id INTEGER NOT NULL,
    selected_slice INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sampler_slices (
    instrument_id INTEGER NOT NULL,
    slice_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    start_pos REAL NOT NULL,
    end_pos REAL NOT NULL,
    name TEXT NOT NULL,
    root_note INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, slice_id)
);

-- ============================================================
-- VST Plugins
-- ============================================================

CREATE TABLE IF NOT EXISTS vst_plugins (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    plugin_path TEXT NOT NULL,
    kind TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vst_plugin_params (
    plugin_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    param_index INTEGER NOT NULL,
    name TEXT NOT NULL,
    default_val REAL NOT NULL,
    label TEXT,
    PRIMARY KEY (plugin_id, position),
    FOREIGN KEY (plugin_id) REFERENCES vst_plugins(id)
);

-- ============================================================
-- Automation
-- ============================================================

CREATE TABLE IF NOT EXISTS automation_lanes (
    id INTEGER PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER,
    target_bus_id INTEGER,
    target_effect_id INTEGER,
    target_param_idx INTEGER,
    target_extra TEXT,
    enabled INTEGER NOT NULL,
    record_armed INTEGER NOT NULL DEFAULT 0,
    min_value REAL NOT NULL,
    max_value REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS automation_points (
    lane_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    value REAL NOT NULL,
    curve_type TEXT NOT NULL DEFAULT 'Linear',
    PRIMARY KEY (lane_id, tick)
);

-- ============================================================
-- Custom SynthDefs
-- ============================================================

CREATE TABLE IF NOT EXISTS custom_synthdefs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    synthdef_name TEXT NOT NULL,
    source_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_synthdef_params (
    synthdef_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    default_val REAL NOT NULL,
    min_val REAL NOT NULL,
    max_val REAL NOT NULL,
    PRIMARY KEY (synthdef_id, position),
    FOREIGN KEY (synthdef_id) REFERENCES custom_synthdefs(id)
);

-- ============================================================
-- Drum Sequencer
-- ============================================================

CREATE TABLE IF NOT EXISTS drum_sequencer_state (
    instrument_id INTEGER PRIMARY KEY,
    current_pattern INTEGER NOT NULL DEFAULT 0,
    next_buffer_id INTEGER NOT NULL DEFAULT 0,
    swing_amount REAL NOT NULL DEFAULT 0.0,
    chain_enabled INTEGER NOT NULL DEFAULT 0,
    step_resolution TEXT NOT NULL DEFAULT 'Sixteenth'
);

CREATE TABLE IF NOT EXISTS drum_sequencer_chain (
    instrument_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    pattern_index INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, position)
);

CREATE TABLE IF NOT EXISTS drum_pads (
    instrument_id INTEGER NOT NULL,
    pad_index INTEGER NOT NULL,
    buffer_id INTEGER,
    path TEXT,
    name TEXT NOT NULL DEFAULT '',
    level REAL NOT NULL DEFAULT 0.8,
    slice_start REAL NOT NULL DEFAULT 0.0,
    slice_end REAL NOT NULL DEFAULT 1.0,
    reverse INTEGER NOT NULL DEFAULT 0,
    pitch INTEGER NOT NULL DEFAULT 0,
    trigger_instrument_id INTEGER,
    trigger_freq REAL NOT NULL DEFAULT 440.0,
    PRIMARY KEY (instrument_id, pad_index)
);

CREATE TABLE IF NOT EXISTS drum_patterns (
    instrument_id INTEGER NOT NULL,
    pattern_index INTEGER NOT NULL,
    length INTEGER NOT NULL DEFAULT 16,
    PRIMARY KEY (instrument_id, pattern_index)
);

CREATE TABLE IF NOT EXISTS drum_steps (
    instrument_id INTEGER NOT NULL,
    pattern_index INTEGER NOT NULL,
    pad_index INTEGER NOT NULL,
    step_index INTEGER NOT NULL,
    velocity INTEGER NOT NULL DEFAULT 100,
    probability REAL NOT NULL DEFAULT 1.0,
    pitch_offset INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (instrument_id, pattern_index, pad_index, step_index)
);

-- ============================================================
-- Sample Chopper
-- ============================================================

CREATE TABLE IF NOT EXISTS chopper_states (
    instrument_id INTEGER PRIMARY KEY,
    buffer_id INTEGER,
    path TEXT,
    name TEXT NOT NULL,
    selected_slice INTEGER NOT NULL,
    next_slice_id INTEGER NOT NULL,
    duration_secs REAL NOT NULL,
    waveform_peaks BLOB
);

CREATE TABLE IF NOT EXISTS chopper_slices (
    instrument_id INTEGER NOT NULL,
    slice_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    start_pos REAL NOT NULL,
    end_pos REAL NOT NULL,
    name TEXT NOT NULL,
    root_note INTEGER NOT NULL,
    PRIMARY KEY (instrument_id, slice_id)
);

-- ============================================================
-- MIDI Recording
-- ============================================================

CREATE TABLE IF NOT EXISTS midi_recording_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    live_input_instrument INTEGER,
    note_passthrough INTEGER NOT NULL,
    channel_filter INTEGER
);

CREATE TABLE IF NOT EXISTS midi_cc_mappings (
    id INTEGER PRIMARY KEY,
    cc_number INTEGER NOT NULL,
    channel INTEGER,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER,
    target_bus_id INTEGER,
    target_effect_id INTEGER,
    target_param_idx INTEGER,
    target_extra TEXT,
    min_value REAL NOT NULL,
    max_value REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS midi_pitch_bend_configs (
    id INTEGER PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER,
    target_bus_id INTEGER,
    target_effect_id INTEGER,
    target_param_idx INTEGER,
    target_extra TEXT,
    center_value REAL NOT NULL,
    range REAL NOT NULL,
    sensitivity REAL NOT NULL
);

-- ============================================================
-- Arrangement
-- ============================================================

CREATE TABLE IF NOT EXISTS arrangement_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    play_mode TEXT NOT NULL DEFAULT 'Pattern',
    selected_placement INTEGER,
    selected_lane INTEGER NOT NULL DEFAULT 0,
    view_start_tick INTEGER NOT NULL DEFAULT 0,
    ticks_per_col INTEGER NOT NULL DEFAULT 120,
    cursor_tick INTEGER NOT NULL DEFAULT 0,
    next_clip_id INTEGER NOT NULL DEFAULT 0,
    next_placement_id INTEGER NOT NULL DEFAULT 0,
    next_clip_automation_lane_id INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS arrangement_clips (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    instrument_id INTEGER NOT NULL,
    length_ticks INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS arrangement_clip_notes (
    clip_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    pitch INTEGER NOT NULL,
    velocity INTEGER NOT NULL,
    probability REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (clip_id, position)
);

CREATE TABLE IF NOT EXISTS arrangement_placements (
    id INTEGER PRIMARY KEY,
    clip_id INTEGER NOT NULL,
    instrument_id INTEGER NOT NULL,
    start_tick INTEGER NOT NULL,
    length_override INTEGER
);

CREATE TABLE IF NOT EXISTS arrangement_clip_automation_lanes (
    id INTEGER PRIMARY KEY,
    clip_id INTEGER NOT NULL,
    target_type TEXT NOT NULL,
    target_instrument_id INTEGER,
    target_bus_id INTEGER,
    target_effect_id INTEGER,
    target_param_idx INTEGER,
    target_extra TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    record_armed INTEGER NOT NULL DEFAULT 0,
    min_value REAL NOT NULL DEFAULT 0.0,
    max_value REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE IF NOT EXISTS arrangement_clip_automation_points (
    lane_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    value REAL NOT NULL,
    curve_type TEXT NOT NULL DEFAULT 'Linear',
    PRIMARY KEY (lane_id, tick)
);

-- ============================================================
-- Checkpoints (Phase 2+3)
-- ============================================================

CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL,
    parent_id INTEGER REFERENCES checkpoints(id),
    session_blob BLOB NOT NULL,
    instrument_blob BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoint_changesets (
    id INTEGER PRIMARY KEY,
    checkpoint_id INTEGER NOT NULL REFERENCES checkpoints(id),
    changeset BLOB NOT NULL
);
";

const DELETE_ALL_SQL: &str = "
DELETE FROM schema_version;
DELETE FROM session;
DELETE FROM theme;
DELETE FROM instruments;
DELETE FROM instrument_source_params;
DELETE FROM instrument_effects;
DELETE FROM instrument_effect_params;
DELETE FROM instrument_sends;
DELETE FROM instrument_modulations;
DELETE FROM instrument_filter_extra_params;
DELETE FROM instrument_eq_bands;
DELETE FROM instrument_processing_chain;
DELETE FROM instrument_vst_params;
DELETE FROM effect_vst_params;
DELETE FROM mixer_buses;
DELETE FROM mixer_master;
DELETE FROM layer_group_mixers;
DELETE FROM layer_group_eq_bands;
DELETE FROM layer_group_sends;
DELETE FROM bus_effects;
DELETE FROM bus_effect_params;
DELETE FROM bus_effect_vst_params;
DELETE FROM layer_group_effects;
DELETE FROM layer_group_effect_params;
DELETE FROM layer_group_effect_vst_params;
DELETE FROM musical_settings;
DELETE FROM piano_roll_tracks;
DELETE FROM piano_roll_notes;
DELETE FROM sampler_configs;
DELETE FROM sampler_slices;
DELETE FROM vst_plugins;
DELETE FROM vst_plugin_params;
DELETE FROM automation_lanes;
DELETE FROM automation_points;
DELETE FROM custom_synthdefs;
DELETE FROM custom_synthdef_params;
DELETE FROM drum_sequencer_state;
DELETE FROM drum_sequencer_chain;
DELETE FROM drum_pads;
DELETE FROM drum_patterns;
DELETE FROM drum_steps;
DELETE FROM chopper_states;
DELETE FROM chopper_slices;
DELETE FROM midi_recording_settings;
DELETE FROM midi_cc_mappings;
DELETE FROM midi_pitch_bend_configs;
DELETE FROM arrangement_state;
DELETE FROM arrangement_clips;
DELETE FROM arrangement_clip_notes;
DELETE FROM arrangement_placements;
DELETE FROM arrangement_clip_automation_lanes;
DELETE FROM arrangement_clip_automation_points;
";
