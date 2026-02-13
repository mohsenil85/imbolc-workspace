use crate::action::{AudioEffect, DispatchResult};
use crate::state::AppState;
use crate::state::InstrumentId;
use imbolc_types::{DomainAction, InstrumentAction};

pub(super) fn handle_link_layer(
    state: &mut AppState,
    a: InstrumentId,
    b: InstrumentId,
) -> DispatchResult {
    if a == b {
        return DispatchResult::none();
    }
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(InstrumentAction::LinkLayer(a, b)),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildRouting);
    result.audio_effects.push(AudioEffect::RebuildSession);
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
}

pub(super) fn handle_unlink_layer(state: &mut AppState, id: InstrumentId) -> DispatchResult {
    let had_group = state
        .instruments
        .instrument(id)
        .and_then(|i| i.layer.group)
        .is_some();
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(InstrumentAction::UnlinkLayer(id)),
        &mut state.instruments,
        &mut state.session,
    );
    let mut result = DispatchResult::none();
    if had_group {
        result.audio_effects.push(AudioEffect::RebuildRouting);
        result.audio_effects.push(AudioEffect::RebuildSession);
        result.audio_effects.push(AudioEffect::RebuildInstruments);
    }
    result
}

pub(super) fn handle_adjust_layer_octave_offset(
    state: &mut AppState,
    id: InstrumentId,
    delta: i8,
) -> DispatchResult {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(InstrumentAction::AdjustLayerOctaveOffset(id, delta)),
        &mut state.instruments,
        &mut state.session,
    );
    DispatchResult::none()
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use crate::state::SourceType;

    #[test]
    fn adjust_layer_octave_offset_increments() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            0
        );

        handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            1
        );

        handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            2
        );
    }

    #[test]
    fn adjust_layer_octave_offset_decrements() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        handle_adjust_layer_octave_offset(&mut state, id, -1);
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            -1
        );
    }

    #[test]
    fn adjust_layer_octave_offset_clamps_high() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        for _ in 0..10 {
            handle_adjust_layer_octave_offset(&mut state, id, 1);
        }
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            4
        );
    }

    #[test]
    fn adjust_layer_octave_offset_clamps_low() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        for _ in 0..10 {
            handle_adjust_layer_octave_offset(&mut state, id, -1);
        }
        assert_eq!(
            state
                .instruments
                .instrument(id)
                .unwrap()
                .layer
                .octave_offset,
            -4
        );
    }

    #[test]
    fn adjust_layer_octave_offset_no_audio_effects() {
        let mut state = AppState::new();
        let id = state.add_instrument(SourceType::Saw);

        let result = handle_adjust_layer_octave_offset(&mut state, id, 1);
        assert!(!result
            .audio_effects
            .contains(&AudioEffect::RebuildInstruments));
        assert!(!result.audio_effects.contains(&AudioEffect::RebuildRouting));
    }
}
