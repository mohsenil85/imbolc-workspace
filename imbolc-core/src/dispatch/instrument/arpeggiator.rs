use crate::action::{AudioEffect, DispatchResult};
use crate::state::AppState;
use imbolc_types::{DomainAction, InstrumentAction};

pub(super) fn dispatch(state: &mut AppState, action: &InstrumentAction) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}
