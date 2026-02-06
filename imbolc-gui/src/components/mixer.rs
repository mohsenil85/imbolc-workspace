//! Mixer component with channel strips.

use dioxus::prelude::*;

use crate::components::common::{Meter, Slider};
use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;
use imbolc_types::{MixerAction, SessionAction};

/// Mixer panel with channel strips for each instrument.
#[component]
pub fn Mixer() -> Element {
    let state = use_context::<Signal<SharedState>>();

    let (instruments, master_level, master_mute) = {
        let s = state.read();
        let instruments: Vec<_> = s
            .app
            .instruments
            .instruments
            .iter()
            .map(|i| (i.id, i.name.clone(), i.level, i.pan, i.mute, i.solo))
            .collect();
        let master_level = s.app.session.mixer.master_level;
        let master_mute = s.app.session.mixer.master_mute;
        (instruments, master_level, master_mute)
    };

    rsx! {
        div { class: "mixer",
            div { class: "mixer-channels",
                for (id, name, level, pan, mute, solo) in instruments {
                    MixerChannel {
                        key: "{id}",
                        id,
                        name,
                        level,
                        pan,
                        mute,
                        solo,
                    }
                }
            }
            div { class: "mixer-master",
                MasterChannel { level: master_level, mute: master_mute }
            }
        }
    }
}

#[component]
fn MixerChannel(
    id: u32,
    name: String,
    level: f32,
    pan: f32,
    mute: bool,
    solo: bool,
) -> Element {
    let mut dispatch = use_dispatch();
    let state = use_context::<Signal<SharedState>>();

    // Find the index of this instrument for selection
    let idx = {
        let s = state.read();
        s.app
            .instruments
            .instruments
            .iter()
            .position(|i| i.id == id)
            .unwrap_or(0)
    };

    rsx! {
        div { class: "mixer-channel",
            div { class: "channel-name", "{name}" }
            Meter { level }
            Slider {
                value: level,
                min: 0.0,
                max: 1.0,
                vertical: true,
                onchange: move |new_level: f32| {
                    // First select this instrument, then adjust level
                    dispatch.dispatch_action(imbolc_types::Action::Instrument(imbolc_types::InstrumentAction::Select(idx)));
                    let delta = new_level - level;
                    dispatch.dispatch_action(imbolc_types::Action::Mixer(MixerAction::AdjustLevel(delta)));
                }
            }
            Slider {
                value: (pan + 1.0) / 2.0,
                min: 0.0,
                max: 1.0,
                vertical: false,
                onchange: move |new_pan_norm: f32| {
                    let new_pan = new_pan_norm * 2.0 - 1.0;
                    let delta = new_pan - pan;
                    dispatch.dispatch_action(imbolc_types::Action::Instrument(imbolc_types::InstrumentAction::Select(idx)));
                    dispatch.dispatch_action(imbolc_types::Action::Mixer(MixerAction::AdjustPan(delta)));
                }
            }
            div { class: "channel-buttons",
                button {
                    class: if mute { "channel-btn active" } else { "channel-btn" },
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::Instrument(imbolc_types::InstrumentAction::Select(idx)));
                        dispatch.dispatch_action(imbolc_types::Action::Mixer(MixerAction::ToggleMute));
                    },
                    "M"
                }
                button {
                    class: if solo { "channel-btn active" } else { "channel-btn" },
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::Instrument(imbolc_types::InstrumentAction::Select(idx)));
                        dispatch.dispatch_action(imbolc_types::Action::Mixer(MixerAction::ToggleSolo));
                    },
                    "S"
                }
            }
        }
    }
}

#[component]
fn MasterChannel(level: f32, mute: bool) -> Element {
    let mut dispatch = use_dispatch();

    rsx! {
        div { class: "mixer-channel master",
            div { class: "channel-name", "Master" }
            Meter { level }
            Slider {
                value: level,
                min: 0.0,
                max: 1.0,
                vertical: true,
                onchange: move |_new_level: f32| {
                    // Master level adjustment would need a separate action
                    log::debug!("Master level change requested");
                }
            }
            div { class: "channel-buttons",
                button {
                    class: if mute { "channel-btn active" } else { "channel-btn" },
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::Session(SessionAction::ToggleMasterMute));
                    },
                    "M"
                }
            }
        }
    }
}
