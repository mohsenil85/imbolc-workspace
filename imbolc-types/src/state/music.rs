use serde::{Serialize, Deserialize};

/// Musical key (pitch class)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B,
}

impl Key {
    pub const ALL: [Key; 12] = [
        Key::C, Key::Cs, Key::D, Key::Ds, Key::E, Key::F,
        Key::Fs, Key::G, Key::Gs, Key::A, Key::As, Key::B,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Key::C => "C", Key::Cs => "C#", Key::D => "D", Key::Ds => "D#",
            Key::E => "E", Key::F => "F", Key::Fs => "F#", Key::G => "G",
            Key::Gs => "G#", Key::A => "A", Key::As => "A#", Key::B => "B",
        }
    }

    /// MIDI note number for this key in octave 0
    pub fn semitone(&self) -> i32 {
        match self {
            Key::C => 0, Key::Cs => 1, Key::D => 2, Key::Ds => 3,
            Key::E => 4, Key::F => 5, Key::Fs => 6, Key::G => 7,
            Key::Gs => 8, Key::A => 9, Key::As => 10, Key::B => 11,
        }
    }
}

/// Scale definition as intervals from root
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scale {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
    Pentatonic,
    Blues,
    Chromatic,
}

impl Scale {
    pub const ALL: [Scale; 11] = [
        Scale::Major, Scale::Minor, Scale::Dorian, Scale::Phrygian,
        Scale::Lydian, Scale::Mixolydian, Scale::Aeolian, Scale::Locrian,
        Scale::Pentatonic, Scale::Blues, Scale::Chromatic,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Scale::Major => "Major",
            Scale::Minor => "Minor",
            Scale::Dorian => "Dorian",
            Scale::Phrygian => "Phrygian",
            Scale::Lydian => "Lydian",
            Scale::Mixolydian => "Mixolydian",
            Scale::Aeolian => "Aeolian",
            Scale::Locrian => "Locrian",
            Scale::Pentatonic => "Pentatonic",
            Scale::Blues => "Blues",
            Scale::Chromatic => "Chromatic",
        }
    }

    /// Semitone intervals from root for this scale
    pub fn intervals(&self) -> &'static [i32] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::Aeolian => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            Scale::Pentatonic => &[0, 2, 4, 7, 9],
            Scale::Blues => &[0, 3, 5, 6, 7, 10],
            Scale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn key_all_has_12() {
        assert_eq!(Key::ALL.len(), 12);
    }

    #[test]
    fn key_names_unique() {
        let names: HashSet<&str> = Key::ALL.iter().map(|k| k.name()).collect();
        assert_eq!(names.len(), 12);
    }

    #[test]
    fn key_semitones_0_to_11() {
        let semitones: Vec<i32> = Key::ALL.iter().map(|k| k.semitone()).collect();
        assert_eq!(semitones, (0..12).collect::<Vec<i32>>());
    }

    #[test]
    fn key_c_semitone_is_zero() {
        assert_eq!(Key::C.semitone(), 0);
    }

    #[test]
    fn scale_all_has_11() {
        assert_eq!(Scale::ALL.len(), 11);
    }

    #[test]
    fn scale_names_unique() {
        let names: HashSet<&str> = Scale::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(names.len(), 11);
    }

    #[test]
    fn scale_major_intervals() {
        assert_eq!(Scale::Major.intervals(), &[0, 2, 4, 5, 7, 9, 11]);
    }

    #[test]
    fn scale_minor_intervals() {
        assert_eq!(Scale::Minor.intervals(), &[0, 2, 3, 5, 7, 8, 10]);
    }

    #[test]
    fn scale_chromatic_has_12_notes() {
        assert_eq!(Scale::Chromatic.intervals().len(), 12);
    }

    #[test]
    fn scale_pentatonic_has_5_notes() {
        assert_eq!(Scale::Pentatonic.intervals().len(), 5);
    }
}
