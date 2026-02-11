use imbolc_types::{DomainAction, InstrumentAction};
use crate::state::AppState;
use crate::action::DispatchResult;

pub(super) fn handle_select(state: &mut AppState, action: &InstrumentAction) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
    DispatchResult::none()
}
