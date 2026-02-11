//! Dispatch handlers for per-track groove settings.

use crate::action::DispatchResult;
use crate::state::AppState;
use imbolc_types::{DomainAction, InstrumentAction};

pub fn dispatch(state: &mut AppState, action: &InstrumentAction) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
    DispatchResult::none()
}
