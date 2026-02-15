//! Dispatch handler for generative engine actions.

use crate::action::{AudioEffect, DispatchResult};
use crate::state::AppState;
use imbolc_types::GenerativeAction;

pub fn dispatch_generative(
    action: &GenerativeAction,
    state: &mut AppState,
) -> DispatchResult {
    // Delegate pure state mutation to the shared reducer
    imbolc_types::reduce::reduce_action(
        &imbolc_types::DomainAction::Generative(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );

    let mut result = DispatchResult::none();

    // Auto-assign target instrument when adding a voice with no target
    if let GenerativeAction::AddVoice(_) = action {
        if let Some(voice) = state.session.generative.voices.last_mut() {
            if voice.target_instrument.is_none() {
                // Prefer selected instrument, fall back to first
                let target = state
                    .instruments
                    .selected_instrument()
                    .or_else(|| state.instruments.instruments.first())
                    .map(|i| i.id);
                voice.target_instrument = target;
            }
        }
    }

    match action {
        GenerativeAction::CommitCapture => {
            // Commit captured events to piano roll
            let events: Vec<_> = state.session.generative.captured_events.drain(..).collect();
            for event in &events {
                // Add note to the track for this instrument
                if let Some(track) = state
                    .session
                    .piano_roll
                    .tracks
                    .get_mut(&event.instrument_id)
                {
                    track.notes.push(imbolc_types::Note {
                        pitch: event.pitch,
                        tick: event.tick,
                        duration: event.duration_ticks,
                        velocity: event.velocity,
                        probability: 1.0,
                    });
                }
            }
            if !events.is_empty() {
                result.audio_effects.push(AudioEffect::UpdatePianoRoll);
            }
        }
        _ => {
            // All other generative actions just need session sync
            result.audio_effects.push(AudioEffect::RebuildSession);
        }
    }

    result
}
