//! Detail view for selected clips (piano roll or waveform).

use dioxus::prelude::*;

use crate::state::SharedState;

/// Detail view that shows piano roll or waveform for selected clip.
#[component]
pub fn DetailView() -> Element {
    let state = use_context::<Signal<SharedState>>();

    // Check if we have a selected clip
    let _has_selection = {
        let s = state.read();
        // For now, just check if there are any placements
        !s.app.session.arrangement.placements.is_empty()
    };

    rsx! {
        div { class: "detail-view",
            div { class: "detail-placeholder",
                "Select a clip to view details"
            }
        }
    }
}
