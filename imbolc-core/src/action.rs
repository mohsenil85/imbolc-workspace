//! Action types for the dispatch system.
//!
//! Most action types are re-exported from imbolc-types. This module defines
//! IoFeedback which references complex state types that stay in imbolc-core.

use std::path::PathBuf;

use crate::state::custom_synthdef::CustomSynthDef;
use crate::state::instrument_state::InstrumentState;
use crate::state::session::SessionState;

// Re-export all action types from imbolc-types
pub use imbolc_types::{
    Action, ArrangementAction, AudioDirty, AudioFeedback, AutomationAction, BusAction,
    ChopperAction, ClickAction, DispatchResult, FileSelectAction, FilterParamKind, InstrumentAction,
    InstrumentUpdate, LfoParamKind, MidiAction, MixerAction, NavAction, NavIntent,
    PianoRollAction, SequencerAction, ServerAction, SessionAction, StatusEvent,
    ToggleResult, TunerAction, VstParamAction, VstTarget,
};

/// Feedback from async I/O operations to the main thread.
/// This type stays in imbolc-core because it references SessionState and InstrumentState.
#[derive(Debug)]
pub enum IoFeedback {
    SaveComplete { id: u64, path: PathBuf, result: Result<String, String> },
    LoadComplete { id: u64, path: PathBuf, result: Result<(SessionState, InstrumentState, String), String> },
    ImportSynthDefComplete { id: u64, result: Result<(CustomSynthDef, String, PathBuf), String> },
    ImportSynthDefLoaded { id: u64, result: Result<String, String> },
}
