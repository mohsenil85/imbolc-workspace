//! Transport bar component (play/stop/record, BPM, time signature).

use dioxus::prelude::*;

use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;
use imbolc_types::PianoRollAction;

/// Transport bar with playback controls and tempo display.
#[component]
pub fn Transport() -> Element {
    let state = use_context::<Signal<SharedState>>();
    let mut dispatch = use_dispatch();

    let (is_playing, bpm, time_sig) = {
        let s = state.read();
        (
            s.app.session.piano_roll.playing,
            s.app.session.bpm,
            s.app.session.time_signature,
        )
    };

    rsx! {
        div { class: "transport",
            div { class: "transport-controls",
                button {
                    class: "transport-btn",
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::PianoRoll(PianoRollAction::PlayStop));
                    },
                    if is_playing { "||" } else { ">" }
                }
                button {
                    class: "transport-btn",
                    onclick: move |_| {
                        dispatch.dispatch_action(imbolc_types::Action::Server(imbolc_types::ServerAction::Stop));
                    },
                    "[]"
                }
            }
            div { class: "transport-info",
                span { class: "bpm", "{bpm:.1} BPM" }
                span { class: "time-sig", "{time_sig.0}/{time_sig.1}" }
            }
        }
    }
}
