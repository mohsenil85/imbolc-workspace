//! I/O state for async operations (render, export).

use serde::{Deserialize, Serialize};

use super::{IoGeneration, PendingExport, PendingRender};

/// I/O state for render and export operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IoState {
    /// Pending render-to-WAV operation
    #[serde(skip)]
    pub pending_render: Option<PendingRender>,
    /// Pending export operation (master bounce or stem export)
    #[serde(skip)]
    pub pending_export: Option<PendingExport>,
    /// Export progress (0.0 to 1.0)
    #[serde(skip)]
    pub export_progress: f32,
    /// Generation counters for ignoring stale async results
    #[serde(skip)]
    pub generation: IoGeneration,
    /// True while a save operation is in progress
    #[serde(skip)]
    pub save_in_progress: bool,
    /// True while a load operation is in progress
    #[serde(skip)]
    pub load_in_progress: bool,
    /// Last I/O error message (save or load failure)
    #[serde(skip)]
    pub last_io_error: Option<String>,
}
