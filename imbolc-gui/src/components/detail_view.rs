//! Detail view for selected clips (piano roll or waveform).

use dioxus::prelude::*;

use crate::components::{PianoRollView, WaveformView};
use crate::state::SharedState;

/// Detail view that shows piano roll or waveform for selected clip.
#[component]
pub fn DetailView() -> Element {
    let state = use_context::<Signal<SharedState>>();

    // Get selected placement and clip info
    let selection_info = {
        let s = state.read();
        let arrangement = &s.app.session.arrangement;

        if let Some(placement_idx) = arrangement.selected_placement {
            if let Some(placement) = arrangement.placements.get(placement_idx) {
                if let Some(clip) = arrangement.clip(placement.clip_id) {
                    Some((
                        clip.id,
                        clip.instrument_id,
                        clip.length_ticks,
                        clip.notes.is_empty(), // is_audio_clip: if no notes, it might be audio
                        clip.name.clone(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    match selection_info {
        Some((clip_id, instrument_id, length_ticks, is_audio_clip, clip_name)) => {
            if is_audio_clip {
                // Audio clip - show waveform view
                rsx! {
                    div { class: "detail-view",
                        div { class: "detail-header",
                            span { class: "detail-title", "Waveform: {clip_name}" }
                        }
                        WaveformView {
                            clip_id: clip_id,
                            _start_tick: 0,
                            _length_ticks: length_ticks,
                        }
                    }
                }
            } else {
                // MIDI clip - show piano roll
                rsx! {
                    div { class: "detail-view",
                        div { class: "detail-header",
                            span { class: "detail-title", "Piano Roll: {clip_name}" }
                        }
                        PianoRollView {
                            clip_id: clip_id,
                            instrument_id: instrument_id,
                            clip_length: length_ticks,
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "detail-view",
                    div { class: "detail-placeholder",
                        "Select a clip to view details"
                    }
                }
            }
        }
    }
}
