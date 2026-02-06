//! Drum sequencer types.

use serde::{Deserialize, Serialize};

/// A single step in a drum pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrumStep {
    pub active: bool,
    pub velocity: u8,       // 1-127, default 100
    pub probability: f32,   // 0.0-1.0, default 1.0 (always play)
    pub pitch_offset: i8,   // semitone offset per step, default 0
}

impl Default for DrumStep {
    fn default() -> Self {
        Self {
            active: false,
            velocity: 100,
            probability: 1.0,
            pitch_offset: 0,
        }
    }
}
