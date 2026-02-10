//! Audio feedback state from the audio thread.

use imbolc_audio::ServerStatus;
use imbolc_types::VisualizationState;

/// State updated from the audio thread each frame.
#[derive(Debug, Clone)]
pub struct AudioFeedbackState {
    /// Real-time visualization data from audio analysis
    pub visualization: VisualizationState,
    /// SC server status
    pub server_status: ServerStatus,
    /// Audio-owned playhead position
    pub playhead: u32,
    /// Audio-owned BPM
    pub bpm: f32,
    /// Audio-owned playing state
    pub playing: bool,
}

impl Default for AudioFeedbackState {
    fn default() -> Self {
        Self {
            visualization: VisualizationState::default(),
            server_status: ServerStatus::Stopped,
            playhead: 0,
            bpm: 120.0,
            playing: false,
        }
    }
}
