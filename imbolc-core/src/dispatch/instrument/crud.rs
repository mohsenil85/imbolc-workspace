use crate::action::{AudioEffect, DispatchResult, NavIntent, PaneId};
use crate::dispatch::automation::record_automation_point;
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use crate::state::BufferId;
use imbolc_audio::AudioHandle;
use imbolc_types::{DomainAction, InstrumentAction};

fn reduce(state: &mut AppState, action: &InstrumentAction) {
    imbolc_types::reduce::reduce_action(
        &DomainAction::Instrument(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );
}

pub(super) fn handle_add(
    state: &mut AppState,
    source_type: crate::state::SourceType,
) -> DispatchResult {
    let next_id = state.instruments.next_id;
    reduce(state, &InstrumentAction::Add(source_type));
    let mut result = DispatchResult::with_nav(NavIntent::SwitchTo(PaneId::InstrumentEdit));
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result.audio_effects.push(AudioEffect::UpdatePianoRoll);
    result
        .audio_effects
        .push(AudioEffect::AddInstrumentRouting(next_id));
    result
}

pub(super) fn handle_delete(
    state: &mut AppState,
    audio: &mut AudioHandle,
    inst_id: crate::state::InstrumentId,
) -> DispatchResult {
    // Collect buffer IDs from the instrument before removing it
    let mut buffer_ids: Vec<BufferId> = Vec::new();
    if let Some(inst) = state.instruments.instrument(inst_id) {
        if let Some(seq) = inst.drum_sequencer() {
            for pad in &seq.pads {
                if let Some(id) = pad.buffer_id {
                    buffer_ids.push(id);
                }
            }
            if let Some(chopper) = &seq.chopper {
                if let Some(id) = chopper.buffer_id {
                    buffer_ids.push(id);
                }
            }
        }
        if let Some(sampler) = inst.sampler_config() {
            if let Some(id) = sampler.buffer_id {
                buffer_ids.push(id);
            }
        }
    }
    if !buffer_ids.is_empty() {
        audio.free_samples(buffer_ids);
    }

    reduce(state, &InstrumentAction::Delete(inst_id));
    let mut result = if state.instruments.instruments.is_empty() {
        DispatchResult::with_nav(NavIntent::SwitchTo(PaneId::Add))
    } else {
        DispatchResult::none()
    };
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result.audio_effects.push(AudioEffect::UpdatePianoRoll);
    result.audio_effects.push(AudioEffect::UpdateAutomation);
    result
        .audio_effects
        .push(AudioEffect::DeleteInstrumentRouting(inst_id));
    result
}

pub(super) fn handle_edit(state: &mut AppState, id: crate::state::InstrumentId) -> DispatchResult {
    reduce(state, &InstrumentAction::Edit(id));
    DispatchResult::with_nav(NavIntent::SwitchTo(PaneId::InstrumentEdit))
}

pub(super) fn handle_update(
    state: &mut AppState,
    update: &crate::action::InstrumentUpdate,
) -> DispatchResult {
    // Capture old values for automation recording before applying
    let old_values = if state.recording.automation_recording && state.audio.playing {
        state.instruments.instrument(update.id).map(|inst| {
            (
                inst.modulation.lfo.rate,
                inst.modulation.lfo.depth,
                inst.modulation.amp_envelope.attack,
                inst.modulation.amp_envelope.decay,
                inst.modulation.amp_envelope.sustain,
                inst.modulation.amp_envelope.release,
            )
        })
    } else {
        None
    };

    reduce(state, &InstrumentAction::Update(Box::new(update.clone())));

    // Record automation for changed LFO/envelope params
    if let Some((old_lfo_rate, old_lfo_depth, old_attack, old_decay, old_sustain, old_release)) =
        old_values
    {
        let id = update.id;
        let threshold = 0.001;

        if (update.lfo.rate - old_lfo_rate).abs() > threshold {
            let normalized = AutomationTarget::lfo_rate(id).normalize_value(update.lfo.rate);
            record_automation_point(state, AutomationTarget::lfo_rate(id), normalized);
        }
        if (update.lfo.depth - old_lfo_depth).abs() > threshold {
            let normalized = AutomationTarget::lfo_depth(id).normalize_value(update.lfo.depth);
            record_automation_point(state, AutomationTarget::lfo_depth(id), normalized);
        }
        if (update.amp_envelope.attack - old_attack).abs() > threshold {
            let normalized =
                AutomationTarget::attack(id).normalize_value(update.amp_envelope.attack);
            record_automation_point(state, AutomationTarget::attack(id), normalized);
        }
        if (update.amp_envelope.decay - old_decay).abs() > threshold {
            let normalized = AutomationTarget::decay(id).normalize_value(update.amp_envelope.decay);
            record_automation_point(state, AutomationTarget::decay(id), normalized);
        }
        if (update.amp_envelope.sustain - old_sustain).abs() > threshold {
            let normalized =
                AutomationTarget::sustain(id).normalize_value(update.amp_envelope.sustain);
            record_automation_point(state, AutomationTarget::sustain(id), normalized);
        }
        if (update.amp_envelope.release - old_release).abs() > threshold {
            let normalized =
                AutomationTarget::release(id).normalize_value(update.amp_envelope.release);
            record_automation_point(state, AutomationTarget::release(id), normalized);
        }
    }

    let mut result = DispatchResult::none();
    result.audio_effects.push(AudioEffect::RebuildInstruments);
    result
        .audio_effects
        .push(AudioEffect::RebuildRoutingForInstrument(update.id));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::InstrumentUpdate;
    use crate::state::instrument::SourceType;
    use imbolc_types::InstrumentId;

    fn setup_with_instrument() -> (AppState, InstrumentId) {
        let mut state = AppState::new();
        let id = state.instruments.add_instrument(SourceType::Saw);
        (state, id)
    }

    fn default_update(state: &AppState, id: InstrumentId) -> InstrumentUpdate {
        let inst = state.instruments.instrument(id).unwrap();
        InstrumentUpdate {
            id,
            source: inst.source,
            source_params: inst.source_params.clone(),
            processing_chain: inst.processing_chain.clone(),
            lfo: inst.modulation.lfo.clone(),
            amp_envelope: inst.modulation.amp_envelope.clone(),
            polyphonic: inst.polyphonic,
            active: inst.mixer.active,
        }
    }

    #[test]
    fn handle_update_applies_lfo() {
        let (mut state, id) = setup_with_instrument();
        let mut update = default_update(&state, id);
        update.lfo.rate = 8.0;
        update.lfo.depth = 0.9;
        handle_update(&mut state, &update);
        let inst = state.instruments.instrument(id).unwrap();
        assert!((inst.modulation.lfo.rate - 8.0).abs() < f32::EPSILON);
        assert!((inst.modulation.lfo.depth - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn handle_update_records_lfo_rate_when_recording() {
        let (mut state, id) = setup_with_instrument();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;
        state.audio.playhead = 100;

        let mut update = default_update(&state, id);
        update.lfo.rate = 10.0; // changed from default 2.0
        handle_update(&mut state, &update);

        let target = AutomationTarget::lfo_rate(id);
        let lane = state.session.automation.lane_for_target(&target);
        assert!(lane.is_some(), "LfoRate lane should be created");
        assert_eq!(lane.unwrap().points.len(), 1);
    }

    #[test]
    fn handle_update_records_envelope_changes_when_recording() {
        let (mut state, id) = setup_with_instrument();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;
        state.audio.playhead = 200;

        let mut update = default_update(&state, id);
        update.amp_envelope.attack = 0.5; // changed from default
        update.amp_envelope.release = 1.0; // changed from default
        handle_update(&mut state, &update);

        let attack_lane = state
            .session
            .automation
            .lane_for_target(&AutomationTarget::attack(id));
        assert!(
            attack_lane.is_some(),
            "EnvelopeAttack lane should be created"
        );
        assert_eq!(attack_lane.unwrap().points.len(), 1);

        let release_lane = state
            .session
            .automation
            .lane_for_target(&AutomationTarget::release(id));
        assert!(
            release_lane.is_some(),
            "EnvelopeRelease lane should be created"
        );
        assert_eq!(release_lane.unwrap().points.len(), 1);
    }

    #[test]
    fn handle_update_no_automation_when_not_recording() {
        let (mut state, id) = setup_with_instrument();
        state.recording.automation_recording = false;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;

        let mut update = default_update(&state, id);
        update.lfo.rate = 10.0;
        update.amp_envelope.attack = 0.5;
        handle_update(&mut state, &update);

        // Values should still be applied
        let inst = state.instruments.instrument(id).unwrap();
        assert!((inst.modulation.lfo.rate - 10.0).abs() < f32::EPSILON);
        assert!((inst.modulation.amp_envelope.attack - 0.5).abs() < f32::EPSILON);

        // But no automation lanes created
        assert!(state
            .session
            .automation
            .lane_for_target(&AutomationTarget::lfo_rate(id))
            .is_none());
        assert!(state
            .session
            .automation
            .lane_for_target(&AutomationTarget::attack(id))
            .is_none());
    }

    #[test]
    fn handle_update_no_automation_for_unchanged_params() {
        let (mut state, id) = setup_with_instrument();
        state.recording.automation_recording = true;
        state.session.piano_roll.playing = true;
        state.audio.playing = true;

        // Send update with same values — no automation should be recorded
        let update = default_update(&state, id);
        handle_update(&mut state, &update);

        assert!(state.session.automation.lanes.is_empty());
    }
}
