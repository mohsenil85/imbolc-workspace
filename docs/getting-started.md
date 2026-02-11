# Getting Started (First 15 Minutes)

_Last verified: 2026-02-11_

This walkthrough creates a simple loop, applies a basic mix, saves the
project, and exports audio.

## 1. Launch

From the repo root:

```bash
cargo run -p imbolc-ui --release
```

If you only want UI flow while skipping audio setup:

```bash
IMBOLC_NO_AUDIO=1 cargo run -p imbolc-ui
```

## 2. Bring Up Audio Server

1. Open Server pane: `F5`
2. Start server: `s`
3. Connect: `c`
4. If needed, compile/load synthdefs from the same pane:
   - Build synthdefs: `b`
   - Load synthdefs: `l`

## 3. Add an Instrument

1. Open Instruments pane: `F1`
2. Add instrument: `a` (or global `Ctrl+n`)
3. Choose an instrument with arrow keys and confirm with `Enter`
4. Select instrument slots with `1`..`0`

## 4. Enter Notes in Piano Roll

1. Open Piano Roll: `F2`
2. Move cursor with arrow keys
3. Toggle a note at cursor: `Enter`
4. Change note velocity: `+` / `-`
5. Set loop points:
   - Loop start: `[`
   - Loop end: `]`
   - Toggle loop: `l`

## 5. Playback

- Play/stop transport: `Space`
- While playing, edit notes and hear changes immediately.

## 6. Optional: Add Drum Pattern

1. In `F2`, cycle to the sequencer view (pane toggle in that tab)
2. Toggle steps with `Enter`
3. Change step grid resolution: `g`
4. Switch pattern pages: `[` and `]`

## 7. Mix Basics

1. Open Mixer: `F4`
2. Adjust levels/pan for balance
3. Use mute/solo controls to isolate issues while balancing

## 8. Save and Reload

- Save project: `Ctrl+s`
- Load project: `Ctrl+l`

Default project path:

- `~/.config/imbolc/default.sqlite`

## 9. Export Audio

In Piano Roll (`F2`):

- Render selected instrument: `R`
- Bounce master: `B`
- Export stems: `Ctrl+b`

Default output paths:

- Renders: `~/.config/imbolc/renders/`
- Exports: `~/.config/imbolc/exports/`

## 10. Useful Keys While Learning

- Context help: `?`
- Command palette: `:`
- Pane switcher: `;`
- Undo/redo: `Ctrl+z` / `Ctrl+Z`

## Common First-Run Issues

- No sound: verify Server pane is started/connected.
- Missing synthdefs: run `imbolc-core/bin/compile-synthdefs`, then load
  synthdefs in Server pane.
- Strange key behavior: use Kitty or Ghostty terminal.
