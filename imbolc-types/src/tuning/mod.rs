//! Tuning system: pitch-to-frequency conversion for multiple temperaments.
//!
//! All tuning math lives here — pure functions, no audio dependencies.

pub mod adaptive;
pub mod chord_detect;
pub mod ratios;

use crate::state::music::{JIFlavor, Key, Tuning};
pub use chord_detect::detect_chord;
pub use ratios::ChordQuality;

/// Context needed for JI tuning calculations.
/// Passed to `pitch_to_freq` for non-ET tuning systems.
pub struct TuningContext {
    /// Current musical key (tonic)
    pub key: Key,
    /// Which JI ratio philosophy to use
    pub ji_flavor: JIFlavor,
    /// Detected or annotated chord root (semitone 0-11), if any
    pub chord_root: Option<u8>,
    /// Detected or annotated chord quality, if any
    pub chord_quality: Option<ChordQuality>,
    /// Last assigned frequency per MIDI pitch (0.0 = unset). For adaptive continuity.
    pub previous_freqs: [f64; 128],
    /// Accumulated drift in cents (GlobalJI only)
    pub global_drift_cents: f64,
}

impl Default for TuningContext {
    fn default() -> Self {
        Self {
            key: Key::C,
            ji_flavor: JIFlavor::FiveLimit,
            chord_root: None,
            chord_quality: None,
            previous_freqs: [0.0; 128],
            global_drift_cents: 0.0,
        }
    }
}

impl TuningContext {
    pub fn new(key: Key, ji_flavor: JIFlavor) -> Self {
        Self {
            key,
            ji_flavor,
            ..Default::default()
        }
    }
}

/// Convert a MIDI pitch to frequency using the specified tuning system.
///
/// - `pitch`: MIDI note number (0-127)
/// - `tuning_a4`: reference frequency for A4 (MIDI 69), typically 440.0
/// - `tuning`: which tuning system to use
/// - `ctx`: context for JI calculations (key, flavor, chord, continuity state)
pub fn pitch_to_freq(pitch: u8, tuning_a4: f64, tuning: Tuning, ctx: &TuningContext) -> f64 {
    match tuning {
        Tuning::EqualTemperament => et_freq(pitch, tuning_a4),
        Tuning::ScaleJI => scale_ji_freq(pitch, tuning_a4, ctx),
        Tuning::ChordJI => chord_ji_freq(pitch, tuning_a4, ctx),
        Tuning::AdaptiveJI => {
            let ideal = chord_ji_freq(pitch, tuning_a4, ctx);
            let prev = ctx.previous_freqs[pitch as usize];
            adaptive::adaptive_snap(ideal, prev)
        }
        Tuning::GlobalJI => {
            // Same as ChordJI — drift tracking is done externally
            chord_ji_freq(pitch, tuning_a4, ctx)
        }
    }
}

/// Standard 12-TET formula
pub fn et_freq(pitch: u8, tuning_a4: f64) -> f64 {
    tuning_a4 * 2.0_f64.powf((pitch as f64 - 69.0) / 12.0)
}

/// Scale JI: ratio lookup relative to tonic
fn scale_ji_freq(pitch: u8, tuning_a4: f64, ctx: &TuningContext) -> f64 {
    let table = ratios::scale_ratios(ctx.ji_flavor);
    let key_semitone = ctx.key.semitone();

    // Semitone offset from tonic
    let offset = ((pitch as i32 - key_semitone) % 12 + 12) % 12;
    let ratio = table[offset as usize];

    // Compute tonic frequency in the correct octave
    let tonic_midi = key_semitone as f64; // key in octave -1
    let tonic_base_freq = tuning_a4 * 2.0_f64.powf((tonic_midi - 69.0) / 12.0);

    // Which octave is this pitch in relative to the tonic?
    let octave = ((pitch as i32 - key_semitone) as f64 / 12.0).floor();
    tonic_base_freq * 2.0_f64.powf(octave) * ratio
}

/// Chord JI: chord-tone ratios relative to chord root, fallback to scale JI
fn chord_ji_freq(pitch: u8, tuning_a4: f64, ctx: &TuningContext) -> f64 {
    if let (Some(chord_root), Some(chord_quality)) = (ctx.chord_root, ctx.chord_quality) {
        let pc = (pitch % 12) as u8;
        let offset = ((pc + 12 - chord_root) % 12) as u8;

        // Check if this pitch is a chord tone
        let chord_tones = ratios::chord_ratios(chord_quality, ctx.ji_flavor);
        for &(interval, ratio) in chord_tones {
            if interval == offset {
                // Compute chord root frequency using scale JI
                let root_freq = scale_ji_freq_for_pc(chord_root, pitch, tuning_a4, ctx);
                return root_freq * ratio;
            }
        }
    }

    // Fallback: scale JI for non-chord tones
    scale_ji_freq(pitch, tuning_a4, ctx)
}

/// Helper: compute frequency for a pitch class in the same octave as `reference_pitch`
fn scale_ji_freq_for_pc(pc: u8, reference_pitch: u8, tuning_a4: f64, ctx: &TuningContext) -> f64 {
    // Find the MIDI pitch nearest to reference_pitch that has pitch class `pc`
    let ref_octave = (reference_pitch as i32) / 12;
    let candidate = ref_octave * 12 + pc as i32;
    // Adjust octave so the chord root is at or below the reference pitch
    let midi_pitch = if candidate > reference_pitch as i32 {
        candidate - 12
    } else {
        candidate
    };
    let midi_pitch = midi_pitch.clamp(0, 127) as u8;
    scale_ji_freq(midi_pitch, tuning_a4, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: f64 = 440.0;

    #[test]
    fn et_a4_is_440() {
        let freq = pitch_to_freq(69, A4, Tuning::EqualTemperament, &TuningContext::default());
        assert!((freq - 440.0).abs() < 1e-10);
    }

    #[test]
    fn et_a5_is_880() {
        let freq = pitch_to_freq(81, A4, Tuning::EqualTemperament, &TuningContext::default());
        assert!((freq - 880.0).abs() < 1e-6);
    }

    #[test]
    fn et_middle_c() {
        let freq = pitch_to_freq(60, A4, Tuning::EqualTemperament, &TuningContext::default());
        let expected = 261.6256;
        assert!((freq - expected).abs() < 0.001);
    }

    #[test]
    fn scale_ji_tonic_matches_et() {
        // In key of C, C4 (midi 60) should be same as ET
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let ji = pitch_to_freq(60, A4, Tuning::ScaleJI, &ctx);
        let et = pitch_to_freq(60, A4, Tuning::EqualTemperament, &ctx);
        assert!(
            (ji - et).abs() < 0.001,
            "Tonic should match ET: ji={}, et={}",
            ji,
            et
        );
    }

    #[test]
    fn scale_ji_perfect_fifth() {
        // In key of C, G4 (midi 67) should be 3/2 of C4
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let c4 = pitch_to_freq(60, A4, Tuning::ScaleJI, &ctx);
        let g4 = pitch_to_freq(67, A4, Tuning::ScaleJI, &ctx);
        assert!(
            (g4 / c4 - 1.5).abs() < 1e-10,
            "G should be 3/2 of C: ratio={}",
            g4 / c4
        );
    }

    #[test]
    fn scale_ji_major_third_five_limit() {
        // In key of C, E4 (midi 64) should be 5/4 of C4
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let c4 = pitch_to_freq(60, A4, Tuning::ScaleJI, &ctx);
        let e4 = pitch_to_freq(64, A4, Tuning::ScaleJI, &ctx);
        assert!(
            (e4 / c4 - 5.0 / 4.0).abs() < 1e-10,
            "E should be 5/4 of C: ratio={}",
            e4 / c4
        );
    }

    #[test]
    fn scale_ji_octave_equivalence() {
        // C5 should be exactly 2x C4
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let c4 = pitch_to_freq(60, A4, Tuning::ScaleJI, &ctx);
        let c5 = pitch_to_freq(72, A4, Tuning::ScaleJI, &ctx);
        assert!(
            (c5 / c4 - 2.0).abs() < 1e-10,
            "C5 should be 2x C4: ratio={}",
            c5 / c4
        );
    }

    #[test]
    fn scale_ji_key_rotation() {
        // In key of D, the tonic D should equal ET D
        let ctx = TuningContext::new(Key::D, JIFlavor::FiveLimit);
        let d4_ji = pitch_to_freq(62, A4, Tuning::ScaleJI, &ctx);
        let d4_et = pitch_to_freq(62, A4, Tuning::EqualTemperament, &ctx);
        assert!(
            (d4_ji - d4_et).abs() < 0.001,
            "D tonic should match ET: ji={}, et={}",
            d4_ji,
            d4_et
        );
    }

    #[test]
    fn scale_ji_different_flavors_differ() {
        let ctx_5l = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let ctx_7l = TuningContext::new(Key::C, JIFlavor::SevenLimit);
        // Minor 3rd (Eb, midi 63) differs: 6/5 vs 7/6
        let eb_5l = pitch_to_freq(63, A4, Tuning::ScaleJI, &ctx_5l);
        let eb_7l = pitch_to_freq(63, A4, Tuning::ScaleJI, &ctx_7l);
        assert!(
            (eb_5l - eb_7l).abs() > 1.0,
            "Different flavors should produce different frequencies"
        );
    }

    #[test]
    fn chord_ji_chord_tone() {
        // C major chord: E should be 5/4 of C
        let mut ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        ctx.chord_root = Some(0); // C
        ctx.chord_quality = Some(ChordQuality::Major);

        let c4 = pitch_to_freq(60, A4, Tuning::ChordJI, &ctx);
        let e4 = pitch_to_freq(64, A4, Tuning::ChordJI, &ctx);
        assert!(
            (e4 / c4 - 5.0 / 4.0).abs() < 1e-6,
            "E should be 5/4 of C in chord JI: ratio={}",
            e4 / c4
        );
    }

    #[test]
    fn chord_ji_non_chord_tone_falls_back() {
        // C major chord, D (non-chord-tone) should fallback to scale JI
        let mut ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        ctx.chord_root = Some(0);
        ctx.chord_quality = Some(ChordQuality::Major);

        let d_chord = pitch_to_freq(62, A4, Tuning::ChordJI, &ctx);
        let d_scale = pitch_to_freq(62, A4, Tuning::ScaleJI, &ctx);
        assert!(
            (d_chord - d_scale).abs() < 1e-10,
            "Non-chord tone should fallback to scale JI"
        );
    }

    #[test]
    fn chord_ji_no_chord_equals_scale_ji() {
        // When no chord is detected, chord JI should equal scale JI
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let chord = pitch_to_freq(64, A4, Tuning::ChordJI, &ctx);
        let scale = pitch_to_freq(64, A4, Tuning::ScaleJI, &ctx);
        assert!(
            (chord - scale).abs() < 1e-10,
            "No chord context should fallback to scale JI"
        );
    }

    #[test]
    fn adaptive_ji_with_no_history() {
        // With no previous frequencies, adaptive should equal chord JI
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        let adaptive = pitch_to_freq(64, A4, Tuning::AdaptiveJI, &ctx);
        let chord = pitch_to_freq(64, A4, Tuning::ChordJI, &ctx);
        assert!(
            (adaptive - chord).abs() < 1e-10,
            "Adaptive with no history should equal chord JI"
        );
    }

    #[test]
    fn custom_tuning_a4() {
        // With A4=432Hz, A4 should be 432
        let freq = pitch_to_freq(69, 432.0, Tuning::EqualTemperament, &TuningContext::default());
        assert!((freq - 432.0).abs() < 1e-10);
    }

    #[test]
    fn all_pitches_produce_positive_freq() {
        let ctx = TuningContext::new(Key::C, JIFlavor::FiveLimit);
        for tuning in Tuning::ALL {
            for pitch in 0..=127u8 {
                let freq = pitch_to_freq(pitch, A4, tuning, &ctx);
                assert!(freq > 0.0, "{:?} pitch {} produced non-positive freq", tuning, pitch);
            }
        }
    }
}
