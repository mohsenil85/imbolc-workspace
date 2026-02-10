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
