# VST3 Support Roadmap

Goal: move imbolc from "can load a VST" to "full support" with a usable TUI
workflow (parameter browsing, automation, state recall, and advanced features).

Note: docs/vst-integration.md is legacy notes from an earlier prototype. This
file reflects the current Rust codebase and the intended direction.

## Current support (today)

Implemented in the Rust codebase and SC engine:

- VSTPlugin wrapper SynthDefs exist: `synthdefs/compile_vst.scd` generates
  `imbolc_vst_instrument` and `imbolc_vst_effect` (requires the VSTPlugin extension).
  These wrappers are not compiled by default; run `sclang synthdefs/compile_vst.scd`
  and load synthdefs from the Server pane.
- VST registry data model exists (`imbolc-core/src/state/vst_plugin.rs`) and is persisted in
  SQLite (`vst_plugins`, `vst_plugin_params`, `instrument_vst_params`).
- Add pane can import a `.vst` / `.vst3` bundle as a VST instrument; Add Effect can import VST effects.
  The file browser treats bundle directories as selectable.
- Audio engine opens a VST plugin in SuperCollider by sending a `/u_cmd` to the
  VSTPlugin UGen when routing is rebuilt.
- VST instruments receive note-on/off via `/u_cmd` MIDI messages from the
  sequencer.
- VST parameter pane exists (search/list/adjust/reset, add automation lane).
- Automation target for VST parameters is wired in the audio engine (`/set`).
- `VstTarget` enum (`Source | Effect(usize)`) unifies all VST operations
  across source instruments and effect slots.
- VST effect parameter editing: VstParamPane supports both source and effect
  targets. From InstrumentEditPane, pressing 'v' on a VST effect row opens the
  param pane for that effect.
- Parameter discovery: `QueryVstParams` generates synthetic 128-param placeholder
  specs via `AudioFeedback::VstParamsDiscovered` (actual SC reply handling is
  still TODO). Values are stored per-instance in `instrument.vst_param_values`
  (source) or `effect.vst_param_values` (effect slot).
- VST state persistence: state file paths are saved per-instrument and per-effect
  in the SQLite database. On project load, `LoadVstState` commands are queued
  after routing rebuild so saved state is restored.
- Saved VST param values are re-applied after routing rebuild via `/set` commands
  for both source and effect nodes.
- Effect slots carry `vst_param_values: Vec<(u32, f32)>` and
  `vst_state_path: Option<PathBuf>`, persisted in `effect_vst_params` table and
  `instrument_effects.vst_state_path` column (schema v6).

What is missing or stubbed:

- No plugin scanning or cataloging; only manual import by file path.
- Parameter discovery uses synthetic placeholder params (128 x "Param N" @ 0.5 default);
  actual SC OSC reply handling for real param names/defaults is TODO.
- No preset/program handling.
- No param groups, MIDI learn, or latency reporting/compensation.

## Parameter browser (current UI)

Because we cannot open native plugin GUIs in a TUI, a generic parameter browser
is the core user-facing surface for VST3 support.

Implemented in `src/panes/vst_param_pane`:

- Searchable list of parameters with value bar display.
- Adjust (fine/coarse), reset, and "add automation lane" actions.
- Supports both VST source instruments and VST effect slots via `VstTarget`.
- `set_target(instrument_id, target)` configures the pane before navigation.
- Title shows "VST Params" for sources, "VST Effect Params" for effects.
- Discovery ('d') populates placeholder params via the audio thread.

Gaps:

- Real parameter names/units from VSTPlugin OSC replies.
- Range/unit display and richer widgets.
- Favorites/compact views.

Example layout (sketch):

```
+-- VST Params: Serum ---------------------------------------------+
| / cutoff  reso  env  lfo  filter  source  fx                    |
|                                                                  |
| > 001 Cutoff            0.72 [Hz]                               |
|   002 Resonance         0.30 [%]                                |
|   003 Env Amount        0.55                                    |
|                                                                  |
| Range: 20..20000 Hz  Default: 2000  Automation: off             |
| [Left/Right] adjust  [a] add lane  [f] favorite  [r] reset       |
+------------------------------------------------------------------+
```

## Target: "full support"

"Full support" means a VST3 can be loaded, controlled, automated, saved,
reloaded, and used reliably inside a session with no external GUI.

That implies:

- Complete parameter enumeration and editing.
- Automation recording and playback for VST parameters.
- Plugin state persistence and restore.
- Preset/program management.
- Param grouping and MIDI learn for faster control.
- Latency reporting (and compensation where possible).

## Plan A: Param list + automation + state (DONE)

This phase is implemented. VSTs are usable in projects.

1) Parameter discovery — **done** (synthetic placeholders; real SC reply TODO)
   - `QueryVstParams` sends `/param_count` and generates placeholder specs.
   - Specs stored in `VstPlugin` registry; per-instance values in instrument
     and effect slot state.

2) Parameter browser UI — **done**
   - Searchable list with value bars, fine/coarse adjust, reset.
   - Works for both VST source instruments and VST effect slots.
   - Wire edits to `/set` via `AudioCmd::SetVstParam`.

3) Parameter automation — **done**
   - Automation target: `VstParam(instrument_id, param_index)`.
   - Playback applies via `/set` during tick.

4) Plugin state save/restore — **done**
   - State file saved via `/program_write`, loaded via `/program_read`.
   - Paths persisted in SQLite (`instruments.vst_state_path`,
     `instrument_effects.vst_state_path`).
   - Auto-restored on project load (queued after routing rebuild).

## Plan B: Presets + param groups + MIDI learn + latency

This phase makes VSTs feel first-class and fast to use.

1) Presets / programs
   - Load and save preset files.
   - List plugin programs and allow quick switching.

2) Param groups
   - Surface VST3 units/groups in the UI.
   - Allow browsing by group (source/filter/mod/etc.).

3) MIDI learn
   - Map external MIDI CCs to VST parameters.
   - Store mappings per instance or per plugin.

4) Latency
   - Query plugin latency and expose it in UI.
   - If possible, compensate in playback (or at least report).

## Notes

- The SC VSTPlugin path is the current integration strategy; the UI/automation
  design should remain agnostic so a future native host can reuse it.
- Keep all VST metadata and per-instance state in the session DB so projects are
  portable and reload correctly.
