# Installation Guide

_Last verified: 2026-02-11_

This guide targets the public alpha baseline for Imbolc.

## Support Scope (Alpha)

- macOS: supported
- Linux: supported (best-tested on Debian/Ubuntu)
- Windows: not supported in alpha

## Common Prerequisites

1. Install Rust (edition 2021 toolchain):
   - [https://rustup.rs](https://rustup.rs)
2. Install SuperCollider so `scsynth` is on your `PATH`.
3. Use a terminal with Kitty keyboard protocol support (for the TUI):
   - [Kitty](https://sw.kovidgoyal.net/kitty/keyboard-protocol/)
   - [Ghostty](https://ghostty.org/)
4. Clone the repo and compile SynthDefs:

```bash
git clone https://github.com/mohsenil85/imbolc.git
cd imbolc
imbolc-core/bin/compile-synthdefs
```

5. Run the TUI:

```bash
cargo run -p imbolc-ui --release
```

## macOS Setup

1. Install SuperCollider via Homebrew:

```bash
brew install supercollider
```

2. Verify tools are available:

```bash
which scsynth
which sclang
```

3. Build and run:

```bash
imbolc-core/bin/compile-synthdefs
cargo run -p imbolc-ui --release
```

## Linux Setup (Debian/Ubuntu)

Install core dependencies:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  clang \
  libclang-dev \
  supercollider
```

Then build/run:

```bash
imbolc-core/bin/compile-synthdefs
cargo run -p imbolc-ui --release
```

Optional GUI dependencies (only if building `imbolc-gui`):

```bash
sudo apt-get install -y \
  libglib2.0-dev \
  libgtk-3-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev \
  libwebkit2gtk-4.1-dev \
  libxdo-dev
```

## Optional VST Setup

If you want VST support:

1. Install SuperCollider's VSTPlugin extension.
2. Compile VST wrapper SynthDefs:

```bash
imbolc-core/bin/compile-vst-synthdefs
```

3. In Imbolc Server pane, load synthdefs.

## Quick Validation Checklist

After launch, confirm:

- Server pane can start/connect `scsynth`
- You can add an instrument and play notes
- You can save/load a project
- You can export audio (`B` for bounce or `Ctrl+b` for stems in piano roll)

## Troubleshooting

### `scsynth: command not found`

- SuperCollider is not installed or not on your `PATH`.
- Reopen your terminal after installation.

### TUI keybindings behave incorrectly

- You are likely using an unsupported terminal.
- Switch to Kitty or Ghostty.

### Build fails with `libclang` / bindgen errors

- Install `clang` and `libclang-dev` (Linux).
- On macOS, ensure Xcode command line tools are installed:

```bash
xcode-select --install
```

### Startup errors about missing SynthDefs

- Re-run:

```bash
imbolc-core/bin/compile-synthdefs
```

- Confirm `imbolc-core/synthdefs/` contains `.scsyndef` files.

### GUI build fails on Linux

- Install the optional GUI dependencies listed above, or use the TUI
  (`imbolc-ui`) for alpha.
