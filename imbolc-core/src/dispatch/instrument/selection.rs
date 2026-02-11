use crate::action::DispatchResult;
use crate::state::AppState;
use imbolc_types::{DomainAction, InstrumentAction};

pub(super) fn handle_select(state: &mut AppState, action: &InstrumentAction) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
    DispatchResult::none()
}
