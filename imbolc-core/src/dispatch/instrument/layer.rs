use crate::state::AppState;
use crate::action::DispatchResult;

pub(super) fn handle_link_layer(
    state: &mut AppState,
    a: crate::state::InstrumentId,
    b: crate::state::InstrumentId,
) -> DispatchResult {
    if a == b {
        return DispatchResult::none();
    }
    let group_b = state.instruments.instrument(b).and_then(|i| i.layer_group);
    let group_a = state.instruments.instrument(a).and_then(|i| i.layer_group);
    let group_id = match (group_a, group_b) {
        (_, Some(g)) => g,
        (Some(g), None) => g,
        (None, None) => state.instruments.next_layer_group(),
    };
    if let Some(inst) = state.instruments.instrument_mut(a) {
        inst.layer_group = Some(group_id);
    }
    if let Some(inst) = state.instruments.instrument_mut(b) {
        inst.layer_group = Some(group_id);
    }
    DispatchResult::none()
}

pub(super) fn handle_unlink_layer(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    let old_group = state.instruments.instrument(id).and_then(|i| i.layer_group);
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.layer_group = None;
    }
    // If old group now has only 1 member, clear that member too
    if let Some(g) = old_group {
        let remaining: Vec<crate::state::InstrumentId> = state.instruments.instruments.iter()
            .filter(|i| i.layer_group == Some(g))
            .map(|i| i.id)
            .collect();
        if remaining.len() == 1 {
            if let Some(inst) = state.instruments.instrument_mut(remaining[0]) {
                inst.layer_group = None;
            }
        }
    }
    DispatchResult::none()
}
