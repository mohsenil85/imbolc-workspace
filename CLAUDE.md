# Imbolc Workspace

Multi-crate Rust workspace for Imbolc, a terminal-based DAW (Digital Audio Workstation).

## Workspace Structure

```
imbolc/
├── imbolc-ui/      Terminal UI binary (ratatui + crossterm)
├── imbolc-core/    Core engine (state, dispatch, audio, persistence)
├── imbolc-types/   Shared type definitions
└── imbolc-net/     (future) Network/collaboration layer
```

## Dependency Flow

```
imbolc-types  (leaf crate, no internal deps)
     │
     ▼
imbolc-core   (state, dispatch, audio engine)
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
| `imbolc-core` | State management, action dispatch, SuperCollider audio | `src/dispatch/`, `src/audio/` |
| `imbolc-ui` | Terminal rendering, panes, keybindings | `src/panes/`, `src/ui/` |

## Per-Crate Documentation

Each crate has its own `CLAUDE.md` with detailed guidance:

- [imbolc-ui/CLAUDE.md](imbolc-ui/CLAUDE.md) — UI architecture, pane system, keybindings
- [imbolc-core/CLAUDE.md](imbolc-core/CLAUDE.md) — State, dispatch, audio engine
- [imbolc-types/CLAUDE.md](imbolc-types/CLAUDE.md) — Shared type definitions

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

## Configuration

- Musical defaults: `~/.config/imbolc/config.toml`
- Keybindings: `~/.config/imbolc/keybindings.toml`
- Projects: SQLite databases (`.imbolc` / `.sqlite`)

## Documentation

All docs live at workspace root in `./docs/`:
- [docs/architecture.md](docs/architecture.md) — state ownership, instrument model, action dispatch
- [docs/audio-routing.md](docs/audio-routing.md) — bus model, insert vs send, node ordering
- [docs/keybindings.md](docs/keybindings.md) — keybinding philosophy and conventions
- [docs/sqlite-persistence.md](docs/sqlite-persistence.md) — persistence schema
- [docs/custom-synthdef-plan.md](docs/custom-synthdef-plan.md) — custom SynthDef system

## Plans

Implementation plans live in `./plans/` at workspace root.
