# Plan: New Instrument Source Types

Add 14 new `SourceType` variants inspired by historical synthesizers, covering noise, classic analog, FM, physical modeling, and experimental synthesis.

## New Source Types

### Phase 1 — Direct template followers (simplest)

**1. Noise** — White/Pink/Brown noise + Crackle/Dust
- SC UGens: `WhiteNoise`, `PinkNoise`, `BrownNoise`, `Crackle`, `Dust`
- Params: `amp` (0–1), `color` (Int 0–4: white/pink/brown/crackle/dust), `density` (1–100, for Dust only)
- Synthdef: `imbolc_noise` — `Select.ar(color, [WhiteNoise.ar, PinkNoise.ar, BrownNoise.ar, Crackle.ar(1.5), Dust.ar(density)])` × amp × vel × env
- Note: freq_in still wired but unused by most noise types (Dust could optionally use it)

**2. Pulse** — Variable pulse width oscillator
- SC UGen: `Pulse.ar(freq, width)`
- Params: `freq` (20–20000), `amp` (0–1), `width` (0.01–0.99, default 0.5)
- Synthdef: `imbolc_pulse` — same template as `imbolc_sqr` but with `width` param exposed
- This is the PWM-capable version of Sqr (Juno, Prophet)

### Phase 2 — Multi-oscillator sources

**3. SuperSaw** — Detuned saw stack
- SC: 7× `Saw.ar` at slightly detuned frequencies, mixed with `Splay`
- Params: `freq`, `amp`, `detune` (0–1, default 0.3), `mix` (0–1, center vs detuned balance)
- Synthdef: `imbolc_supersaw` — center saw + 6 detuned saws spread stereo

**4. Sync** — Hard-sync oscillator
- SC: `SyncSaw.ar(syncFreq, sawFreq)` or manual approach with `Saw` + sync trigger
- Params: `freq`, `amp`, `sync_ratio` (1.0–8.0, ratio of slave to master freq)
- Synthdef: `imbolc_sync` — master at `freq`, slave at `freq * sync_ratio`

**5. Ring** — Ring modulator
- SC: `SinOsc.ar(freq) * SinOsc.ar(freq * mod_ratio)`
- Params: `freq`, `amp`, `mod_ratio` (0.1–16.0, default 2.0), `mod_depth` (0–1)
- Synthdef: `imbolc_ring` — carrier × modulator with depth crossfade

**6. FBSin** — Self-modulating feedback sine
- SC: `SinOscFB.ar(freq, feedback)`
- Params: `freq`, `amp`, `feedback` (0–3.0, default 0.0, 0=pure sine, >1=chaotic)
- Synthdef: `imbolc_fbsin`

### Phase 3 — Different synthesis paradigms

**7. FM** — 2-operator FM synthesis
- SC: `SinOsc.ar(freq + (SinOsc.ar(freq * ratio) * index * freq))`
- Params: `freq`, `amp`, `ratio` (0.25–16.0, mod:carrier ratio), `index` (0–20.0, modulation depth)
- Synthdef: `imbolc_fm` — classic DX7-style 2-op FM
- The `ratio` param controls harmonic relationship, `index` controls brightness

**8. PhaseMod** — Phase distortion synthesis (Casio CZ style)
- SC: `SinOsc.ar(freq, SinOsc.ar(freq * ratio) * index)`
- Params: `freq`, `amp`, `ratio` (0.25–16.0), `index` (0–10.0)
- Synthdef: `imbolc_phasemod` — phase modulation (mathematically similar to FM but sounds different at edges)

**9. Pluck** — Karplus-Strong string synthesis
- SC: `Pluck.ar(WhiteNoise.ar, 1, freq.reciprocal, freq.reciprocal, decaytime, coef)`
- Params: `freq`, `amp`, `decay` (0.1–10.0, string ring time), `coef` (0–1, tone brightness, low=bright high=dark)
- Synthdef: `imbolc_pluck` — uses `Pluck` UGen, no sustain (naturally decaying, but still gated for note-off)

**10. Formant** — Vocal formant synthesis
- SC: `Formant.ar(fundfreq, formfreq, bwfreq)`
- Params: `freq`, `amp`, `formant` (100–5000, formant center frequency), `bw` (10–1000, bandwidth)
- Synthdef: `imbolc_formant` — vowel-like timbres, formant freq controls vowel character

### Phase 4 — Experimental / complex

**11. Gendy** — Xenakis dynamic stochastic synthesis
- SC: `Gendy1.ar(ampdist, durdist, adparam, ddparam, minfreq, maxfreq)`
- Params: `amp`, `ampdist` (Int 0–6, amplitude distribution), `durdist` (Int 0–6, duration distribution), `minfreq` (20–1000), `maxfreq` (100–10000)
- Synthdef: `imbolc_gendy` — freq_in unused (Gendy generates its own pitch range)

**12. Chaos** — Chaotic attractor oscillators
- SC: `HenonN`/`LorenzL` selected by param
- Params: `amp`, `model` (Int 0=henon, 1=lorenz), `chaos_freq` (20–20000, sample rate), `chaos_param` (0–2.0, attractor parameter)
- Synthdef: `imbolc_chaos` — `Select.ar(model, [HenonN, LorenzL])` scaled to audio range

**13. Additive** — Harmonic series synthesis
- SC: Sum of N `SinOsc.ar` at harmonic multiples with rolloff
- Params: `freq`, `amp`, `harmonics` (Int 1–32, number of harmonics), `rolloff` (0–2.0, amplitude decay per harmonic, 0=organ-like, 1=natural, 2=steep)
- Synthdef: `imbolc_additive` — builds harmonic series dynamically with `Mix.fill`

**14. Wavetable** — Wavetable scanning oscillator
- SC: `VOsc.ar(bufpos, freq)` reading from a set of wavetable buffers
- Params: `freq`, `amp`, `position` (0–1, morph between waveforms)
- Synthdef: `imbolc_wavetable`
- **Extra infrastructure**: Needs a small set of compiled wavetable buffers (basic shapes: sine→saw→square→complex). Loaded at boot alongside synthdefs. Start with 8 precomputed tables embedded as buffer data.

## Deferred Ideas (not separate source types)

- **SubOsc**: Better as a param added to all oscillator synthdefs (`sub_level`, `sub_oct`). Would require modifying existing synthdef templates — separate effort.
- **Unison**: Better as a voice-layer feature (spawn N slightly-detuned voices per note). Requires changes to voice allocation, not source type — separate effort.

## Files to Modify Per Source Type

Every new source type touches the same set of locations. This is the checklist per variant:

### `imbolc-core/src/state/instrument.rs` (~10 match arms)
1. Add variant to `enum SourceType`
2. `name()` — display name (e.g., "Noise")
3. `short_name()` — persistence key (e.g., "noise")
4. `synth_def_name()` — SC synthdef (e.g., "imbolc_noise")
5. `default_params()` — parameter definitions
6. `all()` — add to built-in list
7. `display_name()`, `short_name_with_registry()`, `display_name_vst()`, `short_name_vst()` — fall through to existing `_ =>` or `self.name()` paths (no change needed, handled by existing wildcards)
8. `synth_def_name_with_registry()` — falls through to `self.synth_def_name()` (no change needed)
9. Type predicates (`is_audio_input`, `is_sample`, etc.) — no changes needed, new types are standard polyphonic oscillators

### `imbolc-core/src/state/persistence/conversion.rs`
10. `parse_source_type()` — add deserialization arm (e.g., `"noise" => SourceType::Noise`)

### `synthdefs/compile.scd`
11. Add SynthDef definition following the oscillator template

### `src/panes/instrument_pane.rs` + `src/panes/track_pane.rs`
12. `source_color()` — add color mapping (use `Color::OSC_COLOR` for all new oscillator types, matching existing Saw/Sin/Sqr/Tri pattern)

### No changes needed in:
- `voices.rs` — new types are standard polyphonic, fall through default path
- `routing.rs` — no special persistent-synth handling needed
- `dispatch/server.rs` — no audio-input activation logic needed
- `add_pane.rs` — uses `SourceType::all()`, picks up new types automatically
- `instrument_edit_pane/` — standard param display works via `source_params` vector
- `persistence/save.rs` — uses `short_name()` which handles new types via the new match arm

## Implementation Order

Work through phases 1→4. Within each phase, for each source type:

1. Write the SynthDef in `compile.scd`
2. Add the `SourceType` variant and all match arms in `instrument.rs`
3. Add deserialization in `conversion.rs`
4. Add color in `instrument_pane.rs` and `track_pane.rs`
5. Compile synthdefs (`sclang synthdefs/compile.scd`)
6. `cargo build` to verify
7. Test: create instrument with new type, play notes, verify sound

## Verification

1. `cargo build` — compiles with no errors after all variants added
2. `cargo test --bin imbolc` — existing tests pass
3. Manual test per source type:
   - Launch app, go to add instrument pane
   - Verify all new types appear in the list
   - Create instrument for each new type
   - Play notes via piano keyboard — verify sound output
   - Adjust source params — verify real-time parameter changes
   - Save and reload project — verify persistence round-trip
4. Compile synthdefs: run `sclang synthdefs/compile.scd` and verify all `.scsyndef` files are generated
