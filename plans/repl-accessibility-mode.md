# REPL Accessibility Mode for Imbolc

## Context

Imbolc's ratatui TUI is fundamentally inaccessible to screen readers — the alternate screen buffer with 60fps redraws causes cursor spam, input lag, and even screen reader crashes. Rather than trying to bolt accessibility onto ratatui, we're adding a REPL mode that uses plain stdin/stdout (main terminal buffer), which screen readers handle natively.

**Goal:** A `--repl` entry point that provides full DAW control via text commands, with an extensible parser that starts at ~20 commands and grows to full coverage. Text-only output initially (TTS later). Later, a TUI pane version too.

## Architecture

```
imbolc-ui/src/
  repl/
    mod.rs          — REPL loop (stdin/stdout, readline, dispatch)
    commands.rs     — Command registry + all command definitions
    display.rs      — Human-readable formatting for state types
    parse.rs        — Tokenizer + argument parser
```

The REPL shares the exact same `LocalDispatcher` + `AudioHandle` pipeline as the TUI. Commands either query `&AppState` or return `Action` variants for dispatch. No new state mutation paths.

## Entry Point

**File:** `imbolc-ui/src/main.rs`

Add `--repl` flag alongside existing `--server`, `--discover`, `--connect`:

```rust
let repl_mode = args.iter().any(|a| a == "--repl");
if repl_mode {
    return repl::run_repl(project_arg);
}
```

`run_repl()` initializes the same way as `run()` — creates `LocalDispatcher`, `AudioHandle`, loads config — but instead of entering the ratatui render loop, enters a readline loop on stdin.

## Command Parser Design

### Core Types (`repl/parse.rs` + `repl/commands.rs`)

```rust
/// Result of executing a command
enum CommandResult {
    /// Print text output (for queries)
    Output(String),
    /// Dispatch an action (for mutations)
    Dispatch(Action),
    /// Print output AND dispatch
    OutputAndDispatch(String, Action),
    /// Quit the REPL
    Quit,
}

/// A registered command
struct CommandDef {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    usage: &'static str,
    handler: fn(args: &[&str], state: &AppState) -> Result<CommandResult, String>,
}
```

### Command Registry

Static array of `CommandDef`. Adding a new command = adding one entry + one handler function. Tab completion iterates the registry matching on name/aliases.

### Argument Parsing

Simple positional args. Commands split input by whitespace, first token is the command name, rest are args. No named/flag args needed for a REPL.

```
<command> [subcommand] [args...]
show instruments
show instrument 3
set bpm 140
add saw
mute 3
```

### Tab Completion

Two levels:
1. **Command-level:** Complete command names from registry
2. **Arg-level:** Context-sensitive — `add <TAB>` shows source types, `set key <TAB>` shows keys, `show <TAB>` shows subcommands

Use `rustyline` crate for readline with custom `Completer` impl.

## MVP Commands (~20)

### Query Commands
| Command | Description | Output |
|---------|-------------|--------|
| `show instruments` / `list` | List all instruments | ID, name, source, level, pan, mute/solo status |
| `show instrument <id>` | Detail one instrument | All params: source, filter, effects, envelope, LFO, routing |
| `show transport` | Playback state | Playing/stopped, BPM, key, scale, time sig, playhead position |
| `show mixer` | Mixer overview | All channels: instruments + buses + master with levels |
| `show notes <instrument_id>` | Notes in piano roll | List of notes with pitch, tick, duration, velocity |
| `show effects <instrument_id>` | Effect chain | Effect type, params, enabled status |
| `show buses` | Bus routing | Bus names, levels, sends |
| `help` / `help <command>` | Help text | Command list or specific command usage |

### Action Commands
| Command | Action | Maps To |
|---------|--------|---------|
| `play` / `stop` | Toggle playback | `PianoRollAction::PlayStop` |
| `select <id>` | Select instrument | `InstrumentAction::Select(idx)` |
| `set bpm <n>` | Change BPM | `SessionAction::SetBpm(n)` |
| `set key <k>` | Change key | `SessionAction::SetKey(k)` |
| `set scale <s>` | Change scale | `SessionAction::SetScale(s)` |
| `mute <id>` | Toggle mute | `InstrumentAction::ToggleMute(id)` |
| `solo <id>` | Toggle solo | `InstrumentAction::ToggleSolo(id)` |
| `add <source>` | Add instrument | `InstrumentAction::Add(source)` |
| `delete <id>` | Delete instrument | `InstrumentAction::Delete(id)` |
| `level <id> <val>` | Set instrument level | `MixerAction::SetLevel(...)` |
| `pan <id> <val>` | Set pan | `MixerAction::SetPan(...)` |
| `undo` / `redo` | Undo/redo | `Action::Undo` / `Action::Redo` |
| `save` / `save <path>` | Save project | `SessionAction::Save` / `SessionAction::SaveAs(path)` |
| `load <path>` | Load project | `SessionAction::LoadFrom(path)` |
| `quit` / `exit` | Exit REPL | Clean shutdown |

## Display Formatting (`repl/display.rs`)

Human-readable formatters for state types. Not `impl Display` (that would change behavior elsewhere) — standalone functions:

```rust
pub fn format_instrument_list(state: &AppState) -> String
pub fn format_instrument_detail(instrument: &Instrument, state: &AppState) -> String
pub fn format_transport(state: &AppState) -> String
pub fn format_mixer(state: &AppState) -> String
pub fn format_notes(track: &Track) -> String
pub fn format_effect_chain(instrument: &Instrument) -> String
```

Uses the existing `.name()` methods on SourceType, EffectType, FilterType, Key, Scale. Formats MIDI pitch as note name (e.g., 60 → "C4"). Formats ticks as bar.beat (e.g., 1920 → "2.1").

Example output:
```
Instruments:
  1. Kick Drum      [Kick]       vol:0.80  pan:C
  2. Bass           [Acid]       vol:0.60  pan:C    muted
 *3. Lead           [SuperSaw]   vol:0.70  pan:L20
  4. Pad            [Strings]    vol:0.50  pan:R10  solo
```

## REPL Loop (`repl/mod.rs`)

```rust
pub fn run_repl(project_path: Option<String>) -> std::io::Result<()> {
    // 1. Init logging, config, state (same as run())
    // 2. Init AudioHandle, LocalDispatcher
    // 3. Load project if specified
    // 4. Auto-start SuperCollider
    // 5. Create rustyline Editor with custom completer
    // 6. Print welcome banner

    loop {
        let line = editor.readline("imbolc> ")?;
        editor.add_history_entry(&line);

        let result = execute_command(&line, dispatcher.state());
        match result {
            Ok(CommandResult::Output(text)) => println!("{}", text),
            Ok(CommandResult::Dispatch(action)) => {
                let r = dispatcher.dispatch_with_audio(&action, &mut audio);
                // Apply result, print confirmation
            }
            Ok(CommandResult::OutputAndDispatch(text, action)) => { ... }
            Ok(CommandResult::Quit) => break,
            Err(msg) => eprintln!("Error: {}", msg),
        }

        // Drain audio feedback (non-blocking)
        drain_audio_feedback(&mut dispatcher, &mut audio);
    }
}
```

### Audio Feedback Draining

The REPL loop is blocking on readline. Audio feedback (playhead updates, server status) needs periodic draining. Options:
- Drain after each command (simplest, good enough for MVP)
- Background thread that drains and prints async notifications (future enhancement)

## Dependencies

Add to `imbolc-ui/Cargo.toml`:
```toml
rustyline = "14"  # Readline with history, completion, hints
```

No other new dependencies. No TTS yet.

## Implementation Order

1. **Scaffold** — Create `repl/` module, `--repl` flag, basic REPL loop with `rustyline`
2. **Display** — `display.rs` formatters for instruments, transport, mixer
3. **Parser** — `parse.rs` tokenizer, `commands.rs` registry with MVP commands
4. **Query commands** — `show instruments`, `show instrument`, `show transport`, `show mixer`
5. **Action commands** — `play/stop`, `select`, `set bpm/key/scale`, `mute/solo`, `add/delete`
6. **Tab completion** — Command names + context-sensitive arg completion
7. **Polish** — Error messages, `help` system, welcome banner, confirmation for destructive actions

## Future (Not in this PR)

- TUI pane version (ReplPane inside ratatui)
- TTS output via `tts` crate
- Note editing commands (`note add <pitch> <tick> <duration>`)
- Effect management (`effect add reverb 3`, `effect remove 3 1`)
- Automation commands
- MIDI CC mapping
- Script execution (`--repl-script <file>`)
- Async notifications (background thread for audio feedback)

## Verification

1. `cargo build -p imbolc-ui` — compiles with new module
2. `cargo test -p imbolc-ui --bin imbolc-ui` — unit tests for parser, display, command handlers
3. `cargo run -p imbolc-ui -- --repl` — launches REPL, no TUI
4. Manual testing: type commands, verify output, verify actions dispatch correctly
5. Screen reader testing: verify VoiceOver/NVDA reads output correctly (linear text, no cursor spam)

## Key Files to Modify

| File | Change |
|------|--------|
| `imbolc-ui/src/main.rs` | Add `--repl` flag, call `repl::run_repl()` |
| `imbolc-ui/src/repl/mod.rs` | **New:** REPL loop, initialization, audio feedback draining |
| `imbolc-ui/src/repl/commands.rs` | **New:** Command registry, all command handler functions |
| `imbolc-ui/src/repl/display.rs` | **New:** Human-readable state formatters |
| `imbolc-ui/src/repl/parse.rs` | **New:** Tokenizer, arg parser, completion engine |
| `imbolc-ui/Cargo.toml` | Add `rustyline` dependency |
