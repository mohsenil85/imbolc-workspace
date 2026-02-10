# Imbolc Documentation

Reference documentation for the Imbolc DAW. Per-crate `CLAUDE.md` files are the primary living contracts; these docs provide deeper architectural detail.

## Architecture

| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | State ownership, instrument model, action dispatch, pane system |
| [architecture-deep-dive.md](architecture-deep-dive.md) | Threading model, latency, OSC timetags, voice allocation |
| [sc-engine-architecture.md](sc-engine-architecture.md) | SuperCollider engine internals, node graph, synth management |

## Audio

| Document | Description |
|----------|-------------|
| [audio-routing.md](audio-routing.md) | Bus model, insert vs send effects, node ordering |
| [polyphonic-voice-allocation.md](polyphonic-voice-allocation.md) | Voice stealing, note tracking, polyphony modes |

## Features

| Document | Description |
|----------|-------------|
| [keybindings.md](keybindings.md) | Keybinding philosophy, conventions, configuration |

## Persistence

| Document | Description |
|----------|-------------|
| [sqlite-persistence.md](sqlite-persistence.md) | SQLite schema, save/load implementation |

## See Also

- [CLAUDE.md](../CLAUDE.md) — Workspace overview
- [plans/](../plans/) — Implementation plans (custom-synthdef, VST3 roadmap, network scenarios, scaling analysis)
- [TASKS.md](../TASKS.md) — Current bugs, features, refactors
