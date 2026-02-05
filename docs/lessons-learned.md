# Lessons Learned

Insights from building TUI DAW across iterations (Clojure → Java → potentially Go).

## Architecture

### State Management
- **Immutable state + pure transitions**: `RackState` records with `StateTransitions` class works extremely well. Easy to reason about, trivially testable, undo/redo is just a stack of states.
- **Dispatcher pattern**: Per-view dispatchers (`RackViewDispatcher`, `MixerViewDispatcher`) keep action handling focused and prevent god-class dispatchers.
- **Single source of truth**: All UI state in one state tree, renderers are pure functions of state.

### Input Handling
- **One keybinding scheme**: No modes, no chords, just direct key→action mapping. Cognitive overhead of modal interfaces (vim-style) isn't worth it for a DAW.
- **Uppercase for destructive/toggle actions**: `M` for mute, `S` for solo, lowercase for navigation. Clear visual distinction.

### Persistence
- **SQLite from day 1**: Atomic saves, single-file sharing (`.imbolc`), queryable for search/presets, schema migrations built-in. JSON files seem simpler but cause pain later.

### Terminal UI
- **Abstract the terminal library immediately**: Don't let `TextGraphics`/`KeyStroke` (or Go's tcell types) leak everywhere. Create `Graphics`, `InputEvent` interfaces from the start.
- **Semantic colors/theming**: Abstract from raw ANSI early. `SemanticColor.ACCENT` not `TextColor.CYAN`.
- **Minimum screen size**: Pick a reasonable default (96x32). Don't assume large terminals.
- **`--no-audio` flag from start**: Essential for testing, development, and CI.
- **Don't rely on style for state**: Use explicit characters, not just bold/color, to show selection, mute, solo, etc. `▸` for selected, `[M]` for muted, `[S]` for soloed. Reason: `tmux capture-pane` (E2E tests) only captures text, not styling. If state is only shown via color/bold, tests can't verify it.

### Audio
- **Separate process for audio engine**: OSC to SuperCollider (or similar) keeps audio isolated. Crash in audio doesn't kill UI. Can swap engines.
- **Headless mode**: Audio engine should be optional. UI should work without it.

## Testing

### E2E Testing
- **tmux-based E2E tests**: Catches real integration issues that unit tests miss.
- **Plan for nested tmux**: Test harness must unset `TMUX` env var to allow nested sessions.
- **`--no-audio` in tests**: E2E tests shouldn't require audio hardware.
- **Style-based selection is invisible in captures**: tmux `capture-pane` only gets text, not colors/bold. Tests can't assert on visual selection indicators.

### Unit Testing
- **Pure state transitions are trivially testable**: `StateTransitions.addModule(state, module)` returns new state - no mocks needed.
- **Test behavior, not implementation**: Assert on resulting state, not internal method calls.

## Development Process

### Parallel Development
- **Git worktrees for agents**: Each agent works in its own worktree on its own branch. No stepping on each other.
- **ORCHESTRATOR.md pattern**: Document how to run parallel agents, batch sizes, merge strategy.
- **Task files**: Structured task lists (`tasks/*.md`) that agents can parse and execute.
- **Merge early, merge often**: Don't let branches diverge too far.

### Commits
- **Small, focused commits**: Easier to review, revert, and understand.
- **Conventional commit messages**: `feat:`, `fix:`, `docs:` prefixes help with changelogs.

## Language-Specific

### Java
- **Records for immutable data**: `record RackState(...)` is concise and correct.
- **Sealed interfaces for sum types**: `sealed interface Action permits...` provides exhaustiveness checking.
- **Switch expressions**: Pattern matching on enums/sealed types is clean.

### Clojure (v1)
- **REPL-driven development was great for exploration**
- **Immutability by default was perfect for state management**
- **Startup time was painful for CLI tool**
- **Error messages were cryptic**
- **Refactoring without types was scary at scale**

---

## Go Considerations (v3?)

### Pros
- **Fast startup**: No JVM warmup, instant CLI feel
- **Single binary**: Easy distribution, no runtime dependencies
- **Great concurrency**: Goroutines for audio/UI/input handling
- **tcell/bubbletea ecosystem**: Mature TUI libraries
- **Cross-compilation**: Easy builds for all platforms

### Cons
- **No sum types**: Have to fake with interfaces + type switches (error-prone)
- **No generics until recently**: Collections are less type-safe than Java
- **Error handling verbosity**: `if err != nil` everywhere
- **No immutability enforcement**: Have to be disciplined, language won't help

### Recommendations if Go
- **Use bubbletea**: Elm-architecture for TUI, fits the immutable state model
- **Define clear interfaces early**: `Graphics`, `InputEvent`, `AudioEngine`
- **Embrace structs as values**: Copy instead of mutate, like records
- **Consider ogen/sqlc**: Type-safe database access
- **Separate packages**: `engine/`, `audio/`, `state/`, `ui/` from the start

### Alternative: Rust

If willing to learn the borrow checker:

**Pros:**
- **True sum types**: `enum Action { ... }` with exhaustive matching
- **Immutability by default**: `let` vs `let mut`
- **ratatui ecosystem**: Excellent TUI library
- **No GC pauses**: Relevant for real-time audio
- **Compiler catches more bugs**: If it compiles, it usually works
- **Great OSC support**: `rosc` crate is solid, `async-osc` for non-blocking

**Cons:**
- **Steeper learning curve**: Ownership/borrowing takes time to internalize
- **Slower iteration**: Fighting the compiler vs just running code
- **Async complexity**: `Send + Sync + 'static` bounds can be confusing

**OSC in Rust:**
```rust
use rosc::{OscMessage, OscType, OscPacket, encoder};

let msg = OscMessage {
    addr: "/synth/freq".to_string(),
    args: vec![OscType::Float(440.0)],
};
let buf = encoder::encode(&OscPacket::Message(msg))?;
socket.send_to(&buf, "127.0.0.1:57110")?;
```

**AI + Rust Reality:**

| Aspect | Assessment |
|--------|------------|
| Syntax/basics | Excellent - pattern matching, structs, enums |
| Ownership/borrowing | Good for simple cases, struggles with complex lifetimes |
| Async/tokio | Decent, sometimes gets `Send`/`Sync` bounds wrong |
| Crate ecosystem | Good at finding and using popular crates |
| Compiler errors | Actually helpful - AI can read rustc errors and fix |

The secret advantage: Rust's compiler is so strict that AI-generated code either compiles and works, or fails with helpful errors. Less "silent bugs" than dynamic languages. The AI + rustc feedback loop is productive.

**Where AI struggles with Rust:**
- Complex lifetime annotations (`'a`, `'b`, `where` bounds)
- Interior mutability (`RefCell`, `Arc<Mutex>`)
- Async trait bounds (`impl Future<Output=...> + Send + 'static`)
- Macro-heavy code

**For a TUI DAW**: Mostly "simple Rust" - structs, enums, pattern matching, basic ownership. The hairy lifetime stuff shows up more in libraries than applications.

**Recommended crates for Rust TUI DAW:**
- `ratatui` - TUI framework (successor to tui-rs)
- `crossterm` - Terminal backend (cross-platform)
- `rosc` / `async-osc` - OSC protocol
- `rusqlite` - SQLite bindings
- `tokio` - Async runtime (if needed)
- `serde` - Serialization
