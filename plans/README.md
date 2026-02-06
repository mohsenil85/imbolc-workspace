# Imbolc Implementation Plans

This directory contains implementation plans for features, refactors, and architectural changes.

## Status Key

| Status | Meaning |
|--------|---------|
| COMPLETE | Fully implemented |
| IN_PROGRESS | Partially implemented or actively being worked on |
| FUTURE | Planned but not started |

## Plans by Status

### Complete

| Plan | Description |
|------|-------------|
| [phase1-dispatcher-trait.md](phase1-dispatcher-trait.md) | Dispatcher trait abstraction for local/remote dispatch |
| [copy-paste.md](copy-paste.md) | Copy/cut/paste with range selection for notes, steps, automation |
| [render-to-wav.md](render-to-wav.md) | Render instrument to WAV, convert to sampler |

### In Progress

| Plan | Description |
|------|-------------|
| [imbolc-net.md](imbolc-net.md) | Network layer for multi-client LAN collaboration |
| [groove-panel.md](groove-panel.md) | Per-track groove/swing/humanize controls |

### Future

| Plan | Description |
|------|-------------|
| [imbolc-gui-dioxus.md](imbolc-gui-dioxus.md) | Alternative GUI using Dioxus |
| [c-ffi-layer.md](c-ffi-layer.md) | C FFI bindings for external integration |
| [shared-state-server.md](shared-state-server.md) | State server for collaboration |
| [vst-3-improvments.md](vst-3-improvments.md) | VST3 hosting improvements |
| [new-source-types.md](new-source-types.md) | Additional oscillator/source types |
| [arrangement-timeline-view.md](arrangement-timeline-view.md) | Full arrangement/timeline view |
| [project-management.md](project-management.md) | Project management features |
| [dynamic-mixing-buses.md](dynamic-mixing-buses.md) | Dynamic bus allocation and routing |

### Architecture / Optimization

| Plan | Description |
|------|-------------|
| [architecture-async-io-state-consolidation.md](architecture-async-io-state-consolidation.md) | Async I/O and state consolidation |
| [separate-dispatch-audio-thread.md](separate-dispatch-audio-thread.md) | Dispatch/audio thread separation |
| [latency-jitter-architecture-review.md](latency-jitter-architecture-review.md) | Latency and jitter analysis |
| [low-jitter-refactor.md](low-jitter-refactor.md) | Low-jitter scheduling improvements |
| [phase4-incremental-mixer-diffs.md](phase4-incremental-mixer-diffs.md) | Incremental mixer state updates |
| [phase5-code-organization.md](phase5-code-organization.md) | Code organization improvements |

### Maintenance / Migration

| Plan | Description |
|------|-------------|
| [renderbuf-migration-remaining.md](renderbuf-migration-remaining.md) | Render buffer migration cleanup |
| [be-impromvents.md](be-impromvents.md) | Backend improvements |
| [remaining-features.md](remaining-features.md) | Remaining feature list |

## Adding a New Plan

1. Create a new `.md` file in this directory
2. Add a status header at the top:
   ```markdown
   **Status:** FUTURE
   **Last Updated:** YYYY-MM-DD
   ```
3. Add the plan to the appropriate section in this README
4. Link related items in [TASKS.md](../TASKS.md) if applicable

## See Also

- [TASKS.md](../TASKS.md) — Current bugs, features, and refactors
- [docs/](../docs/) — Technical documentation
- [CLAUDE.md](../CLAUDE.md) — Workspace overview
