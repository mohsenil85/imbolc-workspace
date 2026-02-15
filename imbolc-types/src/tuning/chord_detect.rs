//! Chord detection from a set of sounding pitch classes.

use super::ratios::ChordQuality;

/// Detect chord from sounding MIDI pitches.
/// Returns (root_pitch_class_0_to_11, quality) or None if fewer than 2 unique pitch classes.
pub fn detect_chord(pitches: &[u8]) -> Option<(u8, ChordQuality)> {
    // Deduplicate pitch classes
    let mut pcs = [false; 12];
    for &p in pitches {
        pcs[(p % 12) as usize] = true;
    }
    let pc_set: Vec<u8> = (0..12).filter(|&i| pcs[i as usize]).collect();

    if pc_set.len() < 2 {
        return if pc_set.len() == 1 {
            Some((pc_set[0], ChordQuality::Unison))
        } else {
            None
        };
    }

    // Try each pitch class as root, score matches
    let mut best: Option<(u8, ChordQuality, u8)> = None; // (root, quality, score)

    for &root in &pc_set {
        let intervals: Vec<u8> = pc_set
            .iter()
            .map(|&pc| (pc + 12 - root) % 12)
            .collect();

        for &(quality, template, score) in &CHORD_TEMPLATES {
            if matches_template(&intervals, template) {
                let is_better = match best {
                    None => true,
                    Some((_, _, prev_score)) => score > prev_score,
                };
                if is_better {
                    best = Some((root, quality, score));
                }
            }
        }
    }

    // Fallback: use lowest bass note as root with Unknown quality
    best.map(|(root, quality, _)| (root, quality))
        .or_else(|| {
            let lowest = pitches.iter().min()?;
            Some((lowest % 12, ChordQuality::Unknown))
        })
}

fn matches_template(intervals: &[u8], template: &[u8]) -> bool {
    // All template intervals must be present in the interval set
    template.iter().all(|t| intervals.contains(t))
}

// (quality, interval_template, priority_score)
// Higher score = preferred when multiple match
const CHORD_TEMPLATES: [(ChordQuality, &[u8], u8); 11] = [
    // Triads (prefer simpler)
    (ChordQuality::Major, &[0, 4, 7], 10),
    (ChordQuality::Minor, &[0, 3, 7], 10),
    (ChordQuality::Power, &[0, 7], 5),
    (ChordQuality::Sus4, &[0, 5, 7], 8),
    (ChordQuality::Sus2, &[0, 2, 7], 8),
    (ChordQuality::Diminished, &[0, 3, 6], 7),
    (ChordQuality::Augmented, &[0, 4, 8], 7),
    // Sevenths
    (ChordQuality::Dominant7, &[0, 4, 7, 10], 12),
    (ChordQuality::Major7, &[0, 4, 7, 11], 12),
    (ChordQuality::Minor7, &[0, 3, 7, 10], 12),
    // Unison handled separately
    (ChordQuality::Unison, &[0], 1),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_c_major() {
        // C E G
        let result = detect_chord(&[60, 64, 67]);
        assert_eq!(result, Some((0, ChordQuality::Major)));
    }

    #[test]
    fn detect_c_minor() {
        // C Eb G
        let result = detect_chord(&[60, 63, 67]);
        assert_eq!(result, Some((0, ChordQuality::Minor)));
    }

    #[test]
    fn detect_g_major() {
        // G B D
        let result = detect_chord(&[67, 71, 74]);
        assert_eq!(result, Some((7, ChordQuality::Major)));
    }

    #[test]
    fn detect_d_minor() {
        // D F A
        let result = detect_chord(&[62, 65, 69]);
        assert_eq!(result, Some((2, ChordQuality::Minor)));
    }

    #[test]
    fn detect_c_dom7() {
        // C E G Bb
        let result = detect_chord(&[60, 64, 67, 70]);
        assert_eq!(result, Some((0, ChordQuality::Dominant7)));
    }

    #[test]
    fn detect_c_maj7() {
        // C E G B
        let result = detect_chord(&[60, 64, 67, 71]);
        assert_eq!(result, Some((0, ChordQuality::Major7)));
    }

    #[test]
    fn detect_a_minor7() {
        // A C E G
        let result = detect_chord(&[69, 72, 76, 79]);
        assert_eq!(result, Some((9, ChordQuality::Minor7)));
    }

    #[test]
    fn detect_power_chord() {
        // C G
        let result = detect_chord(&[60, 67]);
        assert_eq!(result, Some((0, ChordQuality::Power)));
    }

    #[test]
    fn detect_sus4() {
        // C F G
        let result = detect_chord(&[60, 65, 67]);
        assert_eq!(result, Some((0, ChordQuality::Sus4)));
    }

    #[test]
    fn detect_unison() {
        let result = detect_chord(&[60, 72]);
        assert_eq!(result, Some((0, ChordQuality::Unison)));
    }

    #[test]
    fn detect_single_note() {
        let result = detect_chord(&[64]);
        assert_eq!(result, Some((4, ChordQuality::Unison)));
    }

    #[test]
    fn detect_empty() {
        let result = detect_chord(&[]);
        assert_eq!(result, None);
    }

    #[test]
    fn detect_inversions() {
        // First inversion C major: E G C
        let result = detect_chord(&[64, 67, 72]);
        assert_eq!(result, Some((0, ChordQuality::Major)));
    }

    #[test]
    fn prefer_seventh_over_triad() {
        // C E G Bb — should detect Dom7, not just Major
        let result = detect_chord(&[60, 64, 67, 70]);
        assert_eq!(result, Some((0, ChordQuality::Dominant7)));
    }

    #[test]
    fn duplicate_pitches_ignored() {
        // C E G with duplicate C
        let result = detect_chord(&[60, 64, 67, 72]);
        assert_eq!(result, Some((0, ChordQuality::Major)));
    }
}
