pub mod arpeggiator;
pub mod arrangement;
pub mod automation;
pub mod clipboard;
pub mod custom_synthdef;
pub mod drum_sequencer;
pub mod groove;
pub mod humanize;
pub mod instrument;
pub mod instrument_state;
pub mod io;
pub mod midi_recording;
pub mod mixer;
pub mod music;
pub mod parameter_target;
pub mod piano_roll;
pub mod project;
pub mod recording;
pub mod sampler;
pub mod session;
pub mod theme;
pub mod vst;

pub use arpeggiator::*;
pub use arrangement::*;
pub use automation::*;
pub use clipboard::{Clipboard, ClipboardContents};
pub use custom_synthdef::*;
pub use drum_sequencer::*;
pub use groove::*;
pub use humanize::*;
pub use instrument::*;
pub use instrument_state::*;
pub use io::*;
pub use midi_recording::*;
pub use mixer::*;
pub use music::*;
pub use parameter_target::*;
pub use piano_roll::*;
pub use project::*;
pub use recording::*;
pub use sampler::*;
pub use session::*;
pub use theme::*;
pub use vst::*;

use std::collections::VecDeque;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ExportKind, InstrumentId};

/// State for a render-to-WAV operation in progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRender {
    pub instrument_id: InstrumentId,
    pub path: PathBuf,
    pub was_looping: bool,
}

/// State for an export operation (master bounce or stem export)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingExport {
    pub kind: ExportKind,
    pub was_looping: bool,
    pub paths: Vec<PathBuf>,
}

/// Keyboard layout configuration for key translation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KeyboardLayout {
    #[default]
    Qwerty,
    Colemak,
}

/// Real-time visualization data from audio analysis synths
#[derive(Debug, Clone)]
pub struct VisualizationState {
    /// 7-band spectrum analyzer amplitudes (60Hz, 150Hz, 400Hz, 1kHz, 2.5kHz, 6kHz, 15kHz)
    pub spectrum_bands: [f32; 7],
    /// Master output peak levels (left, right)
    pub peak_l: f32,
    pub peak_r: f32,
    /// Master output RMS levels (left, right)
    pub rms_l: f32,
    pub rms_r: f32,
    /// Oscilloscope ring buffer (recent peak samples at ~30Hz)
    pub scope_buffer: VecDeque<f32>,
}

impl Default for VisualizationState {
    fn default() -> Self {
        Self {
            spectrum_bands: [0.0; 7],
            peak_l: 0.0,
            peak_r: 0.0,
            rms_l: 0.0,
            rms_r: 0.0,
            scope_buffer: VecDeque::with_capacity(200),
        }
    }
}

/// Generation counters for async I/O results (ignore stale completions).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct IoGeneration {
    pub save: u64,
    pub load: u64,
    pub import_synthdef: u64,
}

/// Ownership status for an instrument in network mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipDisplayStatus {
    /// This client owns the instrument.
    OwnedByMe,
    /// Another client owns the instrument (includes their name).
    OwnedByOther(String),
    /// No one owns the instrument.
    Unowned,
    /// Not in network mode (don't display ownership).
    Local,
}

impl Default for OwnershipDisplayStatus {
    fn default() -> Self {
        Self::Local
    }
}

/// Network collaboration context for UI display.
#[derive(Debug, Clone, Default)]
pub struct NetworkDisplayContext {
    /// Ownership status for each instrument (by ID).
    pub ownership: std::collections::HashMap<InstrumentId, OwnershipDisplayStatus>,
    /// Whether this client has privileged status (can control transport, save, load).
    pub is_privileged: bool,
    /// Name of the privileged client (if any).
    pub privileged_client_name: Option<String>,
}

impl IoGeneration {
    pub fn next_save(&mut self) -> u64 {
        self.save = self.save.wrapping_add(1);
        self.save
    }

    pub fn next_load(&mut self) -> u64 {
        self.load = self.load.wrapping_add(1);
        self.load
    }

    pub fn next_import_synthdef(&mut self) -> u64 {
        self.import_synthdef = self.import_synthdef.wrapping_add(1);
        self.import_synthdef
    }
}
