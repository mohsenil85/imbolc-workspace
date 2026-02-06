//! Clipboard types for copy/paste operations.

use super::drum_sequencer::DrumStep;
use super::piano_roll::ClipboardNote;

/// Clipboard contents — one variant per context
#[derive(Debug, Clone)]
pub enum ClipboardContents {
    /// Piano roll notes with relative positions
    PianoRollNotes(Vec<ClipboardNote>),
    /// Drum sequencer steps: Vec<(pad_index, step_offset, DrumStep)>
    DrumSteps {
        steps: Vec<(usize, usize, DrumStep)>, // (pad_idx, step_offset, step_data)
    },
    /// Automation points: Vec<(tick_offset, value)>
    AutomationPoints {
        points: Vec<(u32, f32)>, // (tick_offset, value)
    },
}

/// App-wide clipboard (lives in AppState)
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    pub contents: Option<ClipboardContents>,
}
