//! Pure state-mutation reducers for the Imbolc DAW.
//!
//! These functions are the single source of truth for action → state mutations.
//! Both imbolc-core dispatch and imbolc-audio's audio thread call into this module.
//!
//! Reducers are pure: they mutate `InstrumentState` and `SessionState` only.
//! They do NOT:
//! - Construct DispatchResult (no nav intents, no status events)
//! - Record automation
//! - Push undo snapshots
//! - Generate AudioEffect events
//! - Send audio commands

mod instrument;
mod mixer;
mod piano_roll;
mod automation;
mod bus;
mod session;
mod vst_param;
mod click;

use crate::{
    DomainAction, InstrumentState, SessionState,
    AutomationAction, PianoRollAction, SessionAction, VstParamAction,
};

/// Check whether an action can be incrementally reduced on the audio thread.
/// Returns false for actions that require full state sync (undo/redo, arrangement,
/// sequencer, chopper, server, and specific sub-actions that involve file I/O or
/// state not available on the audio thread).
pub fn is_reducible(action: &DomainAction) -> bool {
    match action {
        DomainAction::Midi(_)
        | DomainAction::Tuner(_) | DomainAction::AudioFeedback(_) => true,

        DomainAction::Undo | DomainAction::Redo => false,

        DomainAction::Instrument(_) => true,
        DomainAction::Mixer(_) => true,
        DomainAction::Bus(_) => true,
        DomainAction::LayerGroup(_) => true,
        DomainAction::Click(_) => true,

        DomainAction::PianoRoll(a) => !matches!(a,
            PianoRollAction::RenderToWav(_)
            | PianoRollAction::BounceToWav
            | PianoRollAction::ExportStems
            | PianoRollAction::CancelExport
        ),
        DomainAction::Automation(a) => !matches!(a, AutomationAction::ToggleRecording),
        DomainAction::VstParam(a) => !matches!(a,
            VstParamAction::DiscoverParams(_, _)
            | VstParamAction::SaveState(_, _)
        ),
        DomainAction::Session(a) => !matches!(a,
            SessionAction::NewProject
            | SessionAction::Save
            | SessionAction::SaveAs(_)
            | SessionAction::Load
            | SessionAction::LoadFrom(_)
            | SessionAction::ImportCustomSynthDef(_)
            | SessionAction::CreateCheckpoint(_)
            | SessionAction::RestoreCheckpoint(_)
            | SessionAction::DeleteCheckpoint(_)
        ),

        DomainAction::Arrangement(_) => false,
        DomainAction::Sequencer(_) => false,
        DomainAction::Chopper(_) => false,
        DomainAction::Server(_) => false,
    }
}

/// Apply an action's state mutations to the given state.
/// Returns true if the action was handled (state was mutated or no-op).
/// Returns false if the action is not reducible (caller should use full sync).
pub fn reduce_action(
    action: &DomainAction,
    instruments: &mut InstrumentState,
    session: &mut SessionState,
) -> bool {
    match action {
        // Actions that don't affect audio-thread state (no-op, handled)
        DomainAction::Midi(_)
        | DomainAction::Tuner(_) | DomainAction::AudioFeedback(_) => true,

        // Undo/Redo: not reducible (wholesale state replacement)
        DomainAction::Undo | DomainAction::Redo => false,

        DomainAction::Instrument(a) => instrument::reduce(a, instruments, session),
        DomainAction::Mixer(a) => mixer::reduce(a, instruments, session),
        DomainAction::PianoRoll(a) => piano_roll::reduce(a, session),
        DomainAction::Automation(a) => automation::reduce(a, session),
        DomainAction::Bus(a) => bus::reduce_bus(a, session, instruments),
        DomainAction::LayerGroup(a) => bus::reduce_layer_group(a, session),
        DomainAction::VstParam(a) => vst_param::reduce(a, instruments, session),
        DomainAction::Session(a) => session::reduce(a, session, instruments),
        DomainAction::Click(a) => { click::reduce(a, session); true }

        DomainAction::Arrangement(_) => false,
        DomainAction::Sequencer(_) => false,
        DomainAction::Chopper(_) => false,
        DomainAction::Server(_) => false,
    }
}
