//! Track list sidebar component.

use dioxus::prelude::*;

use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;
use imbolc_types::{InstrumentAction, SourceType};

/// Sidebar showing all instruments/tracks.
#[component]
pub fn TrackList() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let mut dispatch = use_dispatch();

    let (instruments, selected_idx) = {
        let s = state.read();
        let instruments: Vec<_> = s
            .app
            .instruments
            .instruments
            .iter()
            .map(|i| (i.id, i.name.clone(), i.mixer.mute, i.mixer.solo))
            .collect();
        let selected_idx = s.app.instruments.selected;
        (instruments, selected_idx)
    };

    rsx! {
        div { class: "track-list",
            h3 { "Instruments" }
            for (idx, (_id, name, mute, solo)) in instruments.iter().enumerate() {
                div {
                    class: if Some(idx) == selected_idx { "track-item selected" } else { "track-item" },
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::Instrument(InstrumentAction::Select(idx)));
                    },
                    span { class: "track-name", "{name}" }
                    span { class: "track-status",
                        if *mute { "M" } else { "" }
                        if *solo { "S" } else { "" }
                    }
                }
            }
            button {
                class: "add-track-btn",
                onclick: move |_| {
                    dispatch.dispatch_action(imbolc_types::Action::Instrument(InstrumentAction::Add(SourceType::Saw)));
                },
                "+ Add Instrument"
            }
        }
    }
}
