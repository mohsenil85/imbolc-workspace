//! Runtime play-state for generative voices on the audio thread.

use std::collections::HashMap;

use imbolc_types::state::generative::GenVoiceId;

/// Per-voice runtime state on the audio thread.
#[derive(Debug, Clone)]
pub struct VoicePlayState {
    /// Fractional step accumulator (advanced by elapsed time)
    pub accumulator: f64,
    /// Current step index within the pattern/sequence
    pub step_index: usize,
    /// Current MIDI pitch being played (for release)
    pub current_pitch: Option<u8>,
    /// Cached Euclidean pattern (invalidated on param change)
    pub euclidean_pattern: Option<Vec<bool>>,
    /// Markov: current pitch class (0-11)
    pub markov_current_pc: u8,
    /// L-System: cached expanded string
    pub lsystem_expanded: Option<String>,
    /// L-System: cursor position in expanded string
    pub lsystem_cursor: usize,
    /// L-System: current pitch
    pub lsystem_current_pitch: i16,
    /// L-System: pitch stack for [ ] operators
    pub lsystem_pitch_stack: Vec<i16>,
    /// Fingerprint of algorithm config for cache invalidation
    pub config_fingerprint: u64,
}

impl Default for VoicePlayState {
    fn default() -> Self {
        Self {
            accumulator: 0.0,
            step_index: 0,
            current_pitch: None,
            euclidean_pattern: None,
            markov_current_pc: 0,
            lsystem_expanded: None,
            lsystem_cursor: 0,
            lsystem_current_pitch: 60,
            lsystem_pitch_stack: Vec::new(),
            config_fingerprint: 0,
        }
    }
}

impl VoicePlayState {
    /// Invalidate all cached patterns/expansions.
    pub fn invalidate_caches(&mut self) {
        self.euclidean_pattern = None;
        self.lsystem_expanded = None;
        self.lsystem_cursor = 0;
    }
}

/// Collection of per-voice play states.
pub type GenerativePlayState = HashMap<GenVoiceId, VoicePlayState>;
