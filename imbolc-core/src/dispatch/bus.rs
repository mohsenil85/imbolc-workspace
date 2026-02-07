use crate::action::{BusAction, DispatchResult};
use crate::audio::AudioHandle;
use crate::state::{AppState, OutputTarget};

/// Dispatch bus management actions
pub fn dispatch_bus(action: &BusAction, state: &mut AppState, _audio: &mut AudioHandle) -> DispatchResult {
    let mut result = DispatchResult::none();

    match action {
        BusAction::Add => {
            if let Some(_new_id) = state.session.add_bus() {
                // Sync all instruments with the new bus
                let bus_ids: Vec<u8> = state.session.bus_ids().collect();
                for inst in &mut state.instruments.instruments {
                    inst.sync_sends_with_buses(&bus_ids);
                }
                result.audio_dirty.routing = true;
                result.audio_dirty.session = true;
            }
        }

        BusAction::Remove(bus_id) => {
            let bus_id = *bus_id;

            // Check if bus exists
            if state.session.bus(bus_id).is_none() {
                return result;
            }

            // Reset instruments that output to this bus
            for inst in &mut state.instruments.instruments {
                if inst.output_target == OutputTarget::Bus(bus_id) {
                    inst.output_target = OutputTarget::Master;
                }
                // Disable sends to this bus
                inst.disable_send_for_bus(bus_id);
            }

            // Remove automation lanes for this bus
            state.session.automation.remove_lanes_for_bus(bus_id);

            // Remove the bus
            state.session.remove_bus(bus_id);

            // Update mixer selection if it was pointing to the removed bus
            if let crate::state::MixerSelection::Bus(id) = state.session.mixer.selection {
                if id == bus_id {
                    // Select first remaining bus, or Master if none
                    let first_bus = state.session.bus_ids().next();
                    state.session.mixer.selection = first_bus
                        .map(crate::state::MixerSelection::Bus)
                        .unwrap_or(crate::state::MixerSelection::Master);
                }
            }

            result.audio_dirty.routing = true;
            result.audio_dirty.session = true;
        }

        BusAction::Rename(bus_id, name) => {
            if let Some(bus) = state.session.bus_mut(*bus_id) {
                bus.name = name.clone();
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SourceType;
    use crate::state::automation::AutomationTarget;

    fn setup() -> (AppState, AudioHandle) {
        let state = AppState::new();
        let audio = AudioHandle::new();
        (state, audio)
    }

    #[test]
    fn add_bus() {
        let (mut state, mut audio) = setup();
        let initial_count = state.session.mixer.buses.len();

        dispatch_bus(&BusAction::Add, &mut state, &mut audio);

        assert_eq!(state.session.mixer.buses.len(), initial_count + 1);
    }

    #[test]
    fn add_bus_syncs_instrument_sends() {
        let (mut state, mut audio) = setup();
        state.add_instrument(SourceType::Saw);
        let initial_sends = state.instruments.instruments[0].sends.len();

        dispatch_bus(&BusAction::Add, &mut state, &mut audio);

        assert_eq!(state.instruments.instruments[0].sends.len(), initial_sends + 1);
    }

    #[test]
    fn remove_bus_resets_instrument_output() {
        let (mut state, mut audio) = setup();
        state.add_instrument(SourceType::Saw);
        state.instruments.instruments[0].output_target = OutputTarget::Bus(3);

        dispatch_bus(&BusAction::Remove(3), &mut state, &mut audio);

        assert_eq!(state.instruments.instruments[0].output_target, OutputTarget::Master);
    }

    #[test]
    fn remove_bus_disables_sends() {
        let (mut state, mut audio) = setup();
        state.add_instrument(SourceType::Saw);
        // Enable send to bus 3
        if let Some(send) = state.instruments.instruments[0].sends.iter_mut().find(|s| s.bus_id == 3) {
            send.enabled = true;
            send.level = 0.5;
        }

        dispatch_bus(&BusAction::Remove(3), &mut state, &mut audio);

        // Send should be disabled but still exist
        let send = state.instruments.instruments[0].sends.iter().find(|s| s.bus_id == 3);
        assert!(send.is_some());
        assert!(!send.unwrap().enabled);
    }

    #[test]
    fn remove_bus_clears_automation() {
        let (mut state, mut audio) = setup();
        state.session.automation.add_lane(AutomationTarget::bus_level(3));
        assert!(!state.session.automation.lanes.is_empty());

        dispatch_bus(&BusAction::Remove(3), &mut state, &mut audio);

        assert!(state.session.automation.lanes.is_empty());
    }

    #[test]
    fn rename_bus() {
        let (mut state, mut audio) = setup();

        dispatch_bus(&BusAction::Rename(1, "Drums".to_string()), &mut state, &mut audio);

        assert_eq!(state.session.bus(1).unwrap().name, "Drums");
    }
}
