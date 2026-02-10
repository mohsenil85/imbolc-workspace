# Imbolc Workspace

Multi-crate Rust workspace for Imbolc, a terminal-based DAW (Digital Audio Workstation).

## Workspace Structure

```
imbolc/
├── imbolc-ui/      Terminal UI binary (ratatui + crossterm)
├── imbolc-core/    Core engine (state, dispatch, persistence)
├── imbolc-audio/   Audio engine (SuperCollider OSC, playback, routing)
├── imbolc-types/   Shared type definitions
└── imbolc-net/     Network/collaboration layer
```

## Dependency Flow

```
imbolc-types  (leaf crate, no internal deps)
     │
     ├──────────────────┐
     ▼                  ▼
imbolc-audio          imbolc-net
(audio engine)        (collaboration)
     │
     ▼
imbolc-core   (state, dispatch, persistence)
     │
     ▼
imbolc-ui     (terminal interface)
```

## Building

```bash
cargo build                 # Build all crates
cargo build -p imbolc-ui    # Build UI only
cargo test                  # Run all tests
cargo run -p imbolc-ui      # Run the DAW
```

## Crate Responsibilities

| Crate | Purpose | Key Files |
|-------|---------|-----------|
| `imbolc-types` | Shared types: Instrument, Action, SourceType, etc. | `src/state/`, `src/action.rs` |
| `imbolc-audio` | Audio engine: SuperCollider OSC, playback, routing | `src/engine/`, `src/handle.rs` |
| `imbolc-core` | State management, action dispatch, persistence | `src/dispatch/`, `src/state/` |
| `imbolc-ui` | Terminal rendering, panes, keybindings | `src/panes/`, `src/ui/` |

## Code Map

**Read [CODE_MAP.md](CODE_MAP.md) first** for dispatch routing tables, type hierarchy, AudioDirty flags reference, module maps, and enum quick reference. This avoids re-exploring the codebase.

## Per-Crate Documentation

Each crate has its own `CLAUDE.md` with detailed guidance:

- [imbolc-ui/CLAUDE.md](imbolc-ui/CLAUDE.md) — UI architecture, pane system, keybindings
- [imbolc-core/CLAUDE.md](imbolc-core/CLAUDE.md) — State, dispatch, persistence
- [imbolc-audio/CLAUDE.md](imbolc-audio/CLAUDE.md) — Audio engine, SuperCollider, playback
- [imbolc-types/CLAUDE.md](imbolc-types/CLAUDE.md) — Shared type definitions
- [imbolc-net/CLAUDE.md](imbolc-net/CLAUDE.md) — Network protocol, LAN collaboration

## Common Workflows

### Adding a new action
1. Add variant to `Action` in `imbolc-types/src/action.rs`
2. Return it from pane handler in `imbolc-ui/src/panes/`
3. Handle it in `imbolc-core/src/dispatch/mod.rs`

### Adding a new pane
1. Create in `imbolc-ui/src/panes/`
2. Register in `imbolc-ui/src/main.rs`
3. Add keybinding if navigable

### Modifying state types
1. Update type in `imbolc-types/src/state/`
2. Update any dispatch handlers in `imbolc-core/`
3. Update persistence if needed in `imbolc-core/src/state/persistence/`

### Adding a new SynthDef
**Hard rule: One SynthDef per file.**
1. Create file in `imbolc-core/synthdefs/defs/<category>/<name>.scd`
2. Compile: `cd imbolc-core/synthdefs && sclang compile.scd`
3. Add `SourceType` variant in `imbolc-types/src/state/instrument/source_type.rs` if needed

See [imbolc-core/synthdefs/defs/README.md](imbolc-core/synthdefs/defs/README.md) for file template and conventions.

## Code Navigation (CCLSP)

An MCP server (`cclsp`) wraps rust-analyzer for LSP-powered code navigation. **Always use CCLSP tools for navigating and editing Rust code** — they understand types, scopes, and cross-file references. Only fall back to Grep/Glob for non-Rust files or text pattern searches that aren't symbol-based.

**Navigation:**
- `find_definition` — Jump to symbol definition
- `find_references` — Find all references to a symbol
- `find_workspace_symbols` — Search symbols by name across workspace
- `find_implementation` — Find implementations of traits/interfaces

**Analysis:**
- `get_diagnostics` — Get compiler errors/warnings for a file
- `get_hover` — Get type info and docs at a position
- `prepare_call_hierarchy` — Prepare for call hierarchy queries
- `get_incoming_calls` — Find all callers of a function
- `get_outgoing_calls` — Find all callees from a function

**Refactoring:**
- `rename_symbol` — Rename a symbol across the workspace
- `rename_symbol_strict` — Rename at a specific position (when multiple candidates)

**Maintenance:**
- `restart_server` — Restart LSP servers if results seem stale

All tools are prefixed `mcp__cclsp__` when invoking. Configuration: `.mcp.json` + `cclsp.json` at workspace root.

## Configuration

- Musical defaults: `~/.config/imbolc/config.toml`
- Keybindings: `~/.config/imbolc/keybindings.toml`
- Projects: SQLite databases (`.imbolc` / `.sqlite`)

## Documentation

All docs live at workspace root in `./docs/`:
- [docs/architecture.md](docs/architecture.md) — state ownership, instrument model, action dispatch
- [docs/architecture-deep-dive.md](docs/architecture-deep-dive.md) — threading model, latency, OSC timetags, voice allocation
- [docs/audio-routing.md](docs/audio-routing.md) — bus model, insert vs send, node ordering
- [docs/keybindings.md](docs/keybindings.md) — keybinding philosophy and conventions
- [docs/sqlite-persistence.md](docs/sqlite-persistence.md) — persistence schema
- [docs/custom-synthdef-plan.md](docs/custom-synthdef-plan.md) — custom SynthDef system
- [docs/network-scenarios.md](docs/network-scenarios.md) — deployment scenarios (local, LAN, pro setup)

## Scratch Space

[SCRATCH.md](SCRATCH.md) is your working scratchpad. Feel free to overwrite it with whatever you need — notes, analysis, intermediate results, code sketches. It's yours to use as additional thinking space.

## Task Tracking

- [TASKS.md](TASKS.md) — current bugs, features, and refactors
- [TASKS_DONE.md](TASKS_DONE.md) — completed work history
- [TASKS_ARCH.md](TASKS_ARCH.md) — architecture-level tasks (from `plans/questions.md`)

## Plans

Implementation plans live in `./plans/` at workspace root.
