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
    /// Audio thread average tick duration in microseconds (1s window)
    pub telemetry_avg_tick_us: u32,
    /// Audio thread max tick duration in microseconds (1s window)
    pub telemetry_max_tick_us: u32,
    /// Audio thread p95 tick duration in microseconds (1s window)
    pub telemetry_p95_tick_us: u32,
    /// Cumulative tick budget overruns
    pub telemetry_overruns: u64,
    /// Dynamic scheduling lookahead in milliseconds
    pub telemetry_lookahead_ms: f32,
    /// Current OSC sender queue depth
    pub telemetry_osc_queue_depth: u16,
    /// Current tuning drift in cents (JI vs ET)
    pub tuning_drift_cents: f64,
}

impl Default for AudioFeedbackState {
    fn default() -> Self {
        Self {
            visualization: VisualizationState::default(),
            server_status: ServerStatus::Stopped,
            playhead: 0,
            bpm: 120.0,
            playing: false,
            telemetry_avg_tick_us: 0,
            telemetry_max_tick_us: 0,
            telemetry_p95_tick_us: 0,
            telemetry_overruns: 0,
            telemetry_lookahead_ms: 0.0,
            telemetry_osc_queue_depth: 0,
            tuning_drift_cents: 0.0,
        }
    }
}
