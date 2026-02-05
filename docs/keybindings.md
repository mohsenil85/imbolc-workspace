# Keybindings Philosophy

imbolc favors a "normie" keybinding scheme: single keys for common actions,
mnemonics where possible, and context-sensitive meaning per pane. The goal is
fast, keyboard-first navigation without modifier chords.

## Source of truth

The canonical list of bindings lives in `keybindings.toml`. Each pane has a
`layer` section there, and the app surfaces context help with `?`. Treat this
document as a guide to intent and conventions, not an exhaustive list.

## Global keys (defaults)

| Key | Action |
|-----|--------|
| `Ctrl+q` | Quit |
| `Ctrl+s` | Save session |
| `Ctrl+S` | Save project as... |
| `Ctrl+l` | Load session |
| `Ctrl+o` | Open project browser |
| `Ctrl+z` | Undo |
| `Ctrl+Z` | Redo |
| `F1` | Instruments |
| `F2` | Piano roll / Sequencer / Waveform (context-driven) |
| `F3` | Track |
| `F4` | Mixer |
| `F5` | Server |
| `F7` | Automation |
| `F8` | Parametric EQ |
| `` ` `` / `~` | Back / Forward |
| `Ctrl+f` | Frame edit (session settings) |
| `Ctrl+m` | MIDI settings |
| `?` | Context help |
| `:` | Command Palette |
| `/` | Toggle piano keyboard |
| `.` | Toggle master mute |
| `Ctrl+r` | Toggle master recording |
| `1`-`9`, `0` | Select instrument 1-10 |
| `_` | Two-digit instrument select |
| `Ctrl+n` | Add instrument |
| `Ctrl+d` | Delete instrument |
| `Ctrl+c` | Copy |
| `Ctrl+x` | Cut |
| `Ctrl+v` | Paste |
| `Ctrl+a` | Select all |

## Pane-specific highlights (defaults)

These are representative examples; check `keybindings.toml` for the full list.

### Instrument pane
| Key | Action |
|-----|--------|
| `a` | Add instrument |
| `d` | Delete instrument |
| `Enter` | Edit instrument |
| `w` | Save instrument |
| `o` | Load instrument |

### Piano roll
| Key | Action |
|-----|--------|
| `Space` | Play/Stop |
| `l` | Toggle loop |
| `[` / `]` | Set loop start / end |
| `+` / `-` | Velocity up / down |
| `Alt+Left` / `Alt+Right` | Shrink / Grow note duration |
| `Shift+Arrows` | Extend selection |
| `A` | Toggle automation overlay |

### Track (Arrangement)
| Key | Action |
|-----|--------|
| `Space` | Play/Stop |
| `n` | Capture clip from piano roll |
| `p` | Place clip |
| `Enter` | Edit clip |
| `d` | Delete placement |
| `y` | Duplicate placement |

### Sequencer
| Key | Action |
|-----|--------|
| `Enter` | Toggle step |
| `Space` | Play/Stop |
| `s` | Load sample |
| `c` | Sample chopper |
| `Shift+Arrows` | Extend selection |

### Mixer
| Key | Action |
|-----|--------|
| `m` | Toggle mute |
| `s` | Toggle solo |
| `o` / `O` | Cycle output target (forward/back) |
| `a` | Add effect (detail view) |
| `e` | Toggle effect (detail view) |

### Automation
| Key | Action |
|-----|--------|
| `Enter` | Place/remove point |
| `a` | Add lane |
| `r` | Toggle recording |
| `R` | Arm/disarm lane |

### Server
| Key | Action |
|-----|--------|
| `s` | Start scsynth |
| `k` | Stop scsynth |
| `b` | Compile synthdefs |
| `l` | Load synthdefs |
| `R` | Toggle master recording |

## Text input mode

When a text input is focused, all keys type characters except:

| Key | Action |
|-----|--------|
| `Enter` | Confirm input |
| `Escape` | Cancel input |
| `Tab` | Next field |
| `Backspace` | Delete char before cursor |
| `Delete` | Delete char at cursor |
| `Left/Right` | Move cursor |
| `Home/End` | Start/end of input |

## Modifier rules

- Shift bindings are used only for special keys (e.g., `Shift+Left`).
- For shifted characters, bind the literal char (`?`, `A`, `+`) rather than a
  `Shift+` form.
