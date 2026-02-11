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
    Action, ArrangementAction, AudioEffect, AudioFeedback, AutomationAction, BusAction,
    ChopperAction, ClickAction, DispatchResult, DomainAction, EqParamKind, FileSelectAction,
    FilterParamKind, InstrumentAction, InstrumentUpdate, LayerGroupAction, LfoParamKind,
    MidiAction, MixerAction, NavAction, NavIntent, PaneId, PianoRollAction, RoutedAction,
    SequencerAction, ServerAction, SessionAction, StatusEvent, ToggleResult, TunerAction, UiAction,
    VstParamAction, VstTarget,
};

/// Feedback from async I/O operations to the main thread.
/// This type stays in imbolc-core because it references SessionState and InstrumentState.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum IoFeedback {
    SaveComplete {
        id: u64,
        path: PathBuf,
        result: Result<String, String>,
    },
    LoadComplete {
        id: u64,
        path: PathBuf,
        result: Result<(SessionState, InstrumentState, String), String>,
    },
    ImportSynthDefComplete {
        id: u64,
        result: Result<(CustomSynthDef, String, PathBuf), String>,
    },
    ImportSynthDefLoaded {
        id: u64,
        result: Result<String, String>,
    },
}
