# SQLite Persistence

## File Format

- Extension: `.imbolc` or `.sqlite`
- Default path: `~/.config/imbolc/default.sqlite`
- Inspectable with any SQLite tool

## Save/Load Model

Imbolc saves a **full snapshot** of state:

1. Ensure schema tables exist (see `imbolc-core/src/state/persistence/schema.rs`).
2. `DELETE` all data tables.
3. Insert the current session/instrument/arrangement state.

Schema versioning is tracked in `schema_version` (`SCHEMA_VERSION = 12` as of this doc). If a schema change breaks load, delete the DB and resave.

## What’s Persisted (High Level)

- Session settings, mixer state, buses, layer groups
- Instruments + processing chains + sends + arpeggiator/groove
- Piano roll tracks/notes + arrangement clips/placements
- Automation (session + clip lanes/points)
- Sampler + drum sequencer + chopper state
- Custom synthdef + VST plugin registries, param values, VST state paths
- MIDI recording mappings and project metadata
- Checkpoints (snapshot saves)

## What’s Not Persisted

- Playback position and audio thread state
- Transient UI overlays / layer stack
- Visualization buffers (meters, scope, waveform)

## Schema Reference

The **authoritative schema** lives in:

- `imbolc-core/src/state/persistence/schema.rs`

Use that file as the single source of truth for table layout and field names.

