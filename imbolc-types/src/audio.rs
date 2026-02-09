//! Audio-related types shared across crates.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::action::VstTarget;
use crate::{InstrumentId, VstPluginId};

/// SuperCollider server status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ServerStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Connected,
    Error,
}

/// Kind of audio export operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    MasterBounce,
    StemExport,
}

/// Feedback messages from the audio thread to the main thread.
#[derive(Debug, Clone)]
pub enum AudioFeedback {
    PlayheadPosition(u32),
    BpmUpdate(f32),
    PlayingChanged(bool),
    DrumSequencerStep {
        instrument_id: InstrumentId,
        step: usize,
    },
    ServerStatus {
        status: ServerStatus,
        message: String,
        server_running: bool,
    },
    RecordingState {
        is_recording: bool,
        elapsed_secs: u64,
    },
    RecordingStopped(PathBuf),
    RenderComplete {
        instrument_id: InstrumentId,
        path: PathBuf,
    },
    CompileResult(Result<String, String>),
    LoadResult(Result<String, String>),
    PendingBufferFreed,
    VstParamsDiscovered {
        instrument_id: InstrumentId,
        target: VstTarget,
        vst_plugin_id: VstPluginId,
        params: Vec<(u32, String, Option<String>, f32)>, // (index, name, label, default)
    },
    VstStateSaved {
        instrument_id: InstrumentId,
        target: VstTarget,
        path: PathBuf,
    },
    ExportComplete {
        kind: ExportKind,
        paths: Vec<PathBuf>,
    },
    ExportProgress {
        progress: f32,
    },
    /// The scsynth server process crashed or became unreachable.
    /// All tracked nodes have been invalidated.
    ServerCrashed {
        message: String,
    },
    /// Periodic telemetry summary from the audio thread.
    TelemetrySummary {
        /// Average tick duration in microseconds
        avg_tick_us: u32,
        /// Maximum tick duration in the window
        max_tick_us: u32,
        /// 95th percentile tick duration
        p95_tick_us: u32,
        /// Cumulative count of ticks exceeding budget
        overruns: u64,
    },
}
