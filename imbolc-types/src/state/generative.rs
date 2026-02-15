//! Generative music engine state types.
//!
//! Session-level generative engine that produces note events via three algorithms:
//! Euclidean rhythms, Markov melodies, and L-system structures.

use serde::{Deserialize, Serialize};

use crate::InstrumentId;

/// Unique identifier for a generative voice.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct GenVoiceId(u32);

impl GenVoiceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for GenVoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Top-level generative engine state. Lives on `SessionState`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeState {
    /// Master on/off for the generative engine
    pub enabled: bool,
    /// Whether capture mode is active (recording generated events)
    pub capture_enabled: bool,
    /// Generative voices (each targets an instrument)
    pub voices: Vec<GenVoice>,
    /// Global macro controls
    pub macros: GenerativeMacros,
    /// Global constraints
    pub constraints: GenerativeConstraints,
    /// Next voice ID counter
    pub next_voice_id: u32,
    /// Runtime buffer of captured events (not persisted)
    #[serde(skip)]
    pub captured_events: Vec<CapturedGenEvent>,
}

impl Default for GenerativeState {
    fn default() -> Self {
        Self {
            enabled: false,
            capture_enabled: false,
            voices: Vec::new(),
            macros: GenerativeMacros::default(),
            constraints: GenerativeConstraints::default(),
            next_voice_id: 1,
            captured_events: Vec::new(),
        }
    }
}

/// A single generative voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenVoice {
    pub id: GenVoiceId,
    pub name: String,
    pub enabled: bool,
    pub muted: bool,
    pub target_instrument: Option<InstrumentId>,
    pub algorithm: GenerativeAlgorithm,
    pub velocity_min: u8,
    pub velocity_max: u8,
    pub octave_min: i8,
    pub octave_max: i8,
}

impl GenVoice {
    pub fn new(id: GenVoiceId, algorithm: GenerativeAlgorithm) -> Self {
        Self {
            id,
            name: format!("Voice {}", id.get()),
            enabled: true,
            muted: false,
            target_instrument: None,
            algorithm,
            velocity_min: 64,
            velocity_max: 127,
            octave_min: 3,
            octave_max: 6,
        }
    }
}

/// Algorithm for a generative voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerativeAlgorithm {
    Euclidean(EuclideanConfig),
    Markov(MarkovConfig),
    LSystem(LSystemConfig),
}

impl GenerativeAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Euclidean(_) => "Euclidean",
            Self::Markov(_) => "Markov",
            Self::LSystem(_) => "L-System",
        }
    }

    pub fn short_name(&self) -> char {
        match self {
            Self::Euclidean(_) => 'E',
            Self::Markov(_) => 'M',
            Self::LSystem(_) => 'L',
        }
    }

    pub fn rate(&self) -> &GenRate {
        match self {
            Self::Euclidean(c) => &c.rate,
            Self::Markov(c) => &c.rate,
            Self::LSystem(c) => &c.rate,
        }
    }

    pub fn rate_mut(&mut self) -> &mut GenRate {
        match self {
            Self::Euclidean(c) => &mut c.rate,
            Self::Markov(c) => &mut c.rate,
            Self::LSystem(c) => &mut c.rate,
        }
    }
}

/// Euclidean rhythm configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EuclideanConfig {
    pub pulses: u8,
    pub steps: u8,
    pub rotation: u8,
    pub rate: GenRate,
    pub pitch_mode: EuclideanPitchMode,
}

impl Default for EuclideanConfig {
    fn default() -> Self {
        Self {
            pulses: 4,
            steps: 8,
            rotation: 0,
            rate: GenRate::Eighth,
            pitch_mode: EuclideanPitchMode::Fixed(60),
        }
    }
}

/// How a Euclidean voice selects pitch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EuclideanPitchMode {
    /// Always play the same MIDI note
    Fixed(u8),
    /// Walk up/down the current scale
    ScaleWalk,
    /// Random note from the current scale
    RandomInScale,
}

impl EuclideanPitchMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fixed(_) => "Fixed",
            Self::ScaleWalk => "Scale Walk",
            Self::RandomInScale => "Random",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            Self::Fixed(_) => Self::ScaleWalk,
            Self::ScaleWalk => Self::RandomInScale,
            Self::RandomInScale => Self::Fixed(60),
        }
    }
}

/// Markov chain melody configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkovConfig {
    /// 12x12 transition matrix (pitch class → pitch class)
    pub transition_matrix: [[f32; 12]; 12],
    /// Starting pitch class (0-11)
    pub initial_pitch_class: u8,
    pub rate: GenRate,
    /// Probability of a rest (0.0-1.0)
    pub rest_probability: f32,
    pub duration_mode: MarkovDurationMode,
}

impl Default for MarkovConfig {
    fn default() -> Self {
        // Default: uniform transition matrix
        let row = [1.0 / 12.0; 12];
        Self {
            transition_matrix: [row; 12],
            initial_pitch_class: 0,
            rate: GenRate::Eighth,
            rest_probability: 0.1,
            duration_mode: MarkovDurationMode::Fixed,
        }
    }
}

impl MarkovConfig {
    /// Normalize a row so it sums to 1.0.
    pub fn normalize_row(&mut self, row: usize) {
        if row >= 12 {
            return;
        }
        let sum: f32 = self.transition_matrix[row].iter().sum();
        if sum > 0.0 {
            for val in &mut self.transition_matrix[row] {
                *val /= sum;
            }
        }
    }

    /// Randomize the entire matrix and normalize.
    pub fn randomize(&mut self, rng: &mut u64) {
        for row in 0..12 {
            for col in 0..12 {
                *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                self.transition_matrix[row][col] = (*rng as f32) / (u64::MAX as f32);
            }
            self.normalize_row(row);
        }
    }
}

/// How Markov voice determines note duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkovDurationMode {
    /// Duration equals one step
    Fixed,
    /// Duration varies 1-4 steps
    Variable,
    /// Legato: holds until next note
    Legato,
}

impl MarkovDurationMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Variable => "Variable",
            Self::Legato => "Legato",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            Self::Fixed => Self::Variable,
            Self::Variable => Self::Legato,
            Self::Legato => Self::Fixed,
        }
    }
}

/// L-system generative configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSystemConfig {
    /// Starting string
    pub axiom: String,
    /// Production rules: (symbol, replacement)
    pub rules: Vec<(char, String)>,
    /// Number of iterations (capped at 6)
    pub iterations: u8,
    /// Interpretation parameters
    pub step_interval: i8,
    pub note_duration_steps: u8,
    pub velocity: u8,
    pub rate: GenRate,
}

impl Default for LSystemConfig {
    fn default() -> Self {
        Self {
            axiom: "F".to_string(),
            rules: vec![('F', "F+G-F".to_string())],
            iterations: 3,
            step_interval: 2,
            note_duration_steps: 1,
            velocity: 100,
            rate: GenRate::Eighth,
        }
    }
}

impl LSystemConfig {
    /// Expand the L-system string. Capped at 10000 symbols.
    pub fn expand(&self) -> String {
        let mut current = self.axiom.clone();
        let iterations = self.iterations.min(6);
        for _ in 0..iterations {
            let mut next = String::new();
            for ch in current.chars() {
                if let Some((_, replacement)) = self.rules.iter().find(|(c, _)| *c == ch) {
                    next.push_str(replacement);
                } else {
                    next.push(ch);
                }
                if next.len() > 10000 {
                    next.truncate(10000);
                    return next;
                }
            }
            current = next;
        }
        if current.len() > 10000 {
            current.truncate(10000);
        }
        current
    }
}

/// Step rate for generative voices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenRate {
    Whole,
    Half,
    DottedQuarter,
    Quarter,
    DottedEighth,
    Eighth,
    DottedSixteenth,
    Sixteenth,
    ThirtySecond,
    TripletQuarter,
    TripletEighth,
    TripletSixteenth,
}

impl GenRate {
    pub const ALL: [GenRate; 12] = [
        GenRate::Whole,
        GenRate::Half,
        GenRate::DottedQuarter,
        GenRate::Quarter,
        GenRate::DottedEighth,
        GenRate::Eighth,
        GenRate::DottedSixteenth,
        GenRate::Sixteenth,
        GenRate::ThirtySecond,
        GenRate::TripletQuarter,
        GenRate::TripletEighth,
        GenRate::TripletSixteenth,
    ];

    /// Ticks per step given ticks-per-beat.
    pub fn ticks_per_step(&self, tpb: u32) -> f64 {
        let tpb = tpb as f64;
        match self {
            GenRate::Whole => tpb * 4.0,
            GenRate::Half => tpb * 2.0,
            GenRate::DottedQuarter => tpb * 1.5,
            GenRate::Quarter => tpb,
            GenRate::DottedEighth => tpb * 0.75,
            GenRate::Eighth => tpb * 0.5,
            GenRate::DottedSixteenth => tpb * 0.375,
            GenRate::Sixteenth => tpb * 0.25,
            GenRate::ThirtySecond => tpb * 0.125,
            GenRate::TripletQuarter => tpb * 2.0 / 3.0,
            GenRate::TripletEighth => tpb / 3.0,
            GenRate::TripletSixteenth => tpb / 6.0,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            GenRate::Whole => "1/1",
            GenRate::Half => "1/2",
            GenRate::DottedQuarter => "1/4.",
            GenRate::Quarter => "1/4",
            GenRate::DottedEighth => "1/8.",
            GenRate::Eighth => "1/8",
            GenRate::DottedSixteenth => "1/16.",
            GenRate::Sixteenth => "1/16",
            GenRate::ThirtySecond => "1/32",
            GenRate::TripletQuarter => "1/4T",
            GenRate::TripletEighth => "1/8T",
            GenRate::TripletSixteenth => "1/16T",
        }
    }

    pub fn cycle(&self) -> Self {
        let idx = Self::ALL.iter().position(|r| r == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn cycle_reverse(&self) -> Self {
        let idx = Self::ALL.iter().position(|r| r == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Global macro controls that influence all voices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeMacros {
    /// Controls pulses/rest_prob/iterations (0.0-1.0)
    pub density: f32,
    /// Controls rotation/entropy (0.0-1.0)
    pub chaos: f32,
    /// Controls velocity range (0.0-1.0)
    pub energy: f32,
    /// Controls intervals/step size (0.0-1.0)
    pub motion: f32,
}

impl Default for GenerativeMacros {
    fn default() -> Self {
        Self {
            density: 0.5,
            chaos: 0.3,
            energy: 0.7,
            motion: 0.5,
        }
    }
}

/// Global constraints applied to all generated events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeConstraints {
    /// Lock generated pitches to the session scale
    pub scale_lock: bool,
    /// Minimum MIDI pitch
    pub pitch_min: u8,
    /// Maximum MIDI pitch
    pub pitch_max: u8,
    /// Maximum notes per beat (0 = unlimited)
    pub max_notes_per_beat: u8,
    /// Timing humanization amount (0.0-1.0)
    pub humanize_timing: f32,
    /// Velocity humanization amount (0.0-1.0)
    pub humanize_velocity: f32,
}

impl Default for GenerativeConstraints {
    fn default() -> Self {
        Self {
            scale_lock: true,
            pitch_min: 36,
            pitch_max: 96,
            max_notes_per_beat: 0,
            humanize_timing: 0.0,
            humanize_velocity: 0.0,
        }
    }
}

/// A captured generative event (for piano roll commit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedGenEvent {
    pub instrument_id: InstrumentId,
    pub pitch: u8,
    pub velocity: u8,
    pub duration_ticks: u32,
    pub tick: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_rate_ticks_per_step() {
        let tpb = 480;
        assert_eq!(GenRate::Quarter.ticks_per_step(tpb), 480.0);
        assert_eq!(GenRate::Eighth.ticks_per_step(tpb), 240.0);
        assert_eq!(GenRate::Sixteenth.ticks_per_step(tpb), 120.0);
        assert_eq!(GenRate::Whole.ticks_per_step(tpb), 1920.0);
        assert_eq!(GenRate::Half.ticks_per_step(tpb), 960.0);
    }

    #[test]
    fn gen_rate_cycle_round_trip() {
        let mut rate = GenRate::Eighth;
        for _ in 0..GenRate::ALL.len() {
            rate = rate.cycle();
        }
        assert_eq!(rate, GenRate::Eighth);
    }

    #[test]
    fn gen_rate_cycle_reverse_round_trip() {
        let mut rate = GenRate::Eighth;
        for _ in 0..GenRate::ALL.len() {
            rate = rate.cycle_reverse();
        }
        assert_eq!(rate, GenRate::Eighth);
    }

    #[test]
    fn lsystem_expand_basic() {
        let cfg = LSystemConfig {
            axiom: "F".to_string(),
            rules: vec![('F', "FG".to_string())],
            iterations: 3,
            ..Default::default()
        };
        let expanded = cfg.expand();
        // F -> FG -> FGG -> FGGG (wait, that's wrong)
        // Actually: F -> FG -> FGG -> FGGG
        // iter 1: F -> FG
        // iter 2: FG -> FGG  (F->FG, G stays)
        // iter 3: FGG -> FGGG
        assert_eq!(expanded, "FGGG");
    }

    #[test]
    fn lsystem_expand_cap() {
        let cfg = LSystemConfig {
            axiom: "F".to_string(),
            rules: vec![('F', "FF".to_string())],
            iterations: 20, // would be 2^20 = 1M chars without cap
            ..Default::default()
        };
        let expanded = cfg.expand();
        assert!(expanded.len() <= 10000);
    }

    #[test]
    fn lsystem_iterations_capped_at_6() {
        let cfg = LSystemConfig {
            axiom: "F".to_string(),
            rules: vec![('F', "FG".to_string())],
            iterations: 10,
            ..Default::default()
        };
        // With 6 iterations of F->FG: F, FG, FGG, FGGG, FGGGG, FGGGGG, FGGGGGG
        let expanded = cfg.expand();
        assert_eq!(expanded, "FGGGGGG");
    }

    #[test]
    fn markov_normalize_row() {
        let mut cfg = MarkovConfig::default();
        cfg.transition_matrix[0] = [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        cfg.normalize_row(0);
        let sum: f32 = cfg.transition_matrix[0].iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
        assert!((cfg.transition_matrix[0][0] - 1.0 / 6.0).abs() < 0.001);
    }

    #[test]
    fn markov_randomize() {
        let mut cfg = MarkovConfig::default();
        let mut rng = 12345u64;
        cfg.randomize(&mut rng);
        for row in 0..12 {
            let sum: f32 = cfg.transition_matrix[row].iter().sum();
            assert!((sum - 1.0).abs() < 0.01, "Row {} sum: {}", row, sum);
        }
    }

    #[test]
    fn euclidean_pitch_mode_cycle() {
        let mode = EuclideanPitchMode::Fixed(60);
        let next = mode.cycle();
        assert!(matches!(next, EuclideanPitchMode::ScaleWalk));
        let next = next.cycle();
        assert!(matches!(next, EuclideanPitchMode::RandomInScale));
        let next = next.cycle();
        assert!(matches!(next, EuclideanPitchMode::Fixed(60)));
    }

    #[test]
    fn markov_duration_mode_cycle() {
        let mode = MarkovDurationMode::Fixed;
        assert_eq!(mode.cycle(), MarkovDurationMode::Variable);
        assert_eq!(mode.cycle().cycle(), MarkovDurationMode::Legato);
        assert_eq!(mode.cycle().cycle().cycle(), MarkovDurationMode::Fixed);
    }

    #[test]
    fn gen_voice_default() {
        let voice = GenVoice::new(GenVoiceId::new(1), GenerativeAlgorithm::Euclidean(EuclideanConfig::default()));
        assert!(voice.enabled);
        assert!(!voice.muted);
        assert!(voice.target_instrument.is_none());
        assert_eq!(voice.velocity_min, 64);
        assert_eq!(voice.velocity_max, 127);
    }

    #[test]
    fn generative_state_default() {
        let state = GenerativeState::default();
        assert!(!state.enabled);
        assert!(!state.capture_enabled);
        assert!(state.voices.is_empty());
        assert!(state.captured_events.is_empty());
    }

    #[test]
    fn algorithm_names() {
        assert_eq!(GenerativeAlgorithm::Euclidean(EuclideanConfig::default()).name(), "Euclidean");
        assert_eq!(GenerativeAlgorithm::Markov(MarkovConfig::default()).name(), "Markov");
        assert_eq!(GenerativeAlgorithm::LSystem(LSystemConfig::default()).name(), "L-System");
    }

    #[test]
    fn algorithm_short_names() {
        assert_eq!(GenerativeAlgorithm::Euclidean(EuclideanConfig::default()).short_name(), 'E');
        assert_eq!(GenerativeAlgorithm::Markov(MarkovConfig::default()).short_name(), 'M');
        assert_eq!(GenerativeAlgorithm::LSystem(LSystemConfig::default()).short_name(), 'L');
    }

    #[test]
    fn gen_rate_names() {
        assert_eq!(GenRate::Quarter.name(), "1/4");
        assert_eq!(GenRate::Eighth.name(), "1/8");
        assert_eq!(GenRate::TripletEighth.name(), "1/8T");
        assert_eq!(GenRate::DottedQuarter.name(), "1/4.");
    }

    #[test]
    fn constraints_default() {
        let c = GenerativeConstraints::default();
        assert!(c.scale_lock);
        assert_eq!(c.pitch_min, 36);
        assert_eq!(c.pitch_max, 96);
    }

    #[test]
    fn macros_default() {
        let m = GenerativeMacros::default();
        assert!((m.density - 0.5).abs() < f32::EPSILON);
        assert!((m.chaos - 0.3).abs() < f32::EPSILON);
        assert!((m.energy - 0.7).abs() < f32::EPSILON);
    }
}
