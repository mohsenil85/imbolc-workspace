use crate::state::AppState;
use crate::action::DispatchResult;
use crate::state::InstrumentId;

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
    // Auto-create LayerGroupMixer if new group
    let bus_ids: Vec<u8> = state.session.mixer.bus_ids().collect();
    if state.session.mixer.layer_group_mixer(group_id).is_none() {
        state.session.mixer.add_layer_group_mixer(group_id, &bus_ids);
    }
    let mut result = DispatchResult::none();
    result.audio_dirty.routing = true;
    result.audio_dirty.session = true;
    result.audio_dirty.instruments = true;
    result
}

pub(super) fn handle_unlink_layer(
    state: &mut AppState,
    id: crate::state::InstrumentId,
) -> DispatchResult {
    let old_group = state.instruments.instrument(id).and_then(|i| i.layer_group);
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.layer_group = None;
    }
    let mut result = DispatchResult::none();
    // If old group now has only 1 member, clear that member too and remove group mixer
    if let Some(g) = old_group {
        let remaining: Vec<crate::state::InstrumentId> = state.instruments.instruments.iter()
            .filter(|i| i.layer_group == Some(g))
            .map(|i| i.id)
            .collect();
        if remaining.len() <= 1 {
            // Clear any remaining singleton
            if remaining.len() == 1 {
                if let Some(inst) = state.instruments.instrument_mut(remaining[0]) {
                    inst.layer_group = None;
                }
            }
            // Remove the group mixer
            state.session.mixer.remove_layer_group_mixer(g);
        }
        result.audio_dirty.routing = true;
        result.audio_dirty.session = true;
        result.audio_dirty.instruments = true;
    }
    result
}

pub(super) fn handle_adjust_layer_octave_offset(
    state: &mut AppState,
    id: InstrumentId,
    delta: i8,
) -> DispatchResult {
    if let Some(inst) = state.instruments.instrument_mut(id) {
        inst.layer_octave_offset = (inst.layer_octave_offset + delta).clamp(-4, 4);
    }
    DispatchResult::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SourceType;

    #[test]
    fn adjust_layer_octave_offset_increments() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, 0);

        handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, 1);

        handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, 2);
    }

    #[test]
    fn adjust_layer_octave_offset_decrements() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        handle_adjust_layer_octave_offset(&mut state, id, -1);
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, -1);
    }

    #[test]
    fn adjust_layer_octave_offset_clamps_high() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        for _ in 0..10 {
            handle_adjust_layer_octave_offset(&mut state, id, 1);
        }
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, 4);
    }

    #[test]
    fn adjust_layer_octave_offset_clamps_low() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        for _ in 0..10 {
            handle_adjust_layer_octave_offset(&mut state, id, -1);
        }
        assert_eq!(state.instruments.instrument(id).unwrap().layer_octave_offset, -4);
    }

    #[test]
    fn adjust_layer_octave_offset_no_audio_dirty() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        let result = handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert!(!result.audio_dirty.instruments);
        assert!(!result.audio_dirty.routing);
    }
}
