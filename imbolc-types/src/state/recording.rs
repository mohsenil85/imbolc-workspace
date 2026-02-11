use crate::InstrumentId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

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
    /// Tracks armed for recording (instrument IDs)
    #[serde(skip)]
    pub armed_tracks: HashSet<InstrumentId>,
    /// Tracks currently recording (instrument IDs)
    #[serde(skip)]
    pub recording_tracks: HashSet<InstrumentId>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if either audio or automation recording is active
    pub fn is_any_recording(&self) -> bool {
        self.recording || self.automation_recording || !self.recording_tracks.is_empty()
    }

    /// Arm a track for recording
    pub fn arm_track(&mut self, id: InstrumentId) {
        self.armed_tracks.insert(id);
    }

    /// Disarm a track
    pub fn disarm_track(&mut self, id: InstrumentId) {
        self.armed_tracks.remove(&id);
    }

    /// Toggle track arm state
    pub fn toggle_arm(&mut self, id: InstrumentId) {
        if self.armed_tracks.contains(&id) {
            self.armed_tracks.remove(&id);
        } else {
            self.armed_tracks.insert(id);
        }
    }

    /// Check if a track is armed
    pub fn is_armed(&self, id: InstrumentId) -> bool {
        self.armed_tracks.contains(&id)
    }

    /// Check if a track is currently recording
    pub fn is_track_recording(&self, id: InstrumentId) -> bool {
        self.recording_tracks.contains(&id)
    }

    /// Start recording on a track
    pub fn start_track_recording(&mut self, id: InstrumentId) {
        self.recording_tracks.insert(id);
    }

    /// Stop recording on a track
    pub fn stop_track_recording(&mut self, id: InstrumentId) {
        self.recording_tracks.remove(&id);
    }

    /// Get all armed track IDs
    pub fn get_armed_tracks(&self) -> Vec<InstrumentId> {
        self.armed_tracks.iter().copied().collect()
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
