//! JI ratio tables per flavor and chord-quality ratio maps.

use crate::state::music::JIFlavor;

/// 12 ratios relative to tonic, indexed by semitone offset (0..12).
pub type RatioTable = [f64; 12];

/// Get the scale JI ratio table for a given flavor.
pub fn scale_ratios(flavor: JIFlavor) -> &'static RatioTable {
    match flavor {
        JIFlavor::FiveLimit => &FIVE_LIMIT,
        JIFlavor::SevenLimit => &SEVEN_LIMIT,
        JIFlavor::Pythagorean => &PYTHAGOREAN,
    }
}

// 5-Limit (classic JI)
const FIVE_LIMIT: RatioTable = [
    1.0,          // Unison      1/1
    16.0 / 15.0,  // Minor 2nd   16/15
    9.0 / 8.0,    // Major 2nd   9/8
    6.0 / 5.0,    // Minor 3rd   6/5
    5.0 / 4.0,    // Major 3rd   5/4
    4.0 / 3.0,    // Perfect 4th 4/3
    45.0 / 32.0,  // Tritone     45/32
    3.0 / 2.0,    // Perfect 5th 3/2
    8.0 / 5.0,    // Minor 6th   8/5
    5.0 / 3.0,    // Major 6th   5/3
    9.0 / 5.0,    // Minor 7th   9/5
    15.0 / 8.0,   // Major 7th   15/8
];

// 7-Limit (septimal)
const SEVEN_LIMIT: RatioTable = [
    1.0,          // Unison      1/1
    15.0 / 14.0,  // Minor 2nd   15/14
    9.0 / 8.0,    // Major 2nd   9/8
    7.0 / 6.0,    // Minor 3rd   7/6
    5.0 / 4.0,    // Major 3rd   5/4
    4.0 / 3.0,    // Perfect 4th 4/3
    7.0 / 5.0,    // Tritone     7/5
    3.0 / 2.0,    // Perfect 5th 3/2
    14.0 / 9.0,   // Minor 6th   14/9
    5.0 / 3.0,    // Major 6th   5/3
    7.0 / 4.0,    // Minor 7th   7/4
    15.0 / 8.0,   // Major 7th   15/8
];

// Pythagorean (pure fifths)
const PYTHAGOREAN: RatioTable = [
    1.0,            // Unison      1/1
    256.0 / 243.0,  // Minor 2nd   256/243
    9.0 / 8.0,      // Major 2nd   9/8
    32.0 / 27.0,    // Minor 3rd   32/27
    81.0 / 64.0,    // Major 3rd   81/64
    4.0 / 3.0,      // Perfect 4th 4/3
    729.0 / 512.0,  // Tritone     729/512
    3.0 / 2.0,      // Perfect 5th 3/2
    128.0 / 81.0,   // Minor 6th   128/81
    27.0 / 16.0,    // Major 6th   27/16
    16.0 / 9.0,     // Minor 7th   16/9
    243.0 / 128.0,  // Major 7th   243/128
];

/// Chord quality determines which intervals get JI ratios relative to chord root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    Dominant7,
    Major7,
    Minor7,
    Sus2,
    Sus4,
    Power,
    Unison,
    Unknown,
}

/// Get chord-tone JI ratios for a chord quality.
/// Returns pairs of (semitone_offset_from_root, ratio).
/// Non-chord tones should fall back to scale JI.
pub fn chord_ratios(quality: ChordQuality, flavor: JIFlavor) -> &'static [(u8, f64)] {
    use ChordQuality::*;
    match quality {
        Major => &CHORD_MAJOR,
        Minor => &CHORD_MINOR,
        Diminished => &CHORD_DIM,
        Augmented => &CHORD_AUG,
        Dominant7 => match flavor {
            JIFlavor::FiveLimit => &CHORD_DOM7_5L,
            JIFlavor::SevenLimit => &CHORD_DOM7_7L,
            JIFlavor::Pythagorean => &CHORD_DOM7_PYTH,
        },
        Major7 => &CHORD_MAJ7,
        Minor7 => match flavor {
            JIFlavor::FiveLimit => &CHORD_MIN7_5L,
            JIFlavor::SevenLimit => &CHORD_MIN7_7L,
            JIFlavor::Pythagorean => &CHORD_MIN7_PYTH,
        },
        Sus2 => &CHORD_SUS2,
        Sus4 => &CHORD_SUS4,
        Power => &CHORD_POWER,
        Unison => &CHORD_UNISON,
        Unknown => &CHORD_UNISON,
    }
}

// Chord-tone ratio tables: (semitone_offset_from_root, ratio)
const CHORD_MAJOR: [(u8, f64); 3] = [(0, 1.0), (4, 5.0 / 4.0), (7, 3.0 / 2.0)];
const CHORD_MINOR: [(u8, f64); 3] = [(0, 1.0), (3, 6.0 / 5.0), (7, 3.0 / 2.0)];
const CHORD_DIM: [(u8, f64); 3] = [(0, 1.0), (3, 6.0 / 5.0), (6, 45.0 / 32.0)];
const CHORD_AUG: [(u8, f64); 3] = [(0, 1.0), (4, 5.0 / 4.0), (8, 8.0 / 5.0)];

// Dom7 with flavor-dependent 7th
const CHORD_DOM7_5L: [(u8, f64); 4] = [(0, 1.0), (4, 5.0 / 4.0), (7, 3.0 / 2.0), (10, 9.0 / 5.0)];
const CHORD_DOM7_7L: [(u8, f64); 4] = [(0, 1.0), (4, 5.0 / 4.0), (7, 3.0 / 2.0), (10, 7.0 / 4.0)];
const CHORD_DOM7_PYTH: [(u8, f64); 4] = [(0, 1.0), (4, 5.0 / 4.0), (7, 3.0 / 2.0), (10, 16.0 / 9.0)];

const CHORD_MAJ7: [(u8, f64); 4] = [(0, 1.0), (4, 5.0 / 4.0), (7, 3.0 / 2.0), (11, 15.0 / 8.0)];

const CHORD_MIN7_5L: [(u8, f64); 4] = [(0, 1.0), (3, 6.0 / 5.0), (7, 3.0 / 2.0), (10, 9.0 / 5.0)];
const CHORD_MIN7_7L: [(u8, f64); 4] = [(0, 1.0), (3, 6.0 / 5.0), (7, 3.0 / 2.0), (10, 7.0 / 4.0)];
const CHORD_MIN7_PYTH: [(u8, f64); 4] = [(0, 1.0), (3, 6.0 / 5.0), (7, 3.0 / 2.0), (10, 16.0 / 9.0)];

const CHORD_SUS2: [(u8, f64); 3] = [(0, 1.0), (2, 9.0 / 8.0), (7, 3.0 / 2.0)];
const CHORD_SUS4: [(u8, f64); 3] = [(0, 1.0), (5, 4.0 / 3.0), (7, 3.0 / 2.0)];
const CHORD_POWER: [(u8, f64); 2] = [(0, 1.0), (7, 3.0 / 2.0)];
const CHORD_UNISON: [(u8, f64); 1] = [(0, 1.0)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ratio_tables_start_at_unison() {
        for flavor in JIFlavor::ALL {
            let table = scale_ratios(flavor);
            assert_eq!(table[0], 1.0, "{:?} table should start at 1/1", flavor);
        }
    }

    #[test]
    fn all_ratio_tables_have_pure_fifth() {
        for flavor in JIFlavor::ALL {
            let table = scale_ratios(flavor);
            assert!(
                (table[7] - 1.5).abs() < 1e-10,
                "{:?} table should have 3/2 perfect fifth",
                flavor
            );
        }
    }

    #[test]
    fn all_ratio_tables_have_pure_fourth() {
        for flavor in JIFlavor::ALL {
            let table = scale_ratios(flavor);
            let expected = 4.0 / 3.0;
            assert!(
                (table[5] - expected).abs() < 1e-10,
                "{:?} table should have 4/3 perfect fourth",
                flavor
            );
        }
    }

    #[test]
    fn all_ratios_monotonically_increasing() {
        for flavor in JIFlavor::ALL {
            let table = scale_ratios(flavor);
            for i in 1..12 {
                assert!(
                    table[i] > table[i - 1],
                    "{:?} table: ratio[{}]={} should be > ratio[{}]={}",
                    flavor,
                    i,
                    table[i],
                    i - 1,
                    table[i - 1]
                );
            }
        }
    }

    #[test]
    fn all_ratios_less_than_2() {
        for flavor in JIFlavor::ALL {
            let table = scale_ratios(flavor);
            for (i, &ratio) in table.iter().enumerate() {
                assert!(
                    ratio < 2.0,
                    "{:?} table: ratio[{}]={} should be < 2.0",
                    flavor,
                    i,
                    ratio
                );
            }
        }
    }

    #[test]
    fn five_limit_minor_seventh() {
        let table = scale_ratios(JIFlavor::FiveLimit);
        assert!((table[10] - 9.0 / 5.0).abs() < 1e-10);
    }

    #[test]
    fn seven_limit_minor_seventh() {
        let table = scale_ratios(JIFlavor::SevenLimit);
        assert!((table[10] - 7.0 / 4.0).abs() < 1e-10);
    }

    #[test]
    fn pythagorean_minor_seventh() {
        let table = scale_ratios(JIFlavor::Pythagorean);
        assert!((table[10] - 16.0 / 9.0).abs() < 1e-10);
    }

    #[test]
    fn major_chord_ratios() {
        let ratios = chord_ratios(ChordQuality::Major, JIFlavor::FiveLimit);
        assert_eq!(ratios.len(), 3);
        assert_eq!(ratios[0], (0, 1.0));
        assert_eq!(ratios[1], (4, 5.0 / 4.0));
        assert_eq!(ratios[2], (7, 3.0 / 2.0));
    }

    #[test]
    fn minor_chord_ratios() {
        let ratios = chord_ratios(ChordQuality::Minor, JIFlavor::FiveLimit);
        assert_eq!(ratios.len(), 3);
        assert_eq!(ratios[1], (3, 6.0 / 5.0));
    }

    #[test]
    fn dom7_seventh_varies_by_flavor() {
        let five = chord_ratios(ChordQuality::Dominant7, JIFlavor::FiveLimit);
        let seven = chord_ratios(ChordQuality::Dominant7, JIFlavor::SevenLimit);
        let pyth = chord_ratios(ChordQuality::Dominant7, JIFlavor::Pythagorean);

        assert!((five[3].1 - 9.0 / 5.0).abs() < 1e-10);
        assert!((seven[3].1 - 7.0 / 4.0).abs() < 1e-10);
        assert!((pyth[3].1 - 16.0 / 9.0).abs() < 1e-10);
    }
}
