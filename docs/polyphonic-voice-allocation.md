# Polyphonic Voice Allocation

**Status:** Implemented in `imbolc-audio/src/engine/voices.rs` and `imbolc-audio/src/engine/voice_allocator.rs`.

## Summary

Imbolc uses per-note voice spawning for oscillator and sampler instruments:

- Each note-on spawns a **voice chain** in `GROUP_SOURCES`: an `imbolc_midi` control node plus the source synth.
- Voices feed a static, per-instrument **processing chain** (filters/EQ/effects/output) built during routing.
- `VoiceAllocator` enforces max polyphony and chooses victims (released voices first, then quietest/oldest).
- Control buses (freq/gate/vel) are pooled and reclaimed on `/n_end` feedback from scsynth.
- `cleanup_expired()` remains as a safety net if `/n_end` is missed.

## VST Instruments

VST instruments are hosted as persistent nodes (`imbolc_vst_instrument`). Note-on/off is sent via `/u_cmd` MIDI messages rather than spawning SC voices, but they still participate in the same mixer/output routing.

