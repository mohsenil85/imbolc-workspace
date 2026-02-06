use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Runtime recording state.
/// Tracks audio recording status and automation recording mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordingState {
    /// Whether audio recording is active
    #[serde(skip)]
    pub recording: bool,
    /// Duration of current recording in seconds
    #[serde(skip)]
    pub recording_secs: u64,
    /// Whether automation recording is enabled
    #[serde(skip)]
    pub automation_recording: bool,
    /// Path to a recently stopped recording, pending waveform load
    #[serde(skip)]
    pub pending_recording_path: Option<PathBuf>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if either audio or automation recording is active
    pub fn is_any_recording(&self) -> bool {
        self.recording || self.automation_recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_recording() {
        let state = RecordingState::default();
        assert!(!state.recording);
        assert!(!state.automation_recording);
        assert_eq!(state.recording_secs, 0);
        assert!(state.pending_recording_path.is_none());
    }
}
