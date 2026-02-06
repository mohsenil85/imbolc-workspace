//! Arrangement view with timeline and clip rectangles.

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::state::SharedState;

/// Arrangement view showing timeline with clip rectangles.
#[component]
pub fn Arrangement() -> Element {
    let state = use_context::<Signal<SharedState>>();

    let (placements, playhead, instruments) = {
        let s = state.read();
        // Build a map of clip_id -> length_ticks
        let clip_lengths: HashMap<_, _> = s
            .app
            .session
            .arrangement
            .clips
            .iter()
            .map(|c| (c.id, c.length_ticks))
            .collect();
        let placements: Vec<_> = s
            .app
            .session
            .arrangement
            .placements
            .iter()
            .map(|p| {
                let length = p.length_override.unwrap_or_else(|| {
                    clip_lengths.get(&p.clip_id).copied().unwrap_or(480)
                });
                (p.id, p.instrument_id, p.clip_id, p.start_tick, length)
            })
            .collect();
        let playhead = s.app.audio.playhead;
        let instruments: Vec<_> = s
            .app
            .instruments
            .instruments
            .iter()
            .map(|i| (i.id, i.name.clone()))
            .collect();
        (placements, playhead, instruments)
    };

    // Calculate playhead position in pixels (assuming 0.1 pixels per tick for now)
    let pixels_per_tick = 0.1_f32;
    let playhead_x = playhead as f32 * pixels_per_tick;

    rsx! {
        div { class: "arrangement",
            // Grid background and track lanes
            div { class: "arrangement-grid",
                for (id, name) in &instruments {
                    div {
                        class: "arrangement-track",
                        "data-instrument-id": "{id}",
                        div { class: "track-label", "{name}" }
                        div { class: "track-content",
                            // Clips for this track
                            for (placement_id, inst_id, _clip_id, start, length) in &placements {
                                if inst_id == id {
                                    div {
                                        class: "clip",
                                        style: "left: {*start as f32 * pixels_per_tick}px; width: {*length as f32 * pixels_per_tick}px;",
                                        "data-placement-id": "{placement_id}",
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Playhead
            div {
                class: "playhead",
                style: "left: {playhead_x}px;"
            }
        }
    }
}
