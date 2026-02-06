//! Arpeggiator types.
//!
//! Serializable config types are re-exported from imbolc-types.
//! Runtime state (ArpPlayState) stays here since it's not serialized.

pub use imbolc_types::{ArpDirection, ArpRate, ArpeggiatorConfig, ChordShape};

/// Arpeggiator play state — runtime state tracked on the audio thread.
#[derive(Debug, Clone)]
pub struct ArpPlayState {
    pub held_notes: Vec<u8>,       // Currently held MIDI pitches (sorted)
    pub step_index: usize,         // Current position in the note sequence
    pub accumulator: f64,          // Fractional step accumulator
    pub ascending: bool,           // For UpDown direction tracking
    pub current_pitch: Option<u8>, // Currently sounding pitch (for release)
}

impl Default for ArpPlayState {
    fn default() -> Self {
        Self {
            held_notes: Vec::new(),
            step_index: 0,
            accumulator: 0.0,
            ascending: true,
            current_pitch: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_direction_next_cycle() {
        let mut dir = ArpDirection::Up;
        for _ in 0..4 {
            dir = dir.next();
        }
        assert_eq!(dir, ArpDirection::Up);
    }

    #[test]
    fn arp_direction_prev_cycle() {
        let mut dir = ArpDirection::Up;
        for _ in 0..4 {
            dir = dir.prev();
        }
        assert_eq!(dir, ArpDirection::Up);
    }

    #[test]
    fn arp_rate_steps_per_beat() {
        assert!((ArpRate::Quarter.steps_per_beat() - 1.0).abs() < f32::EPSILON);
        assert!((ArpRate::Eighth.steps_per_beat() - 2.0).abs() < f32::EPSILON);
        assert!((ArpRate::Sixteenth.steps_per_beat() - 4.0).abs() < f32::EPSILON);
        assert!((ArpRate::ThirtySecond.steps_per_beat() - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn arp_rate_next_prev_cycle() {
        let mut rate = ArpRate::Quarter;
        for _ in 0..4 {
            rate = rate.next();
        }
        assert_eq!(rate, ArpRate::Quarter);

        for _ in 0..4 {
            rate = rate.prev();
        }
        assert_eq!(rate, ArpRate::Quarter);
    }

    #[test]
    fn chord_shape_intervals() {
        assert_eq!(ChordShape::Major.intervals(), &[0, 4, 7]);
        assert_eq!(ChordShape::Minor.intervals(), &[0, 3, 7]);
        assert_eq!(ChordShape::Seventh.intervals(), &[0, 4, 7, 10]);
        assert_eq!(ChordShape::PowerChord.intervals(), &[0, 7]);
        assert_eq!(ChordShape::Octave.intervals(), &[0, 12]);
    }

    #[test]
    fn chord_shape_expand() {
        let pitches = ChordShape::Major.expand(60);
        assert_eq!(pitches, vec![60, 64, 67]);
    }

    #[test]
    fn chord_shape_expand_boundary() {
        // Root 126 with Octave (+12) clips at 127
        let pitches = ChordShape::Octave.expand(126);
        // 126 is valid, 126+12=138 > 127 so filtered out
        assert_eq!(pitches, vec![126]);

        // Root 0 — all intervals are positive so no underflow
        let pitches = ChordShape::Major.expand(0);
        assert_eq!(pitches, vec![0, 4, 7]);

        // Root 127 — intervals would push past 127
        let pitches = ChordShape::Major.expand(127);
        assert_eq!(pitches, vec![127]); // 131 and 134 filtered out
    }

    #[test]
    fn chord_shape_next_prev_cycle() {
        let mut shape = ChordShape::Major;
        for _ in 0..8 {
            shape = shape.next();
        }
        assert_eq!(shape, ChordShape::Major);

        for _ in 0..8 {
            shape = shape.prev();
        }
        assert_eq!(shape, ChordShape::Major);
    }
}
