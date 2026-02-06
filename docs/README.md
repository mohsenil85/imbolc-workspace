# Imbolc Documentation

This directory contains technical documentation for the Imbolc DAW.

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
| [lfo-targets-implementation.md](lfo-targets-implementation.md) | LFO modulation targets and implementation |

## Features

| Document | Description |
|----------|-------------|
| [keybindings.md](keybindings.md) | Keybinding philosophy, conventions, configuration |
| [custom-synthdef-plan.md](custom-synthdef-plan.md) | Custom SynthDef system design and integration |
| [vst3-support-roadmap.md](vst3-support-roadmap.md) | VST3 plugin support roadmap and implementation |

## Persistence

| Document | Description |
|----------|-------------|
| [sqlite-persistence.md](sqlite-persistence.md) | SQLite schema, save/load implementation |

## Sequencing

| Document | Description |
|----------|-------------|
| [sequencer-musical-settings.md](sequencer-musical-settings.md) | Musical settings for the drum sequencer |
| [sequencer-swing.md](sequencer-swing.md) | Swing and groove implementation |

## Network

| Document | Description |
|----------|-------------|
| [network-scenarios.md](network-scenarios.md) | Deployment scenarios: solo, LAN jam, pro setup |

## Development

| Document | Description |
|----------|-------------|
| [ai-coding-affordances.md](ai-coding-affordances.md) | AI coding agent guidance and patterns |
| [ai-integration.md](ai-integration.md) | AI tool integration notes |
| [lessons-learned.md](lessons-learned.md) | Development lessons and retrospectives |

## Archive

| Document | Description |
|----------|-------------|
| [vst-integration.md](vst-integration.md) | Stub - superseded by vst3-support-roadmap.md |
| [old-sibling-repo-recovery.md](old-sibling-repo-recovery.md) | Historical notes on repo recovery |

## See Also

- [CLAUDE.md](../CLAUDE.md) — Workspace overview for AI agents
- [plans/](../plans/) — Implementation plans
- [TASKS.md](../TASKS.md) — Current bugs, features, refactors
