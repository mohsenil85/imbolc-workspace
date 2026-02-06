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

    let (is_playing, bpm, time_sig, save_in_progress, load_in_progress, last_io_error) = {
        let s = state.read();
        (
            s.app.session.piano_roll.playing,
            s.app.session.bpm,
            s.app.session.time_signature,
            s.app.io.save_in_progress,
            s.app.io.load_in_progress,
            s.app.io.last_io_error.clone(),
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
            // I/O Status indicator
            div { class: "transport-status",
                if save_in_progress {
                    span { class: "status-indicator saving", "Saving..." }
                } else if load_in_progress {
                    span { class: "status-indicator loading", "Loading..." }
                } else if let Some(ref error) = last_io_error {
                    span { class: "status-indicator error", "Error: {error}" }
                }
            }
        }
    }
}
