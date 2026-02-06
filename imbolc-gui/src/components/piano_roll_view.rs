//! Piano roll view for MIDI clip editing.

use dioxus::prelude::*;

use crate::dispatch::{use_dispatch, DispatchExt};
use crate::state::SharedState;
use imbolc_types::{Action, PianoRollAction};

/// Number of visible pitch rows in the piano roll.
const VISIBLE_PITCHES: u8 = 24;
/// Default pitch at the bottom of the view.
const DEFAULT_BOTTOM_PITCH: u8 = 48; // C3
/// Pixels per tick.
const PIXELS_PER_TICK: f32 = 0.5;
/// Cell height in pixels.
const CELL_HEIGHT: u32 = 16;
/// Default note duration in ticks.
const DEFAULT_DURATION: u32 = 120;
/// Default velocity.
const DEFAULT_VELOCITY: u8 = 100;

/// Piano roll view for editing notes in a clip.
#[component]
pub fn PianoRollView(
    clip_id: u32,
    instrument_id: u32,
    clip_length: u32,
) -> Element {
    let state = use_context::<Signal<SharedState>>();
    let mut dispatch = use_dispatch();

    // View state
    let mut view_start_tick = use_signal(|| 0u32);
    let mut view_bottom_pitch = use_signal(|| DEFAULT_BOTTOM_PITCH);

    // Get clip data
    let clip_data = {
        let s = state.read();
        s.app.session.arrangement.clips.iter()
            .find(|c| c.id == clip_id)
            .map(|c| (c.notes.clone(), c.length_ticks))
    };

    let (notes, length_ticks) = match clip_data {
        Some(data) => data,
        None => return rsx! {
            div { class: "piano-roll-error",
                "Clip not found"
            }
        },
    };

    // Get playhead position
    let playhead = {
        let s = state.read();
        s.app.session.piano_roll.playhead
    };

    // Get track index for this instrument
    let track_index = {
        let s = state.read();
        s.app.session.piano_roll.track_order
            .iter()
            .position(|&id| id == instrument_id)
            .unwrap_or(0)
    };

    // Calculate dimensions
    let grid_width = (length_ticks as f32 * PIXELS_PER_TICK) as u32;
    let grid_height = VISIBLE_PITCHES as u32 * CELL_HEIGHT;
    let ticks_per_beat = 480u32;
    let pixels_per_beat = (ticks_per_beat as f32 * PIXELS_PER_TICK) as u32;

    // Build pitch labels
    let bottom_pitch = *view_bottom_pitch.read();
    let pitch_labels: Vec<(u8, String)> = (0..VISIBLE_PITCHES)
        .rev()
        .map(|i| {
            let pitch = bottom_pitch + i;
            let note_name = pitch_to_name(pitch);
            (pitch, note_name)
        })
        .collect();

    // Filter visible notes
    let visible_notes: Vec<_> = notes.iter()
        .filter(|n| {
            n.pitch >= bottom_pitch && n.pitch < bottom_pitch + VISIBLE_PITCHES
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "piano-roll",
            // Navigation controls
            div { class: "piano-roll-nav",
                button {
                    class: "piano-roll-nav-btn",
                    onclick: move |_| {
                        let current = *view_start_tick.read();
                        if current >= ticks_per_beat {
                            view_start_tick.set(current - ticks_per_beat);
                        } else {
                            view_start_tick.set(0);
                        }
                    },
                    "<"
                }
                button {
                    class: "piano-roll-nav-btn",
                    onclick: move |_| {
                        let current = *view_bottom_pitch.read();
                        if current < 127 - VISIBLE_PITCHES {
                            view_bottom_pitch.set(current + 1);
                        }
                    },
                    "^"
                }
                button {
                    class: "piano-roll-nav-btn",
                    onclick: move |_| {
                        let current = *view_bottom_pitch.read();
                        if current > 0 {
                            view_bottom_pitch.set(current - 1);
                        }
                    },
                    "v"
                }
                button {
                    class: "piano-roll-nav-btn",
                    onclick: move |_| {
                        let current = *view_start_tick.read();
                        view_start_tick.set(current + ticks_per_beat);
                    },
                    ">"
                }
                span { class: "piano-roll-info",
                    "Clip: {clip_id} | Length: {length_ticks} ticks"
                }
            }

            // Main grid container
            div { class: "piano-roll-container",
                // Pitch labels column
                div { class: "piano-roll-keys",
                    for (pitch, name) in pitch_labels.iter() {
                        div {
                            key: "{pitch}",
                            class: if is_black_key(*pitch) { "piano-roll-key black" } else { "piano-roll-key white" },
                            style: "height: {CELL_HEIGHT}px;",
                            "{name}"
                        }
                    }
                }

                // Grid area
                div {
                    class: "piano-roll-grid-scroll",
                    style: "width: calc(100% - 50px); overflow-x: auto;",
                    div {
                        class: "piano-roll-grid",
                        style: "width: {grid_width}px; height: {grid_height}px; position: relative;",

                        // Beat lines (vertical grid lines)
                        for beat in 0..((length_ticks / ticks_per_beat) + 1) {
                            {
                                let x = beat * pixels_per_beat;
                                let is_bar = beat % 4 == 0;
                                rsx! {
                                    div {
                                        key: "beat-{beat}",
                                        class: if is_bar { "piano-roll-line bar" } else { "piano-roll-line beat" },
                                        style: "left: {x}px; height: 100%;",
                                    }
                                }
                            }
                        }

                        // Pitch rows (horizontal grid lines)
                        for i in 0..VISIBLE_PITCHES {
                            {
                                let y = i as u32 * CELL_HEIGHT;
                                rsx! {
                                    div {
                                        key: "row-{i}",
                                        class: "piano-roll-row",
                                        style: "top: {y}px; width: 100%; height: {CELL_HEIGHT}px;",
                                    }
                                }
                            }
                        }

                        // Render cells for each pitch and time position
                        for pitch_offset in 0..VISIBLE_PITCHES {
                            for cell in 0..(length_ticks / DEFAULT_DURATION) {
                                {
                                    let bp = *view_bottom_pitch.read();
                                    let pitch = bp + (VISIBLE_PITCHES - 1 - pitch_offset);
                                    let tick = cell * DEFAULT_DURATION;
                                    let y = pitch_offset as u32 * CELL_HEIGHT;
                                    let x = (tick as f32 * PIXELS_PER_TICK) as u32;
                                    let width = (DEFAULT_DURATION as f32 * PIXELS_PER_TICK) as u32;

                                    rsx! {
                                        div {
                                            key: "cell-{pitch}-{tick}",
                                            class: "piano-roll-cell",
                                            style: "left: {x}px; top: {y}px; width: {width}px; height: {CELL_HEIGHT}px;",
                                            onclick: move |_| {
                                                dispatch.dispatch_action(Action::PianoRoll(PianoRollAction::ToggleNote {
                                                    pitch,
                                                    tick,
                                                    duration: DEFAULT_DURATION,
                                                    velocity: DEFAULT_VELOCITY,
                                                    track: track_index,
                                                }));
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        // Notes
                        for note in visible_notes.iter() {
                            {
                                let bp = *view_bottom_pitch.read();
                                let pitch_offset = (VISIBLE_PITCHES - 1) - (note.pitch - bp);
                                let y = pitch_offset as u32 * CELL_HEIGHT + 1;
                                let x = (note.tick as f32 * PIXELS_PER_TICK) as u32;
                                let width = (note.duration as f32 * PIXELS_PER_TICK).max(4.0) as u32;
                                let velocity_brightness = 50 + (note.velocity as u32 * 50 / 127);

                                rsx! {
                                    div {
                                        key: "note-{note.tick}-{note.pitch}",
                                        class: "piano-roll-note",
                                        style: "left: {x}px; top: {y}px; width: {width}px; height: {CELL_HEIGHT - 2}px; opacity: {velocity_brightness}%;",
                                    }
                                }
                            }
                        }

                        // Playhead
                        {
                            let playhead_x = (playhead as f32 * PIXELS_PER_TICK) as u32;
                            rsx! {
                                div {
                                    class: "piano-roll-playhead",
                                    style: "left: {playhead_x}px; height: 100%;",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Convert MIDI pitch to note name.
fn pitch_to_name(pitch: u8) -> String {
    let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (pitch / 12) as i8 - 1;
    let note = note_names[(pitch % 12) as usize];
    format!("{}{}", note, octave)
}

/// Check if a MIDI pitch is a black key.
fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}
