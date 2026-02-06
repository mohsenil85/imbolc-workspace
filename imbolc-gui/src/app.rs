//! Root application component and state initialization.

use dioxus::prelude::*;

use crate::components::{Arrangement, Mixer, TrackList, Transport};
use crate::state::SharedState;

const MAIN_CSS: &str = include_str!("styles/main.css");

/// Root application component.
#[component]
pub fn App() -> Element {
    // Initialize shared state with context provider
    let mut shared_state = use_context_provider(|| Signal::new(SharedState::new()));

    // Poll audio feedback at ~30fps using Dioxus spawn
    use_future(move || async move {
        loop {
            {
                let mut state = shared_state.write();
                state.poll_audio_feedback();
            }
            // Use async-std for sleeping
            async_std::task::sleep(std::time::Duration::from_millis(33)).await;
        }
    });

    rsx! {
        style { {MAIN_CSS} }
        div { class: "app",
            Transport {}
            div { class: "main-content",
                TrackList {}
                Arrangement {}
            }
            Mixer {}
        }
    }
}
