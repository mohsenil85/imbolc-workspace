//! Waveform view for audio clip display.
//!
//! This is a placeholder for future waveform rendering functionality.

use dioxus::prelude::*;

/// Waveform view for displaying audio clips.
/// Currently a placeholder - full implementation pending.
#[component]
pub fn WaveformView(
    clip_id: u32,
    #[props(default = 0)] _start_tick: u32,
    #[props(default = 0)] _length_ticks: u32,
) -> Element {
    rsx! {
        div { class: "waveform-view",
            div { class: "waveform-placeholder",
                p { "Waveform View" }
                p { class: "waveform-info",
                    "Audio clip #{clip_id}"
                }
                p { class: "waveform-note",
                    "Waveform rendering not yet implemented."
                }
                p { class: "waveform-note",
                    "Audio clips are currently display-only in the arrangement view."
                }
            }
        }
    }
}
