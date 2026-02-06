# AGENTS.md

Quick start for AI agents working on this repo.

## Read first
- `CLAUDE.md` — primary conventions and codebase map.
- `docs/architecture.md` — state ownership, panes, dispatch flow.
- `docs/ai-coding-affordances.md` — patterns that avoid AI-specific footguns.

## Source of truth
- Keybindings: `keybindings.toml` (use `?` in-app for context help).
- Config defaults: `config.toml` (user overrides live in `~/.config/imbolc/`).
- Actions/dispatch: `src/ui/pane.rs`, `src/dispatch.rs`.

## Build & test
```bash
cargo ck         # fast typecheck (alias)
cargo build      # full build
cargo test --bin imbolc
cargo test
```

## Navigation tips
- Panes never mutate state directly; they return `Action`s handled in `src/dispatch.rs`.
- Input is layer-resolved; pane handlers receive action strings via `handle_action`.
- For layout, prefer `ui::layout_helpers::center_rect`.

## Troubleshooting
- Log AI friction in `COMMENTBOX.md`.
- LSP: `cclsp` is configured via `.mcp.json` / `cclsp.json`.
