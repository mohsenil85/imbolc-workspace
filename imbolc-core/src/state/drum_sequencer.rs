use super::sampler::{BufferId, Slice, SliceId};
use serde::{Serialize, Deserialize};

// Re-export DrumStep from imbolc-types
pub use imbolc_types::DrumStep;

pub const NUM_PADS: usize = 12;
pub const DEFAULT_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChopperState {
    pub buffer_id: Option<BufferId>,
    pub path: Option<String>,
    pub name: String,
    pub slices: Vec<Slice>,
    pub selected_slice: usize,
    pub next_slice_id: SliceId,
    pub waveform_peaks: Vec<f32>,
    pub duration_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumPad {
    pub buffer_id: Option<BufferId>,
    pub path: Option<String>,
    pub name: String,
    pub level: f32, // 0.0-1.0, default 0.8
    pub slice_start: f32, // 0.0-1.0, default 0.0
    pub slice_end: f32,   // 0.0-1.0, default 1.0
    pub reverse: bool,    // play sample backwards
    pub pitch: i8,        // semitone offset, -24 to +24
}

impl Default for DrumPad {
    fn default() -> Self {
        Self {
            buffer_id: None,
            path: None,
            name: String::new(),
            level: 0.8,
            slice_start: 0.0,
            slice_end: 1.0,
            reverse: false,
            pitch: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumPattern {
    pub steps: Vec<Vec<DrumStep>>, // [NUM_PADS][length]
    pub length: usize,
}

impl DrumPattern {
    pub fn new(length: usize) -> Self {
        Self {
            steps: (0..NUM_PADS)
                .map(|_| (0..length).map(|_| DrumStep::default()).collect())
                .collect(),
            length,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumSequencerState {
    pub pads: Vec<DrumPad>,
    pub patterns: Vec<DrumPattern>,
    pub current_pattern: usize,
    #[serde(skip)]
    pub playing: bool,
    #[serde(skip)]
    pub current_step: usize,
    pub next_buffer_id: BufferId,
    #[serde(skip)]
    pub step_accumulator: f64,
    #[serde(skip)]
    pub last_played_step: Option<usize>,
    pub chopper: Option<ChopperState>,
    /// Swing amount: 0.0 = no swing, 1.0 = max swing (delays odd-numbered steps)
    pub swing_amount: f32,
    /// Pattern chain: ordered list of pattern indices to cycle through
    pub chain: Vec<usize>,
    /// Whether pattern chaining is active
    pub chain_enabled: bool,
    /// Current position within the chain (runtime)
    #[serde(skip)]
    pub chain_position: usize,
}

impl DrumSequencerState {
    pub fn new() -> Self {
        Self {
            pads: (0..NUM_PADS).map(|_| DrumPad::default()).collect(),
            patterns: (0..NUM_PATTERNS)
                .map(|_| DrumPattern::new(DEFAULT_STEPS))
                .collect(),
            current_pattern: 0,
            playing: false,
            current_step: 0,
            next_buffer_id: 10000,
            step_accumulator: 0.0,
            last_played_step: None,
            chopper: None,
            swing_amount: 0.0,
            chain: Vec::new(),
            chain_enabled: false,
            chain_position: 0,
        }
    }

    pub fn pattern(&self) -> &DrumPattern {
        &self.patterns[self.current_pattern]
    }

    pub fn pattern_mut(&mut self) -> &mut DrumPattern {
        &mut self.patterns[self.current_pattern]
    }
}

impl Default for DrumSequencerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a Euclidean rhythm pattern using Bjorklund's algorithm.
/// Returns a Vec<bool> of length `steps` with `pulses` evenly distributed.
/// `rotation` rotates the pattern by the given number of steps.
pub fn euclidean_rhythm(pulses: usize, steps: usize, rotation: usize) -> Vec<bool> {
    if steps == 0 {
        return vec![];
    }
    let pulses = pulses.min(steps);
    if pulses == 0 {
        return vec![false; steps];
    }
    if pulses == steps {
        return vec![true; steps];
    }

    // Bjorklund's algorithm
    let mut pattern: Vec<Vec<bool>> = Vec::new();
    let mut remainder: Vec<Vec<bool>> = Vec::new();

    for i in 0..steps {
        if i < pulses {
            pattern.push(vec![true]);
        } else {
            remainder.push(vec![false]);
        }
    }

    loop {
        if remainder.len() <= 1 {
            break;
        }
        let mut new_pattern = Vec::new();
        let min_len = pattern.len().min(remainder.len());
        for i in 0..min_len {
            let mut combined = pattern[i].clone();
            combined.extend_from_slice(&remainder[i]);
            new_pattern.push(combined);
        }
        let leftover_pattern: Vec<Vec<bool>> = pattern[min_len..].to_vec();
        let leftover_remainder: Vec<Vec<bool>> = remainder[min_len..].to_vec();
        pattern = new_pattern;
        remainder = if !leftover_pattern.is_empty() {
            leftover_pattern
        } else {
            leftover_remainder
        };
    }

    let mut result: Vec<bool> = Vec::new();
    for p in &pattern {
        result.extend_from_slice(p);
    }
    for r in &remainder {
        result.extend_from_slice(r);
    }
    result.truncate(steps);

    // Apply rotation
    if rotation > 0 && !result.is_empty() {
        let rot = rotation % result.len();
        result.rotate_right(rot);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drum_sequencer_new() {
        let seq = DrumSequencerState::new();
        assert_eq!(seq.pads.len(), NUM_PADS);
        assert_eq!(seq.patterns.len(), NUM_PATTERNS);
        assert_eq!(seq.pattern().length, DEFAULT_STEPS);
        assert!(!seq.playing);
    }

    #[test]
    fn test_drum_pattern_new() {
        let pattern = DrumPattern::new(16);
        assert_eq!(pattern.steps.len(), NUM_PADS);
        assert_eq!(pattern.steps[0].len(), 16);
        assert!(!pattern.steps[0][0].active);
    }

    #[test]
    fn test_toggle_step() {
        let mut seq = DrumSequencerState::new();
        seq.pattern_mut().steps[0][0].active = true;
        assert!(seq.pattern().steps[0][0].active);
        seq.pattern_mut().steps[0][0].active = false;
        assert!(!seq.pattern().steps[0][0].active);
    }

    #[test]
    fn test_pattern_switching() {
        let mut seq = DrumSequencerState::new();
        seq.pattern_mut().steps[0][0].active = true;
        seq.current_pattern = 1;
        assert!(!seq.pattern().steps[0][0].active);
        seq.current_pattern = 0;
        assert!(seq.pattern().steps[0][0].active);
    }

    #[test]
    fn euclidean_zero_pulses() {
        let result = euclidean_rhythm(0, 8, 0);
        assert_eq!(result.len(), 8);
        assert!(result.iter().all(|&v| !v));
    }

    #[test]
    fn euclidean_all_pulses() {
        let result = euclidean_rhythm(8, 8, 0);
        assert_eq!(result.len(), 8);
        assert!(result.iter().all(|&v| v));
    }

    #[test]
    fn euclidean_zero_steps() {
        let result = euclidean_rhythm(0, 0, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn euclidean_3_of_8() {
        let result = euclidean_rhythm(3, 8, 0);
        assert_eq!(result.len(), 8);
        assert_eq!(result.iter().filter(|&&v| v).count(), 3);
        // Classic Euclidean pattern: [true, false, false, true, false, false, true, false]
        assert_eq!(result, vec![true, false, false, true, false, false, true, false]);
    }

    #[test]
    fn euclidean_5_of_8() {
        let result = euclidean_rhythm(5, 8, 0);
        assert_eq!(result.len(), 8);
        assert_eq!(result.iter().filter(|&&v| v).count(), 5);
    }

    #[test]
    fn euclidean_with_rotation() {
        let unrotated = euclidean_rhythm(3, 8, 0);
        let rotated = euclidean_rhythm(3, 8, 2);
        assert_eq!(rotated.len(), 8);
        // Rotation shifts right by 2
        for i in 0..8 {
            assert_eq!(rotated[(i + 2) % 8], unrotated[i]);
        }
    }

    #[test]
    fn euclidean_pulses_exceeding_steps_clamped() {
        let result = euclidean_rhythm(5, 3, 0);
        assert_eq!(result.len(), 3);
        // Clamped to 3 pulses in 3 steps = all true
        assert!(result.iter().all(|&v| v));
    }
}
