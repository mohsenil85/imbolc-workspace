use crate::action::{DispatchResult, MixerAction};
use crate::audio::AudioHandle;
use crate::dispatch::helpers::{apply_bus_update, apply_layer_group_update, maybe_record_automation};
use crate::state::automation::AutomationTarget;
use crate::state::{AppState, MixerSelection};

pub(super) fn dispatch_mixer(
    action: &MixerAction,
    state: &mut AppState,
    audio: &mut AudioHandle,
) -> DispatchResult {
    let mut result = DispatchResult::none();
    match action {
        MixerAction::Move(delta) => {
            state.mixer_move(*delta);
            if let MixerSelection::Instrument(idx) = state.session.mixer.selection {
                state.instruments.selected = Some(idx);
            }
        }
        MixerAction::Jump(direction) => {
            state.mixer_jump(*direction);
            if let MixerSelection::Instrument(idx) = state.session.mixer.selection {
                state.instruments.selected = Some(idx);
            }
        }
        MixerAction::SelectAt(selection) => {
            state.session.mixer.selection = *selection;
            if let MixerSelection::Instrument(idx) = *selection {
                state.instruments.selected = Some(idx);
            }
        }
        MixerAction::AdjustLevel(delta) => {
            let mut bus_update: Option<(u8, f32, bool, f32)> = None;
            let mut group_update: Option<(u32, f32, bool, f32)> = None;
            let mut record_target: Option<(AutomationTarget, f32)> = None;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        instrument.level = (instrument.level + delta).clamp(0.0, 1.0);
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.mixer_params = true;
                        if state.recording.automation_recording && state.session.piano_roll.playing {
                            record_target = Some((
                                AutomationTarget::level(instrument.id),
                                instrument.level,
                            ));
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        gm.level = (gm.level + delta).clamp(0.0, 1.0);
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                        let mute = state.session.mixer.effective_layer_group_mute(gm);
                        group_update = Some((group_id, gm.level, mute, gm.pan));
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = state.session.bus_mut(id) {
                        bus.level = (bus.level + delta).clamp(0.0, 1.0);
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(bus) = state.session.bus(id) {
                        let mute = state.session.effective_bus_mute(bus);
                        bus_update = Some((id, bus.level, mute, bus.pan));
                        if state.recording.automation_recording && state.session.piano_roll.playing {
                            record_target = Some((
                                AutomationTarget::bus_level(id),
                                bus.level,
                            ));
                        }
                    }
                }
                MixerSelection::Master => {
                    state.session.mixer.master_level = (state.session.mixer.master_level + delta).clamp(0.0, 1.0);
                    result.audio_dirty.session = true;
                    result.audio_dirty.mixer_params = true;
                }
            }
            apply_bus_update(audio, bus_update);
            apply_layer_group_update(audio, group_update);
            if let Some((target, value)) = record_target {
                maybe_record_automation(state, &mut result, target, value);
            }
        }
        MixerAction::ToggleMute => {
            let mut bus_update: Option<(u8, f32, bool, f32)> = None;
            let mut group_update: Option<(u32, f32, bool, f32)> = None;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        instrument.mute = !instrument.mute;
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.mixer_params = true;
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        gm.mute = !gm.mute;
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                        let mute = state.session.mixer.effective_layer_group_mute(gm);
                        group_update = Some((group_id, gm.level, mute, gm.pan));
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = state.session.bus_mut(id) {
                        bus.mute = !bus.mute;
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(bus) = state.session.bus(id) {
                        let mute = state.session.effective_bus_mute(bus);
                        bus_update = Some((id, bus.level, mute, bus.pan));
                    }
                }
                MixerSelection::Master => {
                    state.session.mixer.master_mute = !state.session.mixer.master_mute;
                    result.audio_dirty.session = true;
                    result.audio_dirty.mixer_params = true;
                }
            }
            apply_bus_update(audio, bus_update);
            apply_layer_group_update(audio, group_update);
        }
        MixerAction::ToggleSolo => {
            let mut bus_updates: Vec<(u8, f32, bool, f32)> = Vec::new();
            let mut group_updates: Vec<(u32, f32, bool, f32)> = Vec::new();
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        instrument.solo = !instrument.solo;
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.mixer_params = true;
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        gm.solo = !gm.solo;
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                }
                MixerSelection::Bus(id) => {
                    if let Some(bus) = state.session.bus_mut(id) {
                        bus.solo = !bus.solo;
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                }
                MixerSelection::Master => {}
            }
            for bus in &state.session.mixer.buses {
                let mute = state.session.effective_bus_mute(bus);
                bus_updates.push((bus.id, bus.level, mute, bus.pan));
            }
            for update in bus_updates {
                apply_bus_update(audio, Some(update));
            }
            for gm in &state.session.mixer.layer_group_mixers {
                let mute = state.session.mixer.effective_layer_group_mute(gm);
                group_updates.push((gm.group_id, gm.level, mute, gm.pan));
            }
            for update in group_updates {
                apply_layer_group_update(audio, Some(update));
            }
        }
        MixerAction::CycleSection => {
            state.session.mixer_cycle_section();
            // When cycling back to Instrument section, sync to global selection
            if let MixerSelection::Instrument(_) = state.session.mixer.selection {
                if let Some(idx) = state.instruments.selected {
                    state.session.mixer.selection = MixerSelection::Instrument(idx);
                }
            }
        }
        MixerAction::CycleOutput => {
            state.mixer_cycle_output();
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(inst) = state.instruments.instruments.get(idx) {
                        result.audio_dirty.routing_instrument = Some(inst.id);
                    }
                }
                MixerSelection::LayerGroup(_) => {
                    result.audio_dirty.routing = true;
                }
                _ => {}
            }
        }
        MixerAction::CycleOutputReverse => {
            state.mixer_cycle_output_reverse();
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(inst) = state.instruments.instruments.get(idx) {
                        result.audio_dirty.routing_instrument = Some(inst.id);
                    }
                }
                MixerSelection::LayerGroup(_) => {
                    result.audio_dirty.routing = true;
                }
                _ => {}
            }
        }
        MixerAction::AdjustSend(bus_id, delta) => {
            let bus_id = *bus_id;
            let delta = *delta;
            let mut record_target: Option<(AutomationTarget, f32)> = None;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        if let Some((send_idx, send)) = instrument.sends.iter_mut().enumerate().find(|(_, s)| s.bus_id == bus_id) {
                            send.level = (send.level + delta).clamp(0.0, 1.0);
                            result.audio_dirty.instruments = true;
                            if state.recording.automation_recording && state.session.piano_roll.playing {
                                record_target = Some((
                                    AutomationTarget::send_level(instrument.id, send_idx),
                                    send.level,
                                ));
                            }
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.level = (send.level + delta).clamp(0.0, 1.0);
                            result.audio_dirty.session = true;
                            result.audio_dirty.routing = true;
                        }
                    }
                }
                _ => {}
            }
            if let Some((target, value)) = record_target {
                maybe_record_automation(state, &mut result, target, value);
            }
        }
        MixerAction::AdjustPan(delta) => {
            let mut record_target: Option<(AutomationTarget, f32)> = None;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        instrument.pan = (instrument.pan + delta).clamp(-1.0, 1.0);
                        result.audio_dirty.instruments = true;
                        result.audio_dirty.mixer_params = true;
                        if state.recording.automation_recording && state.session.piano_roll.playing {
                            let target = AutomationTarget::pan(instrument.id);
                            record_target = Some((target.clone(), target.normalize_value(instrument.pan)));
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    let mut group_update: Option<(u32, f32, bool, f32)> = None;
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        gm.pan = (gm.pan + delta).clamp(-1.0, 1.0);
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(gm) = state.session.mixer.layer_group_mixer(group_id) {
                        let mute = state.session.mixer.effective_layer_group_mute(gm);
                        group_update = Some((group_id, gm.level, mute, gm.pan));
                    }
                    apply_layer_group_update(audio, group_update);
                }
                MixerSelection::Bus(id) => {
                    let mut bus_update: Option<(u8, f32, bool, f32)> = None;
                    if let Some(bus) = state.session.bus_mut(id) {
                        bus.pan = (bus.pan + delta).clamp(-1.0, 1.0);
                        result.audio_dirty.session = true;
                        result.audio_dirty.mixer_params = true;
                    }
                    if let Some(bus) = state.session.bus(id) {
                        let mute = state.session.effective_bus_mute(bus);
                        bus_update = Some((id, bus.level, mute, bus.pan));
                    }
                    apply_bus_update(audio, bus_update);
                }
                MixerSelection::Master => {}
            }
            if let Some((target, value)) = record_target {
                maybe_record_automation(state, &mut result, target, value);
            }
        }
        MixerAction::ToggleSend(bus_id) => {
            let bus_id = *bus_id;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        if let Some(send) = instrument.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.enabled = !send.enabled;
                            if send.enabled && send.level <= 0.0 {
                                send.level = 0.5;
                            }
                            result.audio_dirty.instruments = true;
                            result.audio_dirty.routing_instrument = Some(instrument.id);
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.enabled = !send.enabled;
                            if send.enabled && send.level <= 0.0 {
                                send.level = 0.5;
                            }
                            result.audio_dirty.session = true;
                            result.audio_dirty.routing = true;
                        }
                    }
                }
                _ => {}
            }
        }
        MixerAction::CycleSendTapPoint(bus_id) => {
            let bus_id = *bus_id;
            match state.session.mixer.selection {
                MixerSelection::Instrument(idx) => {
                    if let Some(instrument) = state.instruments.instruments.get_mut(idx) {
                        if let Some(send) = instrument.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.tap_point = send.tap_point.cycle();
                            result.audio_dirty.instruments = true;
                            result.audio_dirty.routing_instrument = Some(instrument.id);
                        }
                    }
                }
                MixerSelection::LayerGroup(group_id) => {
                    if let Some(gm) = state.session.mixer.layer_group_mixer_mut(group_id) {
                        if let Some(send) = gm.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.tap_point = send.tap_point.cycle();
                            result.audio_dirty.session = true;
                            result.audio_dirty.routing = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioHandle;

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
        assert!((state.instruments.instruments[0].level - 1.0).abs() < f32::EPSILON);

        dispatch_mixer(&MixerAction::AdjustLevel(-5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].level - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_level_bus_clamps_and_sets_dirty() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Bus(1);
        let result = dispatch_mixer(&MixerAction::AdjustLevel(2.0), &mut state, &mut audio);
        assert!(result.audio_dirty.session);
        assert!((state.session.bus(1).unwrap().level - 1.0).abs() < f32::EPSILON);
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
        assert!(!state.instruments.instruments[0].mute);
        let result = dispatch_mixer(&MixerAction::ToggleMute, &mut state, &mut audio);
        assert!(state.instruments.instruments[0].mute);
        assert!(result.audio_dirty.instruments);
    }

    #[test]
    fn toggle_mute_bus() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Bus(1);
        assert!(!state.session.bus(1).unwrap().mute);
        dispatch_mixer(&MixerAction::ToggleMute, &mut state, &mut audio);
        assert!(state.session.bus(1).unwrap().mute);
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
        assert!(state.instruments.instruments[0].solo);
        assert!(result.audio_dirty.instruments);
    }

    #[test]
    fn toggle_solo_bus() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Bus(1);
        dispatch_mixer(&MixerAction::ToggleSolo, &mut state, &mut audio);
        assert!(state.session.bus(1).unwrap().solo);
    }

    #[test]
    fn adjust_pan_clamps() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Instrument(0);
        dispatch_mixer(&MixerAction::AdjustPan(5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].pan - 1.0).abs() < f32::EPSILON);
        dispatch_mixer(&MixerAction::AdjustPan(-5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].pan - (-1.0)).abs() < f32::EPSILON);

        state.session.mixer.selection = MixerSelection::Bus(1);
        dispatch_mixer(&MixerAction::AdjustPan(5.0), &mut state, &mut audio);
        assert!((state.session.bus(1).unwrap().pan - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cycle_section() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Instrument(0);
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(matches!(state.session.mixer.selection, MixerSelection::Bus(1)));
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(matches!(state.session.mixer.selection, MixerSelection::Master));
        dispatch_mixer(&MixerAction::CycleSection, &mut state, &mut audio);
        assert!(matches!(state.session.mixer.selection, MixerSelection::Instrument(_)));
    }

    #[test]
    fn toggle_send_auto_sets_level() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Instrument(0);
        // Send starts disabled with level 0.0
        assert!(!state.instruments.instruments[0].sends[0].enabled);
        assert!((state.instruments.instruments[0].sends[0].level - 0.0).abs() < f32::EPSILON);

        dispatch_mixer(&MixerAction::ToggleSend(1), &mut state, &mut audio);
        assert!(state.instruments.instruments[0].sends[0].enabled);
        assert!((state.instruments.instruments[0].sends[0].level - 0.5).abs() < f32::EPSILON);

        dispatch_mixer(&MixerAction::ToggleSend(1), &mut state, &mut audio);
        assert!(!state.instruments.instruments[0].sends[0].enabled);
    }

    #[test]
    fn adjust_send_clamps() {
        let (mut state, mut audio) = setup();
        state.session.mixer.selection = MixerSelection::Instrument(0);
        state.instruments.instruments[0].sends[0].enabled = true;
        state.instruments.instruments[0].sends[0].level = 0.5;
        dispatch_mixer(&MixerAction::AdjustSend(1, 2.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].sends[0].level - 1.0).abs() < f32::EPSILON);
        dispatch_mixer(&MixerAction::AdjustSend(1, -5.0), &mut state, &mut audio);
        assert!((state.instruments.instruments[0].sends[0].level - 0.0).abs() < f32::EPSILON);
    }
}
