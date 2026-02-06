use serde::{Serialize, Deserialize};

/// Arpeggiator configuration, stored per-instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArpeggiatorConfig {
    pub enabled: bool,
    pub direction: ArpDirection,
    pub rate: ArpRate,
    pub octaves: u8,     // 1-4
    pub gate: f32,       // 0.1-1.0 (note length as fraction of step)
}

impl Default for ArpeggiatorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            direction: ArpDirection::Up,
            rate: ArpRate::Eighth,
            octaves: 1,
            gate: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpDirection {
    Up,
    Down,
    UpDown,
    Random,
}

impl ArpDirection {
    pub fn name(&self) -> &'static str {
        match self {
            ArpDirection::Up => "Up",
            ArpDirection::Down => "Down",
            ArpDirection::UpDown => "Up/Down",
            ArpDirection::Random => "Random",
        }
    }

    pub fn next(&self) -> ArpDirection {
        match self {
            ArpDirection::Up => ArpDirection::Down,
            ArpDirection::Down => ArpDirection::UpDown,
            ArpDirection::UpDown => ArpDirection::Random,
            ArpDirection::Random => ArpDirection::Up,
        }
    }

    pub fn prev(&self) -> ArpDirection {
        match self {
            ArpDirection::Up => ArpDirection::Random,
            ArpDirection::Down => ArpDirection::Up,
            ArpDirection::UpDown => ArpDirection::Down,
            ArpDirection::Random => ArpDirection::UpDown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpRate {
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl ArpRate {
    pub fn name(&self) -> &'static str {
        match self {
            ArpRate::Quarter => "1/4",
            ArpRate::Eighth => "1/8",
            ArpRate::Sixteenth => "1/16",
            ArpRate::ThirtySecond => "1/32",
        }
    }

    /// Steps per beat (quarter note)
    pub fn steps_per_beat(&self) -> f32 {
        match self {
            ArpRate::Quarter => 1.0,
            ArpRate::Eighth => 2.0,
            ArpRate::Sixteenth => 4.0,
            ArpRate::ThirtySecond => 8.0,
        }
    }

    pub fn next(&self) -> ArpRate {
        match self {
            ArpRate::Quarter => ArpRate::Eighth,
            ArpRate::Eighth => ArpRate::Sixteenth,
            ArpRate::Sixteenth => ArpRate::ThirtySecond,
            ArpRate::ThirtySecond => ArpRate::Quarter,
        }
    }

    pub fn prev(&self) -> ArpRate {
        match self {
            ArpRate::Quarter => ArpRate::ThirtySecond,
            ArpRate::Eighth => ArpRate::Quarter,
            ArpRate::Sixteenth => ArpRate::Eighth,
            ArpRate::ThirtySecond => ArpRate::Sixteenth,
        }
    }
}

/// Chord shape definitions — interval offsets from root in semitones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChordShape {
    Major,
    Minor,
    Seventh,
    MinorSeventh,
    Sus2,
    Sus4,
    PowerChord,
    Octave,
}

impl ChordShape {
    /// Returns semitone offsets including the root (0).
    pub fn intervals(&self) -> &'static [i8] {
        match self {
            ChordShape::Major => &[0, 4, 7],
            ChordShape::Minor => &[0, 3, 7],
            ChordShape::Seventh => &[0, 4, 7, 10],
            ChordShape::MinorSeventh => &[0, 3, 7, 10],
            ChordShape::Sus2 => &[0, 2, 7],
            ChordShape::Sus4 => &[0, 5, 7],
            ChordShape::PowerChord => &[0, 7],
            ChordShape::Octave => &[0, 12],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ChordShape::Major => "Major",
            ChordShape::Minor => "Minor",
            ChordShape::Seventh => "7th",
            ChordShape::MinorSeventh => "m7",
            ChordShape::Sus2 => "sus2",
            ChordShape::Sus4 => "sus4",
            ChordShape::PowerChord => "Power",
            ChordShape::Octave => "Octave",
        }
    }

    pub fn next(&self) -> ChordShape {
        match self {
            ChordShape::Major => ChordShape::Minor,
            ChordShape::Minor => ChordShape::Seventh,
            ChordShape::Seventh => ChordShape::MinorSeventh,
            ChordShape::MinorSeventh => ChordShape::Sus2,
            ChordShape::Sus2 => ChordShape::Sus4,
            ChordShape::Sus4 => ChordShape::PowerChord,
            ChordShape::PowerChord => ChordShape::Octave,
            ChordShape::Octave => ChordShape::Major,
        }
    }

    pub fn prev(&self) -> ChordShape {
        match self {
            ChordShape::Major => ChordShape::Octave,
            ChordShape::Minor => ChordShape::Major,
            ChordShape::Seventh => ChordShape::Minor,
            ChordShape::MinorSeventh => ChordShape::Seventh,
            ChordShape::Sus2 => ChordShape::MinorSeventh,
            ChordShape::Sus4 => ChordShape::Sus2,
            ChordShape::PowerChord => ChordShape::Sus4,
            ChordShape::Octave => ChordShape::PowerChord,
        }
    }

    /// Expand a single MIDI pitch into chord pitches, clamped to valid MIDI range.
    pub fn expand(&self, root: u8) -> Vec<u8> {
        self.intervals()
            .iter()
            .filter_map(|&offset| {
                let pitch = root as i16 + offset as i16;
                if (0..=127).contains(&pitch) {
                    Some(pitch as u8)
                } else {
                    None
                }
            })
            .collect()
    }
}

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
