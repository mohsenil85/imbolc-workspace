use crate::action::{AudioEffect, DispatchResult, MixerAction};
use crate::dispatch::helpers::{
    apply_bus_update, apply_layer_group_update, maybe_record_automation,
};
use crate::state::automation::AutomationTarget;
use crate::state::AppState;
use imbolc_audio::AudioHandle;
use imbolc_types::{DomainAction, MixerSelection};

pub(super) fn dispatch_mixer(
    action: &MixerAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    // Capture pre-mutation state for orchestration
    let selection = state.session.mixer.selection;

    // Delegate pure state mutation to the shared reducer
    imbolc_types::reduce::reduce_action(
        &DomainAction::Mixer(action.clone()),
        &mut state.instruments,
        &mut state.session,
    );

    // Orchestration: AudioEffects, automation recording, real-time audio updates
    let mut result = DispatchResult::none();
    match action {
        // Navigation-only: no audio effects needed
        MixerAction::Move(_)
        | MixerAction::Jump(_)
        | MixerAction::SelectAt(_)
        | MixerAction::CycleSection => {}

        MixerAction::AdjustLevel(_delta) => match selection {
            MixerSelection::Instrument(idx) => {
                result.audio_effects.push(AudioEffect::RebuildInstruments);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(instrument) = state.instruments.instruments.get(idx) {
                    if state.recording.automation_recording && state.audio.playing {
                        maybe_record_automation(
                            state,
                            &mut result,
                            AutomationTarget::level(instrument.id),
                            instrument.mixer.level,
                        );
                    }
                }
            }
            MixerSelection::LayerGroup(group_id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                    let mute = state.session.mixer.effective_layer_group_mute(gm);
                    apply_layer_group_update(audio, Some((group_id, gm.level, mute, gm.pan)));
                }
            }
            MixerSelection::Bus(id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(bus) = state.session.bus(id) {
                    let mute = state.session.effective_bus_mute(bus);
                    apply_bus_update(audio, Some((id, bus.level, mute, bus.pan)));
                    if state.recording.automation_recording && state.audio.playing {
                        maybe_record_automation(
                            state,
                            &mut result,
                            AutomationTarget::bus_level(id),
                            bus.level,
                        );
                    }
                }
            }
            MixerSelection::Master => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
            }
        },

        MixerAction::ToggleMute => match selection {
            MixerSelection::Instrument(_) => {
                result.audio_effects.push(AudioEffect::RebuildInstruments);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
            }
            MixerSelection::LayerGroup(group_id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                    let mute = state.session.mixer.effective_layer_group_mute(gm);
                    apply_layer_group_update(audio, Some((group_id, gm.level, mute, gm.pan)));
                }
            }
            MixerSelection::Bus(id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(bus) = state.session.bus(id) {
                    let mute = state.session.effective_bus_mute(bus);
                    apply_bus_update(audio, Some((id, bus.level, mute, bus.pan)));
                }
            }
            MixerSelection::Master => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
            }
        },

        MixerAction::ToggleSolo => {
            match selection {
                MixerSelection::Instrument(_) => {
                    result.audio_effects.push(AudioEffect::RebuildInstruments);
                    result.audio_effects.push(AudioEffect::UpdateMixerParams);
                }
                MixerSelection::LayerGroup(_) => {
                    result.audio_effects.push(AudioEffect::RebuildSession);
                    result.audio_effects.push(AudioEffect::UpdateMixerParams);
                }
                MixerSelection::Bus(_) => {
                    result.audio_effects.push(AudioEffect::RebuildSession);
                    result.audio_effects.push(AudioEffect::UpdateMixerParams);
                }
                MixerSelection::Master => {}
            }
            // Solo affects all buses/groups — update all
            for bus in &state.session.mixer.buses {
                let mute = state.session.effective_bus_mute(bus);
                apply_bus_update(audio, Some((bus.id, bus.level, mute, bus.pan)));
            }
            for gm in &state.session.mixer.layer_group_mixers {
                let mute = state.session.mixer.effective_layer_group_mute(gm);
                apply_layer_group_update(audio, Some((gm.group_id, gm.level, mute, gm.pan)));
            }
        }

        MixerAction::CycleOutput | MixerAction::CycleOutputReverse => match selection {
            MixerSelection::Instrument(idx) => {
                if let Some(inst) = state.instruments.instruments.get(idx) {
                    result
                        .audio_effects
                        .push(AudioEffect::RebuildRoutingForInstrument(inst.id));
                }
            }
            MixerSelection::LayerGroup(_) => {
                result.audio_effects.push(AudioEffect::RebuildRouting);
            }
            _ => {}
        },

        MixerAction::AdjustSend(bus_id, _delta) => {
            let bus_id = *bus_id;
            match selection {
                MixerSelection::Instrument(idx) => {
                    result.audio_effects.push(AudioEffect::RebuildInstruments);
                    if let Some(instrument) = state.instruments.instruments.get(idx) {
                        if let Some(send) = instrument.mixer.sends.get(&bus_id) {
                            if state.recording.automation_recording && state.audio.playing {
                                maybe_record_automation(
                                    state,
                                    &mut result,
                                    AutomationTarget::send_level(instrument.id, bus_id),
                                    send.level,
                                );
                            }
                        }
                    }
                }
                MixerSelection::LayerGroup(_) => {
                    result.audio_effects.push(AudioEffect::RebuildSession);
                    result.audio_effects.push(AudioEffect::RebuildRouting);
                }
                _ => {}
            }
        }

        MixerAction::ToggleSend(_bus_id) => match selection {
            MixerSelection::Instrument(idx) => {
                if let Some(instrument) = state.instruments.instruments.get(idx) {
                    result.audio_effects.push(AudioEffect::RebuildInstruments);
                    result
                        .audio_effects
                        .push(AudioEffect::RebuildRoutingForInstrument(instrument.id));
                }
            }
            MixerSelection::LayerGroup(_) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::RebuildRouting);
            }
            _ => {}
        },

        MixerAction::CycleSendTapPoint(_bus_id) => match selection {
            MixerSelection::Instrument(idx) => {
                if let Some(instrument) = state.instruments.instruments.get(idx) {
                    result.audio_effects.push(AudioEffect::RebuildInstruments);
                    result
                        .audio_effects
                        .push(AudioEffect::RebuildRoutingForInstrument(instrument.id));
                }
            }
            MixerSelection::LayerGroup(_) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::RebuildRouting);
            }
            _ => {}
        },

        MixerAction::AdjustPan(_delta) => match selection {
            MixerSelection::Instrument(idx) => {
                result.audio_effects.push(AudioEffect::RebuildInstruments);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(instrument) = state.instruments.instruments.get(idx) {
                    if state.recording.automation_recording && state.audio.playing {
                        let target = AutomationTarget::pan(instrument.id);
                        let normalized = target.normalize_value(instrument.mixer.pan);
                        maybe_record_automation(state, &mut result, target, normalized);
                    }
                }
            }
            MixerSelection::LayerGroup(group_id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                    let mute = state.session.mixer.effective_layer_group_mute(gm);
                    apply_layer_group_update(audio, Some((group_id, gm.level, mute, gm.pan)));
                }
            }
            MixerSelection::Bus(id) => {
                result.audio_effects.push(AudioEffect::RebuildSession);
                result.audio_effects.push(AudioEffect::UpdateMixerParams);
                if let Some(bus) = state.session.bus(id) {
                    let mute = state.session.effective_bus_mute(bus);
                    apply_bus_update(audio, Some((id, bus.level, mute, bus.pan)));
                }
            }
            MixerSelection::Master => {}
        },
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use imbolc_audio::AudioHandle;
    use imbolc_types::BusId;

    fn setup() -> (AppState, AudioHandle) {
        let mut state = AppState::new();
        state.add_instrument(crate::state::SourceType::Saw);
        state.add_instrument(crate::state::SourceType::Sin);
        (state, AudioHandle::new())
    }

    #[test]
    fn adjust_level_instrument_clamps() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        dispatch_mixer(&MixerAction::AdjustLevel(2.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].mixer.level - 1.0).abs() < f32::EPSILON);

        dispatch_mixer(&MixerAction::AdjustLevel(-5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].mixer.level - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_level_bus_clamps_and_sets_dirty() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
        let result = dispatch_mixer(&MixerAction::AdjustLevel(2.0), &mut state, &mut audio);
        assert!(result.audio_effects.contains(&AudioEffect::RebuildSession));
        assert!((state.session.bus(BusId::new(1)).unwrap().level - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_level_master_clamps() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Master;
        dispatch_mixer(&MixerAction::AdjustLevel(2.0), &mut state, &mut audio);
        assert!((state.session.mixer.master_level - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn toggle_mute_instrument() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        assert!(!state.instruments.instruments[0].mixer.mute);
        let result = dispatch_mixer(&MixerAction::ToggleMute, &mut state, &mut audio);
        assert!(state.instruments.instruments[0].mixer.mute);
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildInstruments));
    }

    #[test]
    fn toggle_mute_bus() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
        assert!(!state.session.bus(BusId::new(1)).unwrap().mute);
        dispatch_mixer(&MixerAction::ToggleMute, &mut state, &mut audio);
        assert!(state.session.bus(BusId::new(1)).unwrap().mute);
    }

    #[test]
    fn toggle_mute_master() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Master;
        assert!(!state.session.mixer.master_mute);
        dispatch_mixer(&MixerAction::ToggleMute, &mut state, &mut audio);
        assert!(state.session.mixer.master_mute);
    }

    #[test]
    fn toggle_solo_instrument() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        let result = dispatch_mixer(&MixerAction::ToggleSolo, &mut state, &mut audio);
        assert!(state.instruments.instruments[0].mixer.solo);
        assert!(result
            .audio_effects
            .contains(&AudioEffect::RebuildInstruments));
    }

    #[test]
    fn toggle_solo_bus() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
        dispatch_mixer(&MixerAction::ToggleSolo, &mut state, &mut audio);
        assert!(state.session.bus(BusId::new(1)).unwrap().solo);
    }

    #[test]
    fn adjust_pan_clamps() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        dispatch_mixer(&MixerAction::AdjustPan(5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].mixer.pan - 1.0).abs() < f32::EPSILON);
        dispatch_mixer(&MixerAction::AdjustPan(-5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].mixer.pan - (-1.0)).abs() < f32::EPSILON);

        state.session.mixer.selection = MixerSelection::Bus(BusId::new(1));
        dispatch_mixer(&MixerAction::AdjustPan(5.0), &mut state, &mut audio);
        assert!((state.session.bus(BusId::new(1)).unwrap().pan - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cycle_section() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(
            matches!(state.session.mixer.selection, MixerSelection::Bus(id) if id == BusId::new(1))
        );
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(matches!(
            state.session.mixer.selection,
            MixerSelection::Master
        ));
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(matches!(
            state.session.mixer.selection,
            MixerSelection::Instrument(_)
        ));
    }

    #[test]
    fn toggle_send_auto_sets_level() {
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        // Sends start empty (lazily created)
        assert!(state.instruments.instruments[0].mixer.sends.is_empty());

        dispatch_mixer(
            &MixerAction::ToggleSend(BusId::new(1)),
            &mut state,
            &mut audio,
        );
        let send = state.instruments.instruments[0]
            .mixer
            .sends
            .get(&BusId::new(1))
            .unwrap();
        assert!(send.enabled);
        assert!((send.level - 0.5).abs() < f32::EPSILON);

        dispatch_mixer(
            &MixerAction::ToggleSend(BusId::new(1)),
            &mut state,
            &mut audio,
        );
        assert!(
            !state.instruments.instruments[0]
                .mixer
                .sends
                .get(&BusId::new(1))
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn adjust_send_clamps() {
        use imbolc_types::MixerSend;
        let (mut state, mut audio) = setup();

        state.session.mixer.selection = MixerSelection::Instrument(0);
        state.instruments.instruments[0].mixer.sends.insert(
            BusId::new(1),
            MixerSend {
                bus_id: BusId::new(1),
                level: 0.5,
                enabled: true,
                tap_point: Default::default(),
            },
        );
        dispatch_mixer(
            &MixerAction::AdjustSend(BusId::new(1), 2.0),
            &mut state,
            &mut audio,
        );
        assert!(
            (state.instruments.instruments[0]
                .mixer
                .sends
                .get(&BusId::new(1))
                .unwrap()
                .level
                - 1.0)
                .abs()
                < f32::EPSILON
        );
        dispatch_mixer(
            &MixerAction::AdjustSend(BusId::new(1), -5.0),
            &mut state,
            &mut audio,
        );
        assert!(
            (state.instruments.instruments[0]
                .mixer
                .sends
                .get(&BusId::new(1))
                .unwrap()
                .level
                - 0.0)
                .abs()
                < f32::EPSILON
        );
    }
}
