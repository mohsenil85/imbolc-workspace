use crate::state::AppState;
use crate::action::DispatchResult;

pub(super) fn handle_select(state: &mut AppState, idx: usize) -> DispatchResult {
    if idx < state.instruments.instruments.len() {
        state.instruments.selected = Some(idx);
    }
    DispatchResult::none()
}

pub(super) fn handle_select_next(state: &mut AppState) -> DispatchResult {
    state.instruments.select_next();
    DispatchResult::none()
}

pub(super) fn handle_select_prev(state: &mut AppState) -> DispatchResult {
    state.instruments.select_prev();
    DispatchResult::none()
}

pub(super) fn handle_select_first(state: &mut AppState) -> DispatchResult {
    if !state.instruments.instruments.is_empty() {
        state.instruments.selected = Some(0);
    }
    DispatchResult::none()
}

pub(super) fn handle_select_last(state: &mut AppState) -> DispatchResult {
    if !state.instruments.instruments.is_empty() {
        state.instruments.selected = Some(state.instruments.instruments.len() - 1);
    }
    DispatchResult::none()
}
