# Imbolc — Remaining Features for Completion

## Critical Missing Features

### 1. Undo/Redo DONE
Command pattern for reversible state mutations. Every note placement,
parameter change, effect chain edit, and mixer adjustment should be
undoable. Requires a history stack of state diffs or command objects,
with configurable depth. Table-stakes for any editor application.

### 2. Copy/Paste DONE
Clipboard system for notes in the piano roll, patterns in the drum
sequencer, automation points, and effect chains. Should support
cut/copy/paste with keyboard shortcuts. Enables efficient composition
without manual recreation of patterns.

### 3. Arrangement/Timeline View DONE
Complete the TrackPane into a proper song timeline with
sections/clips. Move beyond single-loop playback to support song
structure (intro, verse, chorus, bridge). Clip-based arrangement where
piano roll patterns become reusable clips placed on a timeline.

### 4. Audio Export/Bounce DONE
Offline render to WAV/AIFF using SuperCollider's NRT (Non-Realtime)
mode. Currently only real-time disk recording exists — there's no way
to export a mixdown without playing the entire song. NRT mode allows
rendering the OSC command score to disk faster than real time,
independent of audio hardware. Should support stem export
(per-instrument/bus bounces) in addition to master bounce. Requires
generating an OSC score file from the piano roll/automation/sequencer
state and invoking scsynth in NRT mode.

### 5. Project Management DONE
Full project lifecycle: save, open, load, rename, save-as, new
project. Currently persistence exists (SQLite) but the workflow around
managing multiple projects needs a proper UI — project browser, recent
projects list, dirty-state warnings on close, and rename capability.

### 6. Input/Automation Capture
Live recording of parameter changes and MIDI input as automation
data. When a user tweaks a knob or moves a fader during playback,
those movements should be captured as automation points on the
corresponding lane. MIDI CC input should record directly to automation
lanes. Arm/disarm per-lane recording.

## Significant Gaps

### 7. VST Parameter Discovery
Replace synthetic 128-parameter placeholders with real parameter
names, units, and ranges from the plugin via SuperCollider OSC
replies. Currently usable but clunky — users see "Param 0", "Param 1"
instead of meaningful names.

### 8. MIDI Learn
"Wiggle a knob to assign it" workflow. CC mapping state exists but
there's no interactive UI for binding a physical controller to a
parameter. Should support learn mode where the next incoming CC
automatically maps to the selected target.

### 9. Notification/Feedback System
A one-line status bar across the bottom of the screen that we can
print to programatically.

### 10. Test Coverage
~31 unit tests and a handful of e2e tests — low for a project this
size. The e2e harness (tmux-based) is a good foundation but covers
very little. Needs render snapshot tests, UI interaction tests,
multi-step workflow tests, and regression coverage for input handling.

### 11. Code Organization
`handle.rs` (1372 lines), `instrument.rs` (1070 lines) need
splitting. The async I/O architecture plan (phases 2-6) documents the
path forward: consolidate state mutation, extract audio thread
modules, incremental mixer diffs, split large files, and action string
validation at startup.

### 12. Configuration/Theming
No UI theming, no color customization, no layout presets. Musical
defaults are configurable but the visual experience is fixed. Should
support at least light/dark themes and user color overrides.

## Nice-to-Haves (Expected in DAWs)

### 13. Sidechain Visualization
Compressor gain reduction meters, sidechain input indicators in the
mixer.

### 14. Group/Bus Metering
Level meters for the 8 buses and master in the mixer view.

### 15. Plugin Scanning/Cataloging
Automatic VST3 directory scanning instead of manual file
import. Plugin database with search, favorites, and categories.

### 16. VST Preset/Program Browser
UI for browsing and loading VST presets and programs. Currently state
save/restore works but there's no preset management interface.

### 17. Latency Compensation
Plugin delay compensation (PDC) for VST instruments and
effects. Report and compensate for processing latency to keep tracks
aligned.

### 18. MIDI Clock Sync
Send and receive MIDI clock for synchronization with external hardware
and software. Tempo leader/follower modes.

### 19. CPU/DSP Load Meter
Real-time display of SuperCollider CPU usage and DSP load. Warning
indicators when approaching capacity.
